import axios from 'axios'

const api = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || '/api',
  timeout: 30000,
})

api.interceptors.response.use(
  (response) => response.data,
  (error) => {
    const msg = error.response?.data?.message || error.response?.data?.detail || error.message || '请求失败'
    return Promise.reject(new Error(msg))
  },
)

export const aiApi = {
  chat: (data: {
    prompt: string
    provider?: string
    model?: string
    session_id?: string
    workspace_path?: string
    ai_language?: string
    ai_temperature?: number
    ai_max_tokens?: number
    system_prompt?: string
    auto_approve?: boolean
    // 2026-06-08 TASK-2.3 引入：图片附件（base64 数据 URL 列表）
    images?: string[]
  }) => api.post('/ai/chat', data),

  stop: (sessionId?: string) =>
    api.post('/ai/chat/stop', { session_id: sessionId }),

  getModels: (provider: string, params?: { base_url?: string; api_key?: string; check_health?: boolean }) =>
    api.get(`/ai/models/${provider}`, { params }),

  getProviders: () =>
    api.get('/ai/providers'),

  checkProvider: (data: { provider: string; base_url?: string; api_key?: string }) =>
    api.post('/ai/providers/check', data),

  getOllamaCapabilities: (modelName: string, baseUrl?: string) =>
    api.get('/ai/ollama/capabilities', { params: { model_name: modelName, base_url: baseUrl } }),

  checkOllamaHealth: (data: { model_name: string; base_url?: string; timeout_ms?: number }) =>
    api.post('/ai/ollama/health', data),

  // 2026-06-08 TASK-2.4 引入：Composer Autocomplete
  searchFiles: (params: { workspace_path?: string; query?: string; limit?: number } = {}) =>
    api.get('/ai/files/search', { params }),

  listSlashCommands: () =>
    api.get('/ai/slash-commands'),

  // 2026-06-08 TASK-2.5 引入：语音转录（Whisper 云端 API）
  transcribe: (formData: FormData) =>
    api.post('/ai/transcribe', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    }),
}

export const envApi = {
  check: () =>
    api.get('/env/check'),

  getStatus: () =>
    api.get('/env/status'),

  installComponent: (component: string) =>
    api.post('/env/install', null, { params: { component } }),

  installAll: () =>
    api.post('/env/install-all'),

  getMirror: () =>
    api.get('/env/mirror'),

  setMirror: (mirrorKey: string) =>
    api.post('/env/mirror', { mirror_key: mirrorKey }),

  listServices: () =>
    api.get('/env/services'),

  getServiceInfo: (serviceName: string) =>
    api.get(`/env/services/${serviceName}`),

  startService: (data: { service_name: string; project_dir?: string; port?: number; admin_key?: string }) =>
    api.post('/env/services/start', data),

  stopService: (data: { service_name: string; base_url?: string }) =>
    api.post('/env/services/stop', data),

  startAllServices: () =>
    api.post('/env/services/start-all'),
}

export const projectApi = {
  list: () =>
    api.get('/project/list'),

  create: (data: { name: string; workspace_path?: string }) =>
    api.post('/project/create', data),

  switch: (projectId: string) =>
    api.post(`/project/switch/${projectId}`),

  rename: (projectId: string, newName: string) =>
    api.post(`/project/rename/${projectId}`, null, { params: { new_name: newName } }),

  update: (projectId: string, data: { name?: string; workspace_path?: string }) =>
    api.post(`/project/update/${projectId}`, data),

  delete: (projectId: string) =>
    api.delete(`/project/${projectId}`),

  getActive: () =>
    api.get('/project/active'),

  getDefaultPath: (projectId: string, name: string) =>
    api.get(`/project/${projectId}/default-path`, { params: { name } }),

  getConversations: (projectId: string) =>
    api.get(`/project/${projectId}/conversations`),

  loadConversation: (projectId: string, sessionId: string) =>
    api.get(`/project/${projectId}/conversations/${sessionId}`),

  saveConversation: (data: { project_id: string; session_id: string; messages: unknown[]; title?: string }) =>
    api.post('/project/conversations/save', data),

  copyConversation: (data: { source_project_id: string; session_id: string; target_project_id: string }) =>
    api.post('/project/conversations/copy', data),

  renameConversation: (data: { project_id: string; session_id: string; new_title: string }) =>
    api.post('/project/conversations/rename', data),

  deleteConversation: (projectId: string, sessionId: string) =>
    api.delete(`/project/${projectId}/conversations/${sessionId}`),

  searchConversations: (projectId: string, keyword: string) =>
    api.get(`/project/${projectId}/conversations/search`, { params: { keyword } }),

  getContext: (projectId: string) =>
    api.get(`/project/${projectId}/context`),

  getClaudeMd: (projectId: string) =>
    api.get(`/project/${projectId}/claude-md`),

  saveClaudeMd: (data: { project_id: string; content: string }) =>
    api.post('/project/claude-md/save', data),

  getGlobalClaudeMd: () =>
    api.get('/project/claude-md/global'),

  saveGlobalClaudeMd: (content: string) =>
    api.post('/project/claude-md/global', null, { params: { content } }),

  listMemories: (projectId: string) =>
    api.get(`/project/${projectId}/memories`),

  saveMemory: (data: { project_id: string; filename: string; content: string; mem_type?: string }) =>
    api.post('/project/memories/save', data),

  deleteMemory: (projectId: string, filename: string) =>
    api.delete(`/project/${projectId}/memories/${filename}`),

  searchMemories: (projectId: string, keyword: string) =>
    api.get(`/project/${projectId}/memories/search`, { params: { keyword } }),

  getMemoryStats: (projectId: string) =>
    api.get(`/project/${projectId}/memories/stats`),

  getRelevantMemories: (projectId: string, query: string) =>
    api.get(`/project/${projectId}/memories/relevant`, { params: { query } }),

  listTemplates: (category?: string) =>
    api.get('/project/templates', { params: { category: category || 'all' } }),

  saveCustomTemplate: (data: { name: string; category: string; desc?: string; prompt: string; files?: string }) =>
    api.post('/project/templates/custom', data),

  deleteCustomTemplate: (templateId: string) =>
    api.delete(`/project/templates/custom/${templateId}`),

  createFromTemplate: (data: { project_id: string; template_id: string }) =>
    api.post('/project/create-from-template', data),
}

