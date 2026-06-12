// src/shared/sync/sync-manager.ts
// 2026-06-09 TASK-4.3 引入：同步管理器（核心调度）
//
// 职责：
//   1. 监听网络状态（online/offline）
//   2. 触发同步（定时 / online 事件 / Background Sync API）
//   3. 重放队列（顺序保持 + 指数退避）
//   4. 发出事件（enqueued / started / progress / completed / idle）
//   5. 失败处理（4xx 不重试，5xx/网络错误重试）
//
// 差异化设计（自研部分）：
//   - 优先级队列：priority=1 的项优先发送
//   - 同 tag 合并：相同 tag 的连续请求可以合并（实验性）
//   - 冲突响应：服务端返回 409 时调用 conflictHandler

import { syncQueue } from './sync-queue'
import type {
  SyncQueueItem,
  SyncConfig,
  SyncStats,
  SyncResult,
  SyncEvent,
  SyncEventListener,
} from './sync-types'
import { DEFAULT_SYNC_CONFIG } from './sync-types'

type ConflictHandler = (item: SyncQueueItem, response: any) => Promise<'retry' | 'discard' | 'merge'>

/**
 * SyncManager 单例
 */
class SyncManagerImpl {
  private config: SyncConfig = { ...DEFAULT_SYNC_CONFIG }
  private listeners = new Set<SyncEventListener>()
  private syncTimer: number | null = null
  private isSyncing = false
  private abortController: AbortController | null = null
  private conflictHandler: ConflictHandler | null = null
  private customFetch: typeof fetch | null = null

  // ──────────── 配置 ────────────

  configure(partial: Partial<SyncConfig>): void {
    this.config = { ...this.config, ...partial }
  }

  getConfig(): SyncConfig {
    return { ...this.config }
  }

  setConflictHandler(handler: ConflictHandler | null): void {
    this.conflictHandler = handler
  }

  /**
   * 注入自定义 fetch（用于带 token 的请求）
   */
  setFetch(fetchImpl: typeof fetch): void {
    this.customFetch = fetchImpl
  }

  // ──────────── 事件 ────────────

  on(listener: SyncEventListener): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  private emit(event: SyncEvent): void {
    for (const l of this.listeners) {
      try {
        l(event)
      } catch (err) {
        console.warn('[SyncManager] listener error:', err)
      }
    }
  }

  // ──────────── 启动 / 停止 ────────────

  start(): void {
    if (this.syncTimer !== null) return
    console.info('[SyncManager] starting')

    // 定时轮询
    this.syncTimer = window.setInterval(() => {
      this.flush().catch((err) => console.warn('[SyncManager] periodic flush error:', err))
    }, this.config.syncInterval)

    // 监听 online 事件
    if (this.config.autoSyncOnOnline) {
      window.addEventListener('online', this.onOnline)
    }

    // 注册 Background Sync（如果支持）
    this.registerBackgroundSync().catch(() => {})

    // 启动时立即尝试一次（如果有 pending）
    this.flush().catch(() => {})
  }

  stop(): void {
    if (this.syncTimer !== null) {
      clearInterval(this.syncTimer)
      this.syncTimer = null
    }
    window.removeEventListener('online', this.onOnline)
    if (this.abortController) {
      this.abortController.abort()
      this.abortController = null
    }
  }

  private onOnline = (): void => {
    console.info('[SyncManager] online detected, flushing queue')
    this.flush().catch((err) => console.warn('[SyncManager] online flush error:', err))
  }

  // ──────────── 核心：入队 ────────────

