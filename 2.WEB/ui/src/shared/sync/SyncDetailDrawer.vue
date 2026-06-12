<!--
  SyncDetailDrawer.vue
  2026-06-09 TASK-4.3 引入：同步详情抽屉
  功能：
    - 统计：pending / in_flight / completed / failed
    - 列表：每条显示 tag / endpoint / 时间 / 重试次数 / 错误
    - 操作：立即同步 / 清空 completed / 清空 failed / 重试单项 / 删除单项
-->
<script setup lang="ts">
import { computed, watch } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { useSyncStore } from './sync-store'
import type { SyncQueueItem } from './sync-types'

const store = useSyncStore()

const visible = computed({
  get: () => store.showDetailDrawer,
  set: (v) => {
    if (v) store.openDetail()
    else store.closeDetail()
  },
})

// 打开时刷新
watch(visible, (v) => {
  if (v) {
    store.refreshStats()
    store.refreshItems()
  }
})

function formatTime(ts: number | null): string {
  if (!ts) return '—'
  const d = new Date(ts)
  const now = Date.now()
  const diff = now - ts
  if (diff < 60_000) return `${Math.floor(diff / 1000)} 秒前`
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86400_000) return `${Math.floor(diff / 3600_000)} 小时前`
  return d.toLocaleString('zh-CN', { hour12: false })
}

function methodColor(method: string): string {
  switch (method) {
    case 'POST': return '#10b981'
    case 'PUT': return '#42a5f5'
    case 'PATCH': return '#f59e0b'
    case 'DELETE': return '#ef4444'
    default: return '#888'
  }
}

function statusText(item: SyncQueueItem): string {
  switch (item.status) {
    case 'pending': return item.retryCount > 0 ? `重试 ${item.retryCount} 次` : '等待'
    case 'in_flight': return '发送中'
    case 'completed': return '已完成'
    case 'failed': return '已失败'
  }
}

function statusColor(status: SyncQueueItem['status']): string {
  switch (status) {
    case 'pending': return '#42a5f5'
    case 'in_flight': return '#10b981'
    case 'completed': return '#888'
    case 'failed': return '#ef4444'
  }
}

async function handleFlushNow() {
  if (!store.isOnline) {
    showToast({ type: 'fail', message: '当前离线，恢复网络后会自动同步' })
    return
  }
  showToast({ type: 'loading', message: '正在同步…', duration: 0 })
  try {
    const t0 = performance.now()
    await store.flushNow()
    const duration = Math.round(performance.now() - t0)
    showToast({
      type: 'success',
      message: `同步完成：${store.stats.lastSyncStatus === 'success' ? '全部成功' : '部分失败'}（${duration}ms）`,
    })
  } catch (e) {
    showToast({ type: 'fail', message: '同步失败' })
  }
}

async function handleClearCompleted() {
  try {
    await showConfirmDialog({ title: '清空已完成项', message: '确定要清空所有已完成的同步项吗？' })
  } catch {
    return
  }
  const n = await store.clearCompleted()
  showToast({ type: 'success', message: `已清空 ${n} 项` })
}

async function handleClearFailed() {
  try {
    await showConfirmDialog({ title: '清空失败项', message: '确定要清空所有同步失败项吗？' })
  } catch {
    return
  }
  const n = await store.clearFailed()
  showToast({ type: 'success', message: `已清空 ${n} 项` })
}

async function handleRetry(item: SyncQueueItem) {
  await store.retryFailed(item)
  showToast({ type: 'success', message: '已重新加入队列' })
}

async function handleDelete(item: SyncQueueItem) {
  try {
    await showConfirmDialog({ title: '删除项', message: `确定要删除 ${item.context?.label || item.endpoint} 吗？` })
  } catch {
    return
  }
  await store.deleteItem(item.id)
}
</script>

