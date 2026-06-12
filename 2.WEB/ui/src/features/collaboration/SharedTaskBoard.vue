<script setup lang="ts">
/**
 * SharedTaskBoard — 共享任务看板（基于 WorkspaceDoc 的 Y.Map）
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 注：当前版本任务数据用 WorkspaceDoc 的 conversations 作占位
 * （每个 conversation.task_id 关联任务）
 * 后续可扩展为独立 Y.Map<taskId, Y.Map<SharedTask>>
 */
import { ref, computed, watch } from 'vue'
import { showToast } from 'vant'
import type { Conversation, WorkspaceDoc } from '@/composables/WorkspaceDoc'

const props = defineProps<{
  workspaceDoc: WorkspaceDoc | null
  conversations: Conversation[]
  currentUserId: string | null
}>()

const emit = defineEmits<{
  (e: 'openConversation', id: string): void
}>()

// 用 conversations 推导出"任务"（简化）：每个 conv 是一张任务卡
const statusOf = (c: Conversation): 'todo' | 'in_progress' | 'blocked' | 'review' | 'done' => {
  if (c.archived) return 'done'
  if (c.head_snapshot) return 'review'
  if (c.merged_from.length > 0) return 'review'
  if (c.branch_of) return 'in_progress'
  return 'todo'
}

const columns = [
  { key: 'todo', label: '待办' },
  { key: 'in_progress', label: '进行中' },
  { key: 'blocked', label: '阻塞' },
  { key: 'review', label: '待评审' },
  { key: 'done', label: '已完成' },
] as const

const grouped = computed(() => {
  const map: Record<string, Conversation[]> = {
    todo: [], in_progress: [], blocked: [], review: [], done: [],
  }
  for (const c of props.conversations) {
    map[statusOf(c)].push(c)
  }
  return map
})

const dragConvId = ref<string | null>(null)

function onDragStart(id: string, e: DragEvent) {
  dragConvId.value = id
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move'
    e.dataTransfer.setData('text/plain', id)
  }
}

function onDrop(status: 'todo' | 'in_progress' | 'blocked' | 'review' | 'done', e: DragEvent) {
  e.preventDefault()
  const id = dragConvId.value
  if (!id || !props.workspaceDoc) return
  const conv = props.workspaceDoc.getConversation(id)
  if (!conv) return
  if (status === 'done') {
    props.workspaceDoc.archiveConversation(id, true)
    showToast({ type: 'success', message: '已归档' })
  } else if (status === 'review') {
    props.workspaceDoc.doc.transact(() => {
      conv.set('head_snapshot', 'pending')
    }, 'kanban-review')
    showToast('已标记为待评审')
  }
  dragConvId.value = null
}

function onDragOver(e: DragEvent) {
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
}

// 新建任务弹窗
const showNewTaskDialog = ref(false)
const newTaskTitle = ref('')

function onCreate() {
  if (!props.workspaceDoc || !props.currentUserId) return
  newTaskTitle.value = ''
  showNewTaskDialog.value = true
}

function confirmCreateTask() {
  if (!props.workspaceDoc || !props.currentUserId) return
  const title = newTaskTitle.value.trim()
  if (!title) return
  const id = 'c-' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6)
  props.workspaceDoc.createConversation({
    id, title, creatorId: props.currentUserId,
  })
  showNewTaskDialog.value = false
  showToast({ type: 'success', message: '已创建' })
}
</script>

<template>
  <div class="task-board">
    <div class="board-header">
      <span>共享任务看板</span>
      <van-button size="mini" type="primary" icon="plus" @click="onCreate">新建</van-button>
    </div>

    <div class="columns">
      <div
        v-for="col in columns"
        :key="col.key"
        class="column"
        @dragover="onDragOver"
        @drop="onDrop(col.key, $event)"
      >
        <div class="col-header">
          <span class="col-label">{{ col.label }}</span>
          <span class="col-count">{{ grouped[col.key]?.length || 0 }}</span>
        </div>
        <div class="col-body">
          <div
            v-for="c in grouped[col.key] || []"
            :key="c.id"
            class="task-card"
            draggable="true"
            @dragstart="onDragStart(c.id, $event)"
            @click="emit('openConversation', c.id)"
          >
            <div class="task-title">{{ c.title || '（无标题）' }}</div>
            <div class="task-meta">
              <van-icon name="user-o" />
              <span>{{ c.creator_id.slice(0, 6) }}</span>
              <span class="dot-sep">·</span>
              <van-icon name="clock-o" />
              <span>{{ new Date(c.updated_at).toLocaleDateString() }}</span>
            </div>
            <div v-if="c.branch_of" class="task-tag branch">
              <van-icon name="fork" /> 分支
            </div>
            <div v-else-if="c.merged_from.length > 0" class="task-tag merged">
              <van-icon name="exchange" /> 已合并
            </div>
          </div>
          <div v-if="(grouped[col.key] || []).length === 0" class="col-empty">
            拖拽任务到此处
          </div>
        </div>
      </div>
    </div>

    <!-- 新建任务弹窗 -->
    <van-dialog
      v-model:show="showNewTaskDialog"
      title="新建任务"
      show-cancel-button
      @confirm="confirmCreateTask"
    >
      <div style="padding: 12px;">
        <van-field
          v-model="newTaskTitle"
          label="任务名"
          placeholder="如：实现登录页"
          required
          clearable
          @keydown.enter="confirmCreateTask"
        />
      </div>
    </van-dialog>
  </div>
</template>

<style scoped>
.task-board {
  display: flex; flex-direction: column;
  height: 100%; overflow: hidden;
  background: var(--yj-content-bg, #0f0f0f);
}
.board-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
  font-size: 14px; font-weight: 600;
}
.columns {
  display: flex; gap: 12px; padding: 12px;
  overflow-x: auto; flex: 1;
}
.column {
  flex: 0 0 260px;
  background: var(--yj-card-bg, #1a1a1a);
  border-radius: 8px;
  display: flex; flex-direction: column;
  border: 1px solid var(--yj-border, #2a2a2a);
}
.col-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
}
.col-label { font-size: 13px; font-weight: 600; }
.col-count {
  font-size: 11px; padding: 2px 6px;
  background: rgba(255,255,255,0.1); border-radius: 10px;
}
.col-body { flex: 1; padding: 8px; overflow-y: auto; min-height: 100px; }
.col-empty {
  text-align: center; padding: 16px;
  font-size: 11px; color: var(--yj-text-muted, #666);
  border: 1px dashed var(--yj-border, #333);
  border-radius: 6px;
}
.task-card {
  background: rgba(255,255,255,0.04);
  border-radius: 6px;
  padding: 10px;
  margin-bottom: 8px;
  cursor: grab;
  border: 1px solid transparent;
  transition: all 0.15s;
}
.task-card:hover { background: rgba(255,255,255,0.08); border-color: var(--yj-border, #333); }
.task-card:active { cursor: grabbing; }
.task-title {
  font-size: 13px; font-weight: 500;
  margin-bottom: 6px;
  color: var(--yj-text, #eee);
}
.task-meta {
  display: flex; align-items: center; gap: 4px;
  font-size: 11px; color: var(--yj-text-muted, #888);
}
.task-tag {
  display: inline-flex; align-items: center; gap: 4px;
  font-size: 10px; padding: 2px 6px; border-radius: 4px;
  margin-top: 6px;
}
.task-tag.branch { background: rgba(124,58,237,0.15); color: #a78bfa; }
.task-tag.merged { background: rgba(34,197,94,0.15); color: #4ade80; }
.dot-sep { opacity: 0.5; }
</style>
