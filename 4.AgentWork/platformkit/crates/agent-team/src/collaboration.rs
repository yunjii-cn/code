// 跨岗位协作 MVP（M4.2 D1）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.2 D1
//
// 设计理念：邮件式异步消息（不需要长连接）
//   - 员工之间通过 MessageBus 投递消息
//   - 每个员工有 inbox（收件箱），按时间顺序处理
//   - 支持跨岗位工作流模板（营销活动 = 销售 + 文案 + 客服）
//   - 全部本地处理，云端仅做日志
//
// 核心结构：
//   - AgentMessage: 消息体（from / to / subject / body / payload）
//   - MessageBus: 消息总线（投递 / 取件 / 按员工查询）
//   - WorkflowStep: 工作流步骤（员工 + 触发条件 + 动作）
//   - WorkflowTemplate: 工作流模板（步骤序列 + 触发器）
//   - WorkflowRunner: 工作流执行器（按步骤推进 + 消息派发）
//
// 验收标准：
//   - 一个销售线索可自动分配给销售员工
//   - 销售完成后通知客服员工

use crate::error::{Result, TeamError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// 消息 ID
pub type MessageId = String;

/// 员工 ID
pub type AgentId = String;

/// 消息优先级
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// 低优先级（日常通知）
    Low,
    /// 普通优先级（默认）
    Normal,
    /// 高优先级（紧急事项）
    High,
    /// 紧急（需立即处理）
    Urgent,
}

impl Default for MessagePriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl std::fmt::Display for MessagePriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Urgent => write!(f, "urgent"),
        }
    }
}

/// 消息状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageStatus {
    /// 已发送（待接收）
    Sent,
    /// 已送达（在收件箱）
    Delivered,
    /// 已读
    Read,
    /// 已处理
    Processed,
    /// 已忽略
    Ignored,
}

impl Default for MessageStatus {
    fn default() -> Self {
        Self::Sent
    }
}

/// 消息载荷（结构化数据，用于触发工作流）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessagePayload {
    /// 载荷类型（如 lead / order / complaint / handoff）
    #[serde(default)]
    pub kind: String,
    /// 业务 ID（如线索 ID / 订单 ID）
    #[serde(default)]
    pub business_id: String,
    /// 客户 ID
    #[serde(default)]
    pub customer_id: String,
    /// 自定义字段（key-value）
    #[serde(default)]
    pub fields: HashMap<String, String>,
}

impl MessagePayload {
    /// 创建空载荷
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建指定类型的载荷
    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = kind.into();
        self
    }

    /// 设置业务 ID
    pub fn with_business_id(mut self, id: impl Into<String>) -> Self {
        self.business_id = id.into();
        self
    }

    /// 设置客户 ID
    pub fn with_customer(mut self, id: impl Into<String>) -> Self {
        self.customer_id = id.into();
        self
    }

    /// 添加自定义字段
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// 获取字段
    pub fn get_field(&self, key: &str) -> Option<&String> {
        self.fields.get(key)
    }
}

/// 跨岗位消息（邮件式异步）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// 消息 ID（唯一）
    pub id: MessageId,
    /// 发送者员工 ID
    pub from: AgentId,
    /// 接收者员工 ID（"broadcast" 表示广播）
    pub to: AgentId,
    /// 主题（简短描述）
    pub subject: String,
    /// 正文（详细内容）
    pub body: String,
    /// 结构化载荷
    #[serde(default)]
    pub payload: MessagePayload,
    /// 优先级
    #[serde(default)]
    pub priority: MessagePriority,
    /// 状态
    #[serde(default)]
    pub status: MessageStatus,
    /// 发送时间
    pub sent_at: DateTime<Utc>,
    /// 关联消息 ID（如回复某条消息）
    #[serde(default)]
    pub reply_to: Option<MessageId>,
    /// 关联工作流 ID（如由工作流触发）
    #[serde(default)]
    pub workflow_id: Option<String>,
}

impl AgentMessage {
    /// 创建新消息
    pub fn new(
        from: impl Into<AgentId>,
        to: impl Into<AgentId>,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: generate_message_id(),
            from: from.into(),
            to: to.into(),
            subject: subject.into(),
            body: body.into(),
            payload: MessagePayload::default(),
            priority: MessagePriority::default(),
            status: MessageStatus::default(),
            sent_at: Utc::now(),
            reply_to: None,
            workflow_id: None,
        }
    }

    /// 设置载荷
    pub fn with_payload(mut self, payload: MessagePayload) -> Self {
        self.payload = payload;
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// 设置回复关联
    pub fn with_reply_to(mut self, reply_to: impl Into<String>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }

    /// 设置工作流关联
    pub fn with_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    /// 是否广播消息
    pub fn is_broadcast(&self) -> bool {
        self.to == "broadcast"
    }
}

/// 生成消息 ID（基于时间戳 + 计数器）
fn generate_message_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let ts = Utc::now().timestamp_millis();
    format!("msg_{ts}_{n:06}")
}

/// 消息总线（邮件式异步通信）
#[derive(Debug, Default)]
pub struct MessageBus {
    /// 全部消息（按时间顺序）
    messages: Vec<AgentMessage>,
    /// 按员工 ID 索引收件箱（员工 ID -> 消息索引列表）
    inbox_index: HashMap<AgentId, Vec<usize>>,
}

impl MessageBus {
    /// 创建空消息总线
    pub fn new() -> Self {
        Self::default()
    }

    /// 投递消息
    pub fn send(&mut self, mut msg: AgentMessage) -> Result<MessageId> {
        if msg.from.is_empty() {
            return Err(TeamError::Other("消息发送者为空".into()));
        }
        if msg.to.is_empty() {
            return Err(TeamError::Other("消息接收者为空".into()));
        }
        if msg.subject.is_empty() {
            return Err(TeamError::Other("消息主题为空".into()));
        }

        msg.status = MessageStatus::Delivered;
        let id = msg.id.clone();
        let idx = self.messages.len();

        // 广播消息：投递给所有已知员工（除发送者）
        if msg.is_broadcast() {
            // 仅标记为已送达，不索引到具体收件箱
            // 接收方通过 browse_broadcast() 查询
        } else {
            // 定向消息：索引到收件箱
            self.inbox_index
                .entry(msg.to.clone())
                .or_default()
                .push(idx);
        }

        self.messages.push(msg);
        Ok(id)
    }

