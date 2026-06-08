<!--
  ResponsivePanel.vue
  2026-06-09 TASK-3.6 引入：主动感知面板 UI
  功能：
    - 4 tab 切换（file_change / code_quality / security_risk / progress）
    - WebSocket 实时订阅（10s 自动扫 + scan_now 立即触发）
    - 通知卡片：图标 + 严重度色 + 标题 + 描述 + 动作按钮
    - 启动/停止引擎 / 立即扫描 / dismiss
    - 自动重连机制
    - 严重通知自动晋升到全局通知中心
-->
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { showToast } from 'vant'
import {
  responsiveApi,
  type ResponsiveNotification,
  type ResponsiveEngineStatus,
} from '@/api'
import { useNotificationStore } from '@shared/components'

const notifStore = useNotificationStore()

const notifications = ref<ResponsiveNotification[]>([])
const dismissedIds = ref<Set<string>>(new Set())
const activeTab = ref<string>('all')  // 'all' / 'file_change' / 'code_quality' / 'security_risk' / 'progress'
const engineStatus = ref<ResponsiveEngineStatus | null>(null)
const loading = ref(false)
const scanning = ref(false)
const wsState = ref<'disconnected' | 'connecting' | 'connected'>('disconnected')

let ws: WebSocket | null = null
let reconnectTimer: number | null = null
const WS_INTERVAL_S = 10

const TYPE_META: Record<string, { label: string; icon: string; color: string }> = {
  file_change:  { label: '文件变更', icon: '📝', color: '#3b82f6' },
  code_quality:  { label: '代码质量', icon: '🔍', color: '#f59e0b' },
  security_risk: { label: '依赖风险', icon: '🛡️', color: '#ef4444' },
  progress:      { label: '会话进度', icon: '▶️', color: '#10b981' },
}

const SEVERITY_META: Record<string, { label: string; color: string }> = {
  info:    { label: '提示', color: '#3b82f6' },
  warning: { label: '警告', color: '#f59e0b' },
  error:   { label: '错误', color: '#ef4444' },
}

const filteredNotifications = computed(() => {
  if (activeTab.value === 'all') return notifications.value
  return notifications.value.filter(n => n.type === activeTab.value)
})

const typeCounts = computed(() => {
  const counts: Record<string, number> = { all: notifications.value.length, file_change: 0, code_quality: 0, security_risk: 0, progress: 0 }
  for (const n of notifications.value) {
    if (counts[n.type] !== undefined) counts[n.type]++
  }
  return counts
})

function visibleNotifs() {
  return filteredNotifications.value.filter(n => !dismissedIds.value.has(n.id))
}

