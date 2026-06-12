<!--
  VersionView.vue
  2026-06-08 TASK-1.4 引入：迁移到 features/version/
  原位置: src/views/VersionView.vue
  保留: src/views/VersionView.vue 作为 re-export shim（兼容期）
-->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { useDevice } from '@/composables/useDevice'
import { versionApi } from '@/api'

const { isMobile, isDesktop } = useDevice()

const activeTab = ref<'update' | 'history' | 'commits' | 'stable'>('update')

const currentVersion = ref('')
const remoteVersion = ref('')
const updateAvailable = ref(false)
const checkingUpdate = ref(false)
const downloading = ref(false)
const downloadProgress = ref(0)
const downloadSource = ref('gitee')

const versionHistory = ref<Array<{ version: string; date: string; description?: string }>>([])
const gitCommits = ref<Array<{ hash: string; message: string; author: string; date: string }>>([])
const stableExes = ref<Array<{ name: string; path: string; version?: string; date?: string }>>([])

const loadingHistory = ref(false)
const loadingCommits = ref(false)
const loadingExes = ref(false)

const showSourceSheet = ref(false)

const sourceOptions = [
  { name: 'Gitee（国内加速）', value: 'gitee' },
  { name: 'GitHub', value: 'github' },
]

const tabs = [
  { key: 'update' as const, label: '更新', icon: 'upgrade' },
  { key: 'history' as const, label: '版本历史', icon: 'clock-o' },
  { key: 'commits' as const, label: '提交记录', icon: 'cluster-o' },
  { key: 'stable' as const, label: '稳定版本', icon: 'shield-o' },
]

// 2026-06-08 TASK-1.4 修复：原内联对象推导漏了 'update'，改为 Record 显式声明
const noop = () => {}
const tabLoaders: Record<typeof tabs[number]['key'], () => void> = {
  update: noop,
  history: loadHistory,
  commits: loadGitCommits,
  stable: loadStableExes,
}

async function loadCurrentVersion() {
  try {
    const data: any = await versionApi.getCurrent()
    if (data?.version) currentVersion.value = data.version
  } catch {
    currentVersion.value = '未知'
  }
}

async function loadRemoteVersion() {
  try {
    const data: any = await versionApi.getRemote()
    if (data?.version) remoteVersion.value = data.version
  } catch {}
}

async function checkUpdate() {
  checkingUpdate.value = true
  try {
    const data: any = await versionApi.checkUpdate()
    updateAvailable.value = data?.updateAvailable ?? false
    if (data?.latestVersion) remoteVersion.value = data.latestVersion
    if (updateAvailable.value) {
      showToast(`发现新版本: ${data.latestVersion}`)
    } else {
      showToast('已是最新版本')
    }
  } catch {
    showToast('检查更新失败')
  } finally {
    checkingUpdate.value = false
  }
}

async function downloadUpdate() {
  showSourceSheet.value = true
}

async function doDownload(source: string) {
  showSourceSheet.value = false
  downloading.value = true
  downloadProgress.value = 0
  try {
    const progressInterval = setInterval(() => {
      if (downloadProgress.value < 90) downloadProgress.value += Math.random() * 10
    }, 500)
    await versionApi.downloadUpdate(source)
    clearInterval(progressInterval)
    downloadProgress.value = 100
    showToast('下载完成')
  } catch (e: any) {
    showToast(`下载失败: ${e.message}`)
  } finally {
    setTimeout(() => {
      downloading.value = false
      downloadProgress.value = 0
    }, 1500)
  }
}

async function pullUpdate() {
  try {
    await showConfirmDialog({ title: '拉取更新', message: '确定要从远程仓库拉取最新代码更新吗？' })
    await versionApi.pullUpdate()
    showToast('更新拉取成功')
    await loadCurrentVersion()
  } catch {}
}

async function loadHistory() {
  loadingHistory.value = true
  try {
    const data: any = await versionApi.getHistory()
    versionHistory.value = (data?.versions || data || []).map((v: any) => ({
      version: v.version || v.name,
      date: v.date || v.created_at || '',
      description: v.description || v.changelog || '',
    }))
  } catch {
    versionHistory.value = []
  } finally {
    loadingHistory.value = false
  }
}

async function loadGitCommits() {
  loadingCommits.value = true
  try {
    const data: any = await versionApi.getGitHistory(30)
    gitCommits.value = (data?.commits || data || []).map((c: any) => ({
      hash: c.hash || c.commit?.substring(0, 8) || '',
      message: c.message || c.subject || '',
      author: c.author || '',
      date: c.date || c.committed_date || '',
    }))
  } catch {
    gitCommits.value = []
  } finally {
    loadingCommits.value = false
  }
}