export const systemApi = {
  getState: () =>
    api.get('/system/state'),

  getSettings: () =>
    api.get('/system/settings'),

  saveSettings: (settings: Record<string, string>) =>
    api.post('/system/settings', { settings }),

  clearModelSettings: () =>
    api.post('/system/settings/clear-model'),

  getHardware: () =>
    api.get('/system/hardware'),

  getWorkspace: () =>
    api.get('/system/workspace'),

  chooseWorkspace: () =>
    api.post('/system/workspace/choose'),

  runCommand: (data: { cmd: string; cwd?: string }) =>
    api.post('/system/terminal/run', data),

  loadAppData: () =>
    api.get('/system/app-data'),

  saveAppData: (data: string) =>
    api.post('/system/app-data', { data }),

  getGitStatus: (projectPath: string) =>
    api.get('/system/git/status', { params: { project_path: projectPath } }),

  gitCommit: (projectPath: string, message: string) =>
    api.post('/system/git/commit', null, { params: { project_path: projectPath, message } }),

  getGitLog: (projectPath: string) =>
    api.get('/system/git/log', { params: { project_path: projectPath } }),

  // 2026-06-08 TASK-2.2 引入：DiffView 增强
  getFileDiff: (projectPath: string, filePath: string, staged: boolean = false, contextLines: number = 3) =>
    api.get('/system/git/file/diff', {
      params: { project_path: projectPath, file_path: filePath, staged, context_lines: contextLines },
    }),

  restoreFile: (projectPath: string, filePath: string, staged: boolean = false) =>
    api.post('/system/git/file/restore', {
      project_path: projectPath,
      file_path: filePath,
      staged,
    }),

  getFileTree: (projectPath: string) =>
    api.get('/system/file-tree', { params: { project_path: projectPath } }),

  showNotification: (title: string, body: string) =>
    api.post('/system/notification', null, { params: { title, body } }),

  openUrl: (url: string) =>
    api.post('/system/open-url', null, { params: { url } }),

  openExplorer: (path: string) =>
    api.post('/system/open-explorer', null, { params: { path } }),

  selectDirectory: () =>
    api.post('/system/select-directory'),

  listPlugins: () =>
    api.get('/system/plugins'),

  installPlugin: (pluginJson: string) =>
    api.post('/system/plugins/install', null, { params: { plugin_json: pluginJson } }),

  uninstallPlugin: (pluginId: string) =>
    api.delete(`/system/plugins/${pluginId}`),

  executePlugin: (pluginId: string, inputData?: string) =>
    api.post(`/system/plugins/${pluginId}/execute`, null, { params: { input_data: inputData || '' } }),

  getOfflineModels: () =>
    api.get('/system/models/offline'),

  downloadModel: (url: string) =>
    api.post('/system/models/download', null, { params: { url } }),

  deleteModel: (name: string) =>
    api.post('/system/models/delete', null, { params: { name } }),

  searchOllamaLibrary: (query?: string) =>
    api.get('/system/models/ollama-search', { params: { query: query || '' } }),

  pullModel: (name: string) =>
    api.post('/system/models/pull', null, { params: { name } }),

  recommendModels: () =>
    api.get('/system/models/recommend'),

  getQwenPoolStatus: () =>
    api.get('/qwen/api/admin/status', { headers: { Authorization: 'Bearer admin' } }),

  getQwenAccounts: () =>
    api.get('/qwen/api/admin/accounts', { headers: { Authorization: 'Bearer admin' } }),

  registerQwenAccount: () =>
    api.post('/qwen/api/admin/accounts/register', {}, { headers: { Authorization: 'Bearer admin' } }),

  removeQwenAccount: (email: string) =>
    api.delete(`/qwen/api/admin/accounts/${encodeURIComponent(email)}`, { headers: { Authorization: 'Bearer admin' } }),

  checkQwenHealth: () =>
    api.get('/qwen/healthz'),

  getZhipuKeys: () =>
    api.get('/zhipu/api/admin/accounts', { headers: { Authorization: 'Bearer admin' } }),

  addZhipuKey: (apiKey: string, label?: string) =>
    api.post('/zhipu/api/admin/accounts', { api_key: apiKey, label: label || '' }, { headers: { Authorization: 'Bearer admin' } }),

  removeZhipuKey: (apiKey: string) =>
    api.delete(`/zhipu/api/admin/accounts/${encodeURIComponent(apiKey)}`, { headers: { Authorization: 'Bearer admin' } }),

  checkZhipuHealth: () =>
    api.get('/zhipu/healthz'),
}

