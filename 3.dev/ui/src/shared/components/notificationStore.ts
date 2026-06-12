// shared/components/notificationStore.ts
// 2026-06-08 TASK-2.7 引入：统一通知系统 - 通知中心持久化 store
// 配套 YJNotificationCenter.vue 使用
// 设计：composable + localStorage 持久化（最近 100 条）

import { ref, computed } from 'vue'
import type { ToastType } from './toast'

export interface NotificationItem {
  id: number
  type: ToastType
  message: string
  detail?: string
  timestamp: number
  read: boolean
  // 可选：点击通知跳转
  action?: { label: string; path?: string; onClick?: () => void }
}

const STORAGE_KEY = 'yj_notifications_v1'
const MAX_NOTIFICATIONS = 100
const TOAST_PROMOTE_LEVELS: ToastType[] = ['error', 'warning']

// 单例 ref（跨组件共享）
const notifications = ref<NotificationItem[]>(loadFromStorage())
const toastBridge = ref<((n: Omit<NotificationItem, 'id' | 'timestamp' | 'read'>) => void) | null>(null)

function loadFromStorage(): NotificationItem[] {
  if (typeof localStorage === 'undefined') return []
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return []
    const arr = JSON.parse(raw) as NotificationItem[]
    if (!Array.isArray(arr)) return []
    return arr.slice(0, MAX_NOTIFICATIONS)
  } catch {
    return []
  }
}

function saveToStorage() {
  if (typeof localStorage === 'undefined') return
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(notifications.value))
  } catch {
    // localStorage 满 / 禁用
  }
}

let nextId = Math.max(0, ...notifications.value.map(n => n.id)) + 1

function add(notif: Omit<NotificationItem, 'id' | 'timestamp' | 'read'>): number {
  const item: NotificationItem = {
    ...notif,
    id: nextId++,
    timestamp: Date.now(),
    read: false,
  }
  // 新通知插入到最前面
  notifications.value.unshift(item)
  if (notifications.value.length > MAX_NOTIFICATIONS) {
    notifications.value = notifications.value.slice(0, MAX_NOTIFICATIONS)
  }
  saveToStorage()
  return item.id
}

function remove(id: number): void {
  const idx = notifications.value.findIndex(n => n.id === id)
  if (idx > -1) {
    notifications.value.splice(idx, 1)
    saveToStorage()
  }
}

function markAsRead(id: number): void {
  const n = notifications.value.find(x => x.id === id)
  if (n && !n.read) {
    n.read = true
    saveToStorage()
  }
}

function markAllAsRead(): void {
  let changed = false
  for (const n of notifications.value) {
    if (!n.read) {
      n.read = true
      changed = true
    }
  }
  if (changed) saveToStorage()
}

function clear(): void {
  notifications.value = []
  saveToStorage()
}

const unreadCount = computed(() => notifications.value.filter(n => !n.read).length)
const total = computed(() => notifications.value.length)

/**
 * 注册 toast 桥接：toast 弹出后自动晋升为通知中心条目
 * 应在 YJNotificationCenter 挂载时调用
 */
function setToastBridge(bridge: typeof toastBridge.value) {
  toastBridge.value = bridge
}

/**
 * 从 toast 调用：error / warning 自动晋升，其他类型按需晋升
 */
function promoteFromToast(type: ToastType, message: string) {
  if (!TOAST_PROMOTE_LEVELS.includes(type)) return
  add({ type, message })
}

// 时间格式化（中文）
function formatRelativeTime(ts: number): string {
  const now = Date.now()
  const diff = now - ts
  if (diff < 60_000) return '刚刚'
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.floor(diff / 3600_000)} 小时前`
  if (diff < 7 * 86_400_000) return `${Math.floor(diff / 86_400_000)} 天前`
  return new Date(ts).toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
}

export function useNotificationStore() {
  return {
    notifications,
    unreadCount,
    total,
    add,
    remove,
    markAsRead,
    markAllAsRead,
    clear,
    setToastBridge,
    promoteFromToast,
    formatRelativeTime,
  }
}
