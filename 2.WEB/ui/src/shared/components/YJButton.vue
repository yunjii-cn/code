<!--
  YJButton.vue
  2026-06-08 TASK-1.9 引入：设计系统原子组件 - 按钮
  替代 van-button 的项目内统一封装
-->
<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  type?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'text'
  size?: 'small' | 'medium' | 'large'
  loading?: boolean
  disabled?: boolean
  block?: boolean
  round?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  type: 'primary',
  size: 'medium',
  loading: false,
  disabled: false,
  block: false,
  round: false,
})

const emit = defineEmits<{
  click: [event: MouseEvent]
}>()

const classes = computed(() => [
  'yj-btn',
  `yj-btn--${props.type}`,
  `yj-btn--${props.size}`,
  {
    'is-loading': props.loading,
    'is-disabled': props.disabled,
    'is-block': props.block,
    'is-round': props.round,
  },
])

function onClick(e: MouseEvent) {
  if (props.disabled || props.loading) return
  emit('click', e)
}
</script>

<template>
  <button :class="classes" :disabled="disabled || loading" @click="onClick">
    <span v-if="loading" class="yj-btn__spinner" />
    <span v-else-if="$slots.icon" class="yj-btn__icon">
      <slot name="icon" />
    </span>
    <span class="yj-btn__text">
      <slot />
    </span>
  </button>
</template>

<style scoped>
.yj-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  font-size: var(--font-base);
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-base);
  user-select: none;
  white-space: nowrap;
}

.yj-btn:active {
  transform: scale(0.97);
}

.yj-btn:disabled,
.yj-btn.is-disabled,
.yj-btn.is-loading {
  cursor: not-allowed;
  opacity: 0.5;
}

/* 尺寸 */
.yj-btn--small {
  height: 28px;
  padding: 0 12px;
  font-size: var(--font-sm);
}

.yj-btn--medium {
  height: 34px;
  padding: 0 16px;
}

.yj-btn--large {
  height: 42px;
  padding: 0 20px;
  font-size: var(--font-md);
}

/* 类型 */
.yj-btn--primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
.yj-btn--primary:hover:not(:disabled) {
  background: var(--accent-2);
  border-color: var(--accent-2);
}

.yj-btn--secondary {
  background: var(--bg-card);
  border-color: var(--border);
  color: var(--text-primary);
}
.yj-btn--secondary:hover:not(:disabled) {
  background: var(--bg-card-hover);
  border-color: var(--border-strong);
}

.yj-btn--ghost {
  background: transparent;
  border-color: var(--border);
  color: var(--text-secondary);
}
.yj-btn--ghost:hover:not(:disabled) {
  background: var(--bg-card);
  color: var(--text-primary);
}

.yj-btn--danger {
  background: var(--danger);
  border-color: var(--danger);
  color: #fff;
}
.yj-btn--danger:hover:not(:disabled) {
  background: var(--danger-2);
}

.yj-btn--text {
  background: transparent;
  border-color: transparent;
  color: var(--accent);
  padding: 0 8px;
}
.yj-btn--text:hover:not(:disabled) {
  background: var(--accent-lighter);
}

/* 变体 */
.yj-btn.is-block {
  display: flex;
  width: 100%;
}

.yj-btn.is-round {
  border-radius: var(--radius-full);
}

/* 加载动画 */
.yj-btn__spinner {
  width: 14px;
  height: 14px;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  animation: yj-spin 0.8s linear infinite;
}

@keyframes yj-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
