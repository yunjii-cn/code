<!--
  YJToast.vue
  2026-06-08 TASK-1.9 引入：设计系统原子组件 - 通知
  2026-06-10 TASK-2.7 升级：支持 action 按钮 / detail 多行 / 位置选项 / 队列管理（同时最多 3 个）

  用法（推荐 — 增强 API）：
    import { useToast } from '@shared/composables/useToast'
    const { show, success, error } = useToast()

    show({ type: 'success', message: '保存成功', duration: 3000 })
    show({
      type: 'error',
      message: '上传失败',
      detail: '网络连接已断开',
      duration: 0,
      action: { label: '重试', onClick: () => retry() },
      position: 'top-right',
    })

  用法（兼容 — 基础 API）：
    import { useToast } from '@shared/components'
    const toast = useToast()
    toast.success('操作成功')
-->
<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch, computed } from 'vue'
import { clearToastApi, setToastApi, type ToastApi, type ToastType } from './toast'
import {
  __bindToastBridge,
  __unbindToastBridge,
  MAX_VISIBLE,
  type ToastOptions,
  type ToastPosition,
} from '../composables/useToast'

interface ToastItem {
  id: number
  type: ToastType
  message: string
  detail?: string
  duration: number
  position: ToastPosition
  action?: ToastOptions['action']
  key?: string
  onClose?: () => void
}

const toasts = ref<ToastItem[]>([])
let nextId = 1

/**
 * 按 position 分组（模板用）
 */
const groupedToasts = computed<Record<ToastPosition, ToastItem[]>>(() => {
  const groups: Record<string, ToastItem[]> = {
    top: [],
    'top-right': [],
    'top-left': [],
    center: [],
    bottom: [],
    'bottom-right': [],
    'bottom-left': [],
  }
  for (const t of toasts.value) {
    if (!groups[t.position]) groups[t.position] = []
    groups[t.position]!.push(t)
  }
  return groups as Record<ToastPosition, ToastItem[]>
})

/**
 * 加入队列（FIFO，超出 MAX_VISIBLE 自动移除最早的）
 */
function enqueue(t: ToastItem): void {
  // 同 key 替换
  if (t.key) {
    const existing = toasts.value.find((x) => x.key === t.key)
    if (existing) {
      const idx = toasts.value.indexOf(existing)
      toasts.value.splice(idx, 1, t)
      if (t.duration > 0) {
        setTimeout(() => remove(t.id), t.duration)
      }
      return
    }
  }

  toasts.value.push(t)

  // 队列上限
  while (toasts.value.length > MAX_VISIBLE) {
    const dropped = toasts.value.shift()
    if (dropped?.onClose) dropped.onClose()
  }

  if (t.duration > 0) {
    setTimeout(() => remove(t.id), t.duration)
  }
}

function remove(id: number): void {
  const idx = toasts.value.findIndex((t) => t.id === id)
  if (idx > -1) {
    const [removed] = toasts.value.splice(idx, 1)
    if (removed?.onClose) removed.onClose()
  }
}

function clear(): void {
  const items = toasts.value.slice()
  toasts.value = []
  for (const t of items) {
    if (t.onClose) t.onClose()
  }
}

function onDismissKey(evt: Event) {
  const e = evt as CustomEvent<{ key: string }>
  const key = e.detail?.key
  if (!key) return
  const idx = toasts.value.findIndex((t) => t.key === key)
  if (idx > -1) {
    const [removed] = toasts.value.splice(idx, 1)
    if (removed?.onClose) removed.onClose()
  }
}

function show(message: string, type: ToastType = 'info', duration = 2500): number {
  const id = nextId++
  enqueue({ id, type, message, duration, position: 'top' })
  return id
}

const api: ToastApi = {
  success: (msg) => show(msg, 'success'),
  error: (msg) => show(msg, 'error'),
  warning: (msg) => show(msg, 'warning'),
  info: (msg) => show(msg, 'info'),
  loading: (msg) => show(msg, 'loading', 0),
  show,
  remove,
  clear,
}

