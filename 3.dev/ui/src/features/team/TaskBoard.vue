<!--
  TaskBoard.vue
  2026-06-09 TASK-3.10 引入：任务看板
  4 列：📋 待办 / 🔄 进行中 / ⏸ 待审批 / ✅ 已完成
  每张任务卡显示：角色、ID、标题、依赖关系
-->
<script setup lang="ts">
import { computed } from 'vue'
import type { TeamTask } from '@/api'

interface Props {
  tasks: TeamTask[]
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'select', taskId: string): void
  (e: 'assign', taskId: string): void
  (e: 'start', taskId: string): void
  (e: 'complete', taskId: string): void
  (e: 'fail', taskId: string): void
}>()

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

const columns = computed(() => {
  const pending = props.tasks.filter(t => t.status === 'pending')
  const inProgress = props.tasks.filter(t => t.status === 'in_progress')
  const awaiting = props.tasks.filter(
    t => t.status === 'awaiting_approval' || t.status === 'approved'
  )
  const done = props.tasks.filter(
    t => t.status === 'done' || t.status === 'rejected' || t.status === 'failed' || t.status === 'skipped'
  )
  return [
    { id: 'pending', title: '📋 待办', items: pending, color: '#94a3b8' },
    { id: 'in_progress', title: '🔄 进行中', items: inProgress, color: '#10b981' },
    { id: 'awaiting', title: '⏸ 待审批', items: awaiting, color: '#f59e0b' },
    { id: 'done', title: '✅ 已完成', items: done, color: '#3b82f6' },
  ]
})

function handleCardClick(task: TeamTask) {
  emit('select', task.id)
}
</script>

<template>
  <div class="task-board">
    <div v-for="col in columns" :key="col.id" class="column" :style="{ borderTopColor: col.color }">
      <div class="column-header">
        <span class="column-title">{{ col.title }}</span>
        <span class="column-count" :style="{ background: col.color }">{{ col.items.length }}</span>
      </div>
      <div class="column-body">
        <div
          v-for="task in col.items"
          :key="task.id"
          class="task-card"
          :class="['status-' + task.status]"
          @click="handleCardClick(task)"
        >
          <div class="task-header">
            <span class="role-tag">
              <span class="role-emoji">{{ ROLE_EMOJI[task.role] }}</span>
              <span class="role-label">{{ ROLE_LABEL[task.role] }}</span>
            </span>
            <span class="task-id">#{{ task.id }}</span>
          </div>
          <div class="task-title">{{ task.title }}</div>
          <div v-if="task.dependencies.length" class="task-deps">
            🔗 依赖: {{ task.dependencies.map(d => '#' + d).join(', ') }}
          </div>
          <div v-if="task.assigned_agent" class="task-agent">
            👤 {{ task.assigned_agent }}
          </div>
          <div v-if="task.status === 'rejected' || task.status === 'failed'" class="task-error">
            ❌ {{ task.status === 'rejected' ? '被拒绝' : '失败' }}
          </div>
          <div v-if="task.status === 'pending'" class="task-actions">
            <button class="btn-mini" @click.stop="emit('assign', task.id)">派发</button>
          </div>
          <div v-else-if="task.status === 'in_progress'" class="task-actions">
            <button class="btn-mini primary" @click.stop="emit('complete', task.id)">完成</button>
            <button class="btn-mini danger" @click.stop="emit('fail', task.id)">失败</button>
          </div>
        </div>
        <div v-if="col.items.length === 0" class="empty-col">暂无</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.task-board {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  height: 100%;
  overflow: hidden;
}

.column {
  background: var(--card-bg, #0f172a);
  border: 1px solid var(--border-color, #1e293b);
  border-top: 3px solid;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.column-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-color, #1e293b);
  flex-shrink: 0;
}

.column-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary, #f1f5f9);
}

.column-count {
  font-size: 11px;
  color: #fff;
  padding: 1px 8px;
  border-radius: 10px;
  font-weight: 600;
  min-width: 24px;
  text-align: center;
}

.column-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
}

.task-card {
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 6px;
  padding: 8px;
  cursor: pointer;
  transition: all 0.15s;
}

.task-card:hover {
  border-color: var(--accent-color, #3b82f6);
  transform: translateX(2px);
}

.task-card.status-in_progress {
  border-left: 3px solid #10b981;
}

.task-card.status-awaiting_approval {
  border-left: 3px solid #f59e0b;
}

.task-card.status-approved {
  border-left: 3px solid #3b82f6;
}

.task-card.status-done {
  opacity: 0.6;
}

.task-card.status-rejected,
.task-card.status-failed {
  border-left: 3px solid #ef4444;
  opacity: 0.7;
}

.task-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.role-tag {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11px;
  background: #334155;
  padding: 1px 6px;
  border-radius: 4px;
}

.task-id {
  font-family: monospace;
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
}

.task-title {
  font-size: 12px;
  color: var(--text-primary, #f1f5f9);
  font-weight: 500;
  margin-bottom: 4px;
  word-break: break-word;
}

.task-deps,
.task-agent,
.task-error {
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
  margin-top: 2px;
}

.task-error {
  color: #ef4444;
}

.task-actions {
  display: flex;
  gap: 4px;
  margin-top: 6px;
}

.btn-mini {
  font-size: 11px;
  padding: 3px 10px;
  background: #334155;
  color: #f1f5f9;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s;
}

.btn-mini:hover {
  background: #475569;
}

.btn-mini.primary {
  background: #10b981;
}

.btn-mini.primary:hover {
  background: #059669;
}

.btn-mini.danger {
  background: #ef4444;
  color: #fff;
}

.btn-mini.danger:hover {
  background: #dc2626;
}

.empty-col {
  text-align: center;
  font-size: 12px;
  color: var(--text-tertiary, #64748b);
  padding: 20px 0;
  font-style: italic;
}

/* 移动端：单列堆叠 */
@media (max-width: 768px) {
  .task-board {
    grid-template-columns: 1fr;
    grid-template-rows: repeat(4, 1fr);
  }
}
</style>