export const versionApi = {
  getCurrent: () =>
    api.get('/version/current'),

  getHistory: () =>
    api.get('/version/history'),

  getRemote: () =>
    api.get('/version/remote'),

  checkUpdate: () =>
    api.get('/version/check-update'),

  pullUpdate: () =>
    api.post('/version/pull-update'),

  getCommits: (limit?: number) =>
    api.get('/version/commits', { params: { limit: limit || 30 } }),

  getGitHistory: (limit?: number) =>
    api.get('/version/git-history', { params: { limit: limit || 20 } }),

  switchCommit: (commitHash: string) =>
    api.post('/version/switch-commit', { commit_hash: commitHash }),

  switchExe: (data: { exe_path: string; git_commit?: string }) =>
    api.post('/version/switch-exe', data),

  listStableExes: () =>
    api.get('/version/stable-exes'),

  downloadUpdate: (source?: string) =>
    api.post('/version/download-update', null, { params: { source: source || 'gitee' } }),
}

// 2026-06-08 TASK-2.1 引入：GitHub 集成 API 客户端
export const githubApi = {
  status: () =>
    api.get('/github/status'),

  repo: (projectPath?: string) =>
    api.get('/github/repo', { params: { project_path: projectPath || '' } }),

  listIssues: (params: { project_path?: string; state?: string; assignee?: string; author?: string; limit?: number } = {}) =>
    api.get('/github/issues', { params }),

  getIssue: (issueNumber: number, projectPath?: string) =>
    api.get(`/github/issues/${issueNumber}`, { params: { project_path: projectPath || '' } }),

  listPrs: (params: { project_path?: string; state?: string; search?: string; limit?: number } = {}) =>
    api.get('/github/prs', { params }),

  getPr: (prNumber: number, projectPath?: string) =>
    api.get(`/github/prs/${prNumber}`, { params: { project_path: projectPath || '' } }),

  getPrDiff: (prNumber: number, projectPath?: string) =>
    api.get(`/github/prs/${prNumber}/diff`, { params: { project_path: projectPath || '' } }),

  getPrFiles: (prNumber: number, projectPath?: string) =>
    api.get(`/github/prs/${prNumber}/files`, { params: { project_path: projectPath || '' } }),

  listPrComments: (prNumber: number, projectPath?: string) =>
    api.get(`/github/prs/${prNumber}/comments`, { params: { project_path: projectPath || '' } }),

  createPrComment: (prNumber: number, body: string, projectPath?: string) =>
    api.post(`/github/prs/${prNumber}/comments`, { body }, { params: { project_path: projectPath || '' } }),

  open: (target: string, projectPath?: string) =>
    api.post('/github/open', { target }, { params: { project_path: projectPath || '' } }),
}

// 2026-06-09 TASK-3.3 引入：知识管理 API 客户端
export interface KnowledgeItem {
  id: string
  content: string
  layer: 'L1' | 'L2' | 'L3' | 'L4'
  strength: 'weak' | 'medium' | 'strong'
  scope: 'project' | 'global'
  confirm_count: number
  source?: string | null
  created_at: number
  updated_at: number
}

// 2026-06-10 TASK-2.2 引入：Git DiffView 增强 API 客户端
export interface DiffLine {
  type: 'add' | 'remove' | 'context'
  text: string
  oldLineNo: number | null
  newLineNo: number | null
}

export interface DiffHunk {
  oldStart: number
  oldLines: number
  newStart: number
  newLines: number
  header: string
  lines: DiffLine[]
}

export interface DiffStats {
  additions: number
  deletions: number
}

export interface FileDiffData {
  file: string
  raw: string
  hunks: DiffHunk[]
  stats: DiffStats
}