    /// 获取员工收件箱（未读消息）
    pub fn inbox(&self, agent_id: &str) -> Vec<&AgentMessage> {
        let Some(indices) = self.inbox_index.get(agent_id) else {
            return vec![];
        };
        indices
            .iter()
            .filter_map(|&i| {
                let msg = self.messages.get(i)?;
                if msg.status == MessageStatus::Delivered {
                    Some(msg)
                } else {
                    None
                }
            })
            .collect()
    }

    /// 获取员工收件箱（所有消息，包括已读已处理）
    pub fn inbox_all(&self, agent_id: &str) -> Vec<&AgentMessage> {
        let Some(indices) = self.inbox_index.get(agent_id) else {
            return vec![];
        };
        indices.iter().filter_map(|&i| self.messages.get(i)).collect()
    }

    /// 获取广播消息
    pub fn broadcast_messages(&self) -> Vec<&AgentMessage> {
        self.messages.iter().filter(|m| m.is_broadcast()).collect()
    }

    /// 标记消息为已读
    pub fn mark_read(&mut self, msg_id: &str) -> Result<()> {
        let msg = self
            .messages
            .iter_mut()
            .find(|m| m.id == msg_id)
            .ok_or_else(|| TeamError::Other(format!("消息不存在: {msg_id}")))?;
        msg.status = MessageStatus::Read;
        Ok(())
    }

    /// 标记消息为已处理
    pub fn mark_processed(&mut self, msg_id: &str) -> Result<()> {
        let msg = self
            .messages
            .iter_mut()
            .find(|m| m.id == msg_id)
            .ok_or_else(|| TeamError::Other(format!("消息不存在: {msg_id}")))?;
        msg.status = MessageStatus::Processed;
        Ok(())
    }

    /// 标记消息为已忽略
    pub fn mark_ignored(&mut self, msg_id: &str) -> Result<()> {
        let msg = self
            .messages
            .iter_mut()
            .find(|m| m.id == msg_id)
            .ok_or_else(|| TeamError::Other(format!("消息不存在: {msg_id}")))?;
        msg.status = MessageStatus::Ignored;
        Ok(())
    }

    /// 按状态筛选消息
    pub fn by_status(&self, status: MessageStatus) -> Vec<&AgentMessage> {
        self.messages
            .iter()
            .filter(|m| m.status == status)
            .collect()
    }

    /// 按工作流 ID 筛选消息
    pub fn by_workflow(&self, workflow_id: &str) -> Vec<&AgentMessage> {
        self.messages
            .iter()
            .filter(|m| m.workflow_id.as_deref() == Some(workflow_id))
            .collect()
    }

    /// 获取全部消息
    pub fn all(&self) -> &[AgentMessage] {
        &self.messages
    }

    /// 消息总数
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// 清空（保留索引结构）
    pub fn clear(&mut self) {
        self.messages.clear();
        self.inbox_index.clear();
    }
}

/// 工作流步骤触发条件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepTrigger {
    /// 工作流启动时立即执行
    OnStart,
    /// 接收到指定类型的消息时执行
    OnMessage { kind: String },
    /// 上一步完成后执行
    AfterStep { step_id: String },
    /// 手动触发
    Manual,
}

/// 工作流步骤动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepAction {
    /// 动作类型（send_message / create_task / notify_human / call_tool）
    pub kind: String,
    /// 目标员工 ID
    #[serde(default)]
    pub target_agent: AgentId,
    /// 消息主题模板
    #[serde(default)]
    pub subject_template: String,
    /// 消息正文模板
    #[serde(default)]
    pub body_template: String,
    /// 优先级
    #[serde(default)]
    pub priority: MessagePriority,
}

impl StepAction {
    /// 创建发送消息动作
    pub fn send_message(target: impl Into<AgentId>) -> Self {
        Self {
            kind: "send_message".into(),
            target_agent: target.into(),
            subject_template: String::new(),
            body_template: String::new(),
            priority: MessagePriority::Normal,
        }
    }

    /// 创建通知人工动作
    pub fn notify_human(target: impl Into<AgentId>) -> Self {
        Self {
            kind: "notify_human".into(),
            target_agent: target.into(),
            subject_template: String::new(),
            body_template: String::new(),
            priority: MessagePriority::High,
        }
    }

    /// 设置主题模板
    pub fn with_subject(mut self, template: impl Into<String>) -> Self {
        self.subject_template = template.into();
        self
    }

    /// 设置正文模板
    pub fn with_body(mut self, template: impl Into<String>) -> Self {
        self.body_template = template.into();
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
}

/// 工作流步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 执行员工 ID
    pub agent: AgentId,
    /// 触发条件
    pub trigger: StepTrigger,
    /// 动作
    pub action: StepAction,
    /// 步骤描述
    #[serde(default)]
    pub description: String,
}

impl WorkflowStep {
    /// 创建新步骤
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        agent: impl Into<AgentId>,
        trigger: StepTrigger,
        action: StepAction,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            agent: agent.into(),
            trigger,
            action,
            description: String::new(),
        }
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

