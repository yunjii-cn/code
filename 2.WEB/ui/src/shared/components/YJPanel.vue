<!--
  YJPanel.vue
  2026-06-08 TASK-1.9 引入：设计系统原子组件 - 可调整大小的面板
  使用原生 resize 工具（生产建议 splitpanes）
-->
<script setup lang="ts">
import { ref, computed } from 'vue'

interface Props {
  title?: string
  resizable?: boolean
  collapsible?: boolean
  defaultCollapsed?: boolean
  minWidth?: number
  minHeight?: number
  width?: string | number
  height?: string | number
  bordered?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  resizable: false,
  collapsible: false,
  defaultCollapsed: false,
  minWidth: 200,
  minHeight: 100,
  bordered: true,
})

const collapsed = ref(props.defaultCollapsed)
const panelRef = ref<HTMLElement | null>(null)
const isResizing = ref(false)
const startSize = ref({ w: 0, h: 0, x: 0, y: 0 })

function toggleCollapse() {
  if (props.collapsible) collapsed.value = !collapsed.value
}

function onResizeStart(e: MouseEvent, dir: 'h' | 'v' | 'both') {
  if (!props.resizable) return
  e.preventDefault()
  isResizing.value = true
  const rect = panelRef.value!.getBoundingClientRect()
  startSize.value = { w: rect.width, h: rect.height, x: e.clientX, y: e.clientY }

  function onMove(e: MouseEvent) {
    if (!isResizing.value) return
    const panel = panelRef.value!
    const dx = e.clientX - startSize.value.x
    const dy = e.clientY - startSize.value.y
    if (dir === 'h' || dir === 'both') {
      const w = Math.max(props.minWidth, startSize.value.w + dx)
      panel.style.width = `${w}px`
    }
    if (dir === 'v' || dir === 'both') {
      const h = Math.max(props.minHeight, startSize.value.h + dy)
      panel.style.height = `${h}px`
    }
  }
  function onUp() {
    isResizing.value = false
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
  }
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}

const styleObj = computed(() => {
  const s: Record<string, string> = {}
  if (props.width !== undefined) {
    s.width = typeof props.width === 'number' ? `${props.width}px` : props.width
  }
  if (props.height !== undefined) {
    s.height = typeof props.height === 'number' ? `${props.height}px` : props.height
  }
  return s
})
</script>

<template>
  <div
    ref="panelRef"
    class="yj-panel"
    :class="{ 'is-bordered': bordered, 'is-resizing': isResizing }"
    :style="styleObj"
  >
    <div v-if="title || $slots.header || collapsible" class="yj-panel__header">
      <slot name="header">
        <span class="yj-panel__title">{{ title }}</span>
      </slot>
      <button v-if="collapsible" class="yj-panel__collapse-btn" @click="toggleCollapse">
        {{ collapsed ? '▾' : '▴' }}
      </button>
    </div>
    <div v-show="!collapsed" class="yj-panel__body">
      <slot />
    </div>
    <div
      v-if="resizable"
      class="yj-panel__resize yj-panel__resize--corner"
      @mousedown="(e) => onResizeStart(e, 'both')"
    />
  </div>
</template>

<style scoped>
.yj-panel {
  position: relative;
  display: flex;
  flex-direction: column;
  background: var(--bg-card);
  border-radius: var(--radius-md);
  overflow: hidden;
  transition: box-shadow var(--transition-fast);
}

.yj-panel.is-bordered {
  border: 1px solid var(--border);
}

.yj-panel.is-resizing {
  box-shadow: var(--shadow-lg);
}

.yj-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  user-select: none;
}

.yj-panel__title {
  font-size: var(--font-base);
  font-weight: 600;
  color: var(--text-primary);
}

.yj-panel__collapse-btn {
  color: var(--text-muted);
  font-size: var(--font-sm);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}

.yj-panel__collapse-btn:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.yj-panel__body {
  flex: 1;
  padding: var(--space-4);
  overflow: auto;
  min-height: 0;
}

.yj-panel__resize {
  position: absolute;
  background: transparent;
  z-index: var(--z-base);
}

.yj-panel__resize--corner {
  right: 0;
  bottom: 0;
  width: 14px;
  height: 14px;
  cursor: nwse-resize;
}

.yj-panel__resize--corner::after {
  content: '';
  position: absolute;
  right: 3px;
  bottom: 3px;
  width: 6px;
  height: 6px;
  border-right: 2px solid var(--border-light);
  border-bottom: 2px solid var(--border-light);
}

.yj-panel__resize:hover::after {
  border-color: var(--accent);
}
</style>
