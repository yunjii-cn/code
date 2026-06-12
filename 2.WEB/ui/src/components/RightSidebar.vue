<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { useProjectStore } from '@/stores/project'
import { systemApi, projectApi, aiApi } from '@/api'

export type RightPanel = 'apiservice' | 'git' | 'files' | 'claudemd' | 'memory' | 'plugins'

const props = defineProps<{
  collapsed: boolean
}>()

const emit = defineEmits<{
  (e: 'toggle'): void
}>()

const projectStore = useProjectStore()

const activePanel = ref<RightPanel>('apiservice')

const panels: Array<{ id: RightPanel; icon: string; label: string }> = [
  { id: 'apiservice', icon: 'fire-o', label: 'API 服务' },
  { id: 'git', icon: 'cluster-o', label: '源代码管理' },
  { id: 'files', icon: 'description', label: '文件浏览器' },
  { id: 'claudemd', icon: 'notes-o', label: 'CLAUDE.md' },
  { id: 'memory', icon: 'like-o', label: '记忆管理' },
  { id: 'plugins', icon: 'apps-o', label: '插件' },
]

function togglePanel(id: RightPanel) {
  if (activePanel.value === id && !props.collapsed) {
    emit('toggle')
  } else {
    activePanel.value = id
    if (props.collapsed) {
      emit('toggle')
    }
    nextTick(() => {
      panelLoaders[id]?.()
    })
  }
}

watch(() => props.collapsed, (val) => {
  if (!val && activePanel.value) {
    nextTick(() => {
      panelLoaders[activePanel.value!]?.()
    })
  }
})

onMounted(() => {
  if (!props.collapsed) {
    panelLoaders['apiservice']?.()
  }
})

const apiTopTab = ref<'cloud' | 'local'>('cloud')
const cloudSubTab = ref<'preset' | 'custom'>('preset')

const presetProviders = ref<Array<{
  id: string; name: string; emoji: string; type: string
  expanded: boolean; loading: boolean
}>>([
  { id: 'qwen', name: '千问', emoji: '🌟', type: 'account_pool', expanded: false, loading: false },
  { id: 'zhipu', name: '智谱', emoji: '🧠', type: 'key_pool', expanded: false, loading: false },
])

const customProviders = ref<Array<{
  id: string; name: string; emoji: string; apiKey: string; baseUrl: string
  connected: boolean | null; checking: boolean; expanded: boolean; showKey: boolean
  models: Array<{ id: string; name: string }>; modelsLoading: boolean
}>>([
  { id: 'anthropic', name: 'Anthropic', emoji: '🤖', apiKey: '', baseUrl: 'https://api.anthropic.com', connected: null, checking: false, expanded: false, showKey: false, models: [], modelsLoading: false },
  { id: 'openrouter', name: 'OpenRouter', emoji: '☁️', apiKey: '', baseUrl: 'https://openrouter.ai/api/v1', connected: null, checking: false, expanded: false, showKey: false, models: [], modelsLoading: false },
  { id: 'deepseek', name: 'DeepSeek', emoji: '🔍', apiKey: '', baseUrl: 'https://api.deepseek.com', connected: null, checking: false, expanded: false, showKey: false, models: [], modelsLoading: false },
  { id: 'siliconflow', name: '硅基流动', emoji: '🌊', apiKey: '', baseUrl: 'https://api.siliconflow.cn/v1', connected: null, checking: false, expanded: false, showKey: false, models: [], modelsLoading: false },
])

interface QwenAccount {
  email: string
  status: string
  statusText: string
  inflight: number
  lastUsed: string
}
const qwenPoolStatus = ref<{ total: number; valid: number; rateLimited: number; pending: number; banned: number; inUse: number } | null>(null)
const qwenAccounts = ref<QwenAccount[]>([])
const qwenRegistering = ref(false)
const qwenAutoHeal = ref(true)
const qwenMaxInflight = ref(2)
const qwenServiceOnline = ref<boolean | null>(null)

async function loadQwenPool() {
  const p = presetProviders.value.find(x => x.id === 'qwen')
  if (p) p.loading = true
  try {
    const healthData: any = await systemApi.checkQwenHealth?.()
    qwenServiceOnline.value = healthData?.status === 'ok'
  } catch {
    qwenServiceOnline.value = false
  }
  try {
    const statusData: any = await systemApi.getQwenPoolStatus?.()
    if (statusData) {
      const accStatus = statusData.accounts || {}
      qwenPoolStatus.value = {
        total: accStatus.total || 0,
        valid: accStatus.valid || 0,
        rateLimited: accStatus.rate_limited || 0,
        pending: accStatus.activation_pending || 0,
        banned: accStatus.banned || 0,
        inUse: accStatus.in_use || 0,
      }
    }
    const accountsData: any = await systemApi.getQwenAccounts?.()
    if (accountsData?.accounts && Array.isArray(accountsData.accounts)) {
      qwenAccounts.value = accountsData.accounts.map((a: any) => ({
        email: a.email || '',
        status: a.status || a.status_code || 'unknown',
        statusText: a.status || a.status_code || '未知',
        inflight: a.inflight || 0,
        lastUsed: a.last_request_finished ? new Date(a.last_request_finished * 1000).toLocaleTimeString() : '',
      }))
    }
  } catch {
    qwenPoolStatus.value = null
    qwenAccounts.value = []
  } finally {
    if (p) p.loading = false
  }
}

async function registerQwenAccount() {
  qwenRegistering.value = true
  try {
    await systemApi.registerQwenAccount?.()
    showToast('注册请求已发送')
    await loadQwenPool()
  } catch (e: any) {
    showToast(`注册失败: ${e.message}`)
  } finally {
    qwenRegistering.value = false
  }
}

async function removeQwenAccount(email: string) {
  try {
    await showConfirmDialog({ title: '移除账号', message: `确定移除「${email}」？`, confirmButtonColor: 'var(--error)' })
    await systemApi.removeQwenAccount?.(email)
    showToast('已移除')
    await loadQwenPool()
  } catch { /* cancelled */ }
}

function qwenStatusColor(status: string): string {
  const map: Record<string, string> = {
    valid: '#4CAF50', rate_limited: '#FF9800', pending_activation: '#2196F3',
    banned: '#F44336', auth_error: '#F44336', invalid: '#9E9E9E',
  }
  return map[status] || '#9E9E9E'
}

