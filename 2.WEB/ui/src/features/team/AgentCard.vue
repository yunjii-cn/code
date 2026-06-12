<!--
  AgentCard.vue
  2026-06-09 TASK-3.10 引入：Agent 角色卡片
  显示一个 Agent 角色的状态（协调中 / 工作中 / 等待中 / 空闲）
-->
<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  role: 'coordinator' | 'architect' | 'developer' | 'tester' | 'documenter'
  label: string
  status: 'idle' | 'working' | 'waiting' | 'reviewing' | 'arbitrating'
  currentTask?: string | null
  tasksCompleted?: number
  readOnly?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  currentTask: null,
  tasksCompleted: 0,
  readOnly: false,
})

const ROLE_EMOJI: Record<string, string> = {
  coordinator: '🎯',
  architect: '🏗️',
  developer: '💻',
  tester: '🧪',
  documenter: '📝',
}

const STATUS_META: Record<string, { label: string; color: string; bg: string }> = {
  idle:        { label: '空闲',   color: '#94a3b8', bg: '#1e293b' },
  working:     { label: '工作中', color: '#10b981', bg: 'rgba(16, 185, 129, 0.1)' },
  waiting:     { label: '等待中', color: '#94a3b8', bg: '#1e293b' },
  reviewing:   { label: '审查中', color: '#f59e0b', bg: 'rgba(245, 158, 11, 0.1)' },
  arbitrating: { label: '仲裁中', color: '#8b5cf6', bg: 'rgba(139, 92, 246, 0.1)' },
}

const emoji = computed(() => ROLE_EMOJI[props.role] || '🤖')
const meta = computed(() => STATUS_META[props.status] || STATUS_META.idle)
</script>

<template>
  <div class="agent-card" :class="['status-' + status]">
    <div class="card-header">
      <div class="avatar">
        <span class="emoji">{{ emoji }}</span>
        <span v-if="readOnly" class="readonly-badge" title="只读角色">只读</span>
      </div>
      <div class="info">
        <div class="label">{{ label }}</div>
        <div class="role-id">{{ role }}</div>
      </div>
      <div class="status-dot" :style="{ background: meta.color }" :title="meta.label" />
    </div>

    <div class="card-body">
      <div class="status-line">
        <span class="dot" :style="{ background: meta.color }" />
        <span class="status-text" :style="{ color: meta.color }">{{ meta.label }}</span>
      </div>
      <div v-if="currentTask" class="task-line" :title="currentTask">
        📋 {{ currentTask.length > 30 ? currentTask.slice(0, 30) + '…' : currentTask }}
      </div>
      <div v-else class="task-line muted">暂无任务</div>
    </div>

    <div class="card-footer">
      <span class="count">已完成 {{ tasksCompleted }}</span>
    </div>
  </div>
</template>

<style scoped>
.agent-card {
  background: var(--card-bg, #0f172a);
  border: 1px solid var(--border-color, #1e293b);
  border-radius: 10px;
  padding: 12px;
  transition: all 0.2s ease;
  cursor: default;
}

.agent-card:hover {
  border-color: var(--accent-color, #3b82f6);
  transform: translateY(-1px);
}

.agent-card.status-working {
  border-color: rgba(16, 185, 129, 0.3);
}

.agent-card.status-reviewing {
  border-color: rgba(245, 158, 11, 0.3);
}

.agent-card.status-arbitrating {
  border-color: rgba(139, 92, 246, 0.3);
}

.card-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.avatar {
  position: relative;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: linear-gradient(135deg, #1e293b, #334155);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
}

.readonly-badge {
  position: absolute;
  top: -4px;
  right: -4px;
  font-size: 9px;
  background: #475569;
  color: #f1f5f9;
  padding: 1px 4px;
  border-radius: 4px;
}

.info {
  flex: 1;
  min-width: 0;
}

.label {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary, #f1f5f9);
  line-height: 1.2;
}

.role-id {
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
  font-family: monospace;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  box-shadow: 0 0 8px currentColor;
}

.card-body {
  margin-bottom: 8px;
}

.status-line,
.task-line {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  margin-bottom: 4px;
}

.dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.status-text {
  font-weight: 500;
}

.task-line {
  color: var(--text-secondary, #cbd5e1);
}

.task-line.muted {
  color: var(--text-tertiary, #64748b);
  font-style: italic;
}

.card-footer {
  font-size: 11px;
  color: var(--text-tertiary, #64748b);
  border-top: 1px solid var(--border-color, #1e293b);
  padding-top: 6px;
}

.count {
  font-family: monospace;
}
</style>
