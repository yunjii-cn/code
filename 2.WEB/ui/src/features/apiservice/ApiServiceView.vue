<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { showToast } from 'vant'
import { envApi } from '@/api'
import { useDevice } from '@/composables/useDevice'

const { isMobile } = useDevice()

interface ApiService {
  name: string
  status: 'running' | 'stopped' | 'starting'
  port?: number
  base_url?: string
  description?: string
  version?: string
  pid?: number
  uptime?: string
}

const services = ref<ApiService[]>([])
const loading = ref(false)
const operatingServices = ref<Set<string>>(new Set())
const expandedNames = ref<string[]>([])
let pollTimer: ReturnType<typeof setInterval> | null = null
let visibilityHandler: (() => void) | null = null

const runningCount = computed(() => services.value.filter(s => s.status === 'running').length)
const totalCount = computed(() => services.value.length)

function mapService(raw: any): ApiService {
  return {
    name: raw.name || raw.service_name || '',
    status: raw.status === 'running' ? 'running' : raw.status === 'starting' ? 'starting' : 'stopped',
    port: raw.port,
    base_url: raw.base_url,
    description: raw.description,
    version: raw.version,
    pid: raw.pid,
    uptime: raw.uptime,
  }
}

async function fetchServices() {
  loading.value = true
  try {
    const data: any = await envApi.listServices()
    const list = Array.isArray(data) ? data : data?.services || data?.data || []
    services.value = list.map(mapService)
  } catch {
    services.value = []
    showToast('获取服务列表失败')
  } finally {
    loading.value = false
  }
}

async function fetchServiceDetail(serviceName: string) {
  try {
    const data: any = await envApi.getServiceInfo(serviceName)
    if (data) {
      const idx = services.value.findIndex(s => s.name === serviceName)
      if (idx !== -1) {
        services.value[idx] = mapService(data)
      }
    }
  } catch {
    // silent
  }
}

async function startService(svc: ApiService) {
  if (operatingServices.value.has(svc.name)) return
  operatingServices.value.add(svc.name)
  const prev = svc.status
  svc.status = 'starting'
  try {
    await envApi.startService({ service_name: svc.name })
    svc.status = 'running'
    showToast(`${svc.name} 启动成功`)
    await fetchServiceDetail(svc.name)
  } catch (e: any) {
    svc.status = prev
    showToast(`启动失败: ${e.message}`)
  } finally {
    operatingServices.value.delete(svc.name)
  }
}

async function stopService(svc: ApiService) {
  if (operatingServices.value.has(svc.name)) return
  operatingServices.value.add(svc.name)
  try {
    await envApi.stopService({ service_name: svc.name, base_url: svc.base_url })
    svc.status = 'stopped'
    showToast(`${svc.name} 已停止`)
    await fetchServiceDetail(svc.name)
  } catch (e: any) {
    showToast(`停止失败: ${e.message}`)
  } finally {
    operatingServices.value.delete(svc.name)
  }
}

async function startAllServices() {
  const stopped = services.value.filter(s => s.status !== 'running')
  if (stopped.length === 0) {
    showToast('所有服务已在运行')
    return
  }
  loading.value = true
  try {
    await envApi.startAllServices()
    showToast('已启动全部服务')
    await fetchServices()
  } catch (e: any) {
    showToast(`启动失败: ${e.message}`)
  } finally {
    loading.value = false
  }
}

async function toggleService(svc: ApiService) {
  if (svc.status === 'running') {
    await stopService(svc)
  } else {
    await startService(svc)
  }
}

function statusDotClass(status: string) {
  if (status === 'running') return 'dot-running'
  if (status === 'starting') return 'dot-starting'
  return 'dot-stopped'
}

function statusTagType(status: string) {
  if (status === 'running') return 'success'
  if (status === 'starting') return 'warning'
  return 'danger'
}

function statusLabel(status: string) {
  if (status === 'running') return '运行中'
  if (status === 'starting') return '启动中'
  return '已停止'
}

