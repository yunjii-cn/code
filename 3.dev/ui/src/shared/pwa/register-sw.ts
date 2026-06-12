// src/shared/pwa/register-sw.ts
// 2026-06-09 TASK-4.3 改造：使用 vite-plugin-pwa 的 useRegisterSW（virtual:pwa-register）
// 保留自研逻辑：在线/离线检测、安装提示、版本管理

import { usePwaStore } from './pwa-store'
import { registerSW as vitePwaRegisterSW } from 'virtual:pwa-register'

/**
 * 注册 Service Worker
 *
 * 工作流程：
 *   1. 使用 vite-plugin-pwa 的 registerSW（自动处理 SW 生命周期 + 更新检测）
 *   2. 监听 install / activate / updatefound（vite-plugin-pwa 内置）
 *   3. 监听 controllerchange（更新后激活）
 *   4. 监听 message（SW → 主线程）
 *   5. 监听 beforeinstallprompt（可安装）
 *   6. 监听 online / offline（网络状态）
 */
export async function registerSW(): Promise<void> {
  if (typeof window === 'undefined') return
  if (!('serviceWorker' in navigator)) {
    console.info('[PWA] Service Worker 不支持')
    return
  }

  const store = usePwaStore()

  // 1. 用 vite-plugin-pwa 注册（注入 autoUpdate 模式行为）
  try {
    const updateSW = vitePwaRegisterSW({
      // SW 首次注册成功
      onRegisteredSW(swScriptUrl: string, registration: ServiceWorkerRegistration | undefined) {
        console.info(`[PWA] SW registered: ${swScriptUrl}`)
        store.setSwReady(true)
        if (registration) {
          // 监听 controllerchange（新 SW 接管）
          navigator.serviceWorker.addEventListener('controllerchange', () => {
            console.info('[PWA] controller change, new SW active')
          })
          // 请求 SW 版本
          registration.active?.postMessage({ type: 'GET_VERSION' })
        }
      },
      // 新 SW 已安装等待激活 → 触发更新横幅
      onNeedRefresh() {
        console.info('[PWA] new SW available, showing update banner')
        store.setUpdateAvailable(true)
      },
      // 首次 SW 激活完成，可离线使用 → 提示"已就绪"
      onOfflineReady() {
        console.info('[PWA] offline ready')
        store.setOfflineReady(true)
        // 5 秒后自动隐藏 offline ready 提示
        setTimeout(() => store.setOfflineReady(false), 5000)
      },
      // 注册错误
      onRegisterError(error: any) {
        console.warn('[PWA] SW register error:', error)
      },
    })

    // 暴露给 store：applyUpdate 调用 updateSW
    ;(window as any).__yj_updateSW = updateSW
  } catch (err) {
    console.warn('[PWA] vite-plugin-pwa register failed:', err)
    return
  }

  // 2. 监听 SW 消息（GET_VERSION / CACHE_CLEARED / PONG）
  navigator.serviceWorker.addEventListener('message', (event) => {
    const data = event.data
    if (!data || typeof data !== 'object') return
    switch (data.type) {
      case 'SW_VERSION':
        store.setSwVersion(data.version || '')
        break
      case 'CACHE_CLEARED':
        console.info('[PWA] cache cleared by SW')
        break
      case 'PONG':
        console.debug('[PWA] SW pong:', data)
        break
    }
  })

  // 3. 周期性检查更新（每 60 分钟）
  setInterval(() => {
    navigator.serviceWorker.getRegistration().then((reg) => {
      reg?.update().catch(() => {})
    })
  }, 60 * 60 * 1000)

  // 4. 安装提示捕获
  window.addEventListener('beforeinstallprompt', (event: Event) => {
    console.info('[PWA] beforeinstallprompt captured')
    event.preventDefault()
    store.setInstallPrompt(event as BeforeInstallPromptEvent)
  })

  window.addEventListener('appinstalled', () => {
    console.info('[PWA] app installed')
    store.setInstalled(true)
    store.setInstallPrompt(null)
  })

  // 5. 网络状态
  window.addEventListener('online', () => {
    console.info('[PWA] online')
    store.setOnline(true)
  })
  window.addEventListener('offline', () => {
    console.info('[PWA] offline')
    store.setOnline(false)
  })
}

interface BeforeInstallPromptEvent extends Event {
  readonly platforms: string[]
  readonly userChoice: Promise<{ outcome: 'accepted' | 'dismissed'; platform: string }>
  prompt(): Promise<void>
}
