// Tauri 后端命令封装
// 前端通过这些函数调用 Rust 后端

import { invoke } from "@tauri-apps/api/core";

// ===== 类型定义 =====

export interface SnapshotSummary {
  id: string;
  timestamp: string;
  message: string;
  snapshot_type: "wip" | "progress" | "candidate" | "release";
  build_status: "green" | "yellow" | "red" | "unknown";
  author: string;
}

export interface FileChange {
  path: string;
  status: "added" | "modified" | "deleted";
  additions: number;
  deletions: number;
}

export interface SnapshotDetail {
  id: string;
  parent_id: string | null;
  timestamp: string;
  message: string;
  ai_summary: string | null;
  snapshot_type: "wip" | "progress" | "candidate" | "release";
  build_status: "green" | "yellow" | "red" | "unknown";
  author: string;
  files_changed: FileChange[];
}

export interface AppInfo {
  name: string;
  version: string;
  description: string;
}

export interface SystemInfo {
  os: string;
  arch: string;
  cpu_count: number;
  total_memory: number;
}

export interface BranchInfo {
  name: string;
  head: string;
  created_at: string;
  is_current: boolean;
}

export interface WatcherStatus {
  running: boolean;
  repo_path: string;
}

export interface GitStatusInfo {
  mode: "stealth" | "sync" | "release";
  initialized: boolean;
  current_branch: string | null;
  remote_url: string | null;
  last_commit: string | null;
  unpushed_commits: number;
}

export interface AiConfigInfo {
  provider: "openai" | "ollama" | "mock" | "mg";
  base_url: string;
  model: string;
  has_api_key: boolean;
  max_tokens: number;
  temperature: number;
  enabled: boolean;
}

// ===== 命令调用 =====

export async function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

export async function getSystemInfo(): Promise<SystemInfo> {
  return invoke<SystemInfo>("get_system_info");
}

/// 获取默认仓库路径
export async function getDefaultRepoPath(): Promise<string> {
  return invoke<string>("get_default_repo_path");
}

/// 在系统资源管理器中打开文件夹
export async function openFolder(path: string): Promise<void> {
  return invoke<void>("open_folder", { path });
}

/// 导出诊断日志包（返回 JSON 文件路径）
export async function exportLogs(): Promise<string> {
  return invoke<string>("export_logs");
}

// ===== TimeFlow 仓库命令 =====

export async function initRepository(path: string): Promise<string> {
  return invoke<string>("init_repository", { path });
}

export async function listSnapshots(limit?: number): Promise<SnapshotSummary[]> {
  return invoke<SnapshotSummary[]>("list_snapshots", { limit });
}

export async function getSnapshotDetail(id: string): Promise<SnapshotDetail> {
  return invoke<SnapshotDetail>("get_snapshot_detail", { id });
}

export async function createSnapshot(message?: string): Promise<string> {
  return invoke<string>("create_snapshot", { message });
}

export async function rollbackSnapshot(id: string): Promise<string> {
  return invoke<string>("rollback_snapshot", { id });
}

export async function diffSnapshots(from: string, to: string): Promise<FileChange[]> {
  return invoke<FileChange[]>("diff_snapshots", { from, to });
}

// ===== 分支命令 =====

export async function listBranches(): Promise<BranchInfo[]> {
  return invoke<BranchInfo[]>("list_branches");
}

export async function currentBranch(): Promise<string> {
  return invoke<string>("current_branch");
}

export async function createBranch(name: string, from?: string): Promise<string> {
  return invoke<string>("create_branch", { name, from });
}

export async function switchBranch(name: string): Promise<void> {
  return invoke<void>("switch_branch", { name });
}

export async function deleteBranch(name: string): Promise<void> {
  return invoke<void>("delete_branch", { name });
}

// ===== 文件监控命令 =====

export async function startWatcher(debounceSecs?: number): Promise<string> {
  return invoke<string>("start_watcher", { debounceSecs });
}

export async function stopWatcher(): Promise<void> {
  return invoke<void>("stop_watcher");
}

export async function watcherStatus(): Promise<WatcherStatus> {
  return invoke<WatcherStatus>("watcher_status");
}

// ===== Git 兼容层命令 =====

export async function getGitMode(): Promise<string> {
  return invoke<string>("get_git_mode");
}

export async function setGitMode(mode: "stealth" | "sync" | "release"): Promise<string> {
  return invoke<string>("set_git_mode", { mode });
}