export const gitApi = {
  getFileDiff: (params: {
    project_path: string
    file_path: string
    staged?: boolean
    context_lines?: number
  }) => api.get('/git/file/diff', { params }),

  restoreFile: (data: { project_path: string; file_path: string; staged?: boolean }) =>
    api.post('/git/file/restore', data),
}

export interface KnowledgeStats {
  total: number
  by_layer: Record<string, number>
  by_strength: Record<string, number>
  by_scope: Record<string, number>
}

export const knowledgeApi = {
  list: (params: { workspace_path?: string; layer?: string; strength?: string; scope?: string } = {}) =>
    api.get('/knowledge/list', { params }),

  stats: (workspacePath?: string) =>
    api.get('/knowledge/stats', { params: { workspace_path: workspacePath || '' } }),

  get: (id: string, workspacePath?: string) =>
    api.get(`/knowledge/${id}`, { params: { workspace_path: workspacePath || '' } }),

  add: (data: {
    content: string
    layer: string
    scope?: string
    strength?: string
    source?: string
  }, workspacePath?: string) =>
    api.post('/knowledge', data, { params: { workspace_path: workspacePath || '' } }),

  remove: (id: string, workspacePath?: string) =>
    api.delete(`/knowledge/${id}`, { params: { workspace_path: workspacePath || '' } }),

  confirm: (id: string, forceStrong: boolean = false, workspacePath?: string) =>
    api.post(`/knowledge/${id}/confirm`, { force_strong: forceStrong }, { params: { workspace_path: workspacePath || '' } }),

  deny: (id: string, workspacePath?: string) =>
    api.post(`/knowledge/${id}/deny`, {}, { params: { workspace_path: workspacePath || '' } }),

  promote: (id: string, workspacePath?: string) =>
    api.post(`/knowledge/${id}/promote`, {}, { params: { workspace_path: workspacePath || '' } }),

  relevant: (context: string, topK: number = 5, workspacePath?: string) =>
    api.post('/knowledge/relevant', { context, top_k: topK }, { params: { workspace_path: workspacePath || '' } }),
}

// 2026-06-09 TASK-3.6 引入：主动感知 API 客户端
export interface ResponsiveAction {
  label: string
  action: string
  params: Record<string, unknown>
}

export interface ResponsiveNotification {
  id: string
  type: 'file_change' | 'code_quality' | 'security_risk' | 'progress'
  severity: 'info' | 'warning' | 'error'
  title: string
  description: string
  file_path?: string | null
  actions: ResponsiveAction[]
  created_at: number
  created_at_iso: string
  dismissed: boolean
  source: string
}

export interface ResponsiveWatcherStatus {
  last_scan_at: number | null
}

export interface ResponsiveEngineStatus {
  running: boolean
  watchers: Record<string, ResponsiveWatcherStatus>
}

export const responsiveApi = {
  status: (workspacePath?: string) =>
    api.get('/responsive/status', { params: { workspace_path: workspacePath || '' } }),

  start: (workspacePath?: string) =>
    api.post('/responsive/start', {}, { params: { workspace_path: workspacePath || '' } }),

  stop: (workspacePath?: string) =>
    api.post('/responsive/stop', {}, { params: { workspace_path: workspacePath || '' } }),

  scan: (scanType: string = 'all', workspacePath?: string) =>
    api.post('/responsive/scan', { scan_type: scanType }, { params: { workspace_path: workspacePath || '' } }),

  notifications: (params: { workspace_path?: string; type?: string; severity?: string; limit?: number } = {}) =>
    api.get('/responsive/notifications', { params: { ...params, workspace_path: params.workspace_path || '' } }),

  dismiss: (id: string, workspacePath?: string) =>
    api.post(`/responsive/notifications/${id}/dismiss`, {}, { params: { workspace_path: workspacePath || '' } }),
}

// 2026-06-09 TASK-3.8 引入：独行模式 Code Agent API 客户端
export interface AgentPlanStep {
  id: string
  description: string
  status: 'pending' | 'approved' | 'rejected' | 'executing' | 'done' | 'failed' | 'skipped'
  tool_calls: Array<{ name: string; args: Record<string, unknown>; result: string; ts: number }>
  result: string | null
  reject_reason: string | null
  approved_at: number | null
  executed_at: number | null
}

export interface AgentDiffEntry {
  step_id: string
  file_path: string
  added_lines: number
  removed_lines: number
  summary: string
}

export interface AgentLearningEntry {
  layer: 'L1' | 'L2' | 'L3' | 'L4'
  content: string
  step_id: string | null
  source: string
}

export interface AgentSession {
  id: string
  requirement: string
  status: 'draft' | 'planned' | 'approved' | 'executing' | 'done' | 'failed' | 'learning' | 'closed'
  plan: AgentPlanStep[]
  diffs: AgentDiffEntry[]
  learnings: AgentLearningEntry[]
  created_at: number
  updated_at: number
  started_at: number | null
  completed_at: number | null
}

