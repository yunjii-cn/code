<script setup lang="ts">
/**
 * RouteHistoryList — 路由历史列表
 * 2026-06-09 TASK-4.6
 *
 * 展示最近路由决策记录（最新在前）
 */
import { computed } from 'vue'
import type { RoutingHistoryItem } from '@/api'

const props = defineProps<{
  history: RoutingHistoryItem[]
}>()

const emit = defineEmits<{
  clear: []
  refresh: []
}>()

const tierColors: Record<string, string> = {
  simple: '#10b981',
  medium: '#f59e0b',
  complex: '#ef4444',
}

function formatTime(ts?: number) {
  if (!ts) return '-'
  return new Date(ts * 1000).toLocaleString('zh-CN', { hour12: false })
}

function getTierColor(tier: string) {
  return tierColors[tier] || '#666'
}

const stats = computed(() => {
  const tierCount: Record<string, number> = {}
  const providerCount: Record<string, number> = {}
  for (const h of props.history) {
    tierCount[h.tier] = (tierCount[h.tier] || 0) + 1
    const p = h.primary?.provider || 'unknown'
    providerCount[p] = (providerCount[p] || 0) + 1
  }
  return { tierCount, providerCount, total: props.history.length }
})
</script>

<template>
  <div class="history-list">
    <div class="list-header">
      <div class="list-title">
        <span>路由历史</span>
        <span class="title-count">{{ stats.total }}</span>
      </div>
      <div class="list-actions">
        <button class="btn-mini" @click="emit('refresh')">🔄</button>
        <button class="btn-mini btn-danger" @click="emit('clear')">清空</button>
      </div>
    </div>

    <!-- 统计 -->
    <div v-if="history.length" class="stats-row">
      <div v-for="(count, tier) in stats.tierCount" :key="tier" class="stat-pill" :style="{ borderColor: getTierColor(tier) }">
        <span class="stat-name" :style="{ color: getTierColor(tier) }">{{ tier }}</span>
        <span class="stat-count">{{ count }}</span>
      </div>
    </div>

    <div v-if="!history.length" class="empty">
      <div class="empty-icon">📜</div>
      <div class="empty-text">还没有路由记录</div>
    </div>

    <div v-else class="items">
      <div
        v-for="(item, idx) in history"
        :key="idx"
        class="history-item"
      >
        <div class="item-time">{{ formatTime(item.timestamp) }}</div>
        <div class="item-body">
          <div class="item-head">
            <div class="item-tier" :style="{ background: getTierColor(item.tier) }">
              {{ item.tier }}
            </div>
            <div class="item-route">
              <span class="route-from">{{ item.primary.provider }}:{{ item.primary.model }}</span>
              <span v-if="item.fallback" class="route-arrow">→</span>
              <span v-if="item.fallback" class="route-fallback">
                {{ item.fallback.provider }}:{{ item.fallback.model }}
              </span>
            </div>
            <div class="item-complexity">
              {{ (item.complexity.total * 100).toFixed(0) }}%
            </div>
          </div>
          <div v-if="item.prompt_preview" class="item-prompt">{{ item.prompt_preview }}</div>
          <div v-if="item.complexity.matched_keywords.length" class="item-tags">
            <span v-for="kw in item.complexity.matched_keywords.slice(0, 4)" :key="kw" class="kw-chip">#{{ kw }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.history-list {
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  max-height: 600px;
  overflow: hidden;
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
  flex-shrink: 0;
}

.list-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  color: var(--yj-text-primary, #f5f5f5);
}

.title-count {
  font-size: 11px;
  padding: 1px 8px;
  background: var(--yj-accent, #7c3aed);
  color: #fff;
  border-radius: 8px;
  font-weight: 500;
}

.list-actions {
  display: flex;
  gap: 4px;
}

.btn-mini {
  font-size: 11px;
  padding: 3px 8px;
  background: transparent;
  border: 1px solid var(--yj-border, #2a2a2a);
  color: var(--yj-text-secondary, #aaa);
  border-radius: 4px;
  cursor: pointer;
}

.btn-mini:hover {
  border-color: var(--yj-accent, #7c3aed);
}

.btn-mini.btn-danger {
  color: #fca5a5;
}

.btn-mini.btn-danger:hover {
  border-color: #ef4444;
}

.stats-row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
}

.stat-pill {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 8px;
  border: 1px solid;
  background: var(--yj-bg-base, #0d0d0d);
}

.stat-name {
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.stat-count {
  color: var(--yj-text-primary, #ddd);
  font-family: 'Cascadia Code', 'Consolas', monospace;
}

.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  color: var(--yj-text-secondary, #888);
}

.empty-icon {
  font-size: 32px;
  margin-bottom: 8px;
  opacity: 0.4;
}

.items {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px;
}

.history-item {
  display: flex;
  gap: 8px;
  padding: 8px;
  border-radius: 6px;
  margin: 2px 0;
  background: var(--yj-bg-base, #0d0d0d);
}

.item-time {
  font-size: 10px;
  color: var(--yj-text-secondary, #666);
  font-family: 'Cascadia Code', 'Consolas', monospace;
  min-width: 100px;
  padding-top: 2px;
  white-space: nowrap;
}

.item-body {
  flex: 1;
  min-width: 0;
}

.item-head {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.item-tier {
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 1px 6px;
  border-radius: 4px;
  color: #fff;
}

.item-route {
  font-size: 11px;
  color: var(--yj-text-primary, #ddd);
  font-family: 'Cascadia Code', 'Consolas', monospace;
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}

.route-arrow {
  color: var(--yj-text-secondary, #666);
}

.route-fallback {
  color: var(--yj-text-secondary, #888);
  font-style: italic;
}

.item-complexity {
  font-size: 11px;
  color: var(--yj-accent, #a855f7);
  font-weight: 600;
  font-family: 'Cascadia Code', 'Consolas', monospace;
}

.item-prompt {
  font-size: 11px;
  color: var(--yj-text-secondary, #aaa);
  margin-top: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
}

.item-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  margin-top: 4px;
}

.kw-chip {
  font-size: 9px;
  padding: 1px 5px;
  border-radius: 6px;
  background: rgba(139, 92, 246, 0.12);
  color: #a78bfa;
}
</style>