export async function getGitStatus(): Promise<GitStatusInfo> {
  return invoke<GitStatusInfo>("get_git_status");
}

export async function gitPush(): Promise<string> {
  return invoke<string>("git_push");
}

export async function initGitRepo(remoteUrl?: string): Promise<string> {
  return invoke<string>("init_git_repo", { remoteUrl });
}

// ===== AI 命令（W5 M2.1）=====

export async function getAiConfig(): Promise<AiConfigInfo> {
  return invoke<AiConfigInfo>("get_ai_config");
}

export async function setAiConfig(params: {
  provider: string;
  base_url?: string;
  api_key?: string;
  model?: string;
  max_tokens?: number;
  temperature?: number;
  enabled?: boolean;
}): Promise<string> {
  return invoke<string>("set_ai_config", params);
}

export async function setAiEnabled(enabled: boolean): Promise<void> {
  return invoke<void>("set_ai_enabled", { enabled });
}

export async function generateCommitMsg(snapshotId: string): Promise<string> {
  return invoke<string>("generate_commit_msg", { snapshotId });
}

// ===== AI 语义搜索 + 候选版本（W6 M2.2）=====

export interface SearchResultInfo {
  snapshot_id: string;
  score: number;
  message: string;
  timestamp: string;
  snapshot_type: string;
}

export interface CandidateScoreInfo {
  snapshot_id: string;
  score: number;
  build_score: number;
  scale_score: number;
  diversity_score: number;
  message_score: number;
  recommended_type: string;
  reason: string;
  message: string;
  timestamp: string;
}

export async function semanticSearch(
  query: string,
  limit?: number
): Promise<SearchResultInfo[]> {
  return invoke<SearchResultInfo[]>("semantic_search", { query, limit });
}

export async function detectCandidates(): Promise<CandidateScoreInfo[]> {
  return invoke<CandidateScoreInfo[]>("detect_candidates");
}

export async function classifySnapshot(snapshotId: string): Promise<string> {
  return invoke<string>("classify_snapshot", { snapshotId });
}

// ===== 团队管理（W7 M3.1）=====

export interface TeamTemplateInfo {
  name: string;
  team_name: string;
  description: string;
  role_count: number;
  role_ids: string[];
}

export interface RoleInfo {
  id: string;
  name: string;
  description: string;
  model: string;
  model_fallback: string[];
  skills: string[];
  permissions: string[];
}

export interface ModelInfo {
  id: string;
  provider: string;
  base_url: string;
  model_name: string;
  has_api_key: boolean;
}

export interface WorkerInfo {
  role_id: string;
  role_name: string;
  state: string;
  current_model: string;
  tasks_completed: number;
  tasks_failed: number;
  last_error: string | null;
}

export async function listTeamTemplates(): Promise<TeamTemplateInfo[]> {
  return invoke<TeamTemplateInfo[]>("list_team_templates");
}

export async function loadTeamTemplate(name: string): Promise<string> {
  return invoke<string>("load_team_template", { name });
}

export async function getTeamConfig(): Promise<RoleInfo[]> {
  return invoke<RoleInfo[]>("get_team_config");
}

export async function listModels(): Promise<ModelInfo[]> {
  return invoke<ModelInfo[]>("list_models");
}

export async function registerModel(params: {
  id: string;
  provider: string;
  base_url: string;
  api_key?: string;
  model_name: string;
}): Promise<string> {
  return invoke<string>("register_model", params);
}

/// 测试已注册模型的连接（向 {base_url}/models 发 GET 请求）
export async function testModelConnection(id: string): Promise<string> {
  return invoke<string>("test_model_connection", { id });
}

export async function getTeamStatus(): Promise<WorkerInfo[]> {
  return invoke<WorkerInfo[]>("get_team_status");
}

export async function executeTeamTask(
  roleId: string,
  task: string,
  workMode?: "learn" | "teach" | "collaborate" | "auto" | "approval"
): Promise<string> {
  return invoke<string>("execute_team_task", { roleId, task, work_mode: workMode });
}

/** 工作模式信息 */
export interface WorkModeInfo {
  id: "learn" | "teach" | "collaborate" | "auto" | "approval";
  label: string;
  icon: string;
  description: string;
  userInvolvement: string;
}

