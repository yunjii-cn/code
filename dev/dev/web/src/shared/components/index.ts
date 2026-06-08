// shared/components/index.ts
// 2026-06-08 TASK-1.9 引入：设计系统原子组件统一导出

export { default as YJButton } from './YJButton.vue'
export { default as YJModal } from './YJModal.vue'
export { default as YJPopover } from './YJPopover.vue'
export { default as YJToast } from './YJToast.vue'
export { default as YJPanel } from './YJPanel.vue'

// Toast 全局 API 单例（YJToast 挂载时注入）
export { useToast, setToastApi, clearToastApi } from './toast'
export type { ToastApi, ToastType } from './toast'