async function loadInitial() {
  loading.value = true
  try {
    const [statusRes, notifsRes] = await Promise.all([
      responsiveApi.status(),
      responsiveApi.notifications({ limit: 100 }),
    ])
    engineStatus.value = (statusRes as { data: ResponsiveEngineStatus }).data
    const list = ((notifsRes as { data: ResponsiveNotification[] }).data) || []
    notifications.value = list
  } catch (e: unknown) {
    showToast(`加载失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    loading.value = false
  }
}

async function handleStart() {
  try {
    await responsiveApi.start()
    showToast('引擎已启动')
    await loadInitial()
    connectWs()
  } catch (e: unknown) {
    showToast(`启动失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handleStop() {
  try {
    await responsiveApi.stop()
    showToast('引擎已停止')
    await loadInitial()
    disconnectWs()
  } catch (e: unknown) {
    showToast(`停止失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handleScanNow() {
  scanning.value = true
  try {
    const res = await responsiveApi.scan('all') as { data: ResponsiveNotification[] }
    const newOnes = res.data || []
    // 合并：按 id 去重
    const existing = new Map(notifications.value.map(n => [n.id, n]))
    for (const n of newOnes) {
      if (!existing.has(n.id)) {
        existing.set(n.id, n)
        promoteIfSevere(n)
      }
    }
    notifications.value = Array.from(existing.values()).sort((a, b) => b.created_at - a.created_at)
    showToast(`扫描完成：发现 ${newOnes.length} 条新通知`)
  } catch (e: unknown) {
    showToast(`扫描失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    scanning.value = false
  }
}

async function handleDismiss(n: ResponsiveNotification) {
  dismissedIds.value.add(n.id)
  try {
    await responsiveApi.dismiss(n.id)
  } catch {
    // 即使后端失败，本地也要消失
  }
  // 触发响应式更新
  notifications.value = [...notifications.value]
}

function promoteIfSevere(n: ResponsiveNotification) {
  // 严重通知自动晋升到全局通知中心
  if (n.severity === 'warning' || n.severity === 'error') {
    notifStore.add({
      type: n.severity === 'error' ? 'error' : 'warning',
      message: n.title,
      detail: n.description || undefined,
    })
  }
}

function getWsUrl(): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const host = window.location.host
  return `${protocol}//${host}/api/responsive/ws?interval_s=${WS_INTERVAL_S}`
}

function connectWs() {
  if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) return
  wsState.value = 'connecting'
  try {
    ws = new WebSocket(getWsUrl())
  } catch (e: unknown) {
    wsState.value = 'disconnected'
    scheduleReconnect()
    return
  }
  ws.onopen = () => {
    wsState.value = 'connected'
  }
  ws.onmessage = (ev) => {
    try {
      const msg = JSON.parse(ev.data)
      if (msg.type === 'notification' && msg.data) {
        const n = msg.data as ResponsiveNotification
        if (dismissedIds.value.has(n.id)) return
        const exists = notifications.value.some(x => x.id === n.id)
        if (!exists) {
          notifications.value = [n, ...notifications.value]
          promoteIfSevere(n)
        }
      }
    } catch {
      // ignore
    }
  }
  ws.onerror = () => {
    wsState.value = 'disconnected'
  }
  ws.onclose = () => {
    wsState.value = 'disconnected'
    scheduleReconnect()
  }
}

function disconnectWs() {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  if (ws) {
    ws.onclose = null
    ws.close()
    ws = null
  }
  wsState.value = 'disconnected'
}

function scheduleReconnect() {
  if (reconnectTimer) return
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = null
    connectWs()
  }, 5000)
}

function sendScanNow() {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ action: 'scan_now' }))
  } else {
    handleScanNow()  // 降级到 HTTP
  }
}

