<!--
  PlanPanel.vue
  2026-06-09 TASK-3.10 引入：计划面板（多 Agent 版）
  显示当前会话的需求、角色分布、任务时间线
-->
<script setup lang="ts">
import { computed } from 'vue'
import type { TeamSession, TeamTask } from '@/api'

interface Props {
  session: TeamSession | null
}

const props = defineProps<Props>()

const ROLE_EMOJI: Record<string, string> = {
  coordinator: '🎯',
  architect: '🏗️',
  developer: '💻',
  tester: '🧪',
  documenter: '📝',
}

const ROLE_LABEL: Record<string, string> = {
  coordinator: '主管',
  architect: '架构',
  developer: '开发',
  tester: '测试',
  documenter: '文档',
}

const STATUS_META: Record<string, { label: string; color: string }> = {
  draft:        { label: '草稿',     color: '#94a3b8' },
  planning:     { label: '规划中',   color: '#3b82f6' },
  assigned:     { label: '已派发',   color: '#06b6d4' },
  in_progress:  { label: '执行中',   color: '#10b981' },
  reviewing:    { label: '审查中',   color: '#f59e0b' },
  arbitrating:  { label: '仲裁中',   color: '#8b5cf6' },
  done:         { label: '已完成',   color: '#10b981' },
  failed:       { label: '失败',     color: '#ef4444' },
  closed:       { label: '已归档',   color: '#6b7280' },
}

const statusMeta = computed(() => {
  if (!props.session) return null
  return STATUS_META[props.session.status] || { label: props.session.status, color: '#94a3b8' }
})

const roleDistribution = computed(() => {
  if (!props.session) return []
  const dist: Record<string, number> = {}
  for (const task of props.session.tasks) {
    dist[task.role] = (dist[task.role] || 0) + 1
  }
  return Object.entries(dist).map(([role, count]) => ({
    role,
    emoji: ROLE_EMOJI[role] || '🤖',
    label: ROLE_LABEL[role] || role,
    count,
  }))
})

const progress = computed(() => {
  if (!props.session) return { total: 0, done: 0, percent: 0 }
  const total = props.session.tasks.length
  const done = props.session.tasks.filter(t => t.status === 'done' || t.status === 'approved').length
  return { total, done, percent: total > 0 ? Math.round((done / total) * 100) : 0 }
})

function timeAgo(ts: number): string {
  const diff = Date.now() / 1000 - ts
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  return `${Math.floor(diff / 86400)} 天前`
}

function taskStatusLabel(t: TeamTask): string {
  return {
    pending: '待办',
    in_progress: '进行中',
    awaiting_approval: '待审批',
    approved: '已批准',
    rejected: '已拒绝',
    done: '完成',
    failed: '失败',
    skipped: '跳过',
  }[t.status] || t.status
}
</script>