interface ZhipuKey {
  apiKey: string
  label: string
  valid: boolean
  status: string
  rateLimitedUntil: number
  totalRequests: number
}
const zhipuKeys = ref<ZhipuKey[]>([])
const zhipuNewKey = ref('')
const zhipuNewLabel = ref('')
const zhipuServiceOnline = ref<boolean | null>(null)

async function loadZhipuPool() {
  const p = presetProviders.value.find(x => x.id === 'zhipu')
  if (p) p.loading = true
  try {
    const healthData: any = await systemApi.checkZhipuHealth?.()
    zhipuServiceOnline.value = healthData?.status === 'ok'
  } catch {
    zhipuServiceOnline.value = false
  }
  try {
    const data: any = await systemApi.getZhipuKeys?.()
    if (data?.accounts && Array.isArray(data.accounts)) {
      zhipuKeys.value = data.accounts.map((k: any) => ({
        apiKey: k.full_key || k.api_key || '',
        label: k.label || '',
        valid: k.valid ?? k.status === 'valid',
        status: k.status || (k.valid ? 'valid' : 'invalid'),
        rateLimitedUntil: k.rate_limited_until || 0,
        totalRequests: k.total_requests || 0,
      }))
    }
  } catch {
    zhipuKeys.value = []
  } finally {
    if (p) p.loading = false
  }
}

async function addZhipuKey() {
  if (!zhipuNewKey.value.trim()) {
    showToast('请输入 API Key')
    return
  }
  try {
    await systemApi.addZhipuKey?.(zhipuNewKey.value.trim(), zhipuNewLabel.value.trim())
    zhipuNewKey.value = ''
    zhipuNewLabel.value = ''
    showToast('添加成功')
    await loadZhipuPool()
  } catch (e: any) {
    showToast(`添加失败: ${e.message}`)
  }
}

async function removeZhipuKey(key: string) {
  try {
    await showConfirmDialog({ title: '移除密钥', message: '确定移除？', confirmButtonColor: 'var(--error)' })
    await systemApi.removeZhipuKey?.(key)
    showToast('已移除')
    await loadZhipuPool()
  } catch { /* cancelled */ }
}

function zhipuStatusColor(key: ZhipuKey): string {
  if (key.rateLimitedUntil > Date.now() / 1000) return '#FF9800'
  return key.valid ? '#4CAF50' : '#F44336'
}

function zhipuStatusText(key: ZhipuKey): string {
  if (key.rateLimitedUntil > Date.now() / 1000) return '限流中'
  return key.valid ? '正常' : '失效'
}

async function loadApiSettings() {
  try {
    const data: any = await systemApi.getSettings()
    if (data) {
      for (const p of customProviders.value) {
        if (data[`api_key_${p.id}`]) p.apiKey = data[`api_key_${p.id}`]
        if (data[`base_url_${p.id}`]) p.baseUrl = data[`base_url_${p.id}`]
      }
      if (data.ollama_base_url) ollamaBaseUrl.value = data.ollama_base_url
    }
  } catch { /* defaults */ }
  if (cloudSubTab.value === 'preset') {
    loadQwenPool()
    loadZhipuPool()
  }
}

async function saveApiKey(provider: typeof customProviders.value[0]) {
  try {
    await systemApi.saveSettings({ [`api_key_${provider.id}`]: provider.apiKey, [`base_url_${provider.id}`]: provider.baseUrl })
    showToast('保存成功')
  } catch {
    showToast('保存失败')
  }
}

async function checkApiConnection(provider: typeof customProviders.value[0]) {
  provider.checking = true
  provider.connected = null
  try {
    await aiApi.checkProvider({ provider: provider.id, base_url: provider.baseUrl || undefined, api_key: provider.apiKey || undefined })
    provider.connected = true
    showToast('连接成功')
    await loadProviderModels(provider)
  } catch {
    provider.connected = false
    showToast('连接失败')
  } finally {
    provider.checking = false
  }
}

async function loadProviderModels(provider: typeof customProviders.value[0]) {
  provider.modelsLoading = true
  try {
    const data: any = await aiApi.getModels(provider.id, { base_url: provider.baseUrl, api_key: provider.apiKey || undefined })
    provider.models = (data?.models || data?.data || []).map((m: any) => ({
      id: m.id || m.name,
      name: m.name || m.id,
    }))
  } catch {
    provider.models = []
  } finally {
    provider.modelsLoading = false
  }
}

function toggleCustomExpand(p: typeof customProviders.value[0]) {
  p.expanded = !p.expanded
  if (p.expanded && p.connected === null && p.apiKey) {
    checkApiConnection(p)
  }
}

function togglePresetExpand(p: typeof presetProviders.value[0]) {
  p.expanded = !p.expanded
  if (p.expanded) {
    if (p.id === 'qwen') loadQwenPool()
    if (p.id === 'zhipu') loadZhipuPool()
  }
}

const ollamaBaseUrl = ref('http://localhost:11434')
const ollamaModels = ref<Array<{ name: string; size?: string }>>([])
const ollamaSearch = ref('')
const ollamaPulling = ref<string | null>(null)
const ollamaSearchResults = ref<Array<{ name: string; description?: string }>>([])
const ollamaSearching = ref(false)
const ollamaHealthChecking = ref(false)
const ollamaHealthy = ref<boolean | null>(null)

async function loadOllamaModels() {
  try {
    const data: any = await aiApi.getModels('ollama', { base_url: ollamaBaseUrl.value })
    ollamaModels.value = (data?.models || data?.data || []).map((m: any) => ({
      name: m.name || m.id,
      size: m.size ? `${(m.size / 1e9).toFixed(1)}GB` : undefined,
    }))
    ollamaHealthy.value = true
  } catch {
    ollamaModels.value = []
    ollamaHealthy.value = false
  }
}

async function checkOllamaHealth() {
  ollamaHealthChecking.value = true
  ollamaHealthy.value = null
  try {
    await aiApi.getModels('ollama', { base_url: ollamaBaseUrl.value })
    ollamaHealthy.value = true
    showToast('Ollama 运行正常')
    await loadOllamaModels()
  } catch {
    ollamaHealthy.value = false
    showToast('Ollama 无法连接')
  } finally {
    ollamaHealthChecking.value = false
  }
}

