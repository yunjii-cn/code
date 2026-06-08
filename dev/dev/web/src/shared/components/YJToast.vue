<!--
  YJToast.vue
  2026-06-08 TASK-1.9 引入：设计系统原子组件 - 通知
  替代 showToast 的项目内统一封装
  用法：
    import { useToast } from '@shared/components'
    const toast = useToast()
    toast.success('操作成功')
-->
<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { clearToastApi, setToastApi, type ToastApi, type ToastType } from './toast'

interface ToastItem {
  id: number
  type: ToastType
  message: string
  duration: number
}

const toasts = ref<ToastItem[]>([])
let nextId = 1

function show(message: string, type: ToastType = 'info', duration = 2500): number {
  const id = nextId++
  toasts.value.push({ id, type, message, duration })
  if (duration > 0) {
    setTimeout(() => remove(id), duration)
  }
  return id
}

function remove(id: number): void {
  const idx = toasts.value.findIndex((t) => t.id === id)
  if (idx > -1) toasts.value.splice(idx, 1)
}

function clear(): void {
  toasts.value = []
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

onMounted(() => setToastApi(api))
onUnmounted(() => clearToastApi())
</script>

<template>
  <Teleport to="body">
    <div class="yj-toast-container">
      <TransitionGroup name="yj-toast">
        <div
          v-for="t in toasts"
          :key="t.id"
          class="yj-toast"
          :class="`yj-toast--${t.type}`"
        >
          <span class="yj-toast__icon">
            <template v-if="t.type === 'success'">✓</template>
            <template v-else-if="t.type === 'error'">✕</template>
            <template v-else-if="t.type === 'warning'">⚠</template>
            <template v-else-if="t.type === 'loading'">⟳</template>
            <template v-else>ℹ</template>
          </span>
          <span class="yj-toast__message">{{ t.message }}</span>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.yj-toast-container {
  position: fixed;
  top: var(--space-7);
  left: 50%;
  transform: translateX(-50%);
  z-index: var(--z-toast);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  pointer-events: none;
}

.yj-toast {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-5);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  min-width: 200px;
  max-width: 80vw;
  font-size: var(--font-base);
  pointer-events: auto;
}

.yj-toast__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  font-size: var(--font-sm);
  font-weight: 700;
  flex-shrink: 0;
}

.yj-toast__message {
  flex: 1;
  color: var(--text-primary);
  word-break: break-word;
}

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
  to {
    transform: rotate(360deg);
  }
}

.yj-toast-enter-active,
.yj-toast-leave-active {
  transition: all var(--transition-base);
}
.yj-toast-enter-from,
.yj-toast-leave-to {
  opacity: 0;
  transform: translateY(-20px);
}
</style>