<template>
  <div class="plan-panel">
    <div v-if="!session" class="empty">请选择一个团队会话</div>
    <template v-else>
      <div class="header">
        <div class="requirement">📋 {{ session.requirement }}</div>
        <div class="status" :style="{ color: statusMeta?.color }">
          {{ statusMeta?.label }}
        </div>
      </div>

      <div class="stats-row">
        <div class="stat">
          <div class="stat-num">{{ progress.done }}/{{ progress.total }}</div>
          <div class="stat-label">任务进度</div>
        </div>
        <div class="stat">
          <div class="stat-num">{{ session.approvals.length }}</div>
          <div class="stat-label">审批节点</div>
        </div>
        <div class="stat">
          <div class="stat-num">{{ session.messages.length }}</div>
          <div class="stat-label">消息</div>
        </div>
        <div class="stat">
          <div class="stat-num">{{ session.shared_context.length }}</div>
          <div class="stat-label">共享事实</div>
        </div>
      </div>

      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: progress.percent + '%' }" />
        <div class="progress-text">{{ progress.percent }}%</div>
      </div>

      <div v-if="roleDistribution.length" class="section">
        <div class="section-title">👥 角色分布</div>
        <div class="role-list">
          <div v-for="r in roleDistribution" :key="r.role" class="role-chip">
            <span>{{ r.emoji }}</span>
            <span>{{ r.label }}</span>
            <span class="chip-count">×{{ r.count }}</span>
          </div>
        </div>
      </div>

      <div v-if="session.tasks.length" class="section">
        <div class="section-title">📝 任务清单</div>
        <div class="task-list">
          <div v-for="t in session.tasks" :key="t.id" class="task-row">
            <span class="task-emoji">{{ ROLE_EMOJI[t.role] }}</span>
            <span class="task-id">#{{ t.id }}</span>
            <span class="task-title">{{ t.title }}</span>
            <span class="task-status" :class="'status-' + t.status">
              {{ taskStatusLabel(t) }}
            </span>
            <span v-if="t.assigned_agent" class="task-agent">{{ t.assigned_agent }}</span>
          </div>
        </div>
      </div>

      <div class="footer">
        <span class="footer-item">🕐 创建于 {{ timeAgo(session.created_at) }}</span>
        <span v-if="session.started_at" class="footer-item">▶️ 开始于 {{ timeAgo(session.started_at) }}</span>
        <span v-if="session.completed_at" class="footer-item">✅ 完成于 {{ timeAgo(session.completed_at) }}</span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.plan-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 12px;
  overflow-y: auto;
  min-height: 0;
}

.empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-tertiary, #64748b);
  font-style: italic;
}

.header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color, #1e293b);
}

.requirement {
  flex: 1;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary, #f1f5f9);
}

.status {
  font-size: 12px;
  font-weight: 600;
  padding: 2px 8px;
  background: rgba(59, 130, 246, 0.1);
  border-radius: 4px;
}

.stats-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
  margin-bottom: 10px;
}

.stat {
  background: #1e293b;
  border-radius: 6px;
  padding: 8px;
  text-align: center;
}

.stat-num {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary, #f1f5f9);
  font-family: monospace;
}

.stat-label {
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
  margin-top: 2px;
}

.progress-bar {
  position: relative;
  height: 18px;
  background: #1e293b;
  border-radius: 9px;
  overflow: hidden;
  margin-bottom: 12px;
}

.progress-fill {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  background: linear-gradient(90deg, #10b981, #06b6d4);
  transition: width 0.3s;
}

.progress-text {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-weight: 600;
  color: #f1f5f9;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
}

.section {
  margin-bottom: 12px;
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary, #cbd5e1);
  margin-bottom: 6px;
}

.role-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.role-chip {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11px;
  background: #1e293b;
  border: 1px solid #334155;
  padding: 2px 8px;
  border-radius: 10px;
}

.chip-count {
  font-family: monospace;
  color: var(--accent-color, #60a5fa);
  font-weight: 600;
}

.task-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.task-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  padding: 4px 6px;
  background: #1e293b;
  border-radius: 4px;
}

.task-emoji {
  font-size: 13px;
}

.task-id {
  font-family: monospace;
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
  min-width: 50px;
}

.task-title {
  flex: 1;
  color: var(--text-primary, #f1f5f9);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-status {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  background: #334155;
  color: var(--text-secondary, #cbd5e1);
}

.task-status.status-done,
.task-status.status-approved {
  background: rgba(16, 185, 129, 0.2);
  color: #10b981;
}

.task-status.status-in_progress {
  background: rgba(16, 185, 129, 0.1);
  color: #34d399;
}

.task-status.status-awaiting_approval {
  background: rgba(245, 158, 11, 0.2);
  color: #f59e0b;
}

.task-status.status-rejected,
.task-status.status-failed {
  background: rgba(239, 68, 68, 0.2);
  color: #ef4444;
}

.task-agent {
  font-size: 10px;
  color: var(--text-tertiary, #94a3b8);
  font-family: monospace;
}

.footer {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border-color, #1e293b);
}

.footer-item {
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
}
</style>
