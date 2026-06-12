/// <reference lib="webworker" />
/**
 * YJ Workstation Service Worker (TypeScript)
 * 2026-06-09 TASK-4.3 PWA 完整支持
 *
 * 架构：
 *   - 底座：vite-plugin-pwa + Workbox（precache manifest 自动注入）
 *   - 策略：Workbox 官方策略（NetworkFirst / CacheFirst / StaleWhileRevalidate）
 *   - 兜底：自研 fetch handler 处理未匹配路由
 *
 * 缓存分层：
 *   - precache（Workbox 注入）: Vite 构建产物
 *   - runtime-cache-app: 静态资源（CacheFirst，30 天）
 *   - runtime-cache-api: API GET 响应（NetworkFirst，10s TTL）
 *   - runtime-cache-images: 图片（CacheFirst，30 天，60 份上限）
 *
 * 生命周期：
 *   1. install: skipWaiting() 立即激活
 *   2. activate: clients.claim() 立即接管
 *   3. message: 接收前端指令（SKIP_WAITING / CLEAR_CACHE / PING）
 *   4. push: 显示通知
 *   5. notificationclick: 打开 URL
 */

import { precacheAndRoute, cleanupOutdatedCaches } from 'workbox-precaching'
import { registerRoute, NavigationRoute } from 'workbox-routing'
import {
  NetworkFirst,
  CacheFirst,
  StaleWhileRevalidate,
  NetworkOnly,
} from 'workbox-strategies'
import { ExpirationPlugin } from 'workbox-expiration'
import { CacheableResponsePlugin } from 'workbox-cacheable-response'

declare const self: ServiceWorkerGlobalScope

// ============================================================================
// 0. 版本标识（用于前端检测）
// ============================================================================

const VERSION = '1.1.0'
const RUNTIME_CACHE_APP = 'yj-app-v1'
const RUNTIME_CACHE_API = 'yj-api-v1'
const RUNTIME_CACHE_IMAGES = 'yj-images-v1'

console.info(`[SW ${VERSION}] booting`)

// ============================================================================
// 1. Workbox precache（vite-plugin-pwa 注入的 manifest）
// ============================================================================

// self.__WB_MANIFEST 由 vite-plugin-pwa 在构建时注入
precacheAndRoute(self.__WB_MANIFEST)
cleanupOutdatedCaches()

// ============================================================================
// 2. 导航请求：NetworkFirst + 离线 fallback
// ============================================================================

const navigationStrategy = new NetworkFirst({
  cacheName: RUNTIME_CACHE_APP,
  networkTimeoutSeconds: 3,
  plugins: [
    new CacheableResponsePlugin({ statuses: [200] }),
  ],
})

type NavigationHandlerArgs = {
  event: ExtendableEvent
  request: Request
}

const navigationHandler = async ({ event, request }: NavigationHandlerArgs) => {
  try {
    return await navigationStrategy.handle({ event, request })
  } catch (err) {
    // 离线：返回 precache 根页面或 offline.html
    const cache = await caches.open(RUNTIME_CACHE_APP)
    const cachedRoot = await cache.match('/')
    if (cachedRoot) return cachedRoot
    const offlinePage = await caches.match('/offline.html')
    if (offlinePage) return offlinePage
    return new Response(
      '<!DOCTYPE html><html><body style="background:#0d0d0d;color:#e0e0e0;font-family:sans-serif;padding:40px;text-align:center"><h1>离线模式</h1><p>请检查网络连接</p></body></html>',
      { status: 503, headers: { 'Content-Type': 'text/html; charset=utf-8' } }
    )
  }
}

