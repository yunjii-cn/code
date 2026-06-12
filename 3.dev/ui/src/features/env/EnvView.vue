<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import { showToast, showDialog } from 'vant'
import { envApi } from '@/api'
import { useDevice } from '@/composables/useDevice'

const { isMobile } = useDevice()

interface ComponentStatus {
  name: string
  key: string
  installed: boolean
  version?: string
  installing?: boolean
}

const components = ref<ComponentStatus[]>([
  { name: 'Node.js', key: 'nodejs', installed: false },
  { name: 'Bun', key: 'bun', installed: false },
  { name: 'uv', key: 'uv', installed: false },
  { name: '项目依赖', key: 'dependencies', installed: false },
  { name: 'API服务', key: 'api', installed: false },
])
const logs = ref<Array<{ time: string; message: string; type?: 'info' | 'success' | 'error' }>>([])
const deploying = ref(false)
const progress = ref(0)
const currentMirror = ref('default')
const logContainer = ref<HTMLElement>()

const mirrors = [
  { value: 'default', label: '默认源' },
  { value: 'npmmirror', label: 'npmmirror' },
  { value: 'tuna', label: '清华源' },
  { value: 'huawei', label: '华为源' },
]

const installedCount = computed(() => components.value.filter(c => c.installed).length)
const totalCount = computed(() => components.value.length)

function formatTime(date: Date) {
  const h = String(date.getHours()).padStart(2, '0')
  const m = String(date.getMinutes()).padStart(2, '0')
  const s = String(date.getSeconds()).padStart(2, '0')
  return `${h}:${m}:${s}`
}

function addLog(message: string, type: 'info' | 'success' | 'error' = 'info') {
  logs.value.push({ time: formatTime(new Date()), message, type })
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
  })
}

async function fetchStatus() {
  try {
    const data: any = await envApi.getStatus()
    if (data?.components) {
      for (const comp of components.value) {
        const serverComp = data.components.find((c: any) => c.key === comp.key)
        if (serverComp) {
          comp.installed = serverComp.installed
          comp.version = serverComp.version
        }
      }
    }
  } catch {
    addLog('获取组件状态失败', 'error')
  }
}

async function fetchMirror() {
  try {
    const data: any = await envApi.getMirror()
    if (data?.mirror) {
      currentMirror.value = data.mirror
    }
  } catch {
    // 使用默认值
  }
}

async function switchMirror(mirror: string) {
  try {
    await envApi.setMirror(mirror)
    currentMirror.value = mirror
    addLog(`镜像源已切换为: ${mirrors.find(m => m.value === mirror)?.label}`, 'success')
    showToast('镜像源切换成功')
  } catch {
    addLog('镜像源切换失败', 'error')
    showToast('切换失败')
  }
}

async function installComponent(comp: ComponentStatus) {
  if (comp.installing || deploying.value) return
  comp.installing = true
  addLog(`开始安装 ${comp.name}...`)
  try {
    await envApi.installComponent(comp.key)
    comp.installed = true
    addLog(`${comp.name} 安装完成`, 'success')
    showToast(`${comp.name} 安装成功`)
  } catch (e: any) {
    addLog(`${comp.name} 安装失败: ${e.message}`, 'error')
    showToast(`安装失败: ${e.message}`)
  } finally {
    comp.installing = false
  }
}

async function installAll() {
  if (deploying.value) return
  deploying.value = true
  progress.value = 0
  addLog('开始一键部署...')
  try {
    const installPromise = envApi.installAll()
    const uninstalled = components.value.filter(c => !c.installed)
    const step = 100 / (uninstalled.length || 1)
    let current = 0
    for (const comp of uninstalled) {
      comp.installing = true
      addLog(`正在安装 ${comp.name}...`)
      current += step
      progress.value = Math.min(Math.round(current), 95)
    }
    await installPromise
    for (const comp of components.value) {
      comp.installed = true
      comp.installing = false
    }
    progress.value = 100
    addLog('所有组件部署完成！', 'success')
    showToast('部署完成')
  } catch (e: any) {
    addLog(`部署失败: ${e.message}`, 'error')
    showDialog({ title: '部署失败', message: e.message })
  } finally {
    deploying.value = false
    setTimeout(() => {
      progress.value = 0
    }, 2000)
  }
}

onMounted(async () => {
  await fetchStatus()
  await fetchMirror()
})
</script>

