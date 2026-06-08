<!--
  YJNotificationCenter.vue
  2026-06-08 TASK-2.7 引入：统一通知系统 - 通知中心
  用法：
    <YJNotificationCenter v-model:show="showCenter" />
  配套 useNotificationStore（持久化 + 队列管理）
-->
<script setup lang="ts">
import { computed, watch } from 'vue'
import { showToast } from 'vant'
import { useNotificationStore } from './notificationStore'

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ (e: 'update:show', v: boolean): void }>()

const {
  notifications,
  unreadCount,
  total,
  markAsRead,
  markAllAsRead,
  clear,
  remove,
  formatRelativeTime,
} = useNotificationStore()

const grouped = computed(() => {
  const now = Date.now()
  const today: typeof notifications.value = []
  const earlier: typeof notifications.value = []
  for (const n of notifications.value) {
    if (now - n.timestamp < 24 * 3600 * 1000) {
      today.push(n)
    } else {
      earlier.push(n)
    }
  }
  return { today, earlier }
})

function close() {
  emit('update:show', false)
}

function handleItemClick(id: number) {
  markAsRead(id)
}

function handleClear() {
  if (total.value === 0) return
  if (typeof window !== 'undefined' && window.confirm) {
    if (!window.confirm(`清空全部 ${total.value} 条通知？`)) return
  }
  clear()
  showToast('已清空通知')
}

function handleMarkAllRead() {
  markAllAsRead()
  showToast('全部已读')
}

function typeIcon(type: string): string {
  if (type === 'success') return '✓'
  if (type === 'error') return '✕'
  if (type === 'warning') return '⚠'
  if (type === 'loading') return '⟳'
  return 'ℹ'
}

function typeColor(type: string): string {
  if (type === 'success') return 'var(--success)'
  if (type === 'error') return 'var(--danger)'
  if (type === 'warning') return 'var(--warning)'
  if (type === 'loading') return 'var(--accent)'
  return 'var(--accent)'
}
</script>

<template>
  <van-popup
    :show="props.show"
    @update:show="emit('update:show', $event)"
    position="right"
    :style="{ width: '92vw', maxWidth: '420px', height: '100vh' }"
  >
    <div class="nc-shell">
      <header class="nc-header">
        <h3 class="nc-title">通知中心</h3>
        <div class="nc-header-actions">
          <van-button
            v-if="unreadCount > 0"
            size="mini"
            plain
            @click="handleMarkAllRead"
          >
            全部已读
          </van-button>
          <van-button
            v-if="total > 0"
            size="mini"
            plain
            type="danger"
            @click="handleClear"
          >
            清空
          </van-button>
          <van-button size="mini" plain icon="cross" @click="close" />
        </div>
      </header>

      <div class="nc-summary">
        <span>共 <strong>{{ total }}</strong> 条</span>
        <span v-if="unreadCount > 0" class="nc-unread">未读 <strong>{{ unreadCount }}</strong></span>
        <span v-else class="nc-read">全部已读</span>
      </div>

      <div class="nc-list">
        <div v-if="total === 0" class="nc-empty">
          <div class="nc-empty-icon">🔔</div>
          <div class="nc-empty-text">暂无通知</div>
        </div>
        <template v-else>
          <template v-if="grouped.today.length > 0">
            <div class="nc-section-title">今天</div>
            <div
              v-for="n in grouped.today"
              :key="n.id"
              class="nc-item"
              :class="{ 'nc-item--unread': !n.read }"
              @click="handleItemClick(n.id)"
            >
              <span class="nc-item-icon" :style="{ color: typeColor(n.type), borderColor: typeColor(n.type) }">
                {{ typeIcon(n.type) }}
              </span>
              <div class="nc-item-body">
                <div class="nc-item-message">{{ n.message }}</div>
                <div v-if="n.detail" class="nc-item-detail">{{ n.detail }}</div>
                <div class="nc-item-time">{{ formatRelativeTime(n.timestamp) }}</div>
              </div>
              <van-icon
                name="cross"
                class="nc-item-remove"
                @click.stop="remove(n.id)"
              />
            </div>
          </template>
          <template v-if="grouped.earlier.length > 0">
            <div class="nc-section-title">更早</div>
            <div
              v-for="n in grouped.earlier"
              :key="n.id"
              class="nc-item"
              :class="{ 'nc-item--unread': !n.read }"
              @click="handleItemClick(n.id)"
            >
              <span class="nc-item-icon" :style="{ color: typeColor(n.type), borderColor: typeColor(n.type) }">
                {{ typeIcon(n.type) }}
              </span>
              <div class="nc-item-body">
                <div class="nc-item-message">{{ n.message }}</div>
                <div v-if="n.detail" class="nc-item-detail">{{ n.detail }}</div>
                <div class="nc-item-time">{{ formatRelativeTime(n.timestamp) }}</div>
              </div>
              <van-icon
                name="cross"
                class="nc-item-remove"
                @click.stop="remove(n.id)"
              />
            </div>
          </template>
        </template>
      </div>
    </div>
  </van-popup>