function formatTime(ts: number): string {
  const d = new Date(ts * 1000)
  const now = Date.now()
  const diff = now - d.getTime()
  if (diff < 60_000) return '刚刚'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`
  return d.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
}

function truncate(s: string, max: number = 200): string {
  if (s.length <= max) return s
  return s.slice(0, max) + '...'
}

onMounted(() => {
  loadInitial()
})

onUnmounted(() => {
  disconnectWs()
})
</script>

<template>
  <div class="rp-shell">
    <header class="rp-header">
      <div class="rp-title-row">
        <h2 class="rp-title">🛰️ 主动感知</h2>
        <div class="rp-engine-actions">
          <van-button
            v-if="!engineStatus?.running"
            size="small" type="primary" plain
            @click="handleStart"
          >
            ▶ 启动
          </van-button>
          <van-button
            v-else
            size="small" type="warning" plain
            @click="handleStop"
          >
            ⏸ 停止
          </van-button>
          <van-button
            size="small"
            :loading="scanning"
            @click="sendScanNow"
          >
            🔄 立即扫描
          </van-button>
        </div>
      </div>
      <div class="rp-status-row">
        <span class="rp-status-label">引擎状态：</span>
        <span
          class="rp-status-dot"
          :class="`rp-status-dot--${engineStatus?.running ? 'on' : 'off'}`"
        ></span>
        <span class="rp-status-text">
          {{ engineStatus?.running ? '运行中' : '已停止' }}
        </span>
        <span class="rp-divider">·</span>
        <span class="rp-status-label">WebSocket：</span>
        <span
          class="rp-status-dot"
          :class="`rp-status-dot--${wsState === 'connected' ? 'on' : wsState === 'connecting' ? 'wait' : 'off'}`"
        ></span>
        <span class="rp-status-text">
          {{ wsState === 'connected' ? '已连接' : wsState === 'connecting' ? '连接中' : '已断开' }}
        </span>
      </div>
    </header>

    <van-tabs v-model:active="activeTab" sticky>
      <van-tab :name="'all'" :title="`全部 (${typeCounts.all})`" />
      <van-tab
        v-for="key in ['file_change', 'code_quality', 'security_risk', 'progress']"
        :key="key"
        :name="key"
        :title="`${TYPE_META[key].icon} ${TYPE_META[key].label} (${typeCounts[key]})`"
      />
    </van-tabs>

    <section v-if="loading" class="rp-loading">加载中...</section>
    <section v-else-if="visibleNotifs().length === 0" class="rp-empty">
      <div class="rp-empty-icon">🛰️</div>
      <div class="rp-empty-text">暂无通知</div>
      <div class="rp-empty-hint">点击"立即扫描"或启动引擎自动扫描</div>
    </section>
    <section v-else class="rp-list">
      <div
        v-for="n in visibleNotifs()"
        :key="n.id"
        class="rp-card"
        :style="{ borderLeftColor: TYPE_META[n.type]?.color || 'var(--border)' }"
      >
        <div class="rp-card-header">
          <span class="rp-card-type">
            <span class="rp-card-icon">{{ TYPE_META[n.type]?.icon }}</span>
            {{ TYPE_META[n.type]?.label }}
          </span>
          <span
            class="rp-card-severity"
            :style="{ color: SEVERITY_META[n.severity]?.color }"
          >
            {{ SEVERITY_META[n.severity]?.label }}
          </span>
          <span class="rp-card-time">{{ formatTime(n.created_at) }}</span>
        </div>
        <div class="rp-card-title">{{ n.title }}</div>
        <div v-if="n.description" class="rp-card-desc">{{ truncate(n.description) }}</div>
        <div v-if="n.file_path" class="rp-card-file">📁 {{ n.file_path }}</div>
        <div v-if="n.actions && n.actions.length" class="rp-card-actions">
          <van-button
            v-for="(a, i) in n.actions"
            :key="i"
            size="mini" plain type="primary"
            @click="showToast(`执行: ${a.action}(${JSON.stringify(a.params)})`)"
          >
            {{ a.label }}
          </van-button>
        </div>
        <van-button
          size="mini" plain type="default"
          class="rp-card-dismiss"
          @click="handleDismiss(n)"
        >
          标为已读
        </van-button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.rp-shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: inherit;
  overflow: hidden;
  padding-bottom: var(--tab-bar-total, 24px);
  box-sizing: border-box;
}

.rp-header {
  flex-shrink: 0;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
}

.rp-title-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.rp-title {
  margin: 0;
  font-size: var(--font-xl);
  font-weight: 600;
}

.rp-engine-actions {
  display: flex;
  gap: var(--space-1);
}

.rp-status-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 6px;
  font-size: var(--font-xs);
  color: var(--text-muted);
}

.rp-status-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #6b7280;
}

.rp-status-dot--on { background: #10b981; box-shadow: 0 0 4px #10b981; }
.rp-status-dot--wait { background: #f59e0b; }
.rp-status-dot--off { background: #6b7280; }

.rp-status-text { color: var(--text-primary); }

.rp-divider {
  margin: 0 6px;
  color: var(--text-muted);
}

.rp-loading,
.rp-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-8);
  color: var(--text-muted);
  text-align: center;
}

.rp-empty-icon {
  font-size: 48px;
  margin-bottom: var(--space-2);
  opacity: 0.6;
}

.rp-empty-hint {
  font-size: var(--font-xs);
  margin-top: var(--space-2);
}

.rp-list {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-3) var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.rp-card {
  position: relative;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-left-width: 3px;
  border-radius: var(--radius-md);
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rp-card-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--font-xs);
  color: var(--text-muted);
}

.rp-card-type {
  font-weight: 600;
  color: var(--text-secondary);
}

.rp-card-icon {
  margin-right: 2px;
}

.rp-card-severity {
  font-weight: 600;
}

.rp-card-time {
  margin-left: auto;
}

.rp-card-title {
  font-size: var(--font-base);
  font-weight: 500;
  color: var(--text-primary);
  line-height: 1.4;
}

.rp-card-desc {
  font-size: var(--font-sm);
  color: var(--text-secondary);
  line-height: 1.4;
  white-space: pre-wrap;
  word-break: break-word;
}

.rp-card-file {
  font-size: var(--font-xs);
  color: var(--text-muted);
  font-family: Consolas, monospace;
  background: var(--bg-secondary);
  padding: 2px 6px;
  border-radius: 3px;
  word-break: break-all;
}

.rp-card-actions {
  display: flex;
  gap: var(--space-1);
  flex-wrap: wrap;
  padding-top: 4px;
  border-top: 1px solid var(--border);
  margin-top: 4px;
}

.rp-card-dismiss {
  position: absolute;
  top: var(--space-2);
  right: var(--space-2);
}
</style>
