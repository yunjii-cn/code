<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { systemApi, versionApi } from '@/api'
import { useChatStore } from '@/stores/chat'
import { useModelStore } from '@/stores/model'
import { useDevice } from '@/composables/useDevice'
import TailscaleSettings from './components/TailscaleSettings.vue'

const chatStore = useChatStore()
const modelStore = useModelStore()

const { isMobile, isDesktop } = useDevice()
const activeSection = ref('general')

interface SidebarItem {
  key: string
  label: string
  icon: string
}

const sidebarItems: SidebarItem[] = [
  { key: 'general', label: '通用', icon: 'setting-o' },
  { key: 'ai', label: 'AI 配置', icon: 'chat-o' },
  { key: 'system', label: '系统', icon: 'info-o' },
  { key: 'version', label: '版本更新', icon: 'upgrade' },
]

const defaultProvider = ref('ollama')
const defaultModel = ref('')
const temperature = ref(0.7)
const maxTokens = ref(4096)
const systemPrompt = ref('')

const mirrorSource = ref('default')
const workDir = ref('')

const currentVersion = ref('DEV')
const checkingUpdate = ref(false)
const updateAvailable = ref(false)
const latestVersion = ref('')

const hardwareInfo = ref<Record<string, string>>({})
const hardwareLoading = ref(false)
const appDataSize = ref('')
const appDataLoading = ref(false)
const importingData = ref(false)

const providers = [
  { value: 'ollama', label: 'Ollama' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'openrouter', label: 'OpenRouter' },
  { value: 'zhipu', label: '智谱' },
  { value: 'api', label: 'API服务' },
]

const mirrors = [
  { value: 'default', label: '默认源' },
  { value: 'npmmirror', label: 'npmmirror' },
  { value: 'tuna', label: '清华源' },
  { value: 'huawei', label: '华为源' },
]

const availableModels = computed(() => modelStore.models.map(m => ({ text: m.name, value: m.id })))
const providerOptions = computed(() => providers.map(p => ({ text: p.label, value: p.value })))
const providerLabel = computed(() => providers.find(p => p.value === defaultProvider.value)?.label ?? '')
const mirrorLabel = computed(() => mirrors.find(m => m.value === mirrorSource.value)?.label ?? '')
const showProviderPicker = ref(false)
const showModelPicker = ref(false)
const showMirrorPicker = ref(false)

const hardwareEntries = computed(() => {
  const labels: Record<string, string> = {
    os: '操作系统', cpu: 'CPU', memory: '内存', gpu: 'GPU',
    pythonVersion: 'Python', nodeVersion: 'Node.js',
  }
  return Object.entries(hardwareInfo.value)
    .filter(([, v]) => v)
    .map(([k, v]) => ({ key: k, label: labels[k] || k, value: k === 'memory' ? formatMemory(Number(v)) : v }))
})

