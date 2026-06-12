// shared/composables/useToast.ts
// 2026-06-10 TASK-2.7 引入：统一通知系统 — 增强 useToast composable
//
// 设计：
//   1. 底层单例：components/toast.ts 管理 YJToast 组件实例
//   2. 上层 API：本文件提供 ToastOptions（action / detail / position / duration）增强用法
//   3. 队列管理：同时最多显示 3 个 toast（超出 FIFO 自动移除最早）
//   4. action 按钮：toast 上可附带操作按钮（如"撤销"/"重试"）
//   5. 位置：top / top-right / top-left / center / bottom
//
// 用法：
//   import { useToast } from '@shared/composables/useToast'
//   const { show, success, error, dismiss } = useToast()
//
//   show({ type: 'success', message: '保存成功', duration: 3000 })
//   show({
//     type: 'error',
//     message: '上传失败',
//     detail: '网络错误：connection reset',
//     duration: 0,  // 0 = 不自动关闭
//     action: { label: '重试', onClick: () => retry() },
//   })

import { useToast as useToastBase } from '../components/toast.ts'
import type { ToastType } from '../components/toast.ts'

/** Toast 位置 */
export type ToastPosition =
  | 'top'           // 顶部居中（默认）
  | 'top-right'     // 右上角
  | 'top-left'      // 左上角
  | 'center'        // 屏幕中央
  | 'bottom'        // 底部居中
  | 'bottom-right'  // 右下角
  | 'bottom-left'   // 左下角

/** Toast 操作按钮 */
export interface ToastAction {
  label: string
  onClick: () => void | Promise<void>
  /** 操作按钮点击后是否自动关闭 toast（默认 true） */
  closeOnClick?: boolean
  /** 操作按钮颜色：'primary' | 'danger' | 'default'，默认 'primary' */
  variant?: 'primary' | 'danger' | 'default'
}

/** Toast 选项 */
export interface ToastOptions {
  /** 必填：toast 文本 */
  message: string
  /** 类型：success / error / warning / info / loading（默认 'info'） */
  type?: ToastType
  /** 自动关闭时间（ms），0 = 不自动关闭（默认 3000） */
  duration?: number
  /** 位置 */
  position?: ToastPosition
  /** 详情（多行提示） */
  detail?: string
  /** 操作按钮 */
  action?: ToastAction
  /** 唯一的 key（用于替换已有同 key 的 toast） */
  key?: string
  /** 关闭回调 */
  onClose?: () => void
}

/** 内部 toast 队列项 */
interface QueuedToast extends Required<Omit<ToastOptions, 'action' | 'key' | 'onClose'>> {
  id: number
  key?: string
  action?: ToastAction
  onClose?: () => void
}

/** 全局队列最大数（同时显示） */
const MAX_VISIBLE = 3

/** 全局待处理队列（YJToast 组件通过 onMounted 注入了 callback） */
let _enqueue: ((toast: QueuedToast) => void) | null = null
let _remove: ((id: number) => void) | null = null

/**
 * YJToast.vue 调用：注册内部 queue / remove 句柄
 * @internal
 */
export function __bindToastBridge(handlers: {
  enqueue: (t: QueuedToast) => void
  remove: (id: number) => void
}): void {
  _enqueue = handlers.enqueue
  _remove = handlers.remove
}

export function __unbindToastBridge(): void {
  _enqueue = null
  _remove = null
}

let _id = 1

/**
 * 关闭指定 id 的 toast
 */
function dismissId(id: number): void {
  if (_remove) {
    _remove(id)
  } else {
    // 后备：调底层 api
    useToastBase().remove(id)
  }
}

/**
 * 关闭指定 key 的 toast
 */
export function dismissByKey(key: string): void {
  // 通过遍历内部 queue 查找
  // 简单实现：发送 custom event 让 YJToast 处理
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('yj:toast-dismiss-key', { detail: { key } }))
  }
}

/**
 * 清空所有 toast
 */
export function clearAll(): void {
  useToastBase().clear()
}

/**
 * 增强 useToast composable
 */
export function useToast() {
  const base = useToastBase()

  /**
   * 显示一个 toast（增强 API）
   */
  function show(options: ToastOptions): number {
    const id = _id++
    const normalized: QueuedToast = {
      id,
      type: options.type ?? 'info',
      message: options.message,
      duration: options.duration ?? 3000,
      position: options.position ?? 'top',
      detail: options.detail,
      action: options.action,
      key: options.key,
      onClose: options.onClose,
    }

    // 如果 YJToast 桥接已注册，走增强队列
    if (_enqueue) {
      _enqueue(normalized)
      return id
    }

    // 否则 fallback 到基础 API
    if (options.key) {
      // 带 key：调底层 show，简单实现
    }
    return base.show(options.message, options.type, options.duration)
  }

  return {
    /** 增强 show API（推荐） */
    show,
    /** 关闭指定 toast */
    dismiss: dismissId,
    /** 按 key 关闭 */
    dismissByKey,
    /** 清空所有 */
    clear: clearAll,
    /** 兼容旧 API */
    success: base.success,
    error: base.error,
    warning: base.warning,
    info: base.info,
    loading: base.loading,
    /** 兼容旧 remove */
    remove: base.remove,
  }
}

/** 顶层导出常量 */
export { MAX_VISIBLE }