async function loadStableExes() {
  loadingExes.value = true
  try {
    const data: any = await versionApi.listStableExes()
    stableExes.value = (data?.exes || data || []).map((e: any) => ({
      name: e.name || e.filename || '',
      path: e.path || e.exe_path || '',
      version: e.version || '',
      date: e.date || e.modified || '',
    }))
  } catch {
    stableExes.value = []
  } finally {
    loadingExes.value = false
  }
}

async function switchCommit(hash: string) {
  try {
    await showConfirmDialog({ title: '切换版本', message: `确定要切换到提交 ${hash} 吗？` })
    await versionApi.switchCommit(hash)
    showToast('切换成功')
    await loadCurrentVersion()
  } catch {}
}

async function switchExe(exe: { name: string; path: string }) {
  try {
    await showConfirmDialog({ title: '切换版本', message: `确定要切换到 ${exe.name} 吗？` })
    await versionApi.switchExe({ exe_path: exe.path })
    showToast('切换成功')
    await loadCurrentVersion()
  } catch {}
}

function shortHash(hash: string): string {
  return hash.length > 8 ? hash.substring(0, 8) : hash
}

onMounted(async () => {
  await loadCurrentVersion()
  await loadRemoteVersion()
})
</script>

<template>
  <div class="version-view" :class="{ mobile: isMobile }">
    <div class="page-header">
      <h2 class="page-title">版本管理</h2>
      <div class="header-actions">
        <van-button size="small" plain @click="pullUpdate">拉取更新</van-button>
        <van-button size="small" type="primary" @click="checkUpdate" :loading="checkingUpdate">检查更新</van-button>
      </div>
    </div>

    <div class="tab-nav">
      <div
        v-for="tab in tabs"
        :key="tab.key"
        class="tab-btn"
        :class="{ active: activeTab === tab.key }"
        @click="activeTab = tab.key; (tabLoaders[tab.key] ?? noop)()"
      >
        <van-icon :name="tab.icon" size="14" />
        <span>{{ tab.label }}</span>
      </div>
    </div>

    <div class="tab-content">
      <!-- Update Tab -->
      <div v-if="activeTab === 'update'" class="tab-panel">
        <div class="version-card">
          <div class="version-main">
            <span class="version-number">v{{ currentVersion }}</span>
            <van-tag v-if="updateAvailable" type="success" size="medium">新版本可用</van-tag>
            <van-tag v-else type="primary" size="medium">最新</van-tag>
          </div>
          <div v-if="updateAvailable && remoteVersion" class="version-remote">
            <span class="version-label">远程版本:</span>
            <span class="version-remote-number">v{{ remoteVersion }}</span>
          </div>
          <div class="version-actions">
            <van-button size="small" type="primary" :disabled="!updateAvailable" :loading="downloading" @click="downloadUpdate">下载更新</van-button>
          </div>
          <div v-if="downloading" class="download-progress">
            <van-progress :percentage="Math.round(downloadProgress)" :show-pivot="false" color="var(--accent)" track-color="var(--bg-secondary)" stroke-width="6" />
            <span class="progress-text">{{ Math.round(downloadProgress) }}%</span>
          </div>
        </div>
      </div>

      <!-- History Tab -->
      <div v-if="activeTab === 'history'" class="tab-panel">
        <van-loading v-if="loadingHistory" class="page-loading" color="var(--accent)" vertical>加载中...</van-loading>
        <van-empty v-else-if="versionHistory.length === 0" description="暂无版本历史" image="search" />
        <div v-else class="history-list">
          <div v-for="item in versionHistory" :key="item.version" class="history-item">
            <div class="history-version">
              <van-tag type="primary" size="medium">v{{ item.version }}</van-tag>
            </div>
            <div class="history-info">
              <span v-if="item.description" class="history-desc">{{ item.description }}</span>
              <span v-if="item.date" class="history-date">{{ item.date }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Commits Tab -->
      <div v-if="activeTab === 'commits'" class="tab-panel">
        <van-loading v-if="loadingCommits" class="page-loading" color="var(--accent)" vertical>加载中...</van-loading>
        <van-empty v-else-if="gitCommits.length === 0" description="暂无提交记录" image="search" />
        <div v-else class="commit-list">
          <div v-for="item in gitCommits" :key="item.hash" class="commit-item">
            <div class="commit-hash">{{ shortHash(item.hash) }}</div>
            <div class="commit-message">{{ item.message }}</div>
            <div class="commit-meta">
              <span v-if="item.author">{{ item.author }}</span>
              <span v-if="item.date">{{ item.date }}</span>
            </div>
            <van-button size="mini" plain class="commit-switch-btn" @click="switchCommit(item.hash)">切换</van-button>
          </div>
        </div>
      </div>

      <!-- Stable Tab -->
      <div v-if="activeTab === 'stable'" class="tab-panel">
        <van-loading v-if="loadingExes" class="page-loading" color="var(--accent)" vertical>加载中...</van-loading>
        <van-empty v-else-if="stableExes.length === 0" description="暂无稳定版本" image="search" />
        <div v-else class="exe-list">
          <div v-for="item in stableExes" :key="item.path" class="exe-item">
            <div class="exe-info">
              <span class="exe-name">{{ item.name }}</span>
              <span v-if="item.version" class="exe-version">v{{ item.version }}</span>
              <span v-if="item.date" class="exe-date">{{ item.date }}</span>
            </div>
            <van-button size="mini" type="primary" @click="switchExe(item)">切换</van-button>
          </div>
        </div>
      </div>
    </div>

    <van-action-sheet
      v-model:show="showSourceSheet"
      title="选择下载源"
      :actions="sourceOptions.map(s => ({ name: s.name, value: s.value }))"
      @select="(action: any) => doDownload(action.value)"
    />
  </div>
</template>

<style scoped>
.version-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 24px;
  background: var(--bg-primary);
  color: var(--text-primary);
  overflow: hidden;
}

