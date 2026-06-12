// src/shared/pwa/pwa-store.ts
// 2026-06-09 TASK-4.3 升级：集成 vite-plugin-pwa 的更新检测
// - needRefresh: vite-plugin-pwa 内部状态（onNeedRefresh 触发）
// - offlineReady: vite-plugin-pwa 内部状态（onOfflineReady 触发）
// - applyUpdate 调用 window.__yj_updateSW

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

interface BeforeInstallPromptEvent extends Event {
  readonly platforms: string[]
  readonly userChoice: Promise<{ outcome: 'accepted' | 'dismissed'; platform: string }>
  prompt(): Promise<void>
}

export const usePwaStore = defineStore('pwa', () => {
  // ──────────── 状态 ────────────

  const isOnline = ref<boolean>(typeof navigator !== 'undefined' ? navigator.onLine : true)
  const swReady = ref<boolean>(false)
  const swVersion = ref<string>('')
  const updateAvailable = ref<boolean>(false)
  const installPromptEvent = ref<BeforeInstallPromptEvent | null>(null)
  const isInstalled = ref<boolean>(false)
  const offlineReady = ref<boolean>(false) // vite-plugin-pwa offline ready
  const lastUpdatePromptedAt = ref<number>(0) // 节流：避免重复弹更新

  // ──────────── 派生 ────────────

  const canInstall = computed(() => !!installPromptEvent.value)
  const offlineMode = computed(() => !isOnline.value)
  const shouldShowUpdateBanner = computed(() => updateAvailable.value && !offlineMode.value)
  const shouldShowOfflineReady = computed(() => offlineReady.value && !isInstalled.value)

  // ──────────── 操作 ────────────

  function setOnline(online: boolean) {
    isOnline.value = online
  }

  function setSwReady(ready: boolean) {
    swReady.value = ready
  }

  function setSwVersion(version: string) {
    swVersion.value = version
  }

  function setUpdateAvailable(available: boolean) {
    if (available) {
      // 节流：30 秒内不重复触发
      const now = Date.now()
      if (now - lastUpdatePromptedAt.value < 30_000) return
      lastUpdatePromptedAt.value = now
    }
    updateAvailable.value = available
  }

  function setInstallPrompt(event: BeforeInstallPromptEvent | null) {
    installPromptEvent.value = event
  }

  function setInstalled(installed: boolean) {
    isInstalled.value = installed
  }

  function setOfflineReady(ready: boolean) {
    offlineReady.value = ready
  }

  async function promptInstall(): Promise<'accepted' | 'dismissed' | 'unavailable'> {
    const ev = installPromptEvent.value
    if (!ev) return 'unavailable'
    try {
      await ev.prompt()
      const choice = await ev.userChoice
      installPromptEvent.value = null
      return choice.outcome
    } catch (e) {
      return 'dismissed'
    }
  }

  async function applyUpdate(): Promise<void> {
    const updateFn = (window as any).__yj_updateSW as ((reload?: boolean) => Promise<void>) | undefined
    if (updateFn) {
      await updateFn(true) // true = reload after update
    } else {
      // 兜底：手动 SKIP_WAITING + reload
      if (!('serviceWorker' in navigator)) return
      const reg = await navigator.serviceWorker.getRegistration()
      reg?.waiting?.postMessage({ type: 'SKIP_WAITING' })
      setTimeout(() => window.location.reload(), 500)
    }
  }

  async function clearCache(): Promise<void> {
    if (!('serviceWorker' in navigator)) return
    try {
      const reg = await navigator.serviceWorker.getRegistration()
      reg?.active?.postMessage({ type: 'CLEAR_CACHE' })
    } catch (e) {
      // ignore
    }
  }

  return {
    // 状态
    isOnline,
    swReady,
    swVersion,
    updateAvailable,
    installPromptEvent,
    isInstalled,
    offlineReady,
    // 派生
    canInstall,
    offlineMode,
    shouldShowUpdateBanner,
    shouldShowOfflineReady,
    // 操作
    setOnline,
    setSwReady,
    setSwVersion,
    setUpdateAvailable,
    setInstallPrompt,
    setInstalled,
    setOfflineReady,
    promptInstall,
    applyUpdate,
    clearCache,
  }
})