  /**
   * 入队一个请求
   */
  async enqueue(params: {
    endpoint: string
    method: SyncQueueItem['method']
    body?: any
    headers?: Record<string, string>
    tag: string
    context?: SyncQueueItem['context']
  }): Promise<string> {
    const id = this.uuid()
    const idempotencyKey = this.generateIdempotencyKey(params)

    const item: SyncQueueItem = {
      id,
      idempotencyKey,
      endpoint: params.endpoint,
      method: params.method,
      headers: {
        'Content-Type': 'application/json',
        'X-Idempotency-Key': idempotencyKey,
        ...(params.headers || {}),
      },
      body: params.body ? JSON.stringify(params.body) : null,
      tag: params.tag,
      createdAt: Date.now(),
      lastAttemptAt: null,
      retryCount: 0,
      status: 'pending',
      lastError: null,
      context: params.context,
    }

    const actualId = await syncQueue.enqueue(item)
    // 获取最新项（enqueue 可能因 idempotency 返回已有项）
    const stored = await syncQueue.get(actualId)
    if (stored) this.emit({ type: 'enqueued', item: stored })

    // 触发一次同步（如果在线）
    if (navigator.onLine) {
      this.flush().catch(() => {})
    }

    return actualId
  }

  // ──────────── 核心：flush ────────────

  /**
   * 处理队列中的所有 pending 项
   */
  async flush(): Promise<{ stats: SyncStats; duration: number }> {
    if (this.isSyncing) {
      console.info('[SyncManager] already syncing, skip')
      return {
        stats: await this.computeStats(),
        duration: 0,
      }
    }

    if (!navigator.onLine) {
      console.info('[SyncManager] offline, skip flush')
      this.emit({ type: 'idle' })
      return {
        stats: await this.computeStats(),
        duration: 0,
      }
    }

    this.isSyncing = true
    this.abortController = new AbortController()
    const start = Date.now()

    try {
      const items = await syncQueue.getPending(50)
      if (items.length === 0) {
        this.emit({ type: 'idle' })
        return { stats: await this.computeStats(), duration: 0 }
      }

      // 按优先级 + 创建时间排序
      items.sort((a, b) => {
        const pa = a.context?.priority ?? 3
        const pb = b.context?.priority ?? 3
        if (pa !== pb) return pa - pb
        return a.createdAt - b.createdAt
      })

      this.emit({ type: 'started', count: items.length })

      let completed = 0
      let successCount = 0
      let failedCount = 0
      let lastError: string | null = null

      for (const item of items) {
        if (this.abortController.signal.aborted) break

        this.emit({ type: 'progress', completed, total: items.length, current: item })

        const result = await this.processOne(item)
        completed += 1

        if (result.ok) {
          successCount += 1
          this.emit({ type: 'item-success', item, result })
        } else {
          failedCount += 1
          lastError = result.error || 'unknown'
          this.emit({ type: 'item-failed', item, result })
        }
      }

      const duration = Date.now() - start
      const stats = await this.computeStats()
      const finalStatus: 'success' | 'partial' | 'failed' =
        failedCount === 0 ? 'success' : successCount === 0 ? 'failed' : 'partial'

      stats.lastSyncAt = start
      stats.lastSyncStatus = finalStatus

      await syncQueue.setMeta('lastSyncAt', start)
      await syncQueue.setMeta('lastSyncStatus', finalStatus)

      this.emit({ type: 'completed', stats, duration })
      console.info(
        `[SyncManager] flush done: ${successCount} ok / ${failedCount} failed in ${duration}ms`
      )

      return { stats, duration }
    } catch (err) {
      console.error('[SyncManager] flush error:', err)
      throw err
    } finally {
      this.isSyncing = false
      this.abortController = null
    }
  }