export const WORK_MODES: WorkModeInfo[] = [
  {
    id: "collaborate",
    label: "协作模式",
    icon: "🤝",
    description: "与用户一起完成任务，主动给出建议，关键决策请用户拍板",
    userInvolvement: "中",
  },
  {
    id: "auto",
    label: "自动模式",
    icon: "🚀",
    description: "全自动执行任务，完成后统一汇报结果",
    userInvolvement: "极低",
  },
  {
    id: "approval",
    label: "审批模式",
    icon: "🔒",
    description: "先输出完整计划等待用户批准，每步执行前都要确认",
    userInvolvement: "极高",
  },
  {
    id: "learn",
    label: "学习模式",
    icon: "🧠",
    description: "Agent 主动观察用户操作并记录偏好，任务结束后沉淀知识",
    userInvolvement: "低",
  },
  {
    id: "teach",
    label: "教学模式",
    icon: "👨‍🏫",
    description: "用户手把手教 Agent，结束时封装为可复用 Skill",
    userInvolvement: "高",
  },
];

// ===== 工作流管理（W9 M3.3 D4）=====

/** 任务状态信息 */
export interface TaskStateInfo {
  /** 任务 ID */
  task_id: string;
  /** 状态类型（Pending / Running / Verifying / Merged / Failed / Rejected） */
  state_type: string;
  /** 状态详情（人类可读） */
  state_detail: string;
  /** 分支 */
  branch: string | null;
  /** Agent 角色 */
  agent: string | null;
  /** 错误信息 */
  error: string | null;
  /** 重试次数 */
  retry_count: number;
  /** 快照 ID */
  snapshot_id: string | null;
}

/** 任务信息 */
export interface TaskInfo {
  id: string;
  title: string;
  assignee: string;
  dependencies: string[];
  estimated_effort: string;
  acceptance_criteria: string;
  description: string;
  state: TaskStateInfo;
}

/** 调度动作日志条目 */
export interface ActionLogEntry {
  timestamp: string;
  action_type: string;
  task_id: string;
  description: string;
}

/** 工作流概览 */
export interface WorkflowOverview {
  workflow_id: string | null;
  requirement: string | null;
  tasks: TaskInfo[];
  completed_count: number;
  total_count: number;
  progress_percent: number;
  is_finished: boolean;
  action_log: ActionLogEntry[];
}

/** 调度动作信息 */
export interface ScheduleActionInfo {
  action_type: string;
  task_id: string;
  branch: string | null;
  assignee: string | null;
  errors: string[];
  reason: string | null;
  task_title: string;
}

export async function planWorkflow(
  requirement: string
): Promise<WorkflowOverview> {
  return invoke<WorkflowOverview>("plan_workflow", { requirement });
}

export async function getCurrentWorkflow(): Promise<WorkflowOverview> {
  return invoke<WorkflowOverview>("get_current_workflow");
}

export async function stepWorkflow(): Promise<
  [WorkflowOverview, ScheduleActionInfo[]]
> {
  return invoke<[WorkflowOverview, ScheduleActionInfo[]]>("step_workflow");
}

export async function reportTaskCompleted(
  taskId: string
): Promise<WorkflowOverview> {
  return invoke<WorkflowOverview>("report_task_completed", { taskId });
}

export async function reportTaskMerged(
  taskId: string,
  snapshotId: string
): Promise<WorkflowOverview> {
  return invoke<WorkflowOverview>("report_task_merged", { taskId, snapshotId });
}

export async function reportTaskVerifyFailed(
  taskId: string,
  errors: string[]
): Promise<[WorkflowOverview, ScheduleActionInfo[]]> {
  return invoke<[WorkflowOverview, ScheduleActionInfo[]]>(
    "report_task_verify_failed",
    { taskId, errors }
  );
}

export async function reportTaskFailed(
  taskId: string,
  error: string
): Promise<WorkflowOverview> {
  return invoke<WorkflowOverview>("report_task_failed", { taskId, error });
}

export async function resetWorkflow(): Promise<string> {
  return invoke<string>("reset_workflow");
}

// ===== 流式输出（M4.0 D3）=====

/** 流式事件类型 */
export type StreamEvent =
  | {
      type: "thinking";
      workflow_id: string;
      task_id: string | null;
      role_id: string;
      delta: string;
      accumulated_len: number;
    }
  | {
      type: "tool_call";
      workflow_id: string;
      task_id: string;
      role_id: string;
      tool_name: string;
      arguments: string;
      status: "started" | { succeeded: { result_summary: string } } | { failed: { error: string } };
    }
  | {
      type: "progress";
      workflow_id: string;
      task_id: string;
      task_title: string;
      new_state: string;
      percent: number;
    }
  | {
      type: "error";
      workflow_id: string;
      task_id: string | null;
      message: string;
      retryable: boolean;
    }
  | {
      type: "done";
      workflow_id: string;
      elapsed_ms: number;
      tasks_completed: number;
      tasks_total: number;
      summary: string;
    };

