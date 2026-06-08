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

export default api