async function searchOllamaLibrary() {
  if (!ollamaSearch.value.trim()) return
  ollamaSearching.value = true
  try {
    const data: any = await systemApi.searchOllamaLibrary(ollamaSearch.value.trim())
    ollamaSearchResults.value = (data?.models || data || []).map((m: any) => ({
      name: m.name || m.id,
      description: m.description || '',
    }))
  } catch {
    ollamaSearchResults.value = []
  } finally {
    ollamaSearching.value = false
  }
}

async function pullModel(name: string) {
  ollamaPulling.value = name
  try {
    await systemApi.pullModel(name)
    showToast('拉取成功')
    await loadOllamaModels()
  } catch {
    showToast('拉取失败')
  } finally {
    ollamaPulling.value = null
  }
}

async function deleteOllamaModel(name: string) {
  try {
    await showConfirmDialog({ title: '删除模型', message: `确定删除「${name}」？`, confirmButtonColor: 'var(--error)' })
    await systemApi.deleteModel(name)
    showToast('已删除')
    await loadOllamaModels()
  } catch { /* cancelled */ }
}

const gitStatus = ref<Array<{ file: string; status: string; staged?: boolean }>>([])
const gitLog = ref<Array<{ hash: string; message: string; date: string }>>([])
const gitCommitMsg = ref('')
const gitLoading = ref(false)
const gitStagingAll = ref(false)

async function loadGitStatus() {
  const p = projectStore.activeProject
  if (!p) return
  gitLoading.value = true
  try {
    const data: any = await systemApi.getGitStatus(p.path)
    gitStatus.value = (data?.files || []).map((f: any) => ({
      file: f.file || f.path, status: f.status || f.state, staged: f.staged || f.is_staged || false,
    }))
    const logData: any = await systemApi.getGitLog(p.path)
    gitLog.value = (logData?.commits || logData || []).map((c: any) => ({
      hash: (c.hash || c.commit || '').substring(0, 8), message: c.message || c.subject, date: c.date || '',
    }))
  } catch {
    gitStatus.value = []
    gitLog.value = []
  } finally {
    gitLoading.value = false
  }
}

async function gitStageAll() {
  const p = projectStore.activeProject
  if (!p) return
  gitStagingAll.value = true
  try {
    await systemApi.runCommand({ cmd: 'git add -A', cwd: p.path })
    showToast('已暂存所有更改')
    await loadGitStatus()
  } catch (e: any) {
    showToast(`暂存失败: ${e.message}`)
  } finally {
    gitStagingAll.value = false
  }
}

async function gitStageFile(file: string) {
  const p = projectStore.activeProject
  if (!p) return
  try {
    await systemApi.runCommand({ cmd: `git add "${file}"`, cwd: p.path })
    await loadGitStatus()
  } catch (e: any) {
    showToast(`暂存失败: ${e.message}`)
  }
}

async function gitCommit() {
  if (!gitCommitMsg.value.trim()) {
    showToast('请输入提交信息')
    return
  }
  const p = projectStore.activeProject
  if (!p) return
  try {
    await systemApi.runCommand({ cmd: 'git add -A', cwd: p.path })
    await systemApi.gitCommit(p.path, gitCommitMsg.value.trim())
    showToast('提交成功')
    gitCommitMsg.value = ''
    await loadGitStatus()
  } catch (e: any) {
    showToast(`提交失败: ${e.message}`)
  }
}

function statusColor(s: string) {
  if (s.includes('M')) return '#FF9800'
  if (s.includes('A') || s.includes('new') || s.includes('??')) return '#4CAF50'
  if (s.includes('D') || s.includes('deleted')) return '#F44336'
  return '#9E9E9E'
}

function statusLabel(s: string) {
  const map: Record<string, string> = { 'M': '已修改', 'A': '已添加', 'D': '已删除', 'R': '已重命名', '??': '未跟踪', 'AM': '新增修改', 'MM': '部分暂存' }
  return map[s] || s
}

interface FileNode { name: string; path: string; type: 'file' | 'dir'; children?: FileNode[]; expanded?: boolean }
const fileTree = ref<FileNode[]>([])
const fileContent = ref('')
const viewingFile = ref('')
const fileLoading = ref(false)

async function loadFileTree() {
  const p = projectStore.activeProject
  if (!p) return
  fileLoading.value = true
  try {
    const data: any = await systemApi.getFileTree(p.path)
    fileTree.value = buildTree(data)
  } catch { fileTree.value = [] } finally { fileLoading.value = false }
}

function buildTree(data: any[]): FileNode[] {
  if (!Array.isArray(data)) return []
  return data.map((item) => ({
    name: item.name || item.path?.split(/[\\/]/).pop() || '', path: item.path || '',
    type: item.type === 'directory' || item.type === 'dir' ? 'dir' : 'file',
    children: item.children ? buildTree(item.children) : undefined, expanded: false,
  }))
}

function toggleNode(node: FileNode) {
  if (node.type === 'dir') { node.expanded = !node.expanded } else { viewFile(node) }
}

async function viewFile(node: FileNode) {
  const p = projectStore.activeProject
  if (!p) return
  fileContent.value = '加载中...'
  viewingFile.value = node.name
  // 优先用 type 命令（Windows）/ cat 命令（Unix）读文件，避免新增 readFile 端点
  const isWin = navigator.platform.toLowerCase().includes('win')
  const cmd = isWin ? `type "${node.path}"` : `cat "${node.path}"`
  try {
    const data: any = await systemApi.runCommand({ cmd, cwd: p.path })
    fileContent.value = data?.stdout || '(空文件)'
  } catch {
    fileContent.value = '无法读取'
  }
}

function renderTree(nodes: FileNode[], depth: number = 0): Array<{ node: FileNode; depth: number }> {
  const result: Array<{ node: FileNode; depth: number }> = []
  for (const node of nodes) {
    result.push({ node, depth })
    if (node.type === 'dir' && node.expanded && node.children) { result.push(...renderTree(node.children, depth + 1)) }
  }
  return result
}

const flatTree = computed(() => renderTree(fileTree.value))

const claudeMdContent = ref('')
const claudeMdTab = ref<'project' | 'global'>('project')
const claudeMdSaving = ref(false)