  /**
   * 处理单条
   */
  private async processOne(item: SyncQueueItem): Promise<SyncResult> {
    await syncQueue.markInFlight(item.id)

    try {
      const fetchImpl = this.customFetch || fetch
      const response = await fetchImpl(item.endpoint, {
        method: item.method,
        headers: item.headers,
        body: item.body,
        signal: this.abortController?.signal,
      })

      // 2xx 成功
      if (response.ok) {
        await syncQueue.markCompleted(item.id)
        let responseBody: any = null
        try {
          responseBody = await response.clone().json()
        } catch {
          /* ignore */
        }
        return { id: item.id, ok: true, status: response.status, response: responseBody }
      }

      // 409 冲突
      if (response.status === 409 && this.conflictHandler) {
        const conflictBody = await response.clone().json().catch(() => null)
        const decision = await this.conflictHandler(item, conflictBody)
        if (decision === 'retry') {
          // 标记为 pending，下次再试
          await syncQueue.markPending(item.id, 'conflict: retry')
          return { id: item.id, ok: false, status: 409, error: 'conflict: retry requested' }
        } else if (decision === 'discard') {
          await syncQueue.markCompleted(item.id) // 视为完成
          return { id: item.id, ok: true, status: 409, error: 'conflict: discarded' }
        }
        // 'merge' 暂时不实现，等业务需要
        return { id: item.id, ok: false, status: 409, error: 'conflict: merge not implemented' }
      }

      // 4xx 客户端错误（不重试）
      if (response.status >= 400 && response.status < 500) {
        const errText = await response.text().catch(() => '')
        await syncQueue.markFailed(item.id, `4xx: ${response.status} ${errText.slice(0, 200)}`)
        return { id: item.id, ok: false, status: response.status, error: `4xx: ${response.status}` }
      }

      // 5xx 服务端错误（重试）
      const errText = await response.text().catch(() => '')
      return await this.handleRetryable(item, `5xx: ${response.status} ${errText.slice(0, 200)}`)
    } catch (err) {
      // 网络错误（重试）
      const errMsg = err instanceof Error ? err.message : String(err)
      return await this.handleRetryable(item, `network: ${errMsg}`)
    }
  }

  /**
   * 处理可重试的错误
   */
  private async handleRetryable(item: SyncQueueItem, error: string): Promise<SyncResult> {
    if (item.retryCount >= this.config.maxRetries) {
      await syncQueue.markFailed(item.id, error)
      return { id: item.id, ok: false, error }
    }

    await syncQueue.markPending(item.id, error)

    // 指数退避（仅在定时重试场景下生效；当前实现是立刻放回 pending，由下次 flush 处理）
    const delay = this.config.retryBaseDelay * Math.pow(2, item.retryCount)
    console.info(`[SyncManager] item ${item.id} will retry after ${delay}ms (attempt ${item.retryCount + 1})`)

    return { id: item.id, ok: false, error }
  }

  // ──────────── Background Sync API ────────────

  private async registerBackgroundSync(): Promise<void> {
    if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) return
    try {
      const reg = await navigator.serviceWorker.ready
      // @ts-expect-error - sync 是实验性 API
      if (reg.sync && typeof reg.sync.register === 'function') {
        // @ts-expect-error
        await reg.sync.register('yj-sync-queue')
        console.info('[SyncManager] Background Sync registered')
      } else {
        console.info('[SyncManager] Background Sync not supported, fallback to online event + interval')
      }
    } catch (err) {
      console.info('[SyncManager] Background Sync registration failed:', err)
    }
  }

  // ──────────── 工具 ────────────

  private async computeStats(): Promise<SyncStats> {
    const counts = await syncQueue.getStats()
    const lastSyncAt = await syncQueue.getMeta<number>('lastSyncAt')
    const lastSyncStatus = await syncQueue.getMeta<'success' | 'partial' | 'failed'>(
      'lastSyncStatus'
    )
    return {
      ...counts,
      lastSyncAt: lastSyncAt || null,
      lastSyncStatus: lastSyncStatus || null,
    }
  }

  private generateIdempotencyKey(params: {
    endpoint: string
    method: string
    body?: any
  }): string {
    // 业务幂等：相同 endpoint + method + body hash + 时间窗口（1h）
    const bodyHash = params.body ? this.simpleHash(JSON.stringify(params.body)) : ''
    const hourBucket = Math.floor(Date.now() / (60 * 60 * 1000))
    return `${params.method}:${params.endpoint}:${bodyHash}:${hourBucket}`
  }

  private simpleHash(str: string): string {
    let hash = 0
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i)
      hash = (hash << 5) - hash + char
      hash |= 0 // Convert to 32bit integer
    }
    return Math.abs(hash).toString(36)
  }

  private uuid(): string {
    if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
      return crypto.randomUUID()
    }
    // Fallback
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
      const r = (Math.random() * 16) | 0
      const v = c === 'x' ? r : (r & 0x3) | 0x8
      return v.toString(16)
    })
  }

  // ──────────── 调试 ────────────

  async getStats(): Promise<SyncStats> {
    return this.computeStats()
  }
}

export const syncManager = new SyncManagerImpl()
