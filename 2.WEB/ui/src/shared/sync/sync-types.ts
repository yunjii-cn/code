// src/shared/sync/sync-types.ts
// 2026-06-09 TASK-4.3 引入：同步队列类型定义
//
// 设计原则：
// 1. 幂等性：每个请求带 idempotencyKey，服务端可识别重复
// 2. 顺序保持：队列按 timestamp 排序，顺序重放
// 3. 状态可追踪：每条记录有 status / retryCount / lastError
// 4. 自动重试：指数退避，最多 N 次
// 5. 冲突处理：4xx 不重试，5xx 和网络错误重试

/**
 * 同步状态
 * - pending: 待发送
 * - in_flight: 正在发送
 * - completed: 已成功（保留 N 分钟后清理）
 * - failed: 失败次数过多，已放弃
 */
export type SyncStatus = 'pending' | 'in_flight' | 'completed' | 'failed'

/**
 * 队列条目
 */
export interface SyncQueueItem {
  /** 唯一 ID（UUID v4） */
  id: string
  /** 幂等键（用于服务端去重） */
  idempotencyKey: string
  /** 请求 URL（相对路径，如 /api/chat/save） */
  endpoint: string
  /** HTTP 方法 */
  method: 'POST' | 'PUT' | 'PATCH' | 'DELETE'
  /** 请求头 */
  headers: Record<string, string>
  /** 请求体（已序列化） */
  body: string | null
  /** 业务标签（用于分组 / 调试） */
  tag: string
  /** 创建时间戳（用于排序） */
  createdAt: number
  /** 最后尝试时间 */
  lastAttemptAt: number | null
  /** 尝试次数 */
  retryCount: number
  /** 状态 */
  status: SyncStatus
  /** 最后的错误信息（仅 failed 状态） */
  lastError: string | null
  /** 业务上下文（用于 UI 显示 / 冲突处理） */
  context?: {
    /** 用户可见的描述，如 "发送消息：你好" */
    label?: string
    /** 关联的会话/项目 ID */
    refId?: string
    /** 关联的类型 */
    refType?: 'conversation' | 'message' | 'project' | 'setting' | 'other'
    /** 优先级：1 = 最高 */
    priority?: 1 | 2 | 3
  }
}

/**
 * 同步统计
 */
export interface SyncStats {
  pending: number
  inFlight: number
  completed: number
  failed: number
  lastSyncAt: number | null
  lastSyncStatus: 'success' | 'partial' | 'failed' | null
}

/**
 * 同步结果（单条）
 */
export interface SyncResult {
  id: string
  ok: boolean
  status?: number
  error?: string
  /** 响应体（用于本地更新） */
  response?: any
}

/**
 * 同步事件
 */
export type SyncEvent =
  | { type: 'enqueued'; item: SyncQueueItem }
  | { type: 'started'; count: number }
  | { type: 'progress'; completed: number; total: number; current: SyncQueueItem }
  | { type: 'item-success'; item: SyncQueueItem; result: SyncResult }
  | { type: 'item-failed'; item: SyncQueueItem; result: SyncResult }
  | { type: 'completed'; stats: SyncStats; duration: number }
  | { type: 'idle' }

export type SyncEventListener = (event: SyncEvent) => void

/**
 * 同步配置
 */
export interface SyncConfig {
  /** 最大重试次数（默认 5） */
  maxRetries: number
  /** 指数退避基数（毫秒，默认 1000） */
  retryBaseDelay: number
  /** 同步间隔（毫秒，默认 30s，仅定时轮询模式） */
  syncInterval: number
  /** 是否自动在 online 事件触发同步（默认 true） */
  autoSyncOnOnline: boolean
  /** completed 状态保留时间（毫秒，默认 5 分钟） */
  completedRetention: number
  /** 失败状态保留时间（毫秒，默认 24 小时） */
  failedRetention: number
}

export const DEFAULT_SYNC_CONFIG: SyncConfig = {
  maxRetries: 5,
  retryBaseDelay: 1000,
  syncInterval: 30_000,
  autoSyncOnOnline: true,
  completedRetention: 5 * 60 * 1000,
  failedRetention: 24 * 60 * 60 * 1000,
}