.version-view.mobile {
  padding: 16px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
  flex-shrink: 0;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.tab-nav {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.tab-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  font-size: 13px;
  color: var(--text-muted);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
}

.tab-btn:hover {
  color: var(--text-primary);
}

.tab-btn.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.tab-content {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

.tab-panel {
  padding-bottom: 16px;
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
  margin-bottom: 8px;
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
  margin-bottom: 12px;
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

.version-actions {
  margin-bottom: 8px;
}

.download-progress {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 8px;
}

.download-progress :deep(.van-progress) {
  flex: 1;
}

.progress-text {
  font-size: 12px;
  color: var(--text-secondary);
  font-family: 'Consolas', monospace;
  min-width: 36px;
  text-align: right;
}

.page-loading {
  display: flex;
  justify-content: center;
  padding: 40px 0;
}

.history-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.history-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 10px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
}

.history-version {
  flex-shrink: 0;
}

.history-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.history-desc {
  font-size: 13px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.history-date {
  font-size: 11px;
  color: var(--text-muted);
}

.commit-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.commit-item {
  position: relative;
  padding: 10px 14px;
  padding-right: 70px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
}

.commit-hash {
  font-family: 'Consolas', monospace;
  font-size: 12px;
  color: var(--accent);
  margin-bottom: 2px;
}

.commit-message {
  font-size: 13px;
  color: var(--text-primary);
  margin-bottom: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.commit-meta {
  display: flex;
  gap: 12px;
  font-size: 11px;
  color: var(--text-muted);
}

.commit-switch-btn {
  position: absolute;
  right: 10px;
  top: 50%;
  transform: translateY(-50%);
}

.exe-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.exe-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
}

.exe-info {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
}

.exe-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.exe-version {
  font-size: 12px;
  color: var(--accent);
  font-family: 'Consolas', monospace;
}

.exe-date {
  font-size: 11px;
  color: var(--text-muted);
}

:deep(.van-button--primary) {
  background: var(--accent);
  border-color: var(--accent);
}

:deep(.van-button--plain) {
  background: transparent;
  border-color: var(--border);
  color: var(--text-secondary);
}

:deep(.van-tag--primary) {
  background: var(--accent-light);
  border-color: var(--accent-border);
  color: var(--accent);
}

:deep(.van-tag--success) {
  background: var(--success-light);
  border-color: var(--success-border);
  color: var(--success);
}

:deep(.van-empty__description) {
  color: var(--text-muted);
}

:deep(.van-action-sheet) {
  background: var(--bg-card);
}

:deep(.van-action-sheet__header) {
  color: var(--text-primary);
}

:deep(.van-action-sheet__item) {
  color: var(--text-primary);
  background: var(--bg-card);
}

:deep(.van-action-sheet__item:active) {
  background: var(--bg-card-hover);
}
</style>