</template>

<style scoped>
.nc-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: inherit;
}

.nc-header {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-4);
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  padding-top: max(var(--space-4), var(--safe-top));
}

.nc-title {
  font-size: var(--font-lg);
  font-weight: 600;
  margin: 0;
  color: var(--text-primary);
}

.nc-header-actions {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}

.nc-summary {
  flex-shrink: 0;
  display: flex;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  font-size: var(--font-sm);
  color: var(--text-muted);
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border);
}

.nc-summary strong {
  color: var(--text-primary);
  font-weight: 600;
  margin: 0 2px;
}

.nc-unread { color: var(--accent); }
.nc-unread strong { color: var(--accent); }
.nc-read { color: var(--text-muted); }

.nc-list {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-2) 0;
}

.nc-section-title {
  padding: var(--space-3) var(--space-4) var(--space-2);
  font-size: var(--font-xs);
  color: var(--text-muted);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.nc-item {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  background: var(--bg-card);
  margin: 0 var(--space-2) var(--space-1);
  border-radius: var(--radius-md);
  border-left: 3px solid transparent;
  cursor: pointer;
  transition: background var(--transition-fast);
  position: relative;
}

.nc-item:hover {
  background: var(--bg-card-hover);
}

.nc-item--unread {
  border-left-color: var(--accent);
  background: var(--accent-lighter);
}

.nc-item--unread:hover {
  background: var(--accent-light);
}

.nc-item-icon {
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--font-sm);
  font-weight: 700;
  background: var(--bg-primary);
  border: 2px solid;
}

.nc-item-body {
  flex: 1;
  min-width: 0;
}

.nc-item-message {
  font-size: var(--font-base);
  color: var(--text-primary);
  word-break: break-word;
  line-height: 1.4;
}

.nc-item-detail {
  margin-top: var(--space-1);
  font-size: var(--font-sm);
  color: var(--text-secondary);
  line-height: 1.4;
}

.nc-item-time {
  margin-top: var(--space-2);
  font-size: var(--font-xs);
  color: var(--text-muted);
}

.nc-item-remove {
  flex-shrink: 0;
  color: var(--text-muted);
  font-size: 14px;
  cursor: pointer;
  padding: 2px;
  border-radius: 50%;
  transition: color var(--transition-fast), background var(--transition-fast);
}

.nc-item-remove:hover {
  color: var(--danger);
  background: var(--danger-light);
}

.nc-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-9);
  color: var(--text-muted);
  text-align: center;
}

.nc-empty-icon {
  font-size: 48px;
  margin-bottom: var(--space-3);
  opacity: 0.5;
}

.nc-empty-text {
  font-size: var(--font-base);
}
</style>
