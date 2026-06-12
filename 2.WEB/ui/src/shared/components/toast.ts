// shared/components/toast.ts
// 2026-06-08 TASK-1.9 引入：YJToast 全局 API 单例
// 模块级单例，避免 window 挂载和 any 转换

export type ToastType = 'success' | 'error' | 'warning' | 'info' | 'loading'

export interface ToastApi {
  success: (message: string) => number
  error: (message: string) => number
  warning: (message: string) => number
  info: (message: string) => number
  loading: (message: string) => number
  show: (message: string, type?: ToastType, duration?: number) => number
  remove: (id: number) => void
  clear: () => void
}

let api: ToastApi | null = null

export function setToastApi(instance: ToastApi): void {
  api = instance
}

export function clearToastApi(): void {
  api = null
}

export function useToast(): ToastApi {
  if (!api) {
    // 静默失败：未挂载 YJToast 时调用任何方法都是 no-op
    // 避免开发期控制台噪音
    const noop = (): number => -1
    const noopVoid = (): void => {}
    return {
      success: noop,
      error: noop,
      warning: noop,
      info: noop,
      loading: noop,
      show: noop,
      remove: noopVoid,
      clear: noopVoid,
    }
  }
  return api
}