onMounted(() => {
  setToastApi(api)
  __bindToastBridge({ enqueue, remove })
  window.addEventListener('yj:toast-dismiss-key', onDismissKey)
})
onUnmounted(() => {
  clearToastApi()
  __unbindToastBridge()
  window.removeEventListener('yj:toast-dismiss-key', onDismissKey)
})

function onActionClick(item: ToastItem): void {
  if (!item.action) return
  Promise.resolve(item.action.onClick()).catch(() => {
    /* 错误由 action 自己处理 */
  })
  if (item.action.closeOnClick !== false) {
    remove(item.id)
  }
}

function iconFor(type: ToastType): string {
  switch (type) {
    case 'success': return '✓'
    case 'error': return '✕'
    case 'warning': return '⚠'
    case 'loading': return '⟳'
    default: return 'ℹ'
  }
}

const positionClass = (pos: ToastPosition) => `yj-toast-pos--${pos}`

// 监听 toasts 变化（开发调试）
watch(toasts, (v) => {
  if (v.length > MAX_VISIBLE) {
    // eslint-disable-next-line no-console
    console.debug(`[YJToast] 队列上限 ${MAX_VISIBLE}，已自动移除最早`)
  }
})
</script>

<template>
  <Teleport to="body">
    <div class="yj-toast-root">
      <TransitionGroup
        v-for="(group, pos) in groupedToasts"
        v-show="group.length > 0"
        :key="pos"
        :name="`yj-toast-${pos}`"
        tag="div"
        class="yj-toast-container"
        :class="`yj-toast-pos--${pos}`"
      >
        <div
          v-for="t in group"
          :key="t.id"
          class="yj-toast"
          :class="`yj-toast--${t.type}`"
          role="alert"
        >
          <span class="yj-toast__icon">{{ iconFor(t.type) }}</span>
          <div class="yj-toast__body">
            <div class="yj-toast__message">{{ t.message }}</div>
            <div v-if="t.detail" class="yj-toast__detail">{{ t.detail }}</div>
          </div>
          <button
            v-if="t.action"
            class="yj-toast__action"
            :class="`yj-toast__action--${t.action.variant ?? 'primary'}`"
            @click="onActionClick(t)"
          >
            {{ t.action.label }}
          </button>
          <button
            class="yj-toast__close"
            aria-label="关闭"
            @click="remove(t.id)"
          >
            ×
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
/* ============ 根容器（所有位置 toast 的容器） ============ */
.yj-toast-root {
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: var(--z-toast);
}

/* ============ 各位置容器 ============ */
.yj-toast-container {
  position: absolute;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  pointer-events: none;
}

/* top: 顶部居中 */
.yj-toast-pos--top {
  top: var(--space-7);
  left: 50%;
  transform: translateX(-50%);
  align-items: center;
}

/* top-right: 右上 */
.yj-toast-pos--top-right {
  top: var(--space-7);
  right: var(--space-5);
  align-items: flex-end;
}

/* top-left: 左上 */
.yj-toast-pos--top-left {
  top: var(--space-7);
  left: var(--space-5);
  align-items: flex-start;
}

/* center: 屏幕中央 */
.yj-toast-pos--center {
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  align-items: center;
}

/* bottom: 底部居中 */
.yj-toast-pos--bottom {
  bottom: var(--space-7);
  left: 50%;
  transform: translateX(-50%);
  align-items: center;
  flex-direction: column-reverse;
}

/* bottom-right: 右下 */
.yj-toast-pos--bottom-right {
  bottom: var(--space-7);
  right: var(--space-5);
  align-items: flex-end;
  flex-direction: column-reverse;
}

/* bottom-left: 左下 */
.yj-toast-pos--bottom-left {
  bottom: var(--space-7);
  left: var(--space-5);
  align-items: flex-start;
  flex-direction: column-reverse;
}

/* ============ 单个 toast ============ */
.yj-toast {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  min-width: 240px;
  max-width: 360px;
  font-size: var(--font-base);
  pointer-events: auto;
  color: var(--text-primary);
}

.yj-toast__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  font-size: var(--font-sm);
  font-weight: 700;
  flex-shrink: 0;
  margin-top: 1px;
}

.yj-toast__body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.yj-toast__message {
  color: var(--text-primary);
  word-break: break-word;
  line-height: 1.4;
}

