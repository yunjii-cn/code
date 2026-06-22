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

export async function getTeamStatus(): Promise<WorkerInfo[]> {
  return invoke<WorkerInfo[]>("get_team_status");
}

export async function executeTeamTask(
  roleId: string,
  task: string
): Promise<string> {
  return invoke<string>("execute_team_task", { roleId, task });
}

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