<template>
  <!-- 手机端提示 -->
  <div v-if="isMobile" class="env-mobile">
    <van-empty
      image="search"
      description="环境部署仅支持桌面端"
    />
  </div>

  <!-- 桌面端布局 -->
  <div v-else class="env-desktop">
    <!-- 左侧：组件状态 -->
    <div class="env-left">
      <div class="panel-header">
        <span class="panel-title">组件状态</span>
        <van-tag type="primary" size="medium">{{ installedCount }}/{{ totalCount }}</van-tag>
      </div>

      <div class="component-list">
        <div
          v-for="comp in components"
          :key="comp.key"
          class="component-item"
          :class="{ installed: comp.installed, installing: comp.installing }"
        >
          <div class="comp-status">
            <van-icon
              :name="comp.installed ? 'success' : 'cross'"
              :class="comp.installed ? 'icon-success' : 'icon-fail'"
              size="18"
            />
          </div>
          <div class="comp-info">
            <span class="comp-name">{{ comp.name }}</span>
            <span v-if="comp.version" class="comp-version">{{ comp.version }}</span>
          </div>
          <van-button
            size="mini"
            :type="comp.installed ? 'default' : 'primary'"
            :loading="comp.installing"
            :disabled="deploying"
            plain
            @click="installComponent(comp)"
          >
            {{ comp.installed ? '已安装' : '安装' }}
          </van-button>
        </div>
      </div>

      <!-- 进度条 -->
      <div v-if="deploying" class="progress-area">
        <van-progress :percentage="progress" :show-pivot="true" color="#6c63ff" track-color="#2a2a4a" stroke-width="6" />
      </div>

      <!-- 镜像源 -->
      <div class="mirror-area">
        <div class="mirror-label">镜像源</div>
        <van-radio-group v-model="currentMirror" direction="horizontal" class="mirror-group" @change="switchMirror">
          <van-radio
            v-for="m in mirrors"
            :key="m.value"
            :name="m.value"
            class="mirror-radio"
          >
            {{ m.label }}
          </van-radio>
        </van-radio-group>
      </div>

      <!-- 一键部署 -->
      <div class="deploy-area">
        <van-button
          type="primary"
          block
          :loading="deploying"
          loading-text="部署中..."
          @click="installAll"
        >
          一键部署
        </van-button>
      </div>
    </div>

    <!-- 右侧：部署日志 -->
    <div class="env-right">
      <div class="panel-header">
        <span class="panel-title">部署日志</span>
        <van-button size="mini" plain @click="logs = []">清空</van-button>
      </div>
      <div ref="logContainer" class="log-container">
        <div
          v-for="(log, i) in logs"
          :key="i"
          class="log-line"
          :class="log.type"
        >
          <span class="log-time">[{{ log.time }}]</span>
          <span class="log-msg">{{ log.message }}</span>
        </div>
        <div v-if="logs.length === 0" class="log-empty">暂无日志</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.env-mobile {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-primary);
}

:deep(.van-empty__description) {
  color: var(--text-muted);
}

.env-desktop {
  height: 100vh;
  display: flex;
  background: var(--bg-primary);
  color: #e0e0e0;
}

.env-left {
  width: 320px;
  flex-shrink: 0;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  padding: 20px;
  gap: 16px;
  overflow-y: auto;
}

.env-right {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 20px;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.panel-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.component-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.component-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: var(--bg-card);
  border-radius: 10px;
  border: 1px solid var(--border);
  transition: border-color 0.2s;
}

.component-item.installed {
  border-color: var(--success-border);
}

.component-item.installing {
  border-color: var(--accent-border);
}

.comp-status {
  flex-shrink: 0;
}

.icon-success {
  color: var(--success);
}

.icon-fail {
  color: var(--error);
}

.comp-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.comp-name {
  font-size: 14px;
  color: var(--text-primary);
}

.comp-version {
  font-size: 11px;
  color: var(--accent);
}

.progress-area {
  padding: 4px 0;
}

:deep(.van-progress) {
  background: var(--border);
  border-radius: 4px;
}

.mirror-area {
  padding: 8px 0;
}

.mirror-label {
  font-size: 13px;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.mirror-group {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

:deep(.mirror-radio .van-radio__label) {
  color: var(--text-secondary);
  font-size: 12px;
}

:deep(.mirror-radio .van-radio__icon--checked .van-icon) {
  background-color: var(--accent);
  border-color: var(--accent);
}

.deploy-area {
  margin-top: auto;
  padding-top: 12px;
}

:deep(.van-button--primary) {
  background: var(--accent);
  border-color: var(--accent);
}

:deep(.van-button--plain.van-button--primary) {
  background: transparent;
  border-color: var(--accent);
  color: var(--accent);
}

:deep(.van-button--default) {
  background: var(--border);
  border-color: var(--border-light);
  color: var(--text-muted);
}

:deep(.van-tag--primary) {
  background: var(--accent-light);
  border-color: var(--accent-border);
  color: #a0a0ff;
}

.log-container {
  flex: 1;
  background: var(--bg-primary);
  border-radius: 10px;
  padding: 14px;
  overflow-y: auto;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 12px;
  line-height: 1.8;
}

.log-line {
  display: flex;
  gap: 8px;
}

.log-time {
  color: var(--text-muted);
  flex-shrink: 0;
}

.log-msg {
  color: var(--text-secondary);
  word-break: break-all;
}

.log-line.success .log-msg {
  color: var(--success);
}

.log-line.error .log-msg {
  color: var(--error);
}

.log-empty {
  color: var(--text-muted);
  text-align: center;
  padding-top: 40px;
}
</style>
