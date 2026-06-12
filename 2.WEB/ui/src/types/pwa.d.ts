// 2026-06-09 TASK-4.3 引入：vite-plugin-pwa 虚拟模块类型声明

declare module 'virtual:pwa-register' {
  export interface RegisterSWOptions {
    immediate?: boolean
    onNeedRefresh?: () => void
    onOfflineReady?: () => void
    onRegistered?: (registration: ServiceWorkerRegistration | undefined) => void
    onRegisterError?: (error: any) => void
    onRegisteredSW?: (swScriptUrl: string, registration: ServiceWorkerRegistration | undefined) => void
  }

  export function registerSW(options?: RegisterSWOptions): (reloadPage?: boolean) => Promise<void>
}

declare module 'virtual:pwa-register/vue' {
  export interface RegisterSWOptions {
    immediate?: boolean
    onNeedRefresh?: () => void
    onOfflineReady?: () => void
    onRegistered?: (registration: ServiceWorkerRegistration | undefined) => void
    onRegisterError?: (error: any) => void
    onRegisteredSW?: (swScriptUrl: string, registration: ServiceWorkerRegistration | undefined) => void
  }

  export function useRegisterSW(options?: RegisterSWOptions): {
    needRefresh: Ref<boolean>
    offlineReady: Ref<boolean>
    updateServiceWorker: (reloadPage?: boolean) => Promise<void>
  }
}

// 扩展 window 供 SW 更新调用
declare global {
  interface Window {
    __yj_updateSW?: (reloadPage?: boolean) => Promise<void>
  }
}
