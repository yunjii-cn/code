// shared/components/index.ts
// 2026-06-08 TASK-1.9 引入：设计系统原子组件统一导出

export { default as YJButton } from './YJButton.vue'
export { default as YJModal } from './YJModal.vue'
export { default as YJPopover } from './YJPopover.vue'
export { default as YJToast } from './YJToast.vue'
export { default as YJPanel } from './YJPanel.vue'

// 2026-06-08 TASK-2.7 引入：统一通知中心
export { default as YJNotificationCenter } from './YJNotificationCenter.vue'
export { useNotificationStore } from './notificationStore'
export type { NotificationItem } from './notificationStore'

// 2026-06-09 TASK-2.8 引入：命令面板（Ctrl+K 全局快捷键触发）
export { default as CommandPalette } from './CommandPalette.vue'

// 2026-06-09 TASK-4.7 引入：错误边界 + 骨架屏
export { default as ErrorBoundary } from './ErrorBoundary.vue'
export { default as AppSkeleton } from './AppSkeleton.vue'

// Toast 全局 API 单例（YJToast 挂载时注入）
export { useToast, setToastApi, clearToastApi } from './toast'
export type { ToastApi, ToastType } from './toast'

// 2026-06-10 TASK-2.7 引入：增强 useToast composable（支持 action / detail / position / 队列管理）
export {
  useToast as useToastEnhanced,
  dismissByKey,
  clearAll as clearAllToasts,
  MAX_VISIBLE,
} from '../composables/useToast'
export type { ToastOptions, ToastAction, ToastPosition } from '../composables/useToast'