export interface AgentStats {
  total_sessions: number
  by_status: Record<string, number>
  total_learnings: number
  total_diffs: number
}

export const agentApi = {
  sessions: (params: { workspace_path?: string; status?: string } = {}) =>
    api.get('/agent/sessions', { params: { ...params, workspace_path: params.workspace_path || '' } }),

  session: (id: string, workspacePath?: string) =>
    api.get(`/agent/sessions/${id}`, { params: { workspace_path: workspacePath || '' } }),

  create: (requirement: string, workspacePath?: string) =>
    api.post('/agent/sessions', { requirement }, { params: { workspace_path: workspacePath || '' } }),

  remove: (id: string, workspacePath?: string) =>
    api.delete(`/agent/sessions/${id}`, { params: { workspace_path: workspacePath || '' } }),

  setPlan: (id: string, text: string, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/plan`, { text }, { params: { workspace_path: workspacePath || '' } }),

  appendStep: (id: string, description: string, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/append-step`, { description }, { params: { workspace_path: workspacePath || '' } }),

  approveStep: (id: string, stepId: string, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/steps/${stepId}/approve`, {}, { params: { workspace_path: workspacePath || '' } }),

  rejectStep: (id: string, stepId: string, reason: string, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/steps/${stepId}/reject`, { reason }, { params: { workspace_path: workspacePath || '' } }),

  startStep: (id: string, stepId: string, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/steps/${stepId}/start`, {}, { params: { workspace_path: workspacePath || '' } }),

  completeStep: (id: string, stepId: string, resultSummary: string, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/steps/${stepId}/complete`, { result_summary: resultSummary }, { params: { workspace_path: workspacePath || '' } }),

  failStep: (id: string, stepId: string, error: string, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/steps/${stepId}/fail`, { error }, { params: { workspace_path: workspacePath || '' } }),

  recordTool: (id: string, stepId: string, data: { name: string; args: Record<string, unknown>; result: string }, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/steps/${stepId}/tool`, data, { params: { workspace_path: workspacePath || '' } }),

  recordDiff: (id: string, data: { step_id: string; file_path: string; added_lines?: number; removed_lines?: number; summary?: string }, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/diff`, data, { params: { workspace_path: workspacePath || '' } }),

  addLearning: (id: string, data: { layer: string; content: string; step_id?: string }, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/learnings`, data, { params: { workspace_path: workspacePath || '' } }),

  close: (id: string, workspacePath?: string) =>
    api.post(`/agent/sessions/${id}/close`, {}, { params: { workspace_path: workspacePath || '' } }),

  stats: (workspacePath?: string) =>
    api.get('/agent/stats', { params: { workspace_path: workspacePath || '' } }),
}

// 2026-06-09 TASK-3.9 引入：团队模式 Team Workflow API 客户端
export interface TeamTask {
  id: string
  title: string
  description: string
  role: 'coordinator' | 'architect' | 'developer' | 'tester' | 'documenter'
  dependencies: string[]
  status: 'pending' | 'in_progress' | 'awaiting_approval' | 'approved' | 'rejected' | 'done' | 'failed' | 'skipped'
  assigned_agent: string | null
  output: string | null
  error: string | null
  file_paths: string[]
  created_at: number
  updated_at: number
  started_at: number | null
  completed_at: number | null
}

export interface TeamApproval {
  id: string
  task_id: string
  approver_role: 'coordinator' | 'architect' | 'developer' | 'tester' | 'documenter'
  reason: string
  status: 'pending' | 'approved' | 'rejected'
  decision_comment: string | null
  requested_at: number
  decided_at: number | null
}

export interface TeamSharedContext {
  key: string
  value: string
  source: 'coordinator' | 'architect' | 'developer' | 'tester' | 'documenter'
  updated_at: number
}

export interface TeamMessage {
  id: string
  role: 'coordinator' | 'architect' | 'developer' | 'tester' | 'documenter'
  content: string
  type: 'chat' | 'status' | 'task' | 'approval' | 'review' | 'system'
  task_id: string | null
  ts: number
}

export interface TeamSession {
  id: string
  requirement: string
  status: 'draft' | 'planning' | 'assigned' | 'in_progress' | 'reviewing' | 'arbitrating' | 'done' | 'failed' | 'closed'
  tasks: TeamTask[]
  approvals: TeamApproval[]
  shared_context: TeamSharedContext[]
  messages: TeamMessage[]
  created_at: number
  updated_at: number
  started_at: number | null
  completed_at: number | null
}

export interface TeamStats {
  total_sessions: number
  by_status: Record<string, number>
  total_tasks: number
  total_approvals: number
  total_messages: number
  role_distribution: Record<string, number>
}

export interface TeamRoleInfo {
  id: string
  label: string
  read_only: boolean
  allowed: string[] | null
  denied: string[]
}

