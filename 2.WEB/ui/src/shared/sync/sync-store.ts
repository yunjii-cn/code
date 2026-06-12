// src/shared/sync/sync-store.ts
// 2026-06-09 TASK-4.3 引入：同步状态 Pinia store
//
// 暴露给 UI：
//   - pendingCount / failedCount：徽标
//   - isSyncing：旋转图标
//   - lastSyncAt / lastSyncStatus：详情
//   - pendingItems：列表（点击徽标打开抽屉）

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { syncManager } from './sync-manager'
import { syncQueue } from './sync-queue'
import type { SyncStats, SyncQueueItem, SyncEvent } from './sync-types'

export const useSyncStore = defineStore('sync', () => {
  // ──────────── 状态 ────────────

  const stats = ref<SyncStats>({
    pending: 0,
    inFlight: 0,
    completed: 0,
    failed: 0,
    lastSyncAt: null,
    lastSyncStatus: null,
  })
  const pendingItems = ref<SyncQueueItem[]>([])
  const failedItems = ref<SyncQueueItem[]>([])
  const isSyncing = ref<boolean>(false)
  const isOnline = ref<boolean>(navigator?.onLine ?? true)
  const lastEvent = ref<SyncEvent | null>(null)
  const showDetailDrawer = ref<boolean>(false)

  // ──────────── 派生 ────────────

  const hasPending = computed(() => stats.value.pending > 0 || stats.value.inFlight > 0)
  const hasFailed = computed(() => stats.value.failed > 0)
  const showIndicator = computed(() => hasPending.value || hasFailed.value || isSyncing.value)

  // ──────────── 操作 ────────────

  function init(): void {
    // 订阅 sync manager 事件
    syncManager.on((event) => {
      lastEvent.value = event
      switch (event.type) {
        case 'started':
          isSyncing.value = true
          break
        case 'progress':
        case 'item-success':
        case 'item-failed':
          // progress 中间态
          break
        case 'completed':
          isSyncing.value = false
          stats.value = event.stats
          refreshItems()
          break
        case 'idle':
          isSyncing.value = false
          refreshStats()
          break
        case 'enqueued':
          refreshStats()
          break
      }
    })

    // 监听网络状态
    window.addEventListener('online', () => {
      isOnline.value = true
    })
    window.addEventListener('offline', () => {
      isOnline.value = false
    })

    // 监听 SW 消息（Background Sync 触发）
    if ('serviceWorker' in navigator) {
      navigator.serviceWorker.addEventListener('message', (event) => {
        if (event.data?.type === 'SYNC_QUEUE_FLUSH') {
          console.info('[SyncStore] SW requested flush')
          syncManager.flush().catch(() => {})
        }
      })
    }

    // 启动 sync manager
    syncManager.start()

    // 首次加载
    refreshStats()
    refreshItems()
  }

  async function refreshStats(): Promise<void> {
    stats.value = await syncManager.getStats()
  }

  async function refreshItems(): Promise<void> {
    pendingItems.value = await syncQueue.getPending(100)
    failedItems.value = await syncQueue.getFailed()
  }

  async function flushNow(): Promise<void> {
    await syncManager.flush()
  }

  async function clearCompleted(): Promise<void> {
    await syncQueue.clearCompleted()
    await refreshStats()
    await refreshItems()
  }

  async function clearFailed(): Promise<void> {
    await syncQueue.clearFailed()
    await refreshStats()
    await refreshItems()
  }

  async function retryFailed(item: SyncQueueItem): Promise<void> {
    // 把 failed 项改回 pending
    await syncQueue.markPending(item.id, 'manual retry')
    await refreshStats()
    await refreshItems()
    await flushNow()
  }

  async function deleteItem(id: string): Promise<void> {
    await syncQueue.delete(id)
    await refreshStats()
    await refreshItems()
  }

  function openDetail(): void {
    showDetailDrawer.value = true
  }

  function closeDetail(): void {
    showDetailDrawer.value = false
  }

  return {
    // 状态
    stats,
    pendingItems,
    failedItems,
    isSyncing,
    isOnline,
    lastEvent,
    showDetailDrawer,
    // 派生
    hasPending,
    hasFailed,
    showIndicator,
    // 操作
    init,
    refreshStats,
    refreshItems,
    flushNow,
    clearCompleted,
    clearFailed,
    retryFailed,
    deleteItem,
    openDetail,
    closeDetail,
  }
})
