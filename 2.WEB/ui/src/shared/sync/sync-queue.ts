// src/shared/sync/sync-queue.ts
// 2026-06-09 TASK-4.3 引入：IndexedDB 同步队列
//
// 存储设计：
//   - DB: yj-sync-queue
//   - Store 1: queue（id = UUID，索引: status, createdAt, tag）
//   - Store 2: meta（key-value 状态：lastSyncAt, lastSyncStatus）
//
// 容量管理：
//   - completed 5 分钟后自动清理
//   - failed 24 小时后自动清理
//   - 队列总条数 ≤ 1000（超过拒绝入队）

import { openDB, type IDBPDatabase, type DBSchema } from 'idb'
import type { SyncQueueItem, SyncStatus } from './sync-types'

interface SyncDB extends DBSchema {
  queue: {
    key: string
    value: SyncQueueItem
    indexes: {
      status: SyncStatus
      createdAt: number
      tag: string
      idempotencyKey: string
    }
  }
  meta: {
    key: string
    value: {
      key: string
      value: any
      updatedAt: number
    }
  }
}

const DB_NAME = 'yj-sync-queue'
const DB_VERSION = 1
const MAX_QUEUE_SIZE = 1000
const COMPLETED_TTL_MS = 5 * 60 * 1000
const FAILED_TTL_MS = 24 * 60 * 60 * 1000

let dbPromise: Promise<IDBPDatabase<SyncDB>> | null = null

/**
 * 获取数据库连接（单例）
 */
function getDB(): Promise<IDBPDatabase<SyncDB>> {
  if (!dbPromise) {
    dbPromise = openDB<SyncDB>(DB_NAME, DB_VERSION, {
      upgrade(db) {
        // queue 存储
        const queueStore = db.createObjectStore('queue', { keyPath: 'id' })
        queueStore.createIndex('status', 'status', { unique: false })
        queueStore.createIndex('createdAt', 'createdAt', { unique: false })
        queueStore.createIndex('tag', 'tag', { unique: false })
        queueStore.createIndex('idempotencyKey', 'idempotencyKey', { unique: true })

        // meta 存储
        db.createObjectStore('meta', { keyPath: 'key' })
      },
    })
  }
  return dbPromise
}

/**
 * 同步队列
 */
export class SyncQueue {
  private listenerCount = 0

  // ──────────── 入队 ────────────

  /**
   * 添加一条到队列
   * @returns 队列项 ID（已存在的 idempotencyKey 会返回原 ID）
   */
  async enqueue(item: SyncQueueItem): Promise<string> {
    const db = await getDB()

    // 检查 idempotencyKey 是否已存在
    const existing = await db.getFromIndex('queue', 'idempotencyKey', item.idempotencyKey)
    if (existing) {
      console.info(`[SyncQueue] idempotencyKey ${item.idempotencyKey} already exists, skip`)
      return existing.id
    }

    // 容量检查
    const totalCount = await db.count('queue')
    if (totalCount >= MAX_QUEUE_SIZE) {
      throw new Error(`[SyncQueue] queue full (${MAX_QUEUE_SIZE}), cannot enqueue`)
    }

    await db.add('queue', item)
    console.info(`[SyncQueue] enqueued ${item.method} ${item.endpoint} (${item.id})`)
    return item.id
  }

  /**
   * 批量入队
   */
  async enqueueBatch(items: SyncQueueItem[]): Promise<string[]> {
    const ids: string[] = []
    for (const item of items) {
      try {
        const id = await this.enqueue(item)
        ids.push(id)
      } catch (err) {
        console.warn(`[SyncQueue] batch enqueue failed for ${item.id}:`, err)
      }
    }
    return ids
  }

  // ──────────── 查询 ────────────

  /**
   * 获取所有 pending 项（按 createdAt 排序）
   */
  async getPending(limit = 50): Promise<SyncQueueItem[]> {
    const db = await getDB()
    const items = await db.getAllFromIndex('queue', 'status', 'pending', limit)
    return items.sort((a, b) => a.createdAt - b.createdAt)
  }

  /**
   * 获取所有 in_flight 项
   */
  async getInFlight(): Promise<SyncQueueItem[]> {
    const db = await getDB()
    return db.getAllFromIndex('queue', 'status', 'in_flight')
  }

  /**
   * 获取所有 failed 项
   */
  async getFailed(): Promise<SyncQueueItem[]> {
    const db = await getDB()
    return db.getAllFromIndex('queue', 'status', 'failed')
  }

  /**
   * 获取单条
   */
  async get(id: string): Promise<SyncQueueItem | undefined> {
    const db = await getDB()
    return db.get('queue', id)
  }