<template>
  <van-popup
    v-model:show="visible"
    position="bottom"
    :style="{ height: '75%', maxHeight: '600px' }"
    round
    closeable
    close-icon-position="top-right"
    :safe-area-inset-bottom="true"
  >
    <div class="sync-detail">
      <header class="sync-header">
        <h2>同步队列</h2>
        <p class="sync-subtitle">
          <span v-if="!store.isOnline" class="status-tag offline">⚡ 离线模式</span>
          <span v-else-if="store.isSyncing" class="status-tag syncing">⟳ 同步中</span>
          <span v-else-if="store.isOnline && store.hasPending" class="status-tag online">☁ 待同步</span>
          <span v-else class="status-tag idle">✓ 全部完成</span>
        </p>
      </header>

      <!-- 统计卡片 -->
      <div class="sync-stats">
        <div class="stat-item">
          <div class="stat-value">{{ store.stats.pending }}</div>
          <div class="stat-label">等待</div>
        </div>
        <div class="stat-item">
          <div class="stat-value">{{ store.stats.inFlight }}</div>
          <div class="stat-label">进行中</div>
        </div>
        <div class="stat-item">
          <div class="stat-value">{{ store.stats.completed }}</div>
          <div class="stat-label">已完成</div>
        </div>
        <div class="stat-item failed">
          <div class="stat-value">{{ store.stats.failed }}</div>
          <div class="stat-label">失败</div>
        </div>
      </div>

      <!-- 工具栏 -->
      <div class="sync-toolbar">
        <button class="btn-toolbar primary" :disabled="!store.isOnline || store.isSyncing" @click="handleFlushNow">
          ⟳ 立即同步
        </button>
        <button class="btn-toolbar" :disabled="store.stats.completed === 0" @click="handleClearCompleted">
          清空已完成
        </button>
        <button class="btn-toolbar danger" :disabled="store.stats.failed === 0" @click="handleClearFailed">
          清空失败
        </button>
      </div>

      <!-- 失败项（优先显示） -->
      <section v-if="store.failedItems.length > 0" class="sync-section">
        <h3>失败项 ({{ store.failedItems.length }})</h3>
        <div class="sync-list">
          <div v-for="item in store.failedItems" :key="item.id" class="sync-item failed">
            <div class="sync-item-header">
              <span class="sync-method" :style="{ color: methodColor(item.method) }">{{ item.method }}</span>
              <span class="sync-tag">{{ item.tag }}</span>
              <span class="sync-status" :style="{ color: statusColor(item.status) }">
                {{ statusText(item) }}
              </span>
            </div>
            <div class="sync-item-body">
              <code class="sync-endpoint">{{ item.endpoint }}</code>
            </div>
            <div v-if="item.context?.label" class="sync-item-context">
              {{ item.context.label }}
            </div>
            <div v-if="item.lastError" class="sync-item-error">
              ⚠ {{ item.lastError }}
            </div>
            <div class="sync-item-footer">
              <span class="sync-time">{{ formatTime(item.lastAttemptAt) }}</span>
              <div class="sync-item-actions">
                <button class="btn-item" @click="handleRetry(item)">重试</button>
                <button class="btn-item danger" @click="handleDelete(item)">删除</button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- Pending 项 -->
      <section v-if="store.pendingItems.length > 0" class="sync-section">
        <h3>待同步 ({{ store.pendingItems.length }})</h3>
        <div class="sync-list">
          <div v-for="item in store.pendingItems" :key="item.id" class="sync-item">
            <div class="sync-item-header">
              <span class="sync-method" :style="{ color: methodColor(item.method) }">{{ item.method }}</span>
              <span class="sync-tag">{{ item.tag }}</span>
              <span class="sync-status" :style="{ color: statusColor(item.status) }">
                {{ statusText(item) }}
              </span>
            </div>
            <div class="sync-item-body">
              <code class="sync-endpoint">{{ item.endpoint }}</code>
            </div>
            <div v-if="item.context?.label" class="sync-item-context">
              {{ item.context.label }}
            </div>
            <div class="sync-item-footer">
              <span class="sync-time">入队：{{ formatTime(item.createdAt) }}</span>
              <button class="btn-item danger" @click="handleDelete(item)">删除</button>
            </div>
          </div>
        </div>
      </section>

      <!-- 空状态 -->
      <div v-if="store.pendingItems.length === 0 && store.failedItems.length === 0" class="sync-empty">
        <div class="sync-empty-icon">✓</div>
        <p>所有数据已同步</p>
        <p v-if="store.stats.lastSyncAt" class="sync-empty-hint">
          上次同步：{{ formatTime(store.stats.lastSyncAt) }}
        </p>
      </div>
    </div>
  </van-popup>
</template>

<style scoped>
.sync-detail {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #0d0d0d;
  color: #e0e0e0;
  padding: 20px 16px 16px;
  overflow-y: auto;
}

.sync-header h2 {
  font-size: 18px;
  font-weight: 600;
  margin: 0 0 4px;
}

.sync-subtitle {
  font-size: 12px;
  margin: 0 0 16px;
}

