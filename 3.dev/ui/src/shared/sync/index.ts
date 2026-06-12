// src/shared/sync/index.ts
// 2026-06-09 TASK-4.3 引入：同步模块统一入口

export { syncQueue } from './sync-queue'
export { syncManager } from './sync-manager'
export { useSyncStore } from './sync-store'
export { syncFetch } from './sync-api'
export type {
  SyncQueueItem,
  SyncStatus,
  SyncStats,
  SyncResult,
  SyncEvent,
  SyncEventListener,
  SyncConfig,
} from './sync-types'
export { DEFAULT_SYNC_CONFIG } from './sync-types'
export { default as SyncIndicator } from './SyncIndicator.vue'
export { default as SyncDetailDrawer } from './SyncDetailDrawer.vue'