function formatMemory(mb: number) { return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB` }
function formatBytes(bytes: number) {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}

async function loadSettings() {
  try {
    const data: any = await systemApi.getSettings()
    if (data) {
      if (data.defaultProvider) defaultProvider.value = data.defaultProvider
      if (data.defaultModel) defaultModel.value = data.defaultModel
      if (data.temperature !== undefined) temperature.value = data.temperature
      if (data.maxTokens !== undefined) maxTokens.value = data.maxTokens
      if (data.systemPrompt) systemPrompt.value = data.systemPrompt
      if (data.mirrorSource) mirrorSource.value = data.mirrorSource
      if (data.workDir) workDir.value = data.workDir
    }
  } catch {}
}

async function loadVersion() {
  try {
    const data: any = await versionApi.getCurrent()
    if (data?.version) currentVersion.value = data.version
  } catch {}
}

async function loadHardware() {
  hardwareLoading.value = true
  try {
    const data: any = await systemApi.getHardware()
    if (data) {
      hardwareInfo.value = {
        os: data.os || '', cpu: data.cpu || '',
        memory: data.memory != null ? String(data.memory) : '',
        gpu: data.gpu || '', pythonVersion: data.pythonVersion || '',
        nodeVersion: data.nodeVersion || '',
      }
    }
  } catch {} finally { hardwareLoading.value = false }
}

async function loadAppDataInfo() {
  appDataLoading.value = true
  try {
    const data: any = await systemApi.loadAppData()
    if (data) {
      const raw = typeof data === 'string' ? data : JSON.stringify(data)
      appDataSize.value = formatBytes(new Blob([raw]).size)
    } else { appDataSize.value = '0 B' }
  } catch { appDataSize.value = '未知' } finally { appDataLoading.value = false }
}

async function saveSettings() {
  try {
    await systemApi.saveSettings({
      defaultProvider: defaultProvider.value, defaultModel: defaultModel.value,
      temperature: String(temperature.value), maxTokens: String(maxTokens.value),
      systemPrompt: systemPrompt.value, mirrorSource: mirrorSource.value, workDir: workDir.value,
    })
    chatStore.switchModel(defaultProvider.value, defaultModel.value)
    showToast('设置已保存')
  } catch { showToast('保存失败') }
}

async function checkUpdate() {
  checkingUpdate.value = true
  try {
    const data: any = await versionApi.checkUpdate()
    updateAvailable.value = data?.updateAvailable ?? false
    latestVersion.value = data?.latestVersion ?? ''
    showToast(updateAvailable.value ? `发现新版本: ${latestVersion.value}` : '已是最新版本')
  } catch { showToast('检查更新失败') } finally { checkingUpdate.value = false }
}

async function chooseWorkspace() {
  try {
    const data: any = await systemApi.chooseWorkspace()
    if (data?.path) { workDir.value = data.path; showToast('工作目录已更新') }
  } catch { showToast('选择目录失败') }
}

async function clearModelSettings() {
  try {
    await showConfirmDialog({ title: '确认重置', message: '确定要清除模型设置吗？' })
    await systemApi.clearModelSettings()
    defaultProvider.value = 'ollama'; defaultModel.value = ''
    showToast('模型设置已重置')
  } catch {}
}

async function testNotification() {
  try {
    await systemApi.showNotification('云集智能编程工作站', '这是一条测试通知')
    showToast('通知已发送')
  } catch { showToast('发送通知失败') }
}

async function openExplorer() {
  if (!workDir.value) { showToast('请先设置工作目录'); return }
  try { await systemApi.openExplorer(workDir.value) } catch { showToast('打开目录失败') }
}

async function exportAppData() {
  try {
    const data: any = await systemApi.loadAppData()
    const raw = typeof data === 'string' ? data : JSON.stringify(data, null, 2)
    const blob = new Blob([raw], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url; a.download = `yunji-app-data-${new Date().toISOString().slice(0, 10)}.json`
    a.click(); URL.revokeObjectURL(url)
    showToast('导出成功')
  } catch { showToast('导出失败') }
}

async function importAppData() {
  const input = document.createElement('input')
  input.type = 'file'; input.accept = '.json'
  input.onchange = async () => {
    const file = input.files?.[0]; if (!file) return
    importingData.value = true
    try {
      const text = await file.text(); JSON.parse(text)
      await systemApi.saveAppData(text); await loadAppDataInfo()
      showToast('导入成功')
    } catch { showToast('导入失败，请检查文件格式') } finally { importingData.value = false }
  }
  input.click()
}

function onProviderConfirm({ selectedValues }: { selectedValues: string[] }) {
  defaultProvider.value = selectedValues[0]
  showProviderPicker.value = false
  modelStore.switchProvider(selectedValues[0])
}

function onModelConfirm({ selectedValues }: { selectedValues: string[] }) {
  defaultModel.value = selectedValues[0]
  showModelPicker.value = false
}

function onMirrorConfirm({ selectedValues }: { selectedValues: string[] }) {
  mirrorSource.value = selectedValues[0]
  showMirrorPicker.value = false
}

onMounted(async () => {
  await loadSettings()
  await loadVersion()
  await modelStore.fetchProviders()
  if (defaultProvider.value) await modelStore.switchProvider(defaultProvider.value)
  loadHardware()
  loadAppDataInfo()
})
</script>

<template>
  <div class="settings-view" :class="{ mobile: isMobile }">
    <template v-if="isDesktop">
      <div class="sidebar">
        <div
          v-for="item in sidebarItems"
          :key="item.key"
          class="sidebar-item"
          :class="{ active: activeSection === item.key }"
          @click="activeSection = item.key"
        >
          <van-icon :name="item.icon" size="16" />
          <span class="sidebar-label">{{ item.label }}</span>
        </div>
      </div>

      <div class="content">
        <div v-if="activeSection === 'general'" class="section">
          <h3 class="section-title">通用设置</h3>
          <div class="form-group">
            <div class="form-label">镜像源</div>
            <van-field :model-value="mirrorLabel" readonly is-link class="dark-field" @click="showMirrorPicker = true" />
            <van-popup v-model:show="showMirrorPicker" round position="bottom">
              <van-picker :columns="mirrors.map(m => ({ text: m.label, value: m.value }))" :model-value="[mirrorSource]" @confirm="onMirrorConfirm" @cancel="showMirrorPicker = false" />
            </van-popup>
          </div>
          <div class="form-group">
            <div class="form-label">工作目录</div>
            <div class="workspace-chooser">
              <van-field :model-value="workDir" readonly placeholder="未设置工作目录" class="dark-field workspace-field" />
              <van-button size="small" type="primary" @click="chooseWorkspace">选择目录</van-button>
            </div>
            <van-button v-if="workDir" size="mini" plain class="open-explorer-btn" @click="openExplorer">在文件管理器中打开</van-button>
          </div>
          <div class="form-group">
            <div class="form-label">通知</div>
            <van-button size="small" type="primary" plain @click="testNotification">测试桌面通知</van-button>
          </div>
        </div>

        <div v-if="activeSection === 'ai'" class="section">
          <h3 class="section-title">AI 配置</h3>
          <div class="form-group">
            <div class="form-label">默认提供商</div>
            <van-field :model-value="providerLabel" readonly is-link class="dark-field" @click="showProviderPicker = true" />
            <van-popup v-model:show="showProviderPicker" round position="bottom">
              <van-picker :columns="providerOptions" :model-value="[defaultProvider]" @confirm="onProviderConfirm" @cancel="showProviderPicker = false" />
            </van-popup>
          </div>
          <div class="form-group">
            <div class="form-label">默认模型</div>
            <van-field v-model="defaultModel" readonly is-link placeholder="选择默认模型" class="dark-field" @click="showModelPicker = true" />
            <van-popup v-model:show="showModelPicker" round position="bottom">
              <van-picker :columns="availableModels" :model-value="[defaultModel]" @confirm="onModelConfirm" @cancel="showModelPicker = false" />
            </van-popup>
          </div>
          <div class="form-group">
            <div class="form-label">温度 ({{ temperature.toFixed(1) }})</div>
            <van-slider v-model="temperature" :min="0" :max="2" :step="0.1" bar-height="4px" active-color="var(--accent)" inactive-color="var(--border)" />
          </div>
          <div class="form-group">
            <div class="form-label">最大 Token</div>
            <van-stepper v-model="maxTokens" :min="256" :max="128000" :step="256" theme="round" />
          </div>
          <div class="form-group">
            <div class="form-label">系统提示词</div>
            <van-field v-model="systemPrompt" type="textarea" rows="4" placeholder="输入系统提示词..." class="dark-field" autosize />
          </div>
          <div class="form-group">
            <van-button size="small" type="danger" plain @click="clearModelSettings">重置模型设置</van-button>
          </div>
        </div>

        <div v-if="activeSection === 'system'" class="section">
          <h3 class="section-title">硬件信息</h3>
          <div v-if="hardwareLoading" class="status-row"><van-loading size="20px" color="var(--accent)">加载中...</van-loading></div>
          <div v-else-if="hardwareEntries.length > 0" class="hardware-card">
            <div v-for="entry in hardwareEntries" :key="entry.key" class="hardware-row">
              <span class="h-label">{{ entry.label }}</span>
              <span class="h-value">{{ entry.value }}</span>
            </div>
          </div>
          <div v-else class="status-row">
            <span class="muted">暂无硬件信息</span>
            <van-button size="mini" type="primary" plain @click="loadHardware">重新加载</van-button>
          </div>

          <h3 class="section-title" style="margin-top: 20px">数据管理</h3>
          <div class="appdata-row info-line">
            <span>数据大小</span>
            <span class="muted">{{ appDataLoading ? '计算中...' : appDataSize }}</span>
          </div>
          <div class="form-group action-row">
            <van-button size="small" type="primary" plain @click="exportAppData">导出数据</van-button>
            <van-button size="small" type="primary" plain :loading="importingData" @click="importAppData">导入数据</van-button>
          </div>
        </div>

        <div v-if="activeSection === 'version'" class="section">
          <h3 class="section-title">版本更新</h3>
          <div class="version-card">
            <div class="version-main">
              <span class="version-number">v{{ currentVersion }}</span>
              <van-tag v-if="updateAvailable" type="success" size="medium">新版本可用</van-tag>
              <van-tag v-else type="primary" size="medium">最新</van-tag>
            </div>
            <div v-if="updateAvailable && latestVersion" class="version-remote">
              <span class="version-label">远程版本:</span>
              <span class="version-remote-number">v{{ latestVersion }}</span>
            </div>
            <van-button :loading="checkingUpdate" size="small" type="primary" plain @click="checkUpdate" style="margin-top:8px">检查更新</van-button>
          </div>
        </div>

        <div v-if="activeSection === 'remote'" class="section">
          <TailscaleSettings />
        </div>

        <div class="save-area">
          <van-button type="primary" block @click="saveSettings">保存设置</van-button>
        </div>
      </div>
    </template>

    <template v-else>
      <div class="settings-mobile">
        <h2 class="mobile-title">设置</h2>
        <div class="mobile-section">
          <div class="mobile-section-title">通用</div>
          <van-cell-group inset>
            <van-cell title="镜像源" is-link @click="showMirrorPicker = true">
              <template #value><span>{{ mirrors.find(m => m.value === mirrorSource)?.label }}</span></template>
            </van-cell>
            <van-cell title="工作目录" is-link @click="chooseWorkspace">
              <template #value><span>{{ workDir || '未设置' }}</span></template>
            </van-cell>
            <van-cell v-if="workDir" title="打开目录" is-link @click="openExplorer" />
            <van-cell title="测试通知" is-link @click="testNotification" />
          </van-cell-group>
        </div>

        <div class="mobile-section">
          <div class="mobile-section-title">AI 配置</div>
          <van-cell-group inset>
            <van-cell title="提供商" is-link @click="showProviderPicker = true">
              <template #value><span>{{ providers.find(p => p.value === defaultProvider)?.label }}</span></template>
            </van-cell>
            <van-cell title="模型" is-link @click="showModelPicker = true">
              <template #value><span>{{ defaultModel || '未选择' }}</span></template>
            </van-cell>
            <van-cell title="温度"><template #value><span>{{ temperature.toFixed(1) }}</span></template></van-cell>
            <van-cell title="最大 Token"><template #value><span>{{ maxTokens }}</span></template></van-cell>
            <van-cell title="重置模型设置" is-link @click="clearModelSettings" />
          </van-cell-group>
        </div>

        <div class="mobile-section">
          <div class="mobile-section-title">系统</div>
          <van-cell-group inset>
            <template v-if="hardwareLoading"><van-cell title="硬件信息加载中..." /></template>
            <template v-else-if="hardwareEntries.length > 0">
              <van-cell v-for="entry in hardwareEntries" :key="entry.key" :title="entry.label" :label="entry.value" />
            </template>
            <template v-else><van-cell title="暂无硬件信息" is-link @click="loadHardware" /></template>
          </van-cell-group>
        </div>

        <div class="mobile-section">
          <div class="mobile-section-title">数据管理</div>
          <van-cell-group inset>
            <van-cell title="数据大小"><template #value><span>{{ appDataLoading ? '计算中...' : appDataSize }}</span></template></van-cell>
            <van-cell title="导出数据" is-link @click="exportAppData" />
            <van-cell title="导入数据" is-link @click="importAppData" />
          </van-cell-group>
        </div>

        <div class="mobile-section">
          <div class="mobile-section-title">版本</div>
          <van-cell-group inset>
            <van-cell title="当前版本" :value="`v${currentVersion}`" />
            <van-cell v-if="updateAvailable" title="新版本可用" :value="`v${latestVersion}`" />
            <van-cell title="检查更新" is-link @click="checkUpdate">
              <template #value><van-loading v-if="checkingUpdate" size="14px" /></template>
            </van-cell>
          </van-cell-group>
        </div>

        <div class="save-area">
          <van-button type="primary" block @click="saveSettings">保存设置</van-button>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.settings-view {
  display: flex;
  height: 100%;
  min-height: 0;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.sidebar {
  width: 170px;
  flex-shrink: 0;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  overflow-y: auto;
  padding: 8px 0;
}

.sidebar-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  cursor: pointer;
  color: var(--text-secondary);
  font-size: 13px;
  transition: background 0.15s, color 0.15s;
  border-left: 3px solid transparent;
}

.sidebar-item:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.sidebar-item.active {
  background: var(--accent-light);
  color: var(--accent);
  border-left-color: var(--accent);
}

.sidebar-label {
  flex: 1;
}

.content {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
  min-height: 0;
}

.section {
  margin-bottom: 4px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border);
}

.form-group {
  margin-bottom: 14px;
}

.form-label {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.dark-field {
  background: var(--bg-card) !important;
  border-radius: 6px;
  overflow: hidden;
}

.workspace-chooser {
  display: flex;
  gap: 8px;
  align-items: center;
}

.workspace-field {
  flex: 1;
}

.open-explorer-btn {
  margin-top: 8px;
}

.hardware-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 12px;
}

.hardware-row {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
  font-size: 13px;
}

.hardware-row + .hardware-row {
  border-top: 1px solid var(--border);
}

.h-label { color: var(--text-secondary); }
.h-value { color: var(--text-primary); font-family: Consolas, monospace; font-size: 12px; }

.info-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 0;
  font-size: 13px;
}

.muted { color: var(--text-muted); font-size: 13px; }

.action-row {
  display: flex;
  gap: 8px;
}

.version-card {
  padding: 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
}

.version-main {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 4px;
}

.version-number {
  font-size: 22px;
  font-weight: 700;
  color: var(--accent);
  font-family: 'Consolas', monospace;
}

.version-remote {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}

.version-label {
  color: var(--text-muted);
}

.version-remote-number {
  color: var(--success);
  font-weight: 600;
  font-family: 'Consolas', monospace;
}

.status-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 13px;
}

.save-area {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--border);
}

.settings-mobile {
  flex: 1;
  padding: 16px;
  overflow-y: auto;
}

.mobile-title {
  font-size: 18px;
  font-weight: 700;
  margin-bottom: 16px;
}

.mobile-section {
  margin-bottom: 16px;
}

.mobile-section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 8px;
  padding-left: 4px;
}

.settings-view.mobile .save-area {
  margin-top: 8px;
  padding-top: 12px;
}
</style>
