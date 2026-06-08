<!--
  YJPopover.vue
  2026-06-08 TASK-1.9 引入：设计系统原子组件 - 气泡弹层
-->
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'

interface Props {
  modelValue: boolean
  placement?:
    | 'top'
    | 'bottom'
    | 'left'
    | 'right'
    | 'top-start'
    | 'top-end'
    | 'bottom-start'
    | 'bottom-end'
  trigger?: 'click' | 'hover' | 'manual'
  width?: string | number
  showArrow?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  placement: 'bottom',
  trigger: 'click',
  width: 240,
  showArrow: true,
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const visible = ref(props.modelValue)
const triggerRef = ref<HTMLElement | null>(null)
const popoverRef = ref<HTMLElement | null>(null)

const widthPx = computed(() => (typeof props.width === 'number' ? `${props.width}px` : props.width))

function toggle() {
  visible.value = !visible.value
  emit('update:modelValue', visible.value)
}

function show() {
  visible.value = true
  emit('update:modelValue', true)
}

function hide() {
  visible.value = false
  emit('update:modelValue', false)
}

function onClickOutside(e: MouseEvent) {
  if (!visible.value) return
  const t = e.target as Node
  if (
    triggerRef.value &&
    !triggerRef.value.contains(t) &&
    popoverRef.value &&
    !popoverRef.value.contains(t)
  ) {
    hide()
  }
}

onMounted(() => {
  document.addEventListener('click', onClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', onClickOutside)
})

const triggerProps = computed(() => {
  if (props.trigger === 'hover') {
    return { onMouseenter: show, onMouseleave: hide }
  }
  if (props.trigger === 'click') {
    return { onClick: toggle }
  }
  return {}
})
</script>

<template>
  <div class="yj-popover">
    <div ref="triggerRef" class="yj-popover__trigger" v-bind="triggerProps">
      <slot name="trigger" />
    </div>
    <Teleport to="body">
      <Transition name="yj-popover">
        <div
          v-if="visible"
          ref="popoverRef"
          class="yj-popover__content"
          :class="`yj-popover__content--${placement}`"
          :style="{ width: widthPx }"
        >
          <slot />
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.yj-popover {
  display: inline-block;
  position: relative;
}

.yj-popover__trigger {
  display: inline-block;
}

.yj-popover__content {
  position: absolute;
  z-index: var(--z-popover);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  padding: var(--space-3);
  color: var(--text-primary);
  font-size: var(--font-base);
}

/* 简化定位（生产应使用 floating-ui） */
.yj-popover__content--bottom,
.yj-popover__content--bottom-start,
.yj-popover__content--bottom-end {
  top: calc(100% + 8px);
}

.yj-popover__content--top,
.yj-popover__content--top-start,
.yj-popover__content--top-end {
  bottom: calc(100% + 8px);
}

.yj-popover__content--left {
  right: calc(100% + 8px);
}

.yj-popover__content--right {
  left: calc(100% + 8px);
}

/* 过渡 */
.yj-popover-enter-active,
.yj-popover-leave-active {
  transition:
    opacity var(--transition-fast),
    transform var(--transition-fast);
}

.yj-popover-enter-from,
.yj-popover-leave-to {
  opacity: 0;
  transform: scale(0.95);
}
</style>
