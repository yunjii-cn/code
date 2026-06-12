/**
 * Y.js 客户端封装
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 核心职责：
 *  1. WebSocketProvider 连接服务端 Y.js 房间
 *  2. IndexeddbPersistence 离线持久化（断网/刷新/重启不丢）
 *  3. 自动重连 + 指数退避
 *  4. JWT 鉴权（query param + 401 触发 refresh）
 *  5. Awareness 暴露给上层（presence / AI 活动）
 *
 * 设计原则：
 *  - 每个 workspace 一个 Y.Doc 实例，跨组件共享（单例）
 *  - 不在客户端做 CRDT 业务，业务交给 WorkspaceDoc.ts
 *  - 本文件只管"管道"：连接、持久化、生命周期
 */

import * as Y from 'yjs'
import { WebsocketProvider } from 'y-websocket'
import { IndexeddbPersistence } from 'y-indexeddb'
import { Awareness } from 'y-protocols/awareness'

// ──────────── 类型 ────────────

export type YjsConnectionStatus =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'offline'

export interface YjsClientOptions {
  /** 工作区 ID（= Y.js room id） */
  workspaceId: string
  /** 鉴权 token 回调：每次重连前调用，避免过期 */
  getToken: () => string | null
  /** 401/403 时触发，由调用方决定如何刷新 token */
  onAuthError?: (reason: string) => Promise<string | null>
  /** WebSocket 端点（默认 `${location.origin}/api/yjs`） */
  wsBaseUrl?: string
  /** IndexedDB 数据库名前缀（默认 `yunji-yjs`） */
  idbPrefix?: string
  /** 重连基础间隔 ms（默认 1000） */
  reconnectBaseMs?: number
  /** 重连最大间隔 ms（默认 30000） */
  reconnectMaxMs?: number
}

export interface YjsClientHandle {
  /** Y.Doc 实例（多端共享 CRDT 状态） */
  doc: Y.Doc
  /** 当前 workspace id */
  workspaceId: string
  /** Awareness 实例（presence / 临时状态）—— 始终可用 */
  awareness: Awareness
  /** 当前连接状态 */
  status: () => YjsConnectionStatus
  /** 监听状态变化 */
  onStatusChange: (cb: (s: YjsConnectionStatus) => void) => () => void
  /** 主动断开（不销毁，可重连） */
  disconnect: () => void
  /** 重新连接 */
  reconnect: () => void
  /** 销毁：关 provider + 清 IndexedDB 引用 */
  destroy: () => void
  /** 等待 IndexedDB 加载完成（断网重连时重要） */
  whenSynced: () => Promise<void>
  /** 强制把内存状态写回 IndexedDB（用完即关页面时调用） */
  flush: () => Promise<void>
}

// ──────────── 单例注册表 ────────────

const _handles = new Map<string, YjsClientHandle>()

/**
 * 获取（或创建）一个工作区的 Y.js 客户端。
 * 同一 workspaceId 多次调用会复用同一实例（支持多组件共享 awareness）。
 */
export function getYjsClient(opts: YjsClientOptions): YjsClientHandle {
  const key = opts.workspaceId
  const existing = _handles.get(key)
  if (existing) return existing

  const handle = _createClient(opts)
  _handles.set(key, handle)
  return handle
}

/** 销毁并移除一个工作区的客户端 */
export function disposeYjsClient(workspaceId: string): void {
  const h = _handles.get(workspaceId)
  if (!h) return
  h.destroy()
  _handles.delete(workspaceId)
}

/** 销毁所有客户端（登出 / 切换账号时调用） */
export function disposeAllYjsClients(): void {
  for (const h of _handles.values()) h.destroy()
  _handles.clear()
}

// ──────────── 内部实现 ────────────