registerRoute(
  new NavigationRoute(navigationHandler, {
    denylist: [/^\/api\//, /^\/ws\//],
  })
)

// ============================================================================
// 3. API GET 请求：NetworkFirst + 10s TTL
// ============================================================================

registerRoute(
  ({ url, request }) =>
    request.method === 'GET' &&
    url.pathname.startsWith('/api/') &&
    !url.pathname.startsWith('/api/ai/chat'),
  new NetworkFirst({
    cacheName: RUNTIME_CACHE_API,
    networkTimeoutSeconds: 5,
    plugins: [
      new CacheableResponsePlugin({ statuses: [0, 200] }),
      new ExpirationPlugin({
        maxEntries: 100,
        maxAgeSeconds: 10, // API 缓存 10s
        purgeOnQuotaError: true,
      }),
    ],
  })
)

// AI 对话流不缓存（每次都是新内容）
registerRoute(
  ({ url }) => url.pathname.startsWith('/api/ai/chat'),
  new NetworkOnly()
)

// WebSocket 不拦截
registerRoute(
  ({ url }) => url.pathname.startsWith('/ws/'),
  new NetworkOnly()
)

// ============================================================================
// 4. 静态资源：CacheFirst（JS/CSS/SVG/字体）
// ============================================================================

registerRoute(
  ({ request, url }) => {
    if (request.method !== 'GET') return false
    if (url.origin !== self.location.origin) return false
    return /\.(js|css|svg|woff2?|ttf|eot)$/i.test(url.pathname)
  },
  new CacheFirst({
    cacheName: RUNTIME_CACHE_APP,
    plugins: [
      new CacheableResponsePlugin({ statuses: [0, 200] }),
      new ExpirationPlugin({
        maxEntries: 200,
        maxAgeSeconds: 30 * 24 * 60 * 60, // 30 天
        purgeOnQuotaError: true,
      }),
    ],
  })
)

// ============================================================================
// 5. 图片：CacheFirst + 30 天 + 60 份上限
// ============================================================================

registerRoute(
  ({ request, url }) => {
    if (request.method !== 'GET') return false
    if (url.origin !== self.location.origin) return false
    return /\.(png|jpg|jpeg|gif|webp|ico)$/i.test(url.pathname)
  },
  new CacheFirst({
    cacheName: RUNTIME_CACHE_IMAGES,
    plugins: [
      new CacheableResponsePlugin({ statuses: [0, 200] }),
      new ExpirationPlugin({
        maxEntries: 60,
        maxAgeSeconds: 30 * 24 * 60 * 60,
        purgeOnQuotaError: true,
      }),
    ],
  })
)

// ============================================================================
// 6. HTML 子资源：StaleWhileRevalidate（构建产物未匹配 precache 时的兜底）
// ============================================================================

registerRoute(
  ({ request }) => request.destination === 'script' || request.destination === 'style',
  new StaleWhileRevalidate({
    cacheName: RUNTIME_CACHE_APP,
    plugins: [
      new CacheableResponsePlugin({ statuses: [0, 200] }),
    ],
  })
)

// ============================================================================
// 7. 生命周期：等待用户确认更新（与 vite-plugin-pwa prompt 模式配合）
// ============================================================================

self.addEventListener('install', () => {
  // 不自动 skipWaiting：让新 SW 处于 waiting 状态，
  // 等用户点击 UpdateBanner 后由主线程发 SKIP_WAITING 消息激活
  // 这样可以避免"突然刷新"的体验问题
  console.info(`[SW ${VERSION}] installed, waiting for activation signal`)
})

self.addEventListener('activate', (event) => {
  console.info(`[SW ${VERSION}] activating, claiming clients`)
  event.waitUntil(self.clients.claim())
})

// ============================================================================
// 8. message 通道：与主线程通信
// ============================================================================

self.addEventListener('message', (event) => {
  const data = event.data
  if (!data || typeof data !== 'object') return
  switch (data.type) {
    case 'SKIP_WAITING':
      self.skipWaiting()
      break
    case 'CLEAR_CACHE':
      event.waitUntil(
        caches.keys().then((keys) => Promise.all(keys.map((k) => caches.delete(k))))
          .then(() => {
            if (event.source) {
              event.source.postMessage({ type: 'CACHE_CLEARED' })
            }
          })
      )
      break
    case 'GET_VERSION':
      if (event.source) {
        event.source.postMessage({ type: 'SW_VERSION', version: VERSION })
      }
      break
    case 'PING':
      if (event.source) {
        event.source.postMessage({ type: 'PONG', version: VERSION, timestamp: Date.now() })
      }
      break
    case 'CACHE_URLS':
      if (Array.isArray(data.urls)) {
        event.waitUntil(
          caches.open(RUNTIME_CACHE_APP).then((cache) => cache.addAll(data.urls))
        )
      }
      break
  }
})

// ============================================================================
// 9. push 通知（Web Push 接入占位，D3 完整实现）
// ============================================================================

self.addEventListener('push', (event) => {
  if (!event.data) return
  let payload: { title?: string; body?: string; icon?: string; badge?: string; url?: string; tag?: string } = {}
  try {
    payload = event.data.json()
  } catch {
    payload = { title: '云集编程', body: event.data.text() }
  }
  const options: NotificationOptions & { vibrate?: number[]; renotify?: boolean } = {
    body: payload.body || '',
    icon: payload.icon || '/icons/icon-192.svg',
    badge: payload.badge || '/icons/favicon.svg',
    data: { url: payload.url || '/' },
    tag: payload.tag || 'yj-push',
    renotify: true,
    vibrate: [100, 50, 100],
  }
  event.waitUntil(self.registration.showNotification(payload.title || '云集编程', options))
})

self.addEventListener('notificationclick', (event) => {
  event.notification.close()
  const url = (event.notification.data as { url?: string })?.url || '/'
  event.waitUntil(
    self.clients.matchAll({ type: 'window', includeUncontrolled: true }).then((clients) => {
      for (const c of clients) {
        if (c.url.endsWith(url) && 'focus' in c) return c.focus()
      }
      if (self.clients.openWindow) return self.clients.openWindow(url)
      return undefined
    })
  )
})

// ============================================================================
// 11. Background Sync API（2026-06-09 TASK-4.3 D2-3 引入）
// ============================================================================
//
// 浏览器在网络恢复时自动唤醒 SW，触发 sync 事件。
// 我们用 message 通知主线程触发 flush（SW 内部 fetch 缺少 auth token）。
// 如果没有客户端打开（用户关闭了浏览器），sync 事件直接完成，
// 下次打开应用时由主线程 online 事件 + 启动 flush 兜底。

self.addEventListener('sync', (event: any) => {
  if (event.tag !== 'yj-sync-queue') return
  console.info(`[SW ${VERSION}] background sync fired, notifying clients`)

  event.waitUntil(
    self.clients.matchAll({ type: 'window', includeUncontrolled: true }).then((clients) => {
      if (clients.length === 0) {
        console.info(`[SW ${VERSION}] no clients, sync will retry on next open`)
        return
      }
      for (const c of clients) {
        c.postMessage({ type: 'SYNC_QUEUE_FLUSH', source: 'background-sync' })
      }
    })
  )
})

// ============================================================================
// 10. 未捕获错误兜底
// ============================================================================

self.addEventListener('error', (event) => {
  console.error('[SW] unhandled error:', event.message)
})

self.addEventListener('unhandledrejection', (event) => {
  console.error('[SW] unhandled promise rejection:', event.reason)
})
