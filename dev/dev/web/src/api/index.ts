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

export default api
