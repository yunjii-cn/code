<!--
  YJModal.vue
  2026-06-08 TASK-1.9 引入：设计系统原子组件 - 模态框
  基于 Teleport，可拖拽
-->
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'

interface Props {
  modelValue: boolean
  title?: string
  width?: string | number
  closeOnClickOverlay?: boolean
  showClose?: boolean
  draggable?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  width: 480,
  closeOnClickOverlay: true,
  showClose: true,
  draggable: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  close: []
  open: []
}>()

const dragging = ref(false)
const position = ref({ x: 0, y: 0 })
const startPos = ref({ x: 0, y: 0, mouseX: 0, mouseY: 0 })
const dialogRef = ref<HTMLElement | null>(null)

const widthPx = computed(() => (typeof props.width === 'number' ? `${props.width}px` : props.width))

function close() {
  emit('update:modelValue', false)
  emit('close')
}

function onOverlayClick() {
  if (props.closeOnClickOverlay) close()
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.modelValue) close()
}

function onHeaderMouseDown(e: MouseEvent) {
  if (!props.draggable) return
  dragging.value = true
  startPos.value = {
    x: position.value.x,
    y: position.value.y,
    mouseX: e.clientX,
    mouseY: e.clientY,
  }
}

function onMouseMove(e: MouseEvent) {
  if (!dragging.value) return
  position.value = {
    x: startPos.value.x + (e.clientX - startPos.value.mouseX),
    y: startPos.value.y + (e.clientY - startPos.value.mouseY),
  }
}

function onMouseUp() {
  dragging.value = false
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown)
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
})

onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown)
  document.removeEventListener('mousemove', onMouseMove)
  document.removeEventListener('mouseup', onMouseUp)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="yj-modal">
      <div v-if="modelValue" class="yj-modal" @click.self="onOverlayClick">
        <div
          ref="dialogRef"
          class="yj-modal__dialog"
          :style="{ width: widthPx, transform: `translate(${position.x}px, ${position.y}px)` }"
        >
          <div
            v-if="title || $slots.header || showClose"
            class="yj-modal__header"
            :class="{ 'is-draggable': draggable }"
            @mousedown="onHeaderMouseDown"
          >
            <slot name="header">
              <span class="yj-modal__title">{{ title }}</span>
            </slot>
            <button v-if="showClose" class="yj-modal__close" @click="close">✕</button>
          </div>
          <div class="yj-modal__body">
            <slot />
          </div>
          <div v-if="$slots.footer" class="yj-modal__footer">
            <slot name="footer" />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.yj-modal {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal-backdrop);
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-overlay);
  backdrop-filter: blur(4px);
}

.yj-modal__dialog {
  position: relative;
  max-width: 90vw;
  max-height: 85vh;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-xl);
  display: flex;
  flex-direction: column;
  z-index: var(--z-modal);
}

.yj-modal__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.yj-modal__header.is-draggable {
  cursor: move;
  user-select: none;
}

.yj-modal__title {
  font-size: var(--font-md);
  font-weight: 600;
  color: var(--text-primary);
}

.yj-modal__close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  font-size: 16px;
  transition: all var(--transition-fast);
}

.yj-modal__close:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.yj-modal__body {
  padding: var(--space-5);
  overflow-y: auto;
  flex: 1;
  min-height: 0;
  color: var(--text-primary);
}

.yj-modal__footer {
  padding: var(--space-4) var(--space-5);
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  flex-shrink: 0;
}

/* 过渡 */
.yj-modal-enter-active,
.yj-modal-leave-active {
  transition: opacity var(--transition-base);
}

.yj-modal-enter-active .yj-modal__dialog,
.yj-modal-leave-active .yj-modal__dialog {
  transition: transform var(--transition-base);
}

.yj-modal-enter-from,
.yj-modal-leave-to {
  opacity: 0;
}

.yj-modal-enter-from .yj-modal__dialog,
.yj-modal-leave-to .yj-modal__dialog {
  transform: scale(0.9);
}
</style>