/** 流式推送 Agent 思考过程 */
export async function streamAgentThinking(requirement: string): Promise<string> {
  return invoke<string>("stream_agent_thinking", { requirement });
}

/** 流式推送任务执行进度 */
export async function streamTaskProgress(
  taskId: string,
  taskTitle: string
): Promise<string> {
  return invoke<string>("stream_task_progress", { taskId, taskTitle });
}

// ===== M6.2 进化仪表盘 + 无代码工具构建器 =====

/** 进化统计数据 */
export interface EvolutionStats {
  evolution_index: number;
  memory_count: number;
  session_count: number;
  skill_mastery: number;
  success_rate: number;
  active_employees: number;
}

/** 工具定义（前端） */
export interface ToolDefinitionInput {
  name: string;
  description: string;
  parameters: {
    name: string;
    type: "string" | "number" | "boolean";
    description: string;
    required: boolean;
  }[];
  template: string;
}

/** 工具预览执行结果 */
export interface ToolPreviewResult {
  success: boolean;
  output: string;
}

export async function getEvolutionStats(): Promise<EvolutionStats> {
  return invoke<EvolutionStats>("get_evolution_stats");
}

export async function previewToolPrompt(tool: ToolDefinitionInput): Promise<string> {
  return invoke<string>("preview_tool_prompt", { tool });
}

export async function executeToolPreview(
  tool: ToolDefinitionInput,
  values: Record<string, string>
): Promise<ToolPreviewResult> {
  return invoke<ToolPreviewResult>("execute_tool_preview", { tool, values });
}

// ===== M6.2 记忆管理 + 进化包 =====

/** 记忆条目 */
export interface MemoryEntryInfo {
  id: string;
  scope_id: string;
  layer: string;
  kind: string;
  tags: string[];
  content: string;
  priority: number;
}

/** 技能信息 */
export interface SkillInfo {
  id: string;
  name: string;
  tags: string[];
  description: string;
  success_count: number;
  failure_count: number;
  success_rate: number;
}

/** 进化报告 */
export interface EvolutionReport {
  employee_id: string;
  memory_count: number;
  session_count: number;
  skill_count: number;
  success_rate: number;
  memories: MemoryEntryInfo[];
  skills: SkillInfo[];
}

/** 工具数据源类型 */
export interface ToolDataSourceHttp {
  type: "http_api";
  url: string;
  method: string;
  headers?: Record<string, string>;
}

export interface ToolDataSourceFile {
  type: "file";
  path: string;
  format: string;
}

export type ToolDataSource = ToolDataSourceHttp | ToolDataSourceFile | { type: "mock" };

/** 完整工具定义（含数据源） */
export interface ToolDefinitionFull {
  name: string;
  description: string;
  parameters: {
    name: string;
    type: "string" | "number" | "boolean";
    description: string;
    required: boolean;
  }[];
  template: string;
  data_source?: ToolDataSource;
}

export async function listMemories(employeeId: string): Promise<MemoryEntryInfo[]> {
  return invoke<MemoryEntryInfo[]>("list_memories", { employeeId });
}

export async function deleteMemory(memoryId: string): Promise<boolean> {
  return invoke<boolean>("delete_memory", { memoryId });
}

export async function addMemory(
  employeeId: string,
  content: string,
  kind: string,
  priority?: number
): Promise<MemoryEntryInfo> {
  return invoke<MemoryEntryInfo>("add_memory", { employeeId, content, kind, priority });
}

export async function getEvolutionReport(employeeId: string): Promise<EvolutionReport> {
  return invoke<EvolutionReport>("get_evolution_report", { employeeId });
}

export async function exportEvolutionPack(employeeId: string): Promise<string> {
  return invoke<string>("export_evolution_pack", { employeeId });
}

export async function importEvolutionPack(packJson: string): Promise<string> {
  return invoke<string>("import_evolution_pack", { packJson });
}

export async function executeToolFull(
  tool: ToolDefinitionFull,
  values: Record<string, string>
): Promise<ToolPreviewResult> {
  return invoke<ToolPreviewResult>("execute_tool_full", { tool, values });
}