export interface TeamStatus {
  roles: TeamRoleInfo[]
  review_pairs: Record<string, string[]>
  arbitration_pairs: Record<string, string[]>
  tools: Record<string, string[]>
}

export const teamApi = {
  status: (workspacePath?: string) =>
    api.get('/team/status', { params: { workspace_path: workspacePath || '' } }),

  sessions: (params: { workspace_path?: string; status?: string } = {}) =>
    api.get('/team/sessions', { params: { ...params, workspace_path: params.workspace_path || '' } }),

  session: (id: string, workspacePath?: string) =>
    api.get(`/team/sessions/${id}`, { params: { workspace_path: workspacePath || '' } }),

  create: (requirement: string, workspacePath?: string) =>
    api.post('/team/sessions', { requirement }, { params: { workspace_path: workspacePath || '' } }),

  remove: (id: string, workspacePath?: string) =>
    api.delete(`/team/sessions/${id}`, { params: { workspace_path: workspacePath || '' } }),

  addTask: (id: string, data: { title: string; description: string; role: string; dependencies?: string[] }, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/tasks`, data, { params: { workspace_path: workspacePath || '' } }),

  assignTask: (id: string, taskId: string, agentId: string, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/tasks/${taskId}/assign`, { agent_id: agentId }, { params: { workspace_path: workspacePath || '' } }),

  startTask: (id: string, taskId: string, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/tasks/${taskId}/start`, {}, { params: { workspace_path: workspacePath || '' } }),

  completeTask: (id: string, taskId: string, data: { output?: string; file_paths?: string[] }, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/tasks/${taskId}/complete`, data, { params: { workspace_path: workspacePath || '' } }),

  failTask: (id: string, taskId: string, error: string, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/tasks/${taskId}/fail`, { error }, { params: { workspace_path: workspacePath || '' } }),

  requestApproval: (id: string, data: { task_id: string; approver_role: string; reason: string }, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/approvals`, data, { params: { workspace_path: workspacePath || '' } }),

  decideApproval: (id: string, approvalId: string, data: { approve: boolean; comment?: string }, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/approvals/${approvalId}/decide`, data, { params: { workspace_path: workspacePath || '' } }),

  addMessage: (id: string, data: { role: string; content: string; type?: string; task_id?: string }, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/messages`, data, { params: { workspace_path: workspacePath || '' } }),

  updateContext: (id: string, data: { key: string; value: string; source: string }, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/context`, data, { params: { workspace_path: workspacePath || '' } }),

  checkBoundary: (id: string, data: { role: string; path: string; action?: string }, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/check-boundary`, data, { params: { workspace_path: workspacePath || '' } }),

  close: (id: string, workspacePath?: string) =>
    api.post(`/team/sessions/${id}/close`, {}, { params: { workspace_path: workspacePath || '' } }),

  stats: (workspacePath?: string) =>
    api.get('/team/stats', { params: { workspace_path: workspacePath || '' } }),
}

// 2026-06-09 TASK-4.2 引入：Tailscale 远程访问 API 客户端
export interface TailscaleInfo {
  status: 'unknown' | 'not_installed' | 'installed_stopped' | 'running_logged_out' | 'running_online' | 'error'
  installed: boolean
  running: boolean
  logged_in: boolean
  online: boolean
  ipv4: string | null
  ipv6: string | null
  hostname: string | null
  tailnet: string | null
  account: string | null
  error: string | null
  detected_at: number
  raw_output: string | null
}

export interface PairingCode {
  value: string
  generated_at: number
  ttl_seconds: number
}

export interface RemoteAccessHint {
  primary_url: string | null
  backup_urls: string[]
  pairing_code: string | null
  note: string | null
  requires_token: boolean
}

export interface TailscaleSummary {
  tailscale: TailscaleInfo
  platform: string
  lan_ip: string | null
  remote_hint: RemoteAccessHint
  pairing_code: PairingCode
}

export const tailscaleApi = {
  status: (useCache = true) =>
    api.get('/tailscale/status', { params: { use_cache: useCache } }),

  remoteHint: (port = 18080, includeCode = true, regenerate = false) =>
    api.get('/tailscale/remote-hint', {
      params: { port, include_code: includeCode, regenerate },
    }),

  regenerateCode: (ttlSeconds = 3600) =>
    api.post('/tailscale/pairing/regenerate', null, {
      params: { ttl_seconds: ttlSeconds },
    }),

  verifyCode: (code: string) =>
    api.post('/tailscale/pairing/verify', { code }),

  summary: () => api.get('/tailscale/summary'),
}

// 2026-06-09 TASK-4.4 引入：鉴权 API 客户端
// 用于多用户协作场景。token 存 localStorage，axios 拦截器自动注入
export interface AuthUser {
  id: string
  username: string
  email: string
  display_name: string
  created_at: number
  last_login_at: number | null
  is_active: boolean
}