.status-tag {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 500;
}

.status-tag.offline { background: rgba(245, 158, 11, 0.15); color: #f59e0b; }
.status-tag.syncing { background: rgba(66, 165, 245, 0.15); color: #42a5f5; }
.status-tag.online { background: rgba(66, 165, 245, 0.15); color: #42a5f5; }
.status-tag.idle { background: rgba(16, 185, 129, 0.15); color: #10b981; }

.sync-stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin-bottom: 16px;
}

.stat-item {
  background: #1a1a2e;
  border: 1px solid #2a2a44;
  border-radius: 8px;
  padding: 12px 8px;
  text-align: center;
}

.stat-item.failed {
  border-color: rgba(239, 68, 68, 0.3);
  background: rgba(239, 68, 68, 0.05);
}

.stat-value {
  font-size: 20px;
  font-weight: 600;
  color: #42a5f5;
  line-height: 1.2;
}

.stat-item.failed .stat-value { color: #ef4444; }

.stat-label {
  font-size: 10px;
  color: #888;
  margin-top: 4px;
}

.sync-toolbar {
  display: flex;
  gap: 6px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.btn-toolbar {
  flex: 1;
  min-width: 80px;
  padding: 8px 12px;
  background: #1a1a2e;
  border: 1px solid #2a2a44;
  color: #e0e0e0;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
  -webkit-tap-highlight-color: transparent;
}

.btn-toolbar:hover:not(:disabled) {
  background: #222240;
}

.btn-toolbar:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn-toolbar.primary {
  background: #42a5f5;
  color: #fff;
  border-color: #42a5f5;
}

.btn-toolbar.primary:hover:not(:disabled) {
  background: #1565c0;
}

.btn-toolbar.danger {
  border-color: rgba(239, 68, 68, 0.3);
  color: #ef4444;
}

.btn-toolbar.danger:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.1);
}

.sync-section {
  margin-bottom: 16px;
}

.sync-section h3 {
  font-size: 13px;
  font-weight: 600;
  color: #888;
  margin: 0 0 8px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.sync-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.sync-item {
  background: #1a1a2e;
  border: 1px solid #2a2a44;
  border-radius: 8px;
  padding: 10px;
  font-size: 12px;
}

.sync-item.failed {
  border-color: rgba(239, 68, 68, 0.3);
  background: rgba(239, 68, 68, 0.05);
}

.sync-item-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.sync-method {
  font-size: 10px;
  font-weight: 600;
  font-family: Consolas, monospace;
}

.sync-tag {
  font-size: 10px;
  color: #888;
  background: #0d0d0d;
  padding: 1px 6px;
  border-radius: 3px;
  font-family: Consolas, monospace;
}

.sync-status {
  font-size: 10px;
  font-weight: 500;
  margin-left: auto;
}

.sync-item-body {
  margin-bottom: 4px;
}

.sync-endpoint {
  font-family: Consolas, monospace;
  font-size: 11px;
  color: #ccc;
  word-break: break-all;
}

.sync-item-context {
  font-size: 11px;
  color: #888;
  margin-bottom: 4px;
}

.sync-item-error {
  font-size: 11px;
  color: #ef4444;
  background: rgba(239, 68, 68, 0.05);
  padding: 4px 6px;
  border-radius: 4px;
  margin-bottom: 6px;
  word-break: break-all;
}

.sync-item-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 6px;
}

.sync-time {
  font-size: 10px;
  color: #666;
}

.sync-item-actions {
  display: flex;
  gap: 4px;
}

.btn-item {
  background: transparent;
  border: 1px solid #2a2a44;
  color: #ccc;
  padding: 3px 8px;
  font-size: 10px;
  border-radius: 4px;
  cursor: pointer;
  font-family: inherit;
  -webkit-tap-highlight-color: transparent;
}

.btn-item:hover {
  background: #222240;
}

.btn-item.danger {
  color: #ef4444;
  border-color: rgba(239, 68, 68, 0.3);
}

.btn-item.danger:hover {
  background: rgba(239, 68, 68, 0.1);
}

.sync-empty {
  text-align: center;
  padding: 60px 20px;
  color: #888;
}

.sync-empty-icon {
  font-size: 48px;
  color: #10b981;
  margin-bottom: 12px;
}

.sync-empty p {
  margin: 4px 0;
}

.sync-empty-hint {
  font-size: 11px;
  color: #666;
}
</style>