async function loadClaudeMd() {
  if (claudeMdTab.value === 'project') {
    const p = projectStore.activeProject
    if (p) { try { const d: any = await projectApi.getClaudeMd(p.id); claudeMdContent.value = d?.content || '' } catch { claudeMdContent.value = '' } }
  } else {
    try { const d: any = await projectApi.getGlobalClaudeMd(); claudeMdContent.value = d?.content || '' } catch { claudeMdContent.value = '' }
  }
}

function switchClaudeMdTab(tab: 'project' | 'global') { claudeMdTab.value = tab; loadClaudeMd() }

async function saveClaudeMd() {
  claudeMdSaving.value = true
  try {
    if (claudeMdTab.value === 'project' && projectStore.activeProject) {
      await projectApi.saveClaudeMd({ project_id: projectStore.activeProject.id, content: claudeMdContent.value })
    } else { await projectApi.saveGlobalClaudeMd(claudeMdContent.value) }
    showToast('保存成功')
  } catch { showToast('保存失败') } finally { claudeMdSaving.value = false }
}

interface MemoryItem { filename: string; type: string; size: number }
const memories = ref<MemoryItem[]>([])
const memoryLoading = ref(false)
const memorySearch = ref('')

async function loadMemories() {
  const p = projectStore.activeProject
  if (!p) return
  memoryLoading.value = true
  try {
    const data: any = await projectApi.listMemories(p.id)
    memories.value = (data?.memories || data || []).map((m: any) => ({ filename: m.filename || m.name, type: m.type || 'project', size: m.size || 0 }))
  } catch { memories.value = [] } finally { memoryLoading.value = false }
}

async function searchMemories() {
  const p = projectStore.activeProject
  if (!p || !memorySearch.value.trim()) { await loadMemories(); return }
  try {
    const data: any = await projectApi.searchMemories(p.id, memorySearch.value.trim())
    memories.value = (data?.memories || data || []).map((m: any) => ({ filename: m.filename || m.name, type: m.type || 'project', size: m.size || 0 }))
  } catch { showToast('搜索失败') }
}

async function deleteMemory(filename: string) {
  const p = projectStore.activeProject
  if (!p) return
  try {
    await showConfirmDialog({ title: '删除记忆', message: `确定删除「${filename}」？`, confirmButtonColor: 'var(--error)' })
    await projectApi.deleteMemory(p.id, filename)
    showToast('已删除')
    await loadMemories()
  } catch { /* cancelled */ }
}

interface PluginItem { id: string; name: string; version: string; description: string }
const plugins = ref<PluginItem[]>([])

async function loadPlugins() {
  try {
    const data: any = await systemApi.listPlugins()
    plugins.value = (data?.plugins || data || []).map((p: any) => ({ id: p.id || p.name, name: p.name || p.id, version: p.version || '1.0.0', description: p.description || '' }))
  } catch { plugins.value = [] }
}

async function uninstallPlugin(id: string) {
  try {
    await showConfirmDialog({ title: '卸载插件', message: '确定卸载？', confirmButtonColor: 'var(--error)' })
    await systemApi.uninstallPlugin(id)
    showToast('已卸载')
    await loadPlugins()
  } catch { /* cancelled */ }
}

