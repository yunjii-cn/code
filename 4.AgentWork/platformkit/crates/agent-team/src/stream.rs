// 流式事件类型（M4.0 D3）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.0 D3
//
// StreamEvent 是 Agent 执行过程中向前端推送的事件，
// 通过 Tauri Event（app_handle.emit）传递，不使用 WebSocket。
//
// 事件类型：
//   - thinking:  LLM 思考过程（流式 token）
//   - tool_call: 工具调用（MCP）
//   - progress:  进度更新（任务状态变化）
//   - error:     错误（友好提示）
//   - done:      完成（携带最终结果）

use serde::{Deserialize, Serialize};

/// 流式事件
///
/// 通过 Tauri Event 推送到前端，前端用 `listen("agent_stream", ...)` 接收。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// LLM 思考过程（流式 token，增量推送）
    Thinking {
        /// 工作流 ID
        workflow_id: String,
        /// 任务 ID（如果是任务级思考）
        task_id: Option<String>,
        /// 角色 ID（如 "orchestrator" / "backend"）
        role_id: String,
        /// 增量文本（本次推送的新 token）
        delta: String,
        /// 累计文本长度（用于前端判断进度）
        accumulated_len: usize,
    },

    /// 工具调用（MCP）
    ToolCall {
        /// 工作流 ID
        workflow_id: String,
        /// 任务 ID
        task_id: String,
        /// 角色 ID
        role_id: String,
        /// 工具名称（如 "file_read" / "shell_exec"）
        tool_name: String,
        /// 工具参数（JSON 字符串）
        arguments: String,
        /// 调用状态
        status: ToolCallStatus,
    },

    /// 进度更新（任务状态变化）
    Progress {
        /// 工作流 ID
        workflow_id: String,
        /// 任务 ID
        task_id: String,
        /// 任务标题
        task_title: String,
        /// 新状态（Pending / Running / Verifying / Merged / Failed / Rejected）
        new_state: String,
        /// 进度百分比（0-100）
        percent: u8,
    },

    /// 错误（友好提示）
    Error {
        /// 工作流 ID
        workflow_id: String,
        /// 任务 ID（可选，全局错误为 None）
        task_id: Option<String>,
        /// 错误消息（给用户看的友好提示）
        message: String,
        /// 是否可重试
        retryable: bool,
    },

    /// 完成（携带最终结果）
    Done {
        /// 工作流 ID
        workflow_id: String,
        /// 总耗时（毫秒）
        elapsed_ms: u64,
        /// 完成的任务数
        tasks_completed: usize,
        /// 总任务数
        tasks_total: usize,
        /// 最终摘要
        summary: String,
    },
}

/// 工具调用状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    /// 开始调用
    Started,
    /// 调用成功（携带结果摘要）
    Succeeded { result_summary: String },
    /// 调用失败（携带错误）
    Failed { error: String },
}

impl StreamEvent {
    /// 事件类型名（用于前端过滤）
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Thinking { .. } => "thinking",
            Self::ToolCall { .. } => "tool_call",
            Self::Progress { .. } => "progress",
            Self::Error { .. } => "error",
            Self::Done { .. } => "done",
        }
    }

    /// 工作流 ID（用于前端按工作流过滤事件）
    pub fn workflow_id(&self) -> &str {
        match self {
            Self::Thinking { workflow_id, .. }
            | Self::ToolCall { workflow_id, .. }
            | Self::Progress { workflow_id, .. }
            | Self::Error { workflow_id, .. }
            | Self::Done { workflow_id, .. } => workflow_id,
        }
    }

    /// 任务 ID（全局错误 / 完成事件可能为 None）
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::Thinking { task_id, .. } | Self::Error { task_id, .. } => task_id.as_deref(),
            Self::ToolCall { task_id, .. } | Self::Progress { task_id, .. } => Some(task_id),
            Self::Done { .. } => None,
        }
    }

    /// 创建 thinking 事件（便捷构造器）
    pub fn thinking(
        workflow_id: impl Into<String>,
        task_id: Option<String>,
        role_id: impl Into<String>,
        delta: impl Into<String>,
        accumulated_len: usize,
    ) -> Self {
        Self::Thinking {
            workflow_id: workflow_id.into(),
            task_id,
            role_id: role_id.into(),
            delta: delta.into(),
            accumulated_len,
        }
    }

    /// 创建 progress 事件（便捷构造器）
    pub fn progress(
        workflow_id: impl Into<String>,
        task_id: impl Into<String>,
        task_title: impl Into<String>,
        new_state: impl Into<String>,
        percent: u8,
    ) -> Self {
        Self::Progress {
            workflow_id: workflow_id.into(),
            task_id: task_id.into(),
            task_title: task_title.into(),
            new_state: new_state.into(),
            percent,
        }
    }

    /// 创建 error 事件（便捷构造器）
    pub fn error(
        workflow_id: impl Into<String>,
        task_id: Option<String>,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self::Error {
            workflow_id: workflow_id.into(),
            task_id,
            message: message.into(),
            retryable,
        }
    }

    /// 创建 done 事件（便捷构造器）
    pub fn done(
        workflow_id: impl Into<String>,
        elapsed_ms: u64,
        tasks_completed: usize,
        tasks_total: usize,
        summary: impl Into<String>,
    ) -> Self {
        Self::Done {
            workflow_id: workflow_id.into(),
            elapsed_ms,
            tasks_completed,
            tasks_total,
            summary: summary.into(),
        }
    }
}