.yj-toast__detail {
  color: var(--text-muted);
  font-size: var(--font-sm);
  line-height: 1.4;
  word-break: break-word;
}

.yj-toast__action {
  flex-shrink: 0;
  background: var(--accent-light);
  color: var(--accent);
  border: 1px solid var(--accent-border);
  border-radius: var(--radius-sm);
  padding: 4px 10px;
  font-size: var(--font-sm);
  font-weight: 500;
  cursor: pointer;
  transition: background var(--transition-fast);
  font-family: inherit;
  margin-top: 1px;
}

.yj-toast__action:hover {
  background: var(--accent-border);
}

.yj-toast__action--danger {
  background: var(--danger-light);
  color: var(--danger);
  border-color: var(--danger-border);
}

.yj-toast__action--danger:hover {
  background: var(--danger-border);
}

.yj-toast__action--default {
  background: var(--bg-card-hover);
  color: var(--text-secondary);
  border-color: var(--border-light);
}

.yj-toast__close {
  flex-shrink: 0;
  background: transparent;
  color: var(--text-muted);
  border: none;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  font-size: 16px;
  line-height: 1;
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
  padding: 0;
  font-family: inherit;
  margin-top: 1px;
}

.yj-toast__close:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

/* 类型颜色 */
.yj-toast--success .yj-toast__icon {
  background: var(--success-light);
  color: var(--success);
}
.yj-toast--success {
  border-color: var(--success-border);
}

.yj-toast--error .yj-toast__icon {
  background: var(--danger-light);
  color: var(--danger);
}
.yj-toast--error {
  border-color: var(--danger-border);
}

.yj-toast--warning .yj-toast__icon {
  background: var(--warning-light);
  color: var(--warning);
}
.yj-toast--warning {
  border-color: var(--warning-border);
}

.yj-toast--info .yj-toast__icon {
  background: var(--accent-light);
  color: var(--accent);
}
.yj-toast--info {
  border-color: var(--accent-border);
}

.yj-toast--loading .yj-toast__icon {
  background: var(--accent-light);
  color: var(--accent);
  animation: yj-spin 1s linear infinite;
}

@keyframes yj-spin {
  to { transform: rotate(360deg); }
}

/* ============ 过渡动画 ============ */
.yj-toast-top-enter-active,
.yj-toast-top-leave-active,
.yj-toast-top-right-enter-active,
.yj-toast-top-right-leave-active,
.yj-toast-top-left-enter-active,
.yj-toast-top-left-leave-active,
.yj-toast-center-enter-active,
.yj-toast-center-leave-active,
.yj-toast-bottom-enter-active,
.yj-toast-bottom-leave-active,
.yj-toast-bottom-right-enter-active,
.yj-toast-bottom-right-leave-active,
.yj-toast-bottom-left-enter-active,
.yj-toast-bottom-left-leave-active {
  transition: all var(--transition-base);
}

.yj-toast-top-enter-from,
.yj-toast-top-leave-to,
.yj-toast-center-enter-from,
.yj-toast-center-leave-to {
  opacity: 0;
  transform: translateY(-20px);
}

.yj-toast-top-right-enter-from,
.yj-toast-top-right-leave-to,
.yj-toast-bottom-right-enter-from,
.yj-toast-bottom-right-leave-to {
  opacity: 0;
  transform: translateX(20px);
}

.yj-toast-top-left-enter-from,
.yj-toast-top-left-leave-to,
.yj-toast-bottom-left-enter-from,
.yj-toast-bottom-left-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}

.yj-toast-bottom-enter-from,
.yj-toast-bottom-leave-to {
  opacity: 0;
  transform: translateY(20px);
}

/* 移动端适配 */
@media (max-width: 768px) {
  .yj-toast {
    min-width: 200px;
    max-width: calc(100vw - 32px);
  }
  .yj-toast-pos--top,
  .yj-toast-pos--top-right,
  .yj-toast-pos--top-left {
    top: calc(var(--safe-top) + var(--space-3));
  }
  .yj-toast-pos--bottom,
  .yj-toast-pos--bottom-right,
  .yj-toast-pos--bottom-left {
    bottom: calc(var(--safe-bottom) + var(--tab-bar-total) + var(--space-3));
  }
}
</style>