/// 工作流模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    /// 模板 ID
    pub id: String,
    /// 模板名称
    pub name: String,
    /// 模板描述
    #[serde(default)]
    pub description: String,
    /// 涉及的员工 ID 列表
    pub agents: Vec<AgentId>,
    /// 步骤列表
    pub steps: Vec<WorkflowStep>,
    /// 版本号
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl WorkflowTemplate {
    /// 创建新模板
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            agents: vec![],
            steps: vec![],
            version: default_version(),
        }
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 添加员工
    pub fn with_agent(mut self, agent: impl Into<AgentId>) -> Self {
        let a = agent.into();
        if !self.agents.contains(&a) {
            self.agents.push(a);
        }
        self
    }

    /// 添加步骤
    pub fn with_step(mut self, step: WorkflowStep) -> Self {
        // 自动注册步骤涉及的员工
        if !self.agents.contains(&step.agent) {
            self.agents.push(step.agent.clone());
        }
        if !self.agents.contains(&step.action.target_agent) {
            self.agents.push(step.action.target_agent.clone());
        }
        self.steps.push(step);
        self
    }

    /// 按触发条件查找步骤
    pub fn steps_by_trigger(&self, trigger_kind: &str) -> Vec<&WorkflowStep> {
        match trigger_kind {
            "on_start" => self
                .steps
                .iter()
                .filter(|s| matches!(s.trigger, StepTrigger::OnStart))
                .collect(),
            "manual" => self
                .steps
                .iter()
                .filter(|s| matches!(s.trigger, StepTrigger::Manual))
                .collect(),
            "on_message" => self
                .steps
                .iter()
                .filter(|s| matches!(s.trigger, StepTrigger::OnMessage { .. }))
                .collect(),
            "after_step" => self
                .steps
                .iter()
                .filter(|s| matches!(s.trigger, StepTrigger::AfterStep { .. }))
                .collect(),
            _ => vec![],
        }
    }

    /// YAML 序列化
    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self).map_err(|e| TeamError::Other(format!("YAML 序列化失败: {e}")))
    }

    /// 从 YAML 反序列化
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).map_err(|e| TeamError::Other(format!("YAML 反序列化失败: {e}")))
    }
}

/// 工作流执行状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkflowStatus {
    /// 已创建
    Created,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
    /// 失败
    Failed,
}

impl Default for WorkflowStatus {
    fn default() -> Self {
        Self::Created
    }
}

/// 工作流执行实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    /// 实例 ID
    pub id: String,
    /// 模板 ID
    pub template_id: String,
    /// 实例名称
    pub name: String,
    /// 状态
    pub status: WorkflowStatus,
    /// 已完成的步骤 ID
    #[serde(default)]
    pub completed_steps: Vec<String>,
    /// 当前可执行的步骤 ID
    #[serde(default)]
    pub ready_steps: Vec<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 完成时间
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    /// 上下文（业务数据，如线索 ID / 订单 ID）
    #[serde(default)]
    pub context: HashMap<String, String>,
}

impl WorkflowInstance {
    /// 创建新实例
    pub fn new(template: &WorkflowTemplate) -> Self {
        let ready: Vec<String> = template
            .steps
            .iter()
            .filter(|s| matches!(s.trigger, StepTrigger::OnStart))
            .map(|s| s.id.clone())
            .collect();

        Self {
            id: generate_workflow_id(),
            template_id: template.id.clone(),
            name: template.name.clone(),
            status: WorkflowStatus::Created,
            completed_steps: vec![],
            ready_steps: ready,
            created_at: Utc::now(),
            completed_at: None,
            context: HashMap::new(),
        }
    }

    /// 设置上下文字段
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// 标记步骤完成
    pub fn complete_step(&mut self, step_id: &str) {
        if !self.completed_steps.contains(&step_id.to_string()) {
            self.completed_steps.push(step_id.to_string());
        }
        self.ready_steps.retain(|s| s != step_id);
    }

    /// 是否完成
    pub fn is_completed(&self) -> bool {
        self.status == WorkflowStatus::Completed
    }
}

/// 生成工作流实例 ID
fn generate_workflow_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let ts = Utc::now().timestamp_millis();
    format!("wf_{ts}_{n:06}")
}

/// 工作流执行器
pub struct WorkflowRunner<'a> {
    /// 工作流模板
    pub template: &'a WorkflowTemplate,
    /// 执行实例
    pub instance: WorkflowInstance,
    /// 消息总线
    pub bus: MessageBus,
}

impl<'a> WorkflowRunner<'a> {
    /// 创建执行器
    pub fn new(template: &'a WorkflowTemplate) -> Self {
        let instance = WorkflowInstance::new(template);
        Self {
            template,
            instance,
            bus: MessageBus::new(),
        }
    }

    /// 创建执行器（带初始上下文）
    pub fn with_context(template: &'a WorkflowTemplate, context: HashMap<String, String>) -> Self {
        let mut instance = WorkflowInstance::new(template);
        instance.context = context;
        Self {
            template,
            instance,
            bus: MessageBus::new(),
        }
    }

    /// 启动工作流（执行 OnStart 步骤）
    pub fn start(&mut self) -> Result<Vec<MessageId>> {
        if self.instance.status != WorkflowStatus::Created {
            return Err(TeamError::Other("工作流已启动，不能重复启动".into()));
        }
        self.instance.status = WorkflowStatus::Running;

        let start_steps: Vec<WorkflowStep> = self
            .template
            .steps
            .iter()
            .filter(|s| matches!(s.trigger, StepTrigger::OnStart))
            .cloned()
            .collect();

        let mut ids = vec![];
        for step in start_steps {
            let msg_id = self.execute_step(&step)?;
            ids.push(msg_id);
            self.instance.complete_step(&step.id);
        }

        self.refresh_ready_steps();
        self.check_completion();
        Ok(ids)
    }

    /// 执行指定步骤（发送消息）
    fn execute_step(&mut self, step: &WorkflowStep) -> Result<MessageId> {
        let subject = render_template(&step.action.subject_template, &self.instance.context);
        let body = render_template(&step.action.body_template, &self.instance.context);

        let mut msg = AgentMessage::new(
            step.agent.clone(),
            step.action.target_agent.clone(),
            subject,
            body,
        )
        .with_priority(step.action.priority)
        .with_workflow(self.instance.id.clone());

        // 注入业务上下文到 payload
        let mut payload = MessagePayload::new();
        for (k, v) in &self.instance.context {
            payload = payload.with_field(k.clone(), v.clone());
        }
        msg.payload = payload;

        self.bus.send(msg)
    }

