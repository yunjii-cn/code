// src/shared/push/push-store.ts
// 2026-06-09 TASK-4.3 引入：Web Push 状态管理

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  detectPushCapability,
  getCurrentSubscription,
  requestNotificationPermission,
  subscribePush,
  unsubscribePush,
  testPush as testPushApi,
  type PushPermission,
} from './push-subscribe'

export const usePushStore = defineStore('push', () => {
  // ──────────── 状态 ────────────

  const supported = ref<boolean>(false)
  const permission = ref<PushPermission>('default')
  const subscribed = ref<boolean>(false)
  const subId = ref<string>('')
  const lastTestResult = ref<{ delivered: number; failed: number; cleaned: number; at: number } | null>(null)
  const isProcessing = ref<boolean>(false)
  const errorMessage = ref<string>('')
  const reason = ref<string>('')

  // 服务端统计
  const serverStats = ref<{
    vapid_initialized: boolean
    subscription_count: number
    active_count: number
  } | null>(null)

  // ──────────── 派生 ────────────

  const status = computed(() => {
    if (!supported.value) return 'unsupported'
    if (subscribed.value && permission.value === 'granted') return 'subscribed'
    if (permission.value === 'denied') return 'denied'
    if (permission.value === 'granted' && !subscribed.value) return 'not-subscribed'
    return 'default'
  })

  const statusText = computed(() => {
    switch (status.value) {
      case 'subscribed': return '已启用'
      case 'denied': return '已被浏览器拒绝'
      case 'not-subscribed': return '已授权但未订阅'
      case 'default': return '未授权'
      case 'unsupported': return '浏览器不支持'
    }
    return '未知'
  })

  // ──────────── 操作 ────────────

  async function detect() {
    const cap = detectPushCapability()
    supported.value = cap.supported
    permission.value = cap.permission
    reason.value = cap.reason || ''
    if (cap.supported) {
      // 检查是否已订阅
      const sub = await getCurrentSubscription()
      subscribed.value = !!sub
    }
  }

  async function refreshServerStats() {
    try {
      const { api } = await import('@/api')
      const res = await api.get<{ ok: boolean; data: typeof serverStats.value }>(
        '/api/push/status'
      )
      if (res.data.ok) {
        serverStats.value = res.data.data
      }
    } catch (e) {
      console.warn('[PushStore] refresh stats failed:', e)
    }
  }

  async function subscribe(): Promise<{ ok: boolean; error?: string }> {
    isProcessing.value = true
    errorMessage.value = ''
    try {
      const result = await subscribePush()
      if (result.ok) {
        subscribed.value = true
        subId.value = result.subId || ''
        permission.value = 'granted'
      } else {
        errorMessage.value = result.error || '订阅失败'
        if (result.permission) permission.value = result.permission
      }
      await refreshServerStats()
      return { ok: result.ok, error: result.error }
    } finally {
      isProcessing.value = false
    }
  }

  async function unsubscribe(): Promise<{ ok: boolean; error?: string }> {
    isProcessing.value = true
    try {
      const result = await unsubscribePush()
      if (result.ok) {
        subscribed.value = false
        subId.value = ''
      } else {
        errorMessage.value = result.error || '取消订阅失败'
      }
      await refreshServerStats()
      return result
    } finally {
      isProcessing.value = false
    }
  }

  async function sendTest(): Promise<{ delivered: number; failed: number; cleaned: number } | null> {
    isProcessing.value = true
    try {
      const result = await testPushApi(
        '🔔 云集编程',
        '推送通知测试成功！你将在感知引擎检测到重要事件时收到通知。'
      )
      lastTestResult.value = { ...result, at: Date.now() }
      await refreshServerStats()
      return result
    } catch (err) {
      errorMessage.value = err instanceof Error ? err.message : String(err)
      return null
    } finally {
      isProcessing.value = false
    }
  }

  async function requestPermission(): Promise<PushPermission> {
    const result = await requestNotificationPermission()
    permission.value = result
    return result
  }

  return {
    // 状态
    supported,
    permission,
    subscribed,
    subId,
    lastTestResult,
    isProcessing,
    errorMessage,
    reason,
    serverStats,
    // 派生
    status,
    statusText,
    // 操作
    detect,
    refreshServerStats,
    subscribe,
    unsubscribe,
    sendTest,
    requestPermission,
  }
})
