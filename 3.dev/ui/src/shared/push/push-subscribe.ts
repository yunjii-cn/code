// src/shared/push/push-subscribe.ts
// 2026-06-09 TASK-4.3 引入：Web Push 订阅逻辑
//
// 流程：
//   1. 请求 notification 权限
//   2. 调用 /api/push/vapid-public-key 获取公钥
//   3. 通过 serviceWorker.pushManager.subscribe() 订阅
//   4. 调用 /api/push/subscribe 把订阅信息注册到后端
//
// 兼容性：
//   - Chrome / Edge / Firefox：完整支持
//   - Safari 16.4+ iOS PWA：支持（必须先添加到主屏幕）
//   - Safari 桌面版：16.5+ 有限支持
//   - 不支持的环境：静默失败，UI 提示"当前浏览器不支持推送"

import { api } from '@/api'

export type PushPermission = 'default' | 'granted' | 'denied' | 'unsupported'

export interface PushSubscribeResult {
  ok: boolean
  permission: PushPermission
  error?: string
  subId?: string
}

/**
 * 检测推送能力
 */
export function detectPushCapability(): {
  supported: boolean
  permission: PushPermission
  hasSw: boolean
  reason?: string
} {
  if (typeof window === 'undefined' || typeof navigator === 'undefined') {
    return { supported: false, permission: 'unsupported', hasSw: false, reason: 'SSR' }
  }
  const hasSw = 'serviceWorker' in navigator
  const hasPush = 'PushManager' in window
  const hasNotification = 'Notification' in window
  if (!hasSw) return { supported: false, permission: 'unsupported', hasSw: false, reason: 'no ServiceWorker' }
  if (!hasPush) return { supported: false, permission: 'unsupported', hasSw: true, reason: 'no PushManager' }
  if (!hasNotification) return { supported: false, permission: 'unsupported', hasSw: true, reason: 'no Notification' }

  const permission = (Notification.permission as PushPermission) || 'default'
  return { supported: true, permission, hasSw: true }
}

/**
 * 请求通知权限
 */
export async function requestNotificationPermission(): Promise<PushPermission> {
  if (typeof Notification === 'undefined') return 'unsupported'
  if (Notification.permission === 'granted') return 'granted'
  if (Notification.permission === 'denied') return 'denied'
  try {
    const result = await Notification.requestPermission()
    return result as PushPermission
  } catch (e) {
    console.warn('[Push] request permission failed:', e)
    return 'denied'
  }
}

/**
 * 订阅 Web Push
 */
export async function subscribePush(): Promise<PushSubscribeResult> {
  const cap = detectPushCapability()
  if (!cap.supported) {
    return { ok: false, permission: 'unsupported', error: cap.reason }
  }

  // 1. 请求权限
  const permission = await requestNotificationPermission()
  if (permission !== 'granted') {
    return { ok: false, permission, error: '通知权限被拒绝' }
  }

  try {
    // 2. 获取公钥
    const keyRes = await api.get<{ ok: boolean; data: { publicKey: string } }>('/api/push/vapid-public-key')
    if (!keyRes.data.ok) {
      return { ok: false, permission, error: '获取 VAPID 公钥失败' }
    }
    const publicKey = keyRes.data.data.publicKey

    // 3. 通过 Service Worker 订阅
    const reg = await navigator.serviceWorker.ready
    let subscription = await reg.pushManager.getSubscription()

    if (!subscription) {
      // 转换 base64 公钥为 Uint8Array（PushManager 接受 BufferSource）
      const applicationServerKey = urlBase64ToUint8Array(publicKey)
      // BufferSource 联合类型（ArrayBufferView | ArrayBuffer）以适配新版 lib.dom
      subscription = await reg.pushManager.subscribe({
        userVisibleOnly: true, // 必须 true，否则会被拒
        applicationServerKey: applicationServerKey as BufferSource,
      })
    }

    // 4. 发送到后端
    const subJson = subscription.toJSON()
    const subscribeRes = await api.post<{ ok: boolean; data: { id: string } }>(
      '/api/push/subscribe',
      {
        endpoint: subJson.endpoint,
        keys: subJson.keys,
        expirationTime: subJson.expirationTime,
        userAgent: navigator.userAgent,
      }
    )

    if (!subscribeRes.data.ok) {
      return { ok: false, permission, error: '注册到服务器失败' }
    }

    return {
      ok: true,
      permission,
      subId: subscribeRes.data.data.id,
    }
  } catch (err) {
    const error = err instanceof Error ? err.message : String(err)
    console.error('[Push] subscribe failed:', err)
    return { ok: false, permission, error }
  }
}

/**
 * 取消订阅
 */
export async function unsubscribePush(): Promise<{ ok: boolean; error?: string }> {
  try {
    const reg = await navigator.serviceWorker.ready
    const subscription = await reg.pushManager.getSubscription()
    if (!subscription) {
      return { ok: true }
    }
    // 1. 通知后端
    await api.post('/api/push/unsubscribe', {
      endpoint: subscription.endpoint,
    })
    // 2. 浏览器端取消
    await subscription.unsubscribe()
    return { ok: true }
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    }
  }
}

/**
 * 获取当前订阅状态
 */
export async function getCurrentSubscription(): Promise<PushSubscription | null> {
  if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) return null
  try {
    const reg = await navigator.serviceWorker.ready
    return await reg.pushManager.getSubscription()
  } catch {
    return null
  }
}

/**
 * 测试推送
 */
export async function testPush(title: string, body: string): Promise<{ delivered: number; failed: number; cleaned: number }> {
  const res = await api.post<{ ok: boolean; data: { delivered: number; failed: number; cleaned: number } }>(
    '/api/push/test',
    { title, body, url: '/' }
  )
  return res.data.data
}

// ──────────── 工具函数 ────────────

function urlBase64ToUint8Array(base64String: string): Uint8Array {
  const padding = '='.repeat((4 - (base64String.length % 4)) % 4)
  const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/')
  const rawData = window.atob(base64)
  const outputArray = new Uint8Array(rawData.length)
  for (let i = 0; i < rawData.length; ++i) {
    outputArray[i] = rawData.charCodeAt(i)
  }
  return outputArray
}