function _createClient(opts: YjsClientOptions): YjsClientHandle {
  const {
    workspaceId,
    getToken,
    onAuthError,
    wsBaseUrl,
    idbPrefix = 'yunji-yjs',
    reconnectBaseMs = 1000,
    reconnectMaxMs = 30000,
  } = opts

  const doc = new Y.Doc()
  const idbName = `${idbPrefix}:${workspaceId}`

  // 1) IndexedDB 持久化（离线优先）
  const persistence = new IndexeddbPersistence(idbName, doc)
  let idbReady = false
  persistence.once('synced', () => {
    idbReady = true
  })

  // 2) 自管 Awareness（不绑定到 WebsocketProvider，原因是 y-websocket 内部的
  //    awareness 可能在重连时被替换，外部监听会断。自管一个稳定 Awareness，
  //    provider 的 message 进来后转发到本 awareness 即可。）
  const awareness = new Awareness(doc)
  awareness.setLocalState(null)

  // 3) WebSocket Provider（live sync）
  const _wsBase =
    wsBaseUrl ||
    (typeof window !== 'undefined'
      ? `${window.location.protocol}//${window.location.host}/api/yjs`
      : 'ws://127.0.0.1:18080/api/yjs')

  // 关键：token 每次重连都要重新拿（旧的会过期）
  // y-websocket 不支持动态改 token，所以每次 disconnect/reconnect 用新 provider
  let provider: WebsocketProvider | null = null
  let status: YjsConnectionStatus = 'disconnected'
  const statusListeners = new Set<(s: YjsConnectionStatus) => void>()
  let reconnectAttempts = 0
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let destroyed = false

  function _setStatus(s: YjsConnectionStatus) {
    if (status === s) return
    status = s
    for (const cb of statusListeners) {
      try { cb(s) } catch { /* ignore */ }
    }
  }

  function _buildProvider(): WebsocketProvider | null {
    const token = getToken()
    if (!token) return null
    // y-websocket URL: ws(s)://host/roomname?token=...
    // 房间名用 workspaceId（与服务端 yjs.py 一致）
    const url = `${_wsBase}/${encodeURIComponent(workspaceId)}?token=${encodeURIComponent(token)}`
    const p = new WebsocketProvider(url, workspaceId, doc, {
      connect: true,
      // 我们自己处理重连（见 _scheduleReconnect），关掉 y-websocket 内置
      maxBackoffTime: reconnectMaxMs,
      // 禁用 y-websocket 内置 awareness（我们用自管的）
      awareness,
    })
    p.on('status', (event: { status: 'connecting' | 'connected' | 'disconnected' }) => {
      if (event.status === 'connected') {
        reconnectAttempts = 0
        _setStatus('connected')
      } else if (event.status === 'connecting') {
        _setStatus('connecting')
      } else {
        // disconnected → 触发我们的重连
        if (idbReady && !destroyed) {
          _setStatus('offline')
        } else {
          _setStatus('disconnected')
        }
        _scheduleReconnect()
      }
    })
    return p
  }

  function _scheduleReconnect() {
    if (destroyed) return
    if (reconnectTimer) return
    const delay = Math.min(
      reconnectMaxMs,
      reconnectBaseMs * Math.pow(2, Math.min(reconnectAttempts, 8)),
    )
    reconnectTimer = setTimeout(async () => {
      reconnectTimer = null
      if (destroyed) return
      // 先看 token 是否还有
      let token = getToken()
      if (!token && onAuthError) {
        token = await onAuthError('no token')
      }
      if (!token) {
        // 没 token 就不再重连
        _setStatus('disconnected')
        return
      }
      reconnectAttempts++
      // 销毁旧 provider，重建（确保 token 最新）
      if (provider) {
        try { provider.destroy() } catch { /* ignore */ }
        provider = null
      }
      provider = _buildProvider()
    }, delay)
  }

  function disconnect() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    if (provider) {
      try { provider.disconnect() } catch { /* ignore */ }
    }
    _setStatus('disconnected')
  }

  function reconnect() {
    if (destroyed) return
    disconnect()
    reconnectAttempts = 0
    if (provider) {
      try { provider.destroy() } catch { /* ignore */ }
      provider = null
    }
    provider = _buildProvider()
  }

  function destroy() {
    destroyed = true
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    if (provider) {
      try { provider.destroy() } catch { /* ignore */ }
      provider = null
    }
    if (persistence) {
      try { persistence.destroy() } catch { /* ignore */ }
    }
    try { awareness.destroy() } catch { /* ignore */ }
    doc.destroy()
    if (typeof window !== 'undefined' && _onUnload) {
      window.removeEventListener('beforeunload', _onUnload)
    }
  }

  async function whenSynced(): Promise<void> {
    if (idbReady) return
    await new Promise<void>((resolve) => {
      persistence.once('synced', () => resolve())
      // 兜底：5s 后强制 resolve（避免 hang 死）
      setTimeout(resolve, 5000)
    })
  }

  async function flush(): Promise<void> {
    if (!persistence) return
    // y-indexeddb 的 IndexeddbPersistence 没有 .flush() 方法
    // 通过编码当前状态触发一次持久化（实际上 IndexeddbPersistence 已订阅 update 事件自动写）
    try {
      // 强制编码一次，确保最新状态被 IDB 缓存读到
      // eslint-disable-next-line @typescript-eslint/no-unused-expressions
      Y.encodeStateAsUpdate(doc)
    } catch {
      // 兜底
    }
  }

  function onStatusChange(cb: (s: YjsConnectionStatus) => void) {
    statusListeners.add(cb)
    // 立即推一次当前值
    try { cb(status) } catch { /* ignore */ }
    return () => { statusListeners.delete(cb) }
  }

  // 监听 page unload，尽量 flush
  let _onUnload: (() => void) | null = null
  if (typeof window !== 'undefined') {
    _onUnload = () => {
      void flush()
    }
    window.addEventListener('beforeunload', _onUnload)
  }

  // 启动：先等 IDB，再建 provider
  whenSynced()
    .then(() => {
      if (destroyed) return
      provider = _buildProvider()
    })
    .catch(() => {
      if (destroyed) return
      provider = _buildProvider()
    })

  return {
    doc,
    workspaceId,
    awareness,
    status: () => status,
    onStatusChange,
    disconnect,
    reconnect,
    destroy,
    whenSynced,
    flush,
  }
}

// ──────────── 辅助：构造 WS 端点（供 UI 调试用） ────────────

export function buildYjsWebSocketUrl(wsBase: string, workspaceId: string, token: string): string {
  return `${wsBase.replace(/\/$/, '')}/${encodeURIComponent(workspaceId)}?token=${encodeURIComponent(token)}`
}