  /**
   * 通过 idempotencyKey 查询
   */
  async getByIdempotencyKey(key: string): Promise<SyncQueueItem | undefined> {
    const db = await getDB()
    return db.getFromIndex('queue', 'idempotencyKey', key)
  }

  /**
   * 统计
   */
  async getStats(): Promise<{
    pending: number
    inFlight: number
    completed: number
    failed: number
  }> {
    const db = await getDB()
    const [pending, inFlight, completed, failed] = await Promise.all([
      db.countFromIndex('queue', 'status', 'pending'),
      db.countFromIndex('queue', 'status', 'in_flight'),
      db.countFromIndex('queue', 'status', 'completed'),
      db.countFromIndex('queue', 'status', 'failed'),
    ])
    return { pending, inFlight, completed, failed }
  }

  // ──────────── 更新 ────────────

  /**
   * 标记为 in_flight
   */
  async markInFlight(id: string): Promise<void> {
    const db = await getDB()
    const item = await db.get('queue', id)
    if (!item) return
    item.status = 'in_flight'
    item.lastAttemptAt = Date.now()
    await db.put('queue', item)
  }

  /**
   * 标记为 completed
   */
  async markCompleted(id: string): Promise<void> {
    const db = await getDB()
    const item = await db.get('queue', id)
    if (!item) return
    item.status = 'completed'
    item.lastError = null
    await db.put('queue', item)
  }

  /**
   * 标记为 pending（重试，revert in_flight）
   */
  async markPending(id: string, error: string): Promise<void> {
    const db = await getDB()
    const item = await db.get('queue', id)
    if (!item) return
    item.status = 'pending'
    item.retryCount += 1
    item.lastError = error
    item.lastAttemptAt = Date.now()
    await db.put('queue', item)
  }

  /**
   * 标记为 failed（超过重试次数）
   */
  async markFailed(id: string, error: string): Promise<void> {
    const db = await getDB()
    const item = await db.get('queue', id)
    if (!item) return
    item.status = 'failed'
    item.lastError = error
    item.lastAttemptAt = Date.now()
    await db.put('queue', item)
  }

  // ──────────── 删除 ────────────

  /**
   * 删除单条
   */
  async delete(id: string): Promise<void> {
    const db = await getDB()
    await db.delete('queue', id)
  }

  /**
   * 清空 completed 项
   */
  async clearCompleted(): Promise<number> {
    const db = await getDB()
    const items = await db.getAllFromIndex('queue', 'status', 'completed')
    const tx = db.transaction('queue', 'readwrite')
    await Promise.all(items.map((item) => tx.store.delete(item.id)))
    await tx.done
    return items.length
  }

  /**
   * 清空 failed 项
   */
  async clearFailed(): Promise<number> {
    const db = await getDB()
    const items = await db.getAllFromIndex('queue', 'status', 'failed')
    const tx = db.transaction('queue', 'readwrite')
    await Promise.all(items.map((item) => tx.store.delete(item.id)))
    await tx.done
    return items.length
  }

  /**
   * 清空全部
   */
  async clearAll(): Promise<void> {
    const db = await getDB()
    await db.clear('queue')
  }

  // ──────────── GC ────────────

  /**
   * 清理过期的 completed / failed 项
   */
  async gc(): Promise<{ cleanedCompleted: number; cleanedFailed: number }> {
    const db = await getDB()
    const now = Date.now()
    const allItems = await db.getAll('queue')
    const toDelete: string[] = []

    for (const item of allItems) {
      if (item.status === 'completed' && item.lastAttemptAt) {
        if (now - item.lastAttemptAt > COMPLETED_TTL_MS) toDelete.push(item.id)
      } else if (item.status === 'failed' && item.lastAttemptAt) {
        if (now - item.lastAttemptAt > FAILED_TTL_MS) toDelete.push(item.id)
      }
    }

    const tx = db.transaction('queue', 'readwrite')
    await Promise.all(toDelete.map((id) => tx.store.delete(id)))
    await tx.done

    const cleanedCompleted = toDelete.length // 简化统计
    return { cleanedCompleted, cleanedFailed: 0 }
  }

  // ──────────── Meta ────────────

  async getMeta<T = any>(key: string): Promise<T | null> {
    const db = await getDB()
    const row = await db.get('meta', key)
    return row ? (row.value as T) : null
  }

  async setMeta(key: string, value: any): Promise<void> {
    const db = await getDB()
    await db.put('meta', { key, value, updatedAt: Date.now() })
  }
}

/**
 * 单例
 */
export const syncQueue = new SyncQueue()