const panelLoaders: Record<RightPanel, () => Promise<void>> = {
  apiservice: loadApiSettings,
  git: loadGitStatus,
  files: loadFileTree,
  claudemd: loadClaudeMd,
  memory: loadMemories,
  plugins: loadPlugins,
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`
}

const hasProject = computed(() => !!projectStore.activeProject)

function closePanel() { emit('toggle') }
</script>

<template>
  <div class="right-sidebar" :class="{ collapsed }">
    <div class="icon-bar">
      <div v-for="panel in panels" :key="panel.id" class="icon-btn" :class="{ active: activePanel === panel.id && !collapsed }" :title="panel.label" @click="togglePanel(panel.id)">
        <van-icon :name="panel.icon" size="20" />
      </div>
      <div class="icon-spacer" />
      <div class="icon-btn collapse-btn" :title="collapsed ? '展开面板' : '收起面板'" @click="emit('toggle')">
        <van-icon v-if="collapsed" name="arrow-left" size="16" />
        <van-icon v-else name="arrow-right" size="16" />
      </div>
    </div>

    <transition name="panel-slide">
      <div v-if="!collapsed" class="panel-area">
        <div class="panel-header">
          <span class="panel-title">{{ panels.find(p => p.id === activePanel)?.label }}</span>
          <van-icon name="cross" size="16" class="panel-close" @click="closePanel" />
        </div>

        <div class="panel-content">
          <!-- ===== API Service ===== -->
          <template v-if="activePanel === 'apiservice'">
            <div class="tab-row">
              <span class="tab-item" :class="{ active: apiTopTab === 'cloud' }" @click="apiTopTab = 'cloud'">☁️ 云端</span>
              <span class="tab-item" :class="{ active: apiTopTab === 'local' }" @click="apiTopTab = 'local'; loadOllamaModels()">🖥️ 本地</span>
            </div>

            <!-- Cloud Tab -->
            <template v-if="apiTopTab === 'cloud'">
              <div class="tab-row sub-tab">
                <span class="tab-item" :class="{ active: cloudSubTab === 'preset' }" @click="cloudSubTab = 'preset'; loadQwenPool(); loadZhipuPool()">预设</span>
                <span class="tab-item" :class="{ active: cloudSubTab === 'custom' }" @click="cloudSubTab = 'custom'">自定义</span>
              </div>

              <!-- Preset Providers -->
              <template v-if="cloudSubTab === 'preset'">
                <div v-for="p in presetProviders" :key="p.id" class="api-card" :class="{ expanded: p.expanded }">
                  <div class="api-header" @click="togglePresetExpand(p)">
                    <div class="api-header-left">
                      <span class="api-emoji">{{ p.emoji }}</span>
                      <span class="api-name">{{ p.name }}</span>
                      <span v-if="p.id === 'qwen' && qwenPoolStatus" class="pool-badge">{{ qwenPoolStatus.valid }}/{{ qwenPoolStatus.total }}</span>
                      <span v-if="p.id === 'zhipu' && zhipuKeys.length" class="pool-badge">{{ zhipuKeys.filter(k => k.valid).length }}/{{ zhipuKeys.length }}</span>
                    </div>
                    <van-icon :name="p.expanded ? 'arrow-up' : 'arrow-down'" size="12" />
                  </div>

                  <!-- Qwen Account Pool -->
                  <template v-if="p.expanded && p.id === 'qwen'">
                    <div class="service-status-bar">
                      <span class="status-dot" :class="{ online: qwenServiceOnline === true, offline: qwenServiceOnline === false }" />
                      <span class="service-status-text">{{ qwenServiceOnline === true ? '服务运行中' : qwenServiceOnline === false ? '服务未启动' : '检测中...' }}</span>
                    </div>
                    <div v-if="p.loading" class="panel-hint">加载中...</div>
                    <template v-else>
                      <div v-if="qwenPoolStatus" class="pool-status-grid">
                        <div class="pool-stat"><span class="pool-stat-val green">{{ qwenPoolStatus.valid }}</span><span class="pool-stat-label">正常</span></div>
                        <div class="pool-stat"><span class="pool-stat-val orange">{{ qwenPoolStatus.rateLimited }}</span><span class="pool-stat-label">限流</span></div>
                        <div class="pool-stat"><span class="pool-stat-val blue">{{ qwenPoolStatus.pending }}</span><span class="pool-stat-label">待激活</span></div>
                        <div class="pool-stat"><span class="pool-stat-val red">{{ qwenPoolStatus.banned }}</span><span class="pool-stat-label">封禁</span></div>
                      </div>
                      <div class="preset-actions">
                        <van-button size="mini" type="primary" :loading="qwenRegistering" @click="registerQwenAccount">自动注册</van-button>
                        <van-button size="mini" plain @click="loadQwenPool"><van-icon name="replay" size="12" /></van-button>
                      </div>
                      <div class="pool-config">
                        <div class="pool-config-row">
                          <span>自动自愈</span>
                          <van-switch v-model="qwenAutoHeal" size="16px" active-color="var(--accent)" />
                        </div>
                        <div class="pool-config-row">
                          <span>每账号并发</span>
                          <van-stepper v-model="qwenMaxInflight" :min="1" :max="10" theme="round" size="mini" />
                        </div>
                      </div>
                      <div class="sub-title">账号列表</div>
                      <div v-for="acc in qwenAccounts" :key="acc.email" class="pool-item">
                        <div class="pool-item-left">
                          <span class="status-dot" :style="{ background: qwenStatusColor(acc.status) }" />
                          <span class="pool-item-name">{{ acc.email }}</span>
                          <span class="pool-item-status">{{ acc.statusText }}</span>
                        </div>
                        <van-icon name="delete-o" size="14" class="delete-icon" @click="removeQwenAccount(acc.email)" />
                      </div>
                      <div v-if="qwenAccounts.length === 0" class="panel-hint">暂无账号</div>
                    </template>
                  </template>

                  <!-- Zhipu Key Pool -->
                  <template v-if="p.expanded && p.id === 'zhipu'">
                    <div class="service-status-bar">
                      <span class="status-dot" :class="{ online: zhipuServiceOnline === true, offline: zhipuServiceOnline === false }" />
                      <span class="service-status-text">{{ zhipuServiceOnline === true ? '服务运行中' : zhipuServiceOnline === false ? '服务未启动' : '检测中...' }}</span>
                    </div>
                    <div v-if="p.loading" class="panel-hint">加载中...</div>
                    <template v-else>
                      <div v-if="zhipuKeys.length" class="pool-status-grid">
                        <div class="pool-stat"><span class="pool-stat-val green">{{ zhipuKeys.filter(k => k.valid).length }}</span><span class="pool-stat-label">有效</span></div>
                        <div class="pool-stat"><span class="pool-stat-val orange">{{ zhipuKeys.filter(k => k.rateLimitedUntil > Date.now() / 1000).length }}</span><span class="pool-stat-label">限流</span></div>
                        <div class="pool-stat"><span class="pool-stat-val red">{{ zhipuKeys.filter(k => !k.valid).length }}</span><span class="pool-stat-label">失效</span></div>
                      </div>
                      <div class="add-key-row">
                        <van-field v-model="zhipuNewKey" placeholder="输入 API Key" class="panel-field add-key-field" />
                        <van-field v-model="zhipuNewLabel" placeholder="标签(可选)" class="panel-field add-label-field" />
                        <van-button size="mini" type="primary" @click="addZhipuKey">添加</van-button>
                      </div>
                      <div class="sub-title">密钥列表</div>
                      <div v-for="k in zhipuKeys" :key="k.apiKey" class="pool-item">
                        <div class="pool-item-left">
                          <span class="status-dot" :style="{ background: zhipuStatusColor(k) }" />
                          <span class="pool-item-name">{{ k.label || k.apiKey.substring(0, 12) + '...' }}</span>
                          <span class="pool-item-status">{{ zhipuStatusText(k) }}</span>
                          <span class="pool-item-meta">{{ k.totalRequests }}次</span>
                        </div>
                        <van-icon name="delete-o" size="14" class="delete-icon" @click="removeZhipuKey(k.apiKey)" />
                      </div>
                      <div v-if="zhipuKeys.length === 0" class="panel-hint">暂无密钥</div>
                    </template>
                  </template>
                </div>
              </template>

              <!-- Custom Providers -->
              <template v-if="cloudSubTab === 'custom'">
                <div v-for="p in customProviders" :key="p.id" class="api-card" :class="{ expanded: p.expanded }">
                  <div class="api-header" @click="toggleCustomExpand(p)">
                    <div class="api-header-left">
                      <span class="api-emoji">{{ p.emoji }}</span>
                      <span class="api-name">{{ p.name }}</span>
                      <span class="status-dot" :class="{ online: p.connected === true, offline: p.connected === false }" />
                    </div>
                    <div class="api-header-right">
                      <span v-if="p.apiKey" class="api-key-hint">已配置</span>
                      <van-icon :name="p.expanded ? 'arrow-up' : 'arrow-down'" size="12" />
                    </div>
                  </div>
                  <template v-if="p.expanded">
                    <van-field v-model="p.apiKey" :type="p.showKey ? 'text' : 'password'" placeholder="API Key" class="panel-field">
                      <template #button>
                        <van-button size="mini" plain @click.stop="p.showKey = !p.showKey">{{ p.showKey ? '隐藏' : '显示' }}</van-button>
                      </template>
                    </van-field>
                    <van-field v-model="p.baseUrl" placeholder="Base URL" class="panel-field" style="margin-top:4px" />
                    <div class="api-actions">
                      <van-button size="mini" plain :loading="p.checking" @click="checkApiConnection(p)">检测连接</van-button>
                      <van-button size="mini" type="primary" @click="saveApiKey(p)">保存</van-button>
                    </div>
                    <div v-if="p.models.length > 0" class="model-list">
                      <div class="sub-title">可用模型 ({{ p.models.length }})</div>
                      <div v-for="m in p.models.slice(0, 10)" :key="m.id" class="model-item">{{ m.name }}</div>
                      <div v-if="p.models.length > 10" class="model-more">... 还有 {{ p.models.length - 10 }} 个模型</div>
                    </div>
                  </template>
                </div>
              </template>
            </template>

            <!-- Local Tab (Ollama) -->
            <template v-if="apiTopTab === 'local'">
              <van-field v-model="ollamaBaseUrl" placeholder="Ollama 地址" class="panel-field" />
              <div class="panel-toolbar" style="margin-top:8px">
                <div class="ollama-health">
                  <span class="status-dot" :class="{ online: ollamaHealthy === true, offline: ollamaHealthy === false }" />
                  <span>{{ ollamaHealthy === true ? '运行中' : ollamaHealthy === false ? '离线' : '未检测' }}</span>
                </div>
                <div class="ollama-toolbar-actions">
                  <van-button size="mini" plain :loading="ollamaHealthChecking" @click="checkOllamaHealth">检测</van-button>
                  <van-icon name="replay" size="14" class="refresh-icon" @click="loadOllamaModels" />
                </div>
              </div>
              <div class="panel-section">
                <div class="sub-title">本地模型 ({{ ollamaModels.length }})</div>
                <div v-for="m in ollamaModels" :key="m.name" class="ollama-item">
                  <div><span class="ollama-name">{{ m.name }}</span><span v-if="m.size" class="ollama-size">{{ m.size }}</span></div>
                  <van-icon name="delete-o" size="16" class="delete-icon" @click="deleteOllamaModel(m.name)" />
                </div>
                <div v-if="ollamaModels.length === 0" class="panel-hint">暂无本地模型</div>
              </div>
              <div class="panel-section">
                <div class="sub-title">搜索模型库</div>
                <div class="search-row">
                  <van-field v-model="ollamaSearch" placeholder="搜索模型..." class="panel-field" clearable @keydown.enter="searchOllamaLibrary" @clear="ollamaSearchResults = []">
                    <template #left-icon><van-icon name="search" size="14" /></template>
                  </van-field>
                </div>
                <div v-if="ollamaSearching" class="panel-hint">搜索中...</div>
                <div v-for="m in ollamaSearchResults" :key="m.name" class="ollama-item">
                  <div class="ollama-search-info"><span class="ollama-name">{{ m.name }}</span><span v-if="m.description" class="ollama-desc">{{ m.description }}</span></div>
                  <van-button size="mini" plain :loading="ollamaPulling === m.name" @click="pullModel(m.name)">拉取</van-button>
                </div>
              </div>
            </template>
          </template>

          <!-- ===== Git ===== -->
          <template v-if="activePanel === 'git'">
            <div v-if="!hasProject" class="panel-empty">请先选择项目</div>
            <template v-else>
              <div class="panel-section">
                <div class="sub-title">提交</div>
                <div class="commit-row">
                  <van-field v-model="gitCommitMsg" placeholder="提交信息..." class="panel-field" @keydown.enter="gitCommit" />
                  <van-button size="mini" type="primary" :disabled="!gitCommitMsg.trim()" @click="gitCommit">提交</van-button>
                </div>
              </div>
              <div class="panel-section">
                <div class="sub-title">更改 ({{ gitStatus.length }})<van-icon name="replay" size="12" class="refresh-icon" @click="loadGitStatus" /><van-button v-if="gitStatus.length > 0" size="mini" plain :loading="gitStagingAll" @click="gitStageAll" class="stage-all-btn">全部暂存</van-button></div>
                <div v-if="gitLoading" class="panel-hint">加载中...</div>
                <div v-else-if="gitStatus.length === 0" class="panel-hint">无更改</div>
                <div v-for="item in gitStatus" :key="item.file" class="git-item" @click="gitStageFile(item.file)">
                  <span class="git-badge" :style="{ color: statusColor(item.status) }">{{ item.status }}</span>
                  <span class="git-file" :title="item.file">{{ item.file }}</span>
                  <span class="git-status-label">{{ statusLabel(item.status) }}</span>
                </div>
              </div>
              <div class="panel-section">
                <div class="sub-title">历史</div>
                <div v-for="item in gitLog" :key="item.hash" class="log-item"><span class="log-hash">{{ item.hash }}</span><span class="log-msg">{{ item.message }}</span></div>
                <div v-if="gitLog.length === 0" class="panel-hint">暂无提交</div>
              </div>
            </template>
          </template>

          <!-- ===== Files ===== -->
          <template v-if="activePanel === 'files'">
            <div v-if="!hasProject" class="panel-empty">请先选择项目</div>
            <template v-else>
              <div class="panel-toolbar"><van-button size="mini" plain @click="loadFileTree"><van-icon name="replay" size="12" /> 刷新</van-button></div>
              <div v-if="fileLoading" class="panel-hint">加载中...</div>
              <template v-else-if="viewingFile">
                <div class="file-viewer"><div class="file-viewer-header"><van-icon name="arrow-left" size="14" @click="viewingFile = ''; fileContent = ''" /><span>{{ viewingFile }}</span></div><pre class="file-content">{{ fileContent }}</pre></div>
              </template>
              <template v-else>
                <div v-for="{ node, depth } in flatTree" :key="node.path" class="tree-node" :style="{ paddingLeft: `${8 + depth * 16}px` }" @click="toggleNode(node)">
                  <van-icon v-if="node.type === 'dir'" :name="node.expanded ? 'arrow-down' : 'arrow'" size="14" class="tree-arrow" />
                  <van-icon :name="node.type === 'dir' ? 'folder-o' : 'description'" size="14" /><span>{{ node.name }}</span>
                </div>
                <div v-if="fileTree.length === 0" class="panel-hint">暂无文件</div>
              </template>
            </template>
          </template>

          <!-- ===== CLAUDE.md ===== -->
          <template v-if="activePanel === 'claudemd'">
            <div v-if="!hasProject && claudeMdTab === 'project'" class="panel-empty">请先选择项目</div>
            <template v-else>
              <div class="tab-row"><span class="tab-item" :class="{ active: claudeMdTab === 'project' }" @click="switchClaudeMdTab('project')">项目</span><span class="tab-item" :class="{ active: claudeMdTab === 'global' }" @click="switchClaudeMdTab('global')">全局</span></div>
              <van-field v-model="claudeMdContent" type="textarea" :rows="16" placeholder="编写 CLAUDE.md..." class="panel-field md-field" />
              <van-button size="small" type="primary" block :loading="claudeMdSaving" :disabled="!claudeMdContent.trim()" @click="saveClaudeMd" style="margin-top:8px">保存</van-button>
            </template>
          </template>

          <!-- ===== Memory ===== -->
          <template v-if="activePanel === 'memory'">
            <div v-if="!hasProject" class="panel-empty">请先选择项目</div>
            <template v-else>
              <div class="panel-toolbar"><span>{{ memories.length }} 条记忆</span><van-icon name="replay" size="14" class="refresh-icon" @click="loadMemories" /></div>
              <div class="search-row"><van-field v-model="memorySearch" placeholder="搜索记忆..." class="panel-field" clearable @keydown.enter="searchMemories" @clear="loadMemories"><template #left-icon><van-icon name="search" size="14" /></template></van-field></div>
              <div v-if="memoryLoading" class="panel-hint">加载中...</div>
              <template v-else>
                <div v-for="m in memories" :key="m.filename" class="memory-item"><div class="memory-info"><span class="memory-name">{{ m.filename }}</span><span class="memory-meta">{{ m.type }} · {{ formatSize(m.size) }}</span></div><van-icon name="delete-o" size="16" class="delete-icon" @click="deleteMemory(m.filename)" /></div>
                <div v-if="memories.length === 0" class="panel-hint">暂无记忆</div>
              </template>
            </template>
          </template>

          <!-- ===== Plugins ===== -->
          <template v-if="activePanel === 'plugins'">
            <div class="panel-toolbar"><span>{{ plugins.length }} 个插件</span><van-icon name="replay" size="14" class="refresh-icon" @click="loadPlugins" /></div>
            <div v-for="p in plugins" :key="p.id" class="plugin-item"><div class="plugin-info"><span class="plugin-name">{{ p.name }}</span><span class="plugin-ver">v{{ p.version }}</span></div><van-icon name="delete-o" size="16" class="delete-icon" @click="uninstallPlugin(p.id)" /></div>
            <div v-if="plugins.length === 0" class="panel-hint">暂无插件</div>
          </template>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.right-sidebar { display: flex; height: 100%; background: var(--bg-secondary); border-left: 1px solid var(--border); flex-shrink: 0; overflow: hidden; }
.icon-bar { width: 48px; display: flex; flex-direction: column; align-items: center; padding: 8px 0; gap: 4px; flex-shrink: 0; }
.icon-btn { width: 36px; height: 36px; display: flex; align-items: center; justify-content: center; border-radius: 6px; cursor: pointer; color: var(--text-muted); transition: all 0.2s; position: relative; }
.icon-btn:hover { color: var(--text-primary); background: var(--bg-card-hover); }
.icon-btn.active { color: var(--accent); background: rgba(66, 165, 245, 0.1); }
.icon-btn.active::before { content: ''; position: absolute; left: -6px; top: 8px; bottom: 8px; width: 2px; background: var(--accent); border-radius: 1px; }
.icon-spacer { flex: 1; }
.collapse-btn { color: var(--text-muted); }
.panel-area { width: 280px; display: flex; flex-direction: column; overflow: hidden; border-left: 1px solid var(--border); }
.panel-slide-enter-active, .panel-slide-leave-active { transition: width 0.25s ease, opacity 0.25s ease; }
.panel-slide-enter-from, .panel-slide-leave-to { width: 0; opacity: 0; }
.panel-header { display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; border-bottom: 1px solid var(--border); flex-shrink: 0; }
.panel-title { font-size: 13px; font-weight: 600; color: var(--text-primary); text-transform: uppercase; letter-spacing: 0.5px; }
.panel-close { color: var(--text-muted); cursor: pointer; padding: 4px; border-radius: 4px; }
.panel-close:hover { color: var(--text-primary); background: var(--bg-card-hover); }
.panel-content { flex: 1; overflow-y: auto; padding: 12px; }
.panel-empty { text-align: center; padding: 40px 0; color: var(--text-muted); font-size: 13px; }
.panel-hint { text-align: center; padding: 16px 0; color: var(--text-muted); font-size: 12px; }
.panel-section { margin-bottom: 16px; }
.sub-title { font-size: 12px; font-weight: 600; color: var(--text-secondary); margin-bottom: 8px; display: flex; align-items: center; gap: 6px; }
.refresh-icon { cursor: pointer; color: var(--text-muted); }
.refresh-icon:hover { color: var(--accent); }
.panel-field { background: var(--bg-primary); border-radius: 6px; overflow: hidden; }
:deep(.panel-field .van-field__control) { color: var(--text-primary); font-size: 12px; }
:deep(.panel-field .van-field__control::placeholder) { color: var(--text-muted); }
.tab-row { display: flex; gap: 0; margin-bottom: 12px; border-bottom: 1px solid var(--border); }
.tab-row.sub-tab { margin-bottom: 10px; }
.tab-item { padding: 6px 16px; font-size: 12px; color: var(--text-muted); cursor: pointer; border-bottom: 2px solid transparent; transition: all 0.2s; }
.tab-item:hover { color: var(--text-primary); }
.tab-item.active { color: var(--accent); border-bottom-color: var(--accent); }
.api-card { padding: 10px; background: var(--bg-card); border-radius: 8px; margin-bottom: 6px; transition: all 0.2s; }
.api-header { display: flex; align-items: center; justify-content: space-between; cursor: pointer; user-select: none; }
.api-header-left { display: flex; align-items: center; gap: 8px; }
.api-header-right { display: flex; align-items: center; gap: 6px; }
.api-emoji { font-size: 16px; }
.api-name { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.api-key-hint { font-size: 10px; color: var(--accent); background: rgba(66, 165, 245, 0.1); padding: 1px 6px; border-radius: 8px; }
.pool-badge { font-size: 10px; color: var(--accent); background: rgba(66, 165, 245, 0.1); padding: 1px 6px; border-radius: 8px; font-family: 'Consolas', monospace; }
.service-status-bar { display: flex; align-items: center; gap: 8px; padding: 6px 0; margin-bottom: 6px; }
.service-status-text { font-size: 12px; color: var(--text-secondary); }
.status-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--text-muted); flex-shrink: 0; }
.status-dot.online { background: #4CAF50; box-shadow: 0 0 6px rgba(76, 175, 80, 0.4); }
.status-dot.offline { background: #F44336; }
.api-actions { display: flex; gap: 6px; margin-top: 8px; }
.model-list { margin-top: 8px; padding-top: 8px; border-top: 1px solid var(--border); }
.model-item { font-size: 11px; color: var(--text-secondary); padding: 2px 0; font-family: 'Consolas', monospace; }
.model-more { font-size: 11px; color: var(--text-muted); padding-top: 2px; }
.pool-status-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; margin-bottom: 10px; }
.pool-stat { display: flex; flex-direction: column; align-items: center; padding: 6px 0; background: var(--bg-primary); border-radius: 6px; }
.pool-stat-val { font-size: 16px; font-weight: 700; font-family: 'Consolas', monospace; }
.pool-stat-val.green { color: #4CAF50; }
.pool-stat-val.orange { color: #FF9800; }
.pool-stat-val.blue { color: #2196F3; }
.pool-stat-val.red { color: #F44336; }
.pool-stat-label { font-size: 10px; color: var(--text-muted); margin-top: 2px; }
.preset-actions { display: flex; gap: 6px; margin-bottom: 10px; }
.pool-config { margin-bottom: 10px; }
.pool-config-row { display: flex; align-items: center; justify-content: space-between; padding: 4px 0; font-size: 12px; color: var(--text-secondary); }
.pool-item { display: flex; align-items: center; justify-content: space-between; padding: 4px 6px; border-radius: 4px; font-size: 12px; transition: background 0.15s; }
.pool-item:hover { background: var(--bg-card-hover); }
.pool-item-left { display: flex; align-items: center; gap: 6px; min-width: 0; flex: 1; }
.pool-item-name { color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11px; }
.pool-item-status { font-size: 10px; color: var(--text-muted); flex-shrink: 0; }
.pool-item-meta { font-size: 10px; color: var(--text-muted); font-family: 'Consolas', monospace; flex-shrink: 0; }
.add-key-row { display: flex; gap: 4px; align-items: center; margin-bottom: 8px; }
.add-key-field { flex: 2; }
.add-label-field { flex: 1; }
.ollama-health { display: flex; align-items: center; gap: 6px; font-size: 12px; }
.ollama-toolbar-actions { display: flex; align-items: center; gap: 6px; }
.ollama-item { display: flex; align-items: center; justify-content: space-between; padding: 6px 8px; border-radius: 6px; font-size: 12px; background: var(--bg-card); margin-bottom: 4px; }
.ollama-name { color: var(--text-primary); font-weight: 500; }
.ollama-size { color: var(--text-muted); font-size: 11px; margin-left: 6px; }
.ollama-search-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
.ollama-desc { color: var(--text-muted); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.commit-row { display: flex; gap: 6px; }
.commit-row .panel-field { flex: 1; }
.stage-all-btn { margin-left: auto; font-size: 10px; }
.git-item { display: flex; align-items: center; gap: 8px; padding: 4px 0; font-size: 12px; cursor: pointer; border-radius: 4px; transition: background 0.15s; }
.git-item:hover { background: var(--bg-card-hover); }
.git-badge { font-family: 'Consolas', monospace; font-weight: 600; font-size: 11px; min-width: 20px; }
.git-file { color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; min-width: 0; }
.git-status-label { color: var(--text-muted); font-size: 10px; flex-shrink: 0; }
.log-item { padding: 4px 0; font-size: 12px; }
.log-hash { color: var(--accent); font-family: 'Consolas', monospace; font-size: 11px; margin-right: 6px; }
.log-msg { color: var(--text-secondary); }
.panel-toolbar { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; font-size: 12px; color: var(--text-muted); }
.tree-node { display: flex; align-items: center; gap: 6px; padding: 4px 6px; border-radius: 4px; cursor: pointer; font-size: 12px; color: var(--text-secondary); transition: background 0.15s; }
.tree-node:hover { background: var(--bg-card-hover); color: var(--text-primary); }
.tree-arrow { transition: transform 0.2s; }
.file-viewer { margin-top: 8px; }
.file-viewer-header { display: flex; align-items: center; gap: 8px; padding: 4px 0; font-size: 12px; color: var(--accent); }
.file-viewer-header .van-icon { cursor: pointer; }
.file-content { margin: 0; padding: 8px; background: #0d0d0d; border-radius: 6px; font-family: 'Consolas', monospace; font-size: 11px; line-height: 1.5; color: var(--text-primary); max-height: 300px; overflow: auto; white-space: pre-wrap; word-break: break-all; }
.md-field :deep(.van-field__control) { min-height: 300px; font-family: 'Consolas', monospace; font-size: 12px; line-height: 1.6; }
.search-row { margin-bottom: 8px; }
.memory-item { display: flex; align-items: center; justify-content: space-between; padding: 8px; border-radius: 6px; margin-bottom: 4px; background: var(--bg-card); }
.memory-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
.memory-name { font-size: 12px; font-weight: 500; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.memory-meta { font-size: 11px; color: var(--text-muted); }
.delete-icon { color: var(--text-muted); cursor: pointer; flex-shrink: 0; padding: 4px; border-radius: 4px; }
.delete-icon:hover { color: var(--error); background: rgba(244, 67, 54, 0.1); }
.plugin-item { display: flex; align-items: center; justify-content: space-between; padding: 8px; border-radius: 6px; background: var(--bg-card); margin-bottom: 4px; }
.plugin-info { display: flex; align-items: center; gap: 6px; }
.plugin-name { font-size: 12px; font-weight: 500; color: var(--text-primary); }
.plugin-ver { font-size: 10px; color: var(--accent); background: rgba(66, 165, 245, 0.1); padding: 1px 6px; border-radius: 8px; }
:deep(.van-button--primary) { background: var(--accent); border-color: var(--accent); }
:deep(.panel-field .van-field__right-icon) { color: var(--text-muted); cursor: pointer; }
:deep(.panel-field .van-field__right-icon:hover) { color: var(--text-primary); }
</style>
