// src/shared/sync/sync-api.ts
// 2026-06-09 TASK-4.3 引入：API 调用的"队列化"封装
//
// 用法：
//   import { syncFetch } from '@shared/sync'
//   const res = await syncFetch('/api/chat/save', { method: 'POST', body: {...}, tag: 'chat-save' })
//
// 行为：
//   - 在线时：直接 fetch（绕过队列，避免延迟）
//   - 离线时：入队，返回 { queued: true, id }
//   - 写入操作（POST/PUT/PATCH/DELETE）入队；
//     读取操作（GET）不入队（直接走 Service Worker 缓存策略）
//
// 注意：这是一个轻量封装，复杂的业务错误处理在调用方

import { syncManager } from './sync-manager'

export interface SyncFetchOptions extends Omit<RequestInit, 'method' | 'body'> {
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'
  body?: any
  /** 业务标签（用于分组 / 调试） */
  tag: string
  /** 业务上下文 */
  context?: {
    label?: string
    refId?: string
    refType?: 'conversation' | 'message' | 'project' | 'setting' | 'other'
    priority?: 1 | 2 | 3
  }
  /** 强制入队（即使在线也走队列） */
  forceQueue?: boolean
}

export interface SyncFetchResult<T = any> {
  /** 是否直接返回了 fetch 响应 */
  ok: boolean
  /** 响应数据（直接 fetch 模式） */
  data?: T
  /** 是否已入队（离线模式） */
  queued?: boolean
  /** 队列项 ID（入队时） */
  queueId?: string
  /** 状态码（直接 fetch 模式） */
  status?: number
  /** 错误信息 */
  error?: string
}

const WRITE_METHODS: Set<string> = new Set(['POST', 'PUT', 'PATCH', 'DELETE'])

/**
 * 队列化 fetch
 */
export async function syncFetch<T = any>(
  endpoint: string,
  options: SyncFetchOptions
): Promise<SyncFetchResult<T>> {
  const method = options.method || 'GET'
  const isWrite = WRITE_METHODS.has(method.toUpperCase())

  // GET 请求：直接走 fetch（Service Worker 会处理缓存）
  if (!isWrite && !options.forceQueue) {
    return directFetch<T>(endpoint, options)
  }

  // 在线 + 不强制入队：直接 fetch
  if (navigator.onLine && !options.forceQueue) {
    return directFetch<T>(endpoint, options)
  }

  // 离线 / 强制入队：入队
  const id = await syncManager.enqueue({
    endpoint,
    method: method as 'POST' | 'PUT' | 'PATCH' | 'DELETE',
    body: options.body,
    headers: options.headers as Record<string, string> | undefined,
    tag: options.tag,
    context: options.context,
  })

  return {
    ok: true,
    queued: true,
    queueId: id,
  }
}

/**
 * 直接 fetch（不入队）
 */
async function directFetch<T>(
  endpoint: string,
  options: SyncFetchOptions
): Promise<SyncFetchResult<T>> {
  try {
    const response = await fetch(endpoint, {
      method: options.method || 'GET',
      headers: {
        'Content-Type': 'application/json',
        ...(options.headers as Record<string, string> | undefined),
      },
      body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
      credentials: options.credentials,
      signal: options.signal,
    })

    let data: any = null
    try {
      data = await response.clone().json()
    } catch {
      /* ignore */
    }

    return {
      ok: response.ok,
      data: data as T,
      status: response.status,
    }
  } catch (err) {
    // 网络错误：如果写入操作，自动入队
    if (WRITE_METHODS.has((options.method || 'GET').toUpperCase())) {
      const id = await syncManager.enqueue({
        endpoint,
        method: options.method as 'POST' | 'PUT' | 'PATCH' | 'DELETE',
        body: options.body,
        headers: options.headers as Record<string, string> | undefined,
        tag: options.tag,
        context: options.context,
      })
      return { ok: true, queued: true, queueId: id }
    }

    return {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    }
  }
}