/// 流式回调函数类型
///
/// Agent 执行过程中通过此回调推送事件。
/// Tauri 命令层会把此回调桥接到 `app_handle.emit`。
pub type StreamCallback = std::sync::Arc<dyn Fn(StreamEvent) + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thinking_event_serialize() {
        let event = StreamEvent::thinking(
            "wf_001",
            Some("task-1".to_string()),
            "orchestrator",
            "正在分析需求",
            10,
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"thinking\""));
        assert!(json.contains("wf_001"));
        assert!(json.contains("orchestrator"));
    }

    #[test]
    fn test_thinking_event_deserialize() {
        let json = r#"{"type":"thinking","workflow_id":"wf_001","task_id":"task-1","role_id":"orchestrator","delta":"hi","accumulated_len":2}"#;
        let event: StreamEvent = serde_json::from_str(json).unwrap();
        match event {
            StreamEvent::Thinking { workflow_id, role_id, delta, .. } => {
                assert_eq!(workflow_id, "wf_001");
                assert_eq!(role_id, "orchestrator");
                assert_eq!(delta, "hi");
            }
            _ => panic!("应为 Thinking 事件"),
        }
    }

    #[test]
    fn test_tool_call_event() {
        let event = StreamEvent::ToolCall {
            workflow_id: "wf_001".to_string(),
            task_id: "task-1".to_string(),
            role_id: "backend".to_string(),
            tool_name: "file_read".to_string(),
            arguments: r#"{"path":"src/main.rs"}"#.to_string(),
            status: ToolCallStatus::Started,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"tool_call\""));
        assert!(json.contains("file_read"));
        assert!(json.contains("started"));
    }

    #[test]
    fn test_tool_call_status_succeeded() {
        let status = ToolCallStatus::Succeeded {
            result_summary: "读取成功".to_string(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("succeeded"));
        assert!(json.contains("读取成功"));
    }

    #[test]
    fn test_progress_event() {
        let event = StreamEvent::progress("wf_001", "task-1", "编写后端", "Running", 50);
        assert_eq!(event.event_type(), "progress");
        assert_eq!(event.workflow_id(), "wf_001");
        assert_eq!(event.task_id(), Some("task-1"));
    }

    #[test]
    fn test_error_event() {
        let event = StreamEvent::error("wf_001", None, "网络超时", true);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("网络超时"));
        assert!(json.contains("\"retryable\":true"));
    }

    #[test]
    fn test_done_event() {
        let event = StreamEvent::done("wf_001", 5000, 3, 3, "全部完成");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"done\""));
        assert!(json.contains("\"elapsed_ms\":5000"));
        assert!(json.contains("\"tasks_completed\":3"));
    }

    #[test]
    fn test_event_type_names() {
        assert_eq!(
            StreamEvent::thinking("w", None, "r", "d", 1).event_type(),
            "thinking"
        );
        assert_eq!(
            StreamEvent::progress("w", "t", "title", "Running", 50).event_type(),
            "progress"
        );
        assert_eq!(
            StreamEvent::error("w", None, "msg", false).event_type(),
            "error"
        );
        assert_eq!(
            StreamEvent::done("w", 0, 0, 0, "").event_type(),
            "done"
        );
    }

    #[test]
    fn test_workflow_id_extraction() {
        let events = vec![
            StreamEvent::thinking("wf_a", None, "r", "d", 1),
            StreamEvent::progress("wf_a", "t", "title", "Running", 50),
            StreamEvent::error("wf_a", None, "msg", false),
            StreamEvent::done("wf_a", 0, 0, 0, ""),
        ];
        for e in &events {
            assert_eq!(e.workflow_id(), "wf_a");
        }
    }

    #[test]
    fn test_task_id_extraction() {
        let with_task = StreamEvent::progress("w", "task-1", "title", "Running", 50);
        assert_eq!(with_task.task_id(), Some("task-1"));

        let without_task = StreamEvent::thinking("w", None, "r", "d", 1);
        assert_eq!(without_task.task_id(), None);

        let done = StreamEvent::done("w", 0, 0, 0, "");
        assert_eq!(done.task_id(), None);
    }

    #[test]
    fn test_roundtrip_all_variants() {
        let events = vec![
            StreamEvent::thinking("wf", Some("t".to_string()), "r", "delta", 5),
            StreamEvent::ToolCall {
                workflow_id: "wf".to_string(),
                task_id: "t".to_string(),
                role_id: "r".to_string(),
                tool_name: "tool".to_string(),
                arguments: "{}".to_string(),
                status: ToolCallStatus::Failed {
                    error: "err".to_string(),
                },
            },
            StreamEvent::progress("wf", "t", "title", "Merged", 100),
            StreamEvent::error("wf", Some("t".to_string()), "msg", true),
            StreamEvent::done("wf", 100, 1, 1, "done"),
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let parsed: StreamEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.event_type(), event.event_type());
            assert_eq!(parsed.workflow_id(), event.workflow_id());
        }
    }
}