export interface AuthTokens {
  access_token: string
  refresh_token: string
}

export interface RegisterPayload {
  username: string
  password: string
  email?: string
  display_name?: string
}

export interface LoginPayload {
  username: string
  password: string
}

export interface RegisterResponse {
  user: AuthUser
  access_token: string
  refresh_token: string
  workspace_id: string
}

export interface LoginResponse {
  user: AuthUser
  access_token: string
  refresh_token: string
}

export const TOKEN_STORAGE_KEY = 'yunji:auth:tokens'
export const USER_STORAGE_KEY = 'yunji:auth:user'
export const ACTIVE_WS_STORAGE_KEY = 'yunji:auth:active_workspace'

function authHeaders(token?: string) {
  return token ? { Authorization: `Bearer ${token}` } : {}
}

export const authApi = {
  register: (payload: RegisterPayload) =>
    api.post<RegisterResponse>('/auth/register', payload).then(r => r.data),

  login: (payload: LoginPayload) =>
    api.post<LoginResponse>('/auth/login', payload).then(r => r.data),

  refresh: (refreshToken: string) =>
    api.post<AuthTokens>('/auth/refresh', { refresh_token: refreshToken }).then(r => r.data),

  logout: (refreshToken: string) =>
    api.post<{ ok: boolean }>('/auth/logout', { refresh_token: refreshToken }).then(r => r.data),

  me: (accessToken: string) =>
    api.get<AuthUser>('/auth/me', { headers: authHeaders(accessToken) }).then(r => r.data),

  pairDevice: (pairCode: string, deviceName?: string) =>
    api.post<{ device_id: string; device_name: string; user_id: string | null; paired: boolean }>(
      '/auth/devices/pair',
      { pair_code: pairCode, device_name: deviceName || 'Unknown Device' },
    ).then(r => r.data),
}

// 2026-06-09 TASK-4.4 引入：工作区 API 客户端
export interface Workspace {
  id: string
  name: string
  description: string
  owner_id: string
  created_at: number
  role: 'owner' | 'editor' | 'viewer'
}

export interface WorkspaceMember {
  user_id: string
  display_name: string
  role: 'owner' | 'editor' | 'viewer'
  joined_at: number
}

export const workspaceApi = {
  list: (accessToken: string) =>
    api.get<{ data: Workspace[]; total: number }>('/auth/workspaces', { headers: authHeaders(accessToken) }).then(r => r.data),

  create: (accessToken: string, name: string, description: string = '') =>
    api.post<{ data: Workspace }>(
      '/auth/workspaces',
      { name, description },
      { headers: authHeaders(accessToken) },
    ).then(r => r.data),

  listMembers: (accessToken: string, workspaceId: string) =>
    api.get<{ data: WorkspaceMember[] }>(
      `/auth/workspaces/${workspaceId}/members`,
      { headers: authHeaders(accessToken) },
    ).then(r => r.data),

  addMember: (accessToken: string, workspaceId: string, username: string, role: 'editor' | 'viewer' = 'viewer') =>
    api.post<{ data: WorkspaceMember }>(
      `/auth/workspaces/${workspaceId}/members`,
      { username, role },
      { headers: authHeaders(accessToken) },
    ).then(r => r.data),

  removeMember: (accessToken: string, workspaceId: string, userId: string) =>
    api.delete<{ ok: boolean }>(
      `/auth/workspaces/${workspaceId}/members/${userId}`,
      { headers: authHeaders(accessToken) },
    ).then(r => r.data),
}

// 2026-06-09 TASK-4.4 引入：Y.js 房间元数据 API
// 实际协作通道是 WebSocket（路径 /api/yjs/{workspace_id}），这里只暴露管理端点
export interface YjsRoomInfo {
  id: string
  client_count: number
  last_save_at: number
  has_state: boolean
  state_size: number
}

export const yjsApi = {
  listRooms: (accessToken: string) =>
    api.get<{ data: YjsRoomInfo[] }>('/yjs/rooms', { headers: authHeaders(accessToken) }).then(r => r.data),

  roomDetail: (accessToken: string, roomId: string) =>
    api.get<{ data: YjsRoomInfo }>(`/yjs/rooms/${roomId}`, { headers: authHeaders(accessToken) }).then(r => r.data),
}

// 同时提供命名导出（兼容 `import { api } from '@/api'` 调用方）
export { api }

// 2026-06-09 TASK-4.6 引入：智能模型路由 API
export interface ModelRef {
  provider: string
  model: string
}

export interface RoutingTier {
  name: string
  max_complexity: number
  primary: ModelRef
  fallback: ModelRef | null
}

export interface RoutingRules {
  tiers: RoutingTier[]
  auto_fallback: boolean
  history_limit: number
}

export interface ComplexityScore {
  total: number
  length: number
  keywords: number
  structure: number
  history: number
  matched_keywords: string[]
  signals: string[]
}