    /// 接收消息并触发后续步骤
    pub fn on_message(&mut self, msg: &AgentMessage) -> Result<Vec<MessageId>> {
        let mut ids = vec![];

        // 查找匹配的 OnMessage 步骤
        let matching: Vec<WorkflowStep> = self
            .template
            .steps
            .iter()
            .filter(|s| {
                if let StepTrigger::OnMessage { kind } = &s.trigger {
                    kind == &msg.payload.kind || kind == "*"
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        for step in matching {
            if self.instance.completed_steps.contains(&step.id) {
                continue;
            }
            let id = self.execute_step(&step)?;
            ids.push(id);
            self.instance.complete_step(&step.id);
        }

        self.refresh_ready_steps();
        self.check_completion();
        Ok(ids)
    }

    /// 手动执行步骤
    pub fn run_step(&mut self, step_id: &str) -> Result<MessageId> {
        let step = self
            .template
            .steps
            .iter()
            .find(|s| s.id == step_id)
            .ok_or_else(|| TeamError::Other(format!("步骤不存在: {step_id}")))?
            .clone();

        let id = self.execute_step(&step)?;
        self.instance.complete_step(&step.id);
        self.refresh_ready_steps();
        self.check_completion();
        Ok(id)
    }

    /// 刷新可执行步骤（基于 AfterStep 触发条件）
    fn refresh_ready_steps(&mut self) {
        self.instance.ready_steps.clear();

        for step in &self.template.steps {
            if self.instance.completed_steps.contains(&step.id) {
                continue;
            }
            if let StepTrigger::AfterStep { step_id: dep } = &step.trigger {
                if self.instance.completed_steps.contains(dep) {
                    self.instance.ready_steps.push(step.id.clone());
                }
            }
        }
    }

    /// 检查是否全部完成
    fn check_completion(&mut self) {
        let total = self.template.steps.len();
        let done = self.instance.completed_steps.len();
        if done >= total && total > 0 {
            self.instance.status = WorkflowStatus::Completed;
            self.instance.completed_at = Some(Utc::now());
        }
    }

    /// 取消工作流
    pub fn cancel(&mut self) {
        self.instance.status = WorkflowStatus::Cancelled;
        self.instance.completed_at = Some(Utc::now());
    }

    /// 获取执行进度（0.0 - 1.0）
    pub fn progress(&self) -> f32 {
        let total = self.template.steps.len();
        if total == 0 {
            return 1.0;
        }
        self.instance.completed_steps.len() as f32 / total as f32
    }
}

/// 简单模板渲染（替换 {{key}} 为 context 对应值）
fn render_template(template: &str, context: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (k, v) in context {
        let placeholder = format!("{{{{{k}}}}}");
        result = result.replace(&placeholder, v);
    }
    result
}

// ============================================================
// 内置工作流模板
// ============================================================

/// 内置：营销活动工作流（销售 + 文案 + 客服）
///
/// 流程：
/// 1. OnStart: 销售接收新线索 → 通知文案
/// 2. AfterStep(1): 文案撰写营销内容 → 通知客服
/// 3. AfterStep(2): 客服准备接待话术
pub fn builtin_marketing_workflow() -> WorkflowTemplate {
    WorkflowTemplate::new("marketing_campaign", "营销活动工作流")
        .with_description("销售线索 → 文案创作 → 客服接待")
        .with_step(WorkflowStep::new(
            "step1_receive_lead",
            "接收销售线索",
            "sales_agent",
            StepTrigger::OnStart,
            StepAction::send_message("copywriter_agent")
                .with_subject("新销售线索: {{customer_name}}")
                .with_body("客户 {{customer_name}} 已确认意向，请准备营销文案。线索ID: {{lead_id}}")
                .with_priority(MessagePriority::High),
        ))
        .with_step(WorkflowStep::new(
            "step2_create_content",
            "文案创作",
            "copywriter_agent",
            StepTrigger::AfterStep { step_id: "step1_receive_lead".into() },
            StepAction::send_message("customer_service_agent")
                .with_subject("营销文案已就绪: {{lead_id}}")
                .with_body("已为线索 {{lead_id}} 准备营销文案，请客服准备接待话术。")
                .with_priority(MessagePriority::Normal),
        ))
        .with_step(WorkflowStep::new(
            "step3_prepare_service",
            "客服准备",
            "customer_service_agent",
            StepTrigger::AfterStep { step_id: "step2_create_content".into() },
            StepAction::send_message("sales_agent")
                .with_subject("客服就绪: {{lead_id}}")
                .with_body("客服已为线索 {{lead_id}} 准备接待话术，可推进下一步。")
                .with_priority(MessagePriority::Normal),
        ))
}

/// 内置：售后投诉处理工作流（客服 + 主管）
///
/// 流程：
/// 1. OnMessage(complaint): 客服接收投诉 → 通知主管
/// 2. AfterStep(1): 主管决策 → 通知客服处理方案
pub fn builtin_complaint_workflow() -> WorkflowTemplate {
    WorkflowTemplate::new("complaint_handling", "售后投诉处理工作流")
        .with_description("客户投诉 → 主管决策 → 客服执行")
        .with_step(WorkflowStep::new(
            "step1_receive_complaint",
            "接收投诉",
            "customer_service_agent",
            StepTrigger::OnMessage { kind: "complaint".into() },
            StepAction::notify_human("supervisor_agent")
                .with_subject("客户投诉升级: {{customer_id}}")
                .with_body("客户 {{customer_id}} 提交投诉，需主管决策。投诉ID: {{complaint_id}}")
                .with_priority(MessagePriority::Urgent),
        ))
        .with_step(WorkflowStep::new(
            "step2_supervisor_decision",
            "主管决策",
            "supervisor_agent",
            StepTrigger::AfterStep { step_id: "step1_receive_complaint".into() },
            StepAction::send_message("customer_service_agent")
                .with_subject("投诉处理方案: {{complaint_id}}")
                .with_body("主管已决策投诉 {{complaint_id}} 的处理方案，请客服执行。")
                .with_priority(MessagePriority::High),
        ))
}

/// 内置：销售跟进工作流（销售 + 客服）
///
/// 流程：
/// 1. OnStart: 销售创建订单 → 通知客服
/// 2. AfterStep(1): 客服发送感谢消息
pub fn builtin_sales_followup_workflow() -> WorkflowTemplate {
    WorkflowTemplate::new("sales_followup", "销售跟进工作流")
        .with_description("销售成单 → 客服跟进")
        .with_step(WorkflowStep::new(
            "step1_order_created",
            "订单创建",
            "sales_agent",
            StepTrigger::OnStart,
            StepAction::send_message("customer_service_agent")
                .with_subject("新订单: {{order_id}}")
                .with_body("客户 {{customer_id}} 已下单 {{order_id}}，请客服跟进。")
                .with_priority(MessagePriority::High),
        ))
        .with_step(WorkflowStep::new(
            "step2_thank_customer",
            "感谢客户",
            "customer_service_agent",
            StepTrigger::AfterStep { step_id: "step1_order_created".into() },
            StepAction::send_message("sales_agent")
                .with_subject("已感谢客户: {{order_id}}")
                .with_body("已向客户 {{customer_id}} 发送感谢消息，订单 {{order_id}} 跟进完成。")
                .with_priority(MessagePriority::Normal),
        ))
}

/// 获取全部内置工作流模板
pub fn builtin_workflows() -> Vec<WorkflowTemplate> {
    vec![
        builtin_marketing_workflow(),
        builtin_complaint_workflow(),
        builtin_sales_followup_workflow(),
    ]
}

/// 按 ID 查找内置工作流模板
pub fn builtin_workflow_by_id(id: &str) -> Option<WorkflowTemplate> {
    builtin_workflows().into_iter().find(|w| w.id == id)
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = AgentMessage::new("agent_a", "agent_b", "主题", "正文");
        assert_eq!(msg.from, "agent_a");
        assert_eq!(msg.to, "agent_b");
        assert_eq!(msg.subject, "主题");
        assert_eq!(msg.body, "正文");
        assert_eq!(msg.priority, MessagePriority::Normal);
        assert_eq!(msg.status, MessageStatus::Sent);
        assert!(msg.id.starts_with("msg_"));
    }

    #[test]
    fn test_message_with_payload() {
        let payload = MessagePayload::new()
            .with_kind("lead")
            .with_business_id("lead_001")
            .with_customer("cust_123")
            .with_field("source", "website");
        let msg = AgentMessage::new("a", "b", "t", "b").with_payload(payload);
        assert_eq!(msg.payload.kind, "lead");
        assert_eq!(msg.payload.business_id, "lead_001");
        assert_eq!(msg.payload.customer_id, "cust_123");
        assert_eq!(msg.payload.get_field("source"), Some(&"website".to_string()));
    }

    #[test]
    fn test_message_priority_order() {
        assert!(MessagePriority::Urgent > MessagePriority::High);
        assert!(MessagePriority::High > MessagePriority::Normal);
        assert!(MessagePriority::Normal > MessagePriority::Low);
    }

    #[test]
    fn test_message_priority_display() {
        assert_eq!(MessagePriority::Low.to_string(), "low");
        assert_eq!(MessagePriority::Normal.to_string(), "normal");
        assert_eq!(MessagePriority::High.to_string(), "high");
        assert_eq!(MessagePriority::Urgent.to_string(), "urgent");
    }

    #[test]
    fn test_message_is_broadcast() {
        let normal = AgentMessage::new("a", "b", "t", "b");
        assert!(!normal.is_broadcast());

        let broadcast = AgentMessage::new("a", "broadcast", "t", "b");
        assert!(broadcast.is_broadcast());
    }

    #[test]
    fn test_message_bus_send_basic() {
        let mut bus = MessageBus::new();
        let msg = AgentMessage::new("agent_a", "agent_b", "主题", "正文");
        let id = bus.send(msg).unwrap();
        assert!(id.starts_with("msg_"));
        assert_eq!(bus.len(), 1);
    }

    #[test]
    fn test_message_bus_send_empty_from() {
        let mut bus = MessageBus::new();
        let msg = AgentMessage::new("", "agent_b", "主题", "正文");
        let result = bus.send(msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_bus_send_empty_to() {
        let mut bus = MessageBus::new();
        let msg = AgentMessage::new("agent_a", "", "主题", "正文");
        let result = bus.send(msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_bus_send_empty_subject() {
        let mut bus = MessageBus::new();
        let msg = AgentMessage::new("agent_a", "agent_b", "", "正文");
        let result = bus.send(msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_bus_inbox() {
        let mut bus = MessageBus::new();
        bus.send(AgentMessage::new("a", "b", "t1", "b1")).unwrap();
        bus.send(AgentMessage::new("a", "b", "t2", "b2")).unwrap();
        bus.send(AgentMessage::new("a", "c", "t3", "b3")).unwrap();

        let inbox_b = bus.inbox("b");
        assert_eq!(inbox_b.len(), 2);
        let inbox_c = bus.inbox("c");
        assert_eq!(inbox_c.len(), 1);
        let inbox_d = bus.inbox("d");
        assert_eq!(inbox_d.len(), 0);
    }

    #[test]
    fn test_message_bus_inbox_all() {
        let mut bus = MessageBus::new();
        let id1 = bus.send(AgentMessage::new("a", "b", "t1", "b1")).unwrap();
        let _id2 = bus.send(AgentMessage::new("a", "b", "t2", "b2")).unwrap();

        bus.mark_read(&id1).unwrap();

        // inbox 只返回未读
        let unread = bus.inbox("b");
        assert_eq!(unread.len(), 1);
        // inbox_all 返回全部
        let all = bus.inbox_all("b");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_message_bus_broadcast() {
        let mut bus = MessageBus::new();
        bus.send(AgentMessage::new("a", "broadcast", "公告", "内容")).unwrap();
        bus.send(AgentMessage::new("a", "b", "私信", "内容")).unwrap();

        let broadcasts = bus.broadcast_messages();
        assert_eq!(broadcasts.len(), 1);
        assert_eq!(broadcasts[0].subject, "公告");
    }

    #[test]
    fn test_message_bus_mark_read() {
        let mut bus = MessageBus::new();
        let id = bus.send(AgentMessage::new("a", "b", "t", "b")).unwrap();
        assert_eq!(bus.inbox("b").len(), 1);

        bus.mark_read(&id).unwrap();
        assert_eq!(bus.inbox("b").len(), 0);
        assert_eq!(bus.inbox_all("b").len(), 1);
    }

    #[test]
    fn test_message_bus_mark_processed() {
        let mut bus = MessageBus::new();
        let id = bus.send(AgentMessage::new("a", "b", "t", "b")).unwrap();

        bus.mark_processed(&id).unwrap();
        let msg = bus.all().first().unwrap();
        assert_eq!(msg.status, MessageStatus::Processed);
    }

    #[test]
    fn test_message_bus_mark_ignored() {
        let mut bus = MessageBus::new();
        let id = bus.send(AgentMessage::new("a", "b", "t", "b")).unwrap();

        bus.mark_ignored(&id).unwrap();
        let msg = bus.all().first().unwrap();
        assert_eq!(msg.status, MessageStatus::Ignored);
    }

    #[test]
    fn test_message_bus_mark_nonexistent() {
        let mut bus = MessageBus::new();
        let result = bus.mark_read("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_message_bus_by_status() {
        let mut bus = MessageBus::new();
        let id1 = bus.send(AgentMessage::new("a", "b", "t1", "b1")).unwrap();
        let _id2 = bus.send(AgentMessage::new("a", "b", "t2", "b2")).unwrap();

        bus.mark_processed(&id1).unwrap();

        let delivered = bus.by_status(MessageStatus::Delivered);
        assert_eq!(delivered.len(), 1);
        let processed = bus.by_status(MessageStatus::Processed);
        assert_eq!(processed.len(), 1);
    }

    #[test]
    fn test_message_bus_by_workflow() {
        let mut bus = MessageBus::new();
        let mut m1 = AgentMessage::new("a", "b", "t1", "b1");
        m1.workflow_id = Some("wf_001".into());
        let mut m2 = AgentMessage::new("a", "b", "t2", "b2");
        m2.workflow_id = Some("wf_002".into());
        bus.send(m1).unwrap();
        bus.send(m2).unwrap();

        let wf1 = bus.by_workflow("wf_001");
        assert_eq!(wf1.len(), 1);
    }

    #[test]
    fn test_message_bus_clear() {
        let mut bus = MessageBus::new();
        bus.send(AgentMessage::new("a", "b", "t", "b")).unwrap();
        assert!(!bus.is_empty());

        bus.clear();
        assert!(bus.is_empty());
    }

    #[test]
    fn test_step_action_send_message() {
        let action = StepAction::send_message("agent_b")
            .with_subject("主题")
            .with_body("正文")
            .with_priority(MessagePriority::High);
        assert_eq!(action.kind, "send_message");
        assert_eq!(action.target_agent, "agent_b");
        assert_eq!(action.subject_template, "主题");
        assert_eq!(action.body_template, "正文");
        assert_eq!(action.priority, MessagePriority::High);
    }

    #[test]
    fn test_step_action_notify_human() {
        let action = StepAction::notify_human("supervisor");
        assert_eq!(action.kind, "notify_human");
        assert_eq!(action.target_agent, "supervisor");
        assert_eq!(action.priority, MessagePriority::High);
    }

    #[test]
    fn test_workflow_step_creation() {
        let step = WorkflowStep::new(
            "step1",
            "第一步",
            "agent_a",
            StepTrigger::OnStart,
            StepAction::send_message("agent_b"),
        )
        .with_description("测试步骤");
        assert_eq!(step.id, "step1");
        assert_eq!(step.name, "第一步");
        assert_eq!(step.agent, "agent_a");
        assert_eq!(step.description, "测试步骤");
    }

    #[test]
    fn test_step_trigger_equality() {
        let t1 = StepTrigger::OnStart;
        let t2 = StepTrigger::OnStart;
        assert_eq!(t1, t2);

        let t3 = StepTrigger::OnMessage { kind: "lead".into() };
        let t4 = StepTrigger::OnMessage { kind: "lead".into() };
        assert_eq!(t3, t4);

        let t5 = StepTrigger::OnMessage { kind: "order".into() };
        assert_ne!(t3, t5);
    }

    #[test]
    fn test_workflow_template_creation() {
        let template = WorkflowTemplate::new("test_wf", "测试工作流")
            .with_description("测试用")
            .with_agent("agent_a")
            .with_agent("agent_b")
            .with_step(WorkflowStep::new(
                "s1",
                "步骤1",
                "agent_a",
                StepTrigger::OnStart,
                StepAction::send_message("agent_b"),
            ));

        assert_eq!(template.id, "test_wf");
        assert_eq!(template.name, "测试工作流");
        assert_eq!(template.description, "测试用");
        assert_eq!(template.agents.len(), 2);
        assert_eq!(template.steps.len(), 1);
        assert_eq!(template.version, "1.0.0");
    }

    #[test]
    fn test_workflow_template_agent_auto_register() {
        let template = WorkflowTemplate::new("wf", "工作流").with_step(WorkflowStep::new(
            "s1",
            "步骤1",
            "agent_a",
            StepTrigger::OnStart,
            StepAction::send_message("agent_b"),
        ));
        // agent_a 和 agent_b 都应自动注册
        assert!(template.agents.contains(&"agent_a".to_string()));
        assert!(template.agents.contains(&"agent_b".to_string()));
    }

    #[test]
    fn test_workflow_template_steps_by_trigger() {
        let template = builtin_marketing_workflow();
        let on_start = template.steps_by_trigger("on_start");
        assert_eq!(on_start.len(), 1);
        let after_step = template.steps_by_trigger("after_step");
        assert_eq!(after_step.len(), 2);
        let on_msg = template.steps_by_trigger("on_message");
        assert_eq!(on_msg.len(), 0);
    }

    #[test]
    fn test_workflow_template_yaml_roundtrip() {
        let template = builtin_marketing_workflow();
        let yaml = template.to_yaml().unwrap();
        let parsed = WorkflowTemplate::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.id, template.id);
        assert_eq!(parsed.name, template.name);
        assert_eq!(parsed.steps.len(), template.steps.len());
    }

    #[test]
    fn test_workflow_instance_new() {
        let template = builtin_marketing_workflow();
        let instance = WorkflowInstance::new(&template);
        assert_eq!(instance.template_id, "marketing_campaign");
        assert_eq!(instance.status, WorkflowStatus::Created);
        assert_eq!(instance.ready_steps.len(), 1); // OnStart 步骤
        assert!(instance.id.starts_with("wf_"));
    }

    #[test]
    fn test_workflow_instance_with_context() {
        let template = builtin_marketing_workflow();
        let instance = WorkflowInstance::new(&template)
            .with_context("customer_name", "张三")
            .with_context("lead_id", "lead_001");
        assert_eq!(instance.context.get("customer_name"), Some(&"张三".to_string()));
        assert_eq!(instance.context.get("lead_id"), Some(&"lead_001".to_string()));
    }

    #[test]
    fn test_workflow_instance_complete_step() {
        let template = builtin_marketing_workflow();
        let mut instance = WorkflowInstance::new(&template);
        let first_ready = instance.ready_steps[0].clone();
        instance.complete_step(&first_ready);
        assert!(instance.completed_steps.contains(&first_ready));
        assert!(!instance.ready_steps.contains(&first_ready));
    }

    #[test]
    fn test_workflow_runner_start() {
        let template = builtin_marketing_workflow();
        let mut runner = WorkflowRunner::new(&template);
        let mut context = HashMap::new();
        context.insert("customer_name".into(), "张三".into());
        context.insert("lead_id".into(), "lead_001".into());
        runner.instance.context = context;

        let ids = runner.start().unwrap();
        // OnStart 步骤只有 1 个
        assert_eq!(ids.len(), 1);
        assert_eq!(runner.instance.completed_steps.len(), 1);
        assert_eq!(runner.bus.len(), 1);
        // 验证消息内容渲染
        let msg = &runner.bus.all()[0];
        assert!(msg.subject.contains("张三"));
        assert!(msg.body.contains("lead_001"));
    }

    #[test]
    fn test_workflow_runner_start_twice_fails() {
        let template = builtin_marketing_workflow();
        let mut runner = WorkflowRunner::new(&template);
        runner.start().unwrap();
        let result = runner.start();
        assert!(result.is_err());
    }

    #[test]
    fn test_workflow_runner_full_flow() {
        let template = builtin_marketing_workflow();
        let mut runner = WorkflowRunner::new(&template);
        let mut context = HashMap::new();
        context.insert("customer_name".into(), "李四".into());
        context.insert("lead_id".into(), "lead_002".into());
        runner.instance.context = context;

        // 启动（执行 step1）
        runner.start().unwrap();
        assert!(!runner.instance.is_completed());

        // 执行 step2（依赖 step1）
        runner.run_step("step2_create_content").unwrap();
        assert!(!runner.instance.is_completed());

        // 执行 step3（依赖 step2）
        runner.run_step("step3_prepare_service").unwrap();
        assert!(runner.instance.is_completed());
        assert_eq!(runner.bus.len(), 3);
    }

    #[test]
    fn test_workflow_runner_progress() {
        let template = builtin_marketing_workflow();
        let mut runner = WorkflowRunner::new(&template);
        assert_eq!(runner.progress(), 0.0);

        runner.start().unwrap();
        assert!((runner.progress() - 0.333).abs() < 0.01 || (runner.progress() - 0.334).abs() < 0.01);

        runner.run_step("step2_create_content").unwrap();
        assert!((runner.progress() - 0.667).abs() < 0.01);

        runner.run_step("step3_prepare_service").unwrap();
        assert_eq!(runner.progress(), 1.0);
    }

    #[test]
    fn test_workflow_runner_cancel() {
        let template = builtin_marketing_workflow();
        let mut runner = WorkflowRunner::new(&template);
        runner.start().unwrap();
        runner.cancel();
        assert_eq!(runner.instance.status, WorkflowStatus::Cancelled);
        assert!(runner.instance.completed_at.is_some());
    }

    #[test]
    fn test_workflow_runner_on_message() {
        let template = builtin_complaint_workflow();
        let mut runner = WorkflowRunner::new(&template);
        let mut context = HashMap::new();
        context.insert("customer_id".into(), "cust_001".into());
        context.insert("complaint_id".into(), "comp_001".into());
        runner.instance.context = context;

        // 工作流没有 OnStart 步骤，需要通过消息触发
        // 先启动工作流
        runner.instance.status = WorkflowStatus::Running;

        // 模拟接收投诉消息
        let msg = AgentMessage::new("customer", "customer_service_agent", "投诉", "商品质量问题")
            .with_payload(
                MessagePayload::new()
                    .with_kind("complaint")
                    .with_customer("cust_001"),
            );

        let ids = runner.on_message(&msg).unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(runner.instance.completed_steps.len(), 1);
    }

    #[test]
    fn test_workflow_runner_run_nonexistent_step() {
        let template = builtin_marketing_workflow();
        let mut runner = WorkflowRunner::new(&template);
        let result = runner.run_step("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_render_template() {
        let mut ctx = HashMap::new();
        ctx.insert("name".into(), "张三".into());
        ctx.insert("id".into(), "001".into());

        let rendered = render_template("客户 {{name}} 的 ID 是 {{id}}", &ctx);
        assert_eq!(rendered, "客户 张三 的 ID 是 001");
    }

    #[test]
    fn test_render_template_no_placeholders() {
        let ctx = HashMap::new();
        let rendered = render_template("无占位符文本", &ctx);
        assert_eq!(rendered, "无占位符文本");
    }

    #[test]
    fn test_render_template_missing_key() {
        let ctx = HashMap::new();
        let rendered = render_template("客户 {{name}}", &ctx);
        // 缺失的 key 保持原样
        assert_eq!(rendered, "客户 {{name}}");
    }

    #[test]
    fn test_builtin_marketing_workflow() {
        let wf = builtin_marketing_workflow();
        assert_eq!(wf.id, "marketing_campaign");
        assert_eq!(wf.steps.len(), 3);
        assert!(wf.agents.contains(&"sales_agent".to_string()));
        assert!(wf.agents.contains(&"copywriter_agent".to_string()));
        assert!(wf.agents.contains(&"customer_service_agent".to_string()));
    }

    #[test]
    fn test_builtin_complaint_workflow() {
        let wf = builtin_complaint_workflow();
        assert_eq!(wf.id, "complaint_handling");
        assert_eq!(wf.steps.len(), 2);
        assert!(wf.agents.contains(&"customer_service_agent".to_string()));
        assert!(wf.agents.contains(&"supervisor_agent".to_string()));
    }

    #[test]
    fn test_builtin_sales_followup_workflow() {
        let wf = builtin_sales_followup_workflow();
        assert_eq!(wf.id, "sales_followup");
        assert_eq!(wf.steps.len(), 2);
    }

    #[test]
    fn test_builtin_workflows_all() {
        let wfs = builtin_workflows();
        assert_eq!(wfs.len(), 3);
    }

    #[test]
    fn test_builtin_workflow_by_id_found() {
        let wf = builtin_workflow_by_id("marketing_campaign");
        assert!(wf.is_some());
        assert_eq!(wf.unwrap().id, "marketing_campaign");
    }

    #[test]
    fn test_builtin_workflow_by_id_not_found() {
        let wf = builtin_workflow_by_id("nonexistent");
        assert!(wf.is_none());
    }

    #[test]
    fn test_message_with_reply_to() {
        let msg = AgentMessage::new("a", "b", "t", "b").with_reply_to("msg_001");
        assert_eq!(msg.reply_to, Some("msg_001".to_string()));
    }

    #[test]
    fn test_message_with_workflow() {
        let msg = AgentMessage::new("a", "b", "t", "b").with_workflow("wf_001");
        assert_eq!(msg.workflow_id, Some("wf_001".to_string()));
    }

    #[test]
    fn test_message_unique_ids() {
        let m1 = AgentMessage::new("a", "b", "t", "b");
        let m2 = AgentMessage::new("a", "b", "t", "b");
        assert_ne!(m1.id, m2.id);
    }

    #[test]
    fn test_workflow_instance_id_unique() {
        let template = builtin_marketing_workflow();
        let i1 = WorkflowInstance::new(&template);
        let i2 = WorkflowInstance::new(&template);
        assert_ne!(i1.id, i2.id);
    }

    #[test]
    fn test_workflow_runner_with_context() {
        let template = builtin_marketing_workflow();
        let mut ctx = HashMap::new();
        ctx.insert("customer_name".into(), "王五".into());
        ctx.insert("lead_id".into(), "lead_003".into());

        let mut runner = WorkflowRunner::with_context(&template, ctx);
        runner.start().unwrap();
        let msg = &runner.bus.all()[0];
        assert!(msg.subject.contains("王五"));
        assert!(msg.body.contains("lead_003"));
    }

    #[test]
    fn test_workflow_status_default() {
        let s = WorkflowStatus::default();
        assert_eq!(s, WorkflowStatus::Created);
    }

    #[test]
    fn test_message_priority_default() {
        let p = MessagePriority::default();
        assert_eq!(p, MessagePriority::Normal);
    }

    #[test]
    fn test_message_status_default() {
        let s = MessageStatus::default();
        assert_eq!(s, MessageStatus::Sent);
    }

    #[test]
    fn test_payload_get_field_missing() {
        let p = MessagePayload::new();
        assert!(p.get_field("missing").is_none());
    }

    #[test]
    fn test_message_bus_empty_inbox() {
        let bus = MessageBus::new();
        assert_eq!(bus.inbox("anyone").len(), 0);
        assert_eq!(bus.inbox_all("anyone").len(), 0);
    }

    #[test]
    fn test_message_bus_broadcast_not_in_inbox() {
        let mut bus = MessageBus::new();
        bus.send(AgentMessage::new("a", "broadcast", "t", "b")).unwrap();
        // 广播消息不应出现在任何员工的定向收件箱
        assert_eq!(bus.inbox("a").len(), 0);
        assert_eq!(bus.inbox("b").len(), 0);
        assert_eq!(bus.inbox("broadcast").len(), 0);
        // 但应出现在广播列表
        assert_eq!(bus.broadcast_messages().len(), 1);
    }

    #[test]
    fn test_complaint_workflow_full_flow() {
        let template = builtin_complaint_workflow();
        let mut ctx = HashMap::new();
        ctx.insert("customer_id".into(), "cust_001".into());
        ctx.insert("complaint_id".into(), "comp_001".into());

        let mut runner = WorkflowRunner::with_context(&template, ctx);
        runner.instance.status = WorkflowStatus::Running;

        // 触发投诉消息
        let msg = AgentMessage::new("system", "customer_service_agent", "投诉", "质量问题")
            .with_payload(MessagePayload::new().with_kind("complaint"));
        runner.on_message(&msg).unwrap();

        // 执行主管决策步骤
        runner.run_step("step2_supervisor_decision").unwrap();
        assert!(runner.instance.is_completed());
        assert_eq!(runner.bus.len(), 2);
    }

    #[test]
    fn test_sales_followup_workflow_full_flow() {
        let template = builtin_sales_followup_workflow();
        let mut ctx = HashMap::new();
        ctx.insert("order_id".into(), "ord_001".into());
        ctx.insert("customer_id".into(), "cust_001".into());

        let mut runner = WorkflowRunner::with_context(&template, ctx);
        runner.start().unwrap();
        runner.run_step("step2_thank_customer").unwrap();
        assert!(runner.instance.is_completed());
        assert_eq!(runner.bus.len(), 2);
    }

    #[test]
    fn test_workflow_runner_ready_steps_after_start() {
        let template = builtin_marketing_workflow();
        let mut runner = WorkflowRunner::new(&template);
        runner.start().unwrap();
        // 启动后 step2 应该 ready（依赖 step1 已完成）
        assert!(runner.instance.ready_steps.contains(&"step2_create_content".to_string()));
    }

    #[test]
    fn test_message_payload_chain() {
        let p = MessagePayload::new()
            .with_kind("order")
            .with_business_id("ord_123")
            .with_customer("cust_456")
            .with_field("amount", "999")
            .with_field("currency", "CNY");
        assert_eq!(p.kind, "order");
        assert_eq!(p.business_id, "ord_123");
        assert_eq!(p.customer_id, "cust_456");
        assert_eq!(p.fields.len(), 2);
    }
}