function startPolling() {
  stopPolling()
  pollTimer = setInterval(() => {
    fetchServices()
  }, 10000)
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

function handleVisibilityChange() {
  if (document.hidden) {
    stopPolling()
  } else {
    fetchServices()
    startPolling()
  }
}

onMounted(async () => {
  await fetchServices()
  startPolling()
  visibilityHandler = handleVisibilityChange
  document.addEventListener('visibilitychange', visibilityHandler)
})

onUnmounted(() => {
  stopPolling()
  if (visibilityHandler) {
    document.removeEventListener('visibilitychange', visibilityHandler)
    visibilityHandler = null
  }
})
</script>

<template>
  <div class="api-service-view" :class="{ mobile: isMobile }">
    <div class="page-header">
      <div class="header-left">
        <h2 class="page-title">API 服务管理</h2>
        <van-tag size="medium" :type="runningCount === totalCount && totalCount > 0 ? 'success' : 'primary'">
          {{ runningCount }}/{{ totalCount }} 运行中
        </van-tag>
      </div>
      <div class="header-actions">
        <van-button
          size="small"
          plain
          @click="fetchServices"
        >
          刷新
        </van-button>
        <van-button
          size="small"
          type="primary"
          :loading="loading"
          @click="startAllServices"
        >
          全部启动
        </van-button>
      </div>
    </div>

    <van-loading v-if="loading && services.length === 0" class="page-loading" color="var(--accent)" size="24px" vertical>
      加载中...
    </van-loading>

    <van-empty
      v-else-if="services.length === 0"
      image="search"
      description="暂无 API 服务"
    />

    <template v-else>
      <!-- Desktop: grid layout -->
      <div v-if="!isMobile" class="service-grid">
        <div
          v-for="svc in services"
          :key="svc.name"
          class="service-card"
          :class="{ 'card-running': svc.status === 'running', 'card-starting': svc.status === 'starting' }"
        >
          <div class="card-top">
            <div class="card-title-row">
              <span class="status-dot" :class="statusDotClass(svc.status)" />
              <span class="service-name">{{ svc.name }}</span>
            </div>
            <van-tag :type="statusTagType(svc.status)" size="medium">
              {{ statusLabel(svc.status) }}
            </van-tag>
          </div>

          <div class="card-meta">
            <div v-if="svc.base_url" class="meta-row">
              <van-icon name="link-o" size="14" color="var(--text-muted)" />
              <span class="meta-value url-value">{{ svc.base_url }}</span>
            </div>
            <div v-if="svc.port" class="meta-row">
              <van-icon name="cluster-o" size="14" color="var(--text-muted)" />
              <span class="meta-value">端口 {{ svc.port }}</span>
            </div>
            <div v-if="svc.version" class="meta-row">
              <van-icon name="label-o" size="14" color="var(--text-muted)" />
              <span class="meta-value">{{ svc.version }}</span>
            </div>
          </div>

          <div class="card-actions">
            <van-button
              size="small"
              :type="svc.status === 'running' ? 'danger' : 'primary'"
              :loading="operatingServices.has(svc.name)"
              plain
              @click="toggleService(svc)"
            >
              {{ svc.status === 'running' ? '停止' : '启动' }}
            </van-button>
          </div>

          <van-collapse v-model="expandedNames" class="card-detail-collapse">
            <van-collapse-item :name="svc.name" title="详细信息">
              <div class="detail-grid">
                <div class="detail-item">
                  <span class="detail-label">服务名称</span>
                  <span class="detail-value">{{ svc.name }}</span>
                </div>
                <div class="detail-item">
                  <span class="detail-label">状态</span>
                  <span class="detail-value" :class="'status-' + svc.status">{{ statusLabel(svc.status) }}</span>
                </div>
                <div v-if="svc.base_url" class="detail-item">
                  <span class="detail-label">Base URL</span>
                  <span class="detail-value">{{ svc.base_url }}</span>
                </div>
                <div v-if="svc.port" class="detail-item">
                  <span class="detail-label">端口</span>
                  <span class="detail-value">{{ svc.port }}</span>
                </div>
                <div v-if="svc.version" class="detail-item">
                  <span class="detail-label">版本</span>
                  <span class="detail-value">{{ svc.version }}</span>
                </div>
                <div v-if="svc.pid" class="detail-item">
                  <span class="detail-label">PID</span>
                  <span class="detail-value">{{ svc.pid }}</span>
                </div>
                <div v-if="svc.uptime" class="detail-item">
                  <span class="detail-label">运行时间</span>
                  <span class="detail-value">{{ svc.uptime }}</span>
                </div>
                <div v-if="svc.description" class="detail-item detail-full">
                  <span class="detail-label">描述</span>
                  <span class="detail-value">{{ svc.description }}</span>
                </div>
              </div>
            </van-collapse-item>
          </van-collapse>
        </div>
      </div>

      <!-- Mobile: stacked list -->
      <div v-else class="service-list">
        <van-cell-group inset>
          <van-cell
            v-for="svc in services"
            :key="svc.name"
            :title="svc.name"
            :label="svc.base_url || (svc.port ? `端口 ${svc.port}` : '')"
            :value="statusLabel(svc.status)"
            is-link
            @click="toggleService(svc)"
          >
            <template #icon>
              <span class="status-dot cell-dot" :class="statusDotClass(svc.status)" />
            </template>
            <template #right-icon>
              <van-button
                size="mini"
                :type="svc.status === 'running' ? 'danger' : 'primary'"
                :loading="operatingServices.has(svc.name)"
                plain
                @click.stop="toggleService(svc)"
              >
                {{ svc.status === 'running' ? '停止' : '启动' }}
              </van-button>
            </template>
          </van-cell>
        </van-cell-group>
      </div>
    </template>
  </div>
</template>

<style scoped>
.api-service-view {
  height: 100%;
  overflow-y: auto;
  padding: 24px;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.api-service-view.mobile {
  padding: 16px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
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

.page-loading {
  display: flex;
  justify-content: center;
  padding: 40px 0;
}

:deep(.van-empty__description) {
  color: var(--text-muted);
}

.status-dot {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dot-running {
  background: var(--success);
  box-shadow: 0 0 6px rgba(76, 175, 80, 0.5);
}

.dot-stopped {
  background: var(--error);
  box-shadow: 0 0 6px rgba(244, 67, 54, 0.4);
}

.dot-starting {
  background: var(--warning);
  box-shadow: 0 0 6px rgba(255, 152, 0, 0.4);
  animation: pulse 1s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.service-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}

.service-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  transition: border-color 0.25s, box-shadow 0.25s;
}

.service-card:hover {
  border-color: rgba(66, 165, 245, 0.3);
  box-shadow: 0 4px 20px var(--shadow);
}

.service-card.card-running {
  border-color: rgba(76, 175, 80, 0.25);
}

.service-card.card-starting {
  border-color: rgba(255, 152, 0, 0.3);
}

.card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
}

.service-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-meta {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.meta-row {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.meta-value {
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.url-value {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  color: var(--accent-light);
}

.card-actions {
  display: flex;
  justify-content: flex-end;
}

.card-detail-collapse {
  border: none;
  margin: 0 -16px -16px;
}

:deep(.card-detail-collapse .van-collapse-item__title) {
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  padding: 8px 16px;
}

:deep(.card-detail-collapse .van-collapse-item__title::after) {
  border-color: var(--border);
}

:deep(.card-detail-collapse .van-cell__right-icon) {
  color: var(--text-muted);
}

:deep(.card-detail-collapse .van-collapse-item__wrapper) {
  background: var(--bg-input);
}

:deep(.card-detail-collapse .van-collapse-item__content) {
  background: var(--bg-input);
  padding: 12px 16px;
}

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.detail-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.detail-full {
  grid-column: 1 / -1;
}

.detail-label {
  font-size: 11px;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.detail-value {
  font-size: 13px;
  color: var(--text-primary);
  word-break: break-all;
}

.status-running {
  color: var(--success);
}

.status-stopped {
  color: var(--error);
}

.status-starting {
  color: var(--warning);
}

/* Mobile list styles */
.service-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.cell-dot {
  margin-right: 10px;
  margin-top: 2px;
}

:deep(.van-cell-group--inset) {
  margin: 0;
  border-radius: 10px;
  overflow: hidden;
}

:deep(.van-cell) {
  background: var(--bg-card);
  color: var(--text-primary);
  padding: 12px 14px;
}

:deep(.van-cell::after) {
  border-color: var(--border);
}

:deep(.van-cell__title) {
  color: var(--text-primary);
  font-weight: 500;
}

:deep(.van-cell__label) {
  color: var(--text-muted);
  font-size: 11px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
}

:deep(.van-cell__value) {
  color: var(--text-secondary);
  font-size: 12px;
}

/* Vant tag overrides */
:deep(.van-tag--success) {
  background: rgba(76, 175, 80, 0.15);
  border-color: rgba(76, 175, 80, 0.4);
  color: var(--success);
}

:deep(.van-tag--danger) {
  background: rgba(244, 67, 54, 0.15);
  border-color: rgba(244, 67, 54, 0.4);
  color: var(--error);
}

:deep(.van-tag--warning) {
  background: rgba(255, 152, 0, 0.15);
  border-color: rgba(255, 152, 0, 0.4);
  color: var(--warning);
}

:deep(.van-tag--primary) {
  background: rgba(66, 165, 245, 0.15);
  border-color: rgba(66, 165, 245, 0.4);
  color: var(--accent);
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

:deep(.van-button--plain.van-button--danger) {
  background: transparent;
  border-color: var(--error);
  color: var(--error);
}

:deep(.van-button--default) {
  background: var(--bg-card);
  border-color: var(--border);
  color: var(--text-secondary);
}

:deep(.van-loading__text) {
  color: var(--text-secondary);
}
</style>