export interface RoutingDecision {
  tier: string
  primary: ModelRef
  fallback: ModelRef | null
  complexity: ComplexityScore
  rationale: string
  timestamp: number
}

export interface RoutingHistoryItem extends RoutingDecision {
  prompt_preview: string
}

export const modelRouterApi = {
  getDefaults: () =>
    api.get<{ data: RoutingRules }>('/model-router/defaults').then(r => r.data),

  getRules: () =>
    api.get<{ data: RoutingRules }>('/model-router/rules').then(r => r.data),

  saveRules: (rules: RoutingRules) =>
    api.put('/model-router/rules', rules).then(r => r.data),

  resetRules: () =>
    api.post('/model-router/reset').then(r => r.data),

  evaluate: (data: { prompt: string; history?: Array<Record<string, unknown>> | null; has_images?: boolean }) =>
    api.post<{ data: ComplexityScore }>('/model-router/evaluate', {
      prompt: data.prompt,
      history: data.history ?? null,
      has_images: data.has_images ?? false,
    }).then(r => r.data),

  decide: (data: { prompt: string; history?: Array<Record<string, unknown>> | null; has_images?: boolean; record?: boolean }) =>
    api.post<{ data: RoutingDecision }>('/model-router/decide', {
      prompt: data.prompt,
      history: data.history ?? null,
      has_images: data.has_images ?? false,
      record: data.record ?? true,
    }).then(r => r.data),

  getHistory: (limit: number = 50) =>
    api.get<{ data: RoutingHistoryItem[] }>('/model-router/history', { params: { limit } }).then(r => r.data),

  clearHistory: () =>
    api.delete('/model-router/history').then(r => r.data),
}

// 2026-06-09 TASK-4.5 引入：技能市场 API 客户端
export interface SkillFile {
  [path: string]: string
}

export interface Skill {
  id: string
  name: string
  version: string
  author: string
  category: string
  tags: string[]
  description: string
  long_description?: string
  icon?: string
  repository?: string
  license?: string
  min_yunji_version?: string
  rating?: number
  rating_count?: number
  downloads?: number
  prompt_template: string
  files?: SkillFile
  dependencies?: string[]
}

export interface SkillCategory {
  id: string
  label: string
  count: number
}

export interface InstalledSkill {
  id: string
  name?: string
  description?: string
  path: string
  version?: string
  scope?: string
  installed_at?: number
  source?: string
}

export interface SkillRatingInfo {
  skill_id: string
  average: number
  count: number
}

export interface SkillMarketStats {
  official_count: number
  global_installed: number
  avg_rating: number
  total_downloads: number
  categories: number
}

export const skillMarketApi = {
  stats: () =>
    api.get<{ data: SkillMarketStats }>('/skill-market/stats').then(r => r.data),

  categories: () =>
    api.get<{ data: { categories: SkillCategory[]; total: number } }>('/skill-market/categories').then(r => r.data),

  list: (params: { category?: string; search?: string; tag?: string; limit?: number } = {}) =>
    api.get<{ data: Skill[] }>('/skill-market/list', { params }).then(r => r.data),

  get: (skillId: string) =>
    api.get<{ data: Skill }>(`/skill-market/${encodeURIComponent(skillId)}`).then(r => r.data),

  installed: (params: { scope?: 'global' | 'project'; workspace_path?: string } = {}) =>
    api.get<{ data: InstalledSkill[] }>('/skill-market/installed', { params }).then(r => r.data),

  install: (data: { skill_id: string; scope?: 'global' | 'project'; workspace_path?: string }) =>
    api.post('/skill-market/install', data).then(r => r.data),

  uninstall: (data: { skill_id: string; scope?: 'global' | 'project'; workspace_path?: string }) =>
    api.post('/skill-market/uninstall', data).then(r => r.data),

  upload: (data: { meta: Partial<Skill> & { id: string; name: string; version: string; category: string; prompt_template: string }; scope?: 'global' | 'project'; workspace_path?: string }) =>
    api.post('/skill-market/upload', data).then(r => r.data),

  rate: (skillId: string, score: number) =>
    api.post('/skill-market/rate', { skill_id: skillId, score }).then(r => r.data),

  getRating: (skillId: string) =>
    api.get<{ data: SkillRatingInfo }>(`/skill-market/${encodeURIComponent(skillId)}/rating`).then(r => r.data),

  apply: (data: { skill_id: string; variables?: Record<string, string>; scope?: 'global' | 'project'; workspace_path?: string }) =>
    api.post<{ data: { rendered_prompt: string; skill_id: string; scope: string; path: string } }>('/skill-market/apply', data).then(r => r.data),

  exportSkill: (data: { skill_id: string; scope?: 'global' | 'project'; workspace_path?: string }) =>
    api.post<{ data: { filename: string; size: number; base64: string } }>('/skill-market/export', data).then(r => r.data),
}

export default api
