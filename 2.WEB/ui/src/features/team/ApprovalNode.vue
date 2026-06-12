<!--
  ApprovalNode.vue
  2026-06-09 TASK-3.10 引入：审批节点
  显示一个审批请求：来源角色、原因、决策按钮（批准/拒绝 + 评论）
-->
<script setup lang="ts">
import { ref } from 'vue'
import { showToast } from 'vant'
import type { TeamApproval } from '@/api'

interface Props {
  approval: TeamApproval
  taskTitle?: string
  canDecide?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  taskTitle: '',
  canDecide: false,
})

const emit = defineEmits<{
  (e: 'decide', approvalId: string, approve: boolean, comment: string): void
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
  architect: '架构师',
  developer: '开发',
  tester: '测试',
  documenter: '文档',
}

const STATUS_META: Record<string, { label: string; icon: string; color: string }> = {
  pending:  { label: '待决策', icon: '⏸', color: '#f59e0b' },
  approved: { label: '已批准', icon: '✅', color: '#10b981' },
  rejected: { label: '已拒绝', icon: '❌', color: '#ef4444' },
}

const showComment = ref(false)
const comment = ref('')
const submitting = ref(false)

function handleDecide(approve: boolean) {
  if (showComment.value) {
    submit(approve)
  } else {
    showComment.value = true
  }
}

async function submit(approve: boolean) {
  if (submitting.value) return
  submitting.value = true
  try {
    emit('decide', props.approval.id, approve, comment.value)
    showToast({ type: 'success', message: approve ? '已批准' : '已拒绝' })
  } finally {
    submitting.value = false
    showComment.value = false
    comment.value = ''
  }
}
</script>

<template>
  <div class="approval-node" :class="['status-' + approval.status]">
    <div class="node-header">
      <span class="approver">
        <span class="approver-emoji">{{ ROLE_EMOJI[approval.approver_role] }}</span>
        <span class="approver-label">{{ ROLE_LABEL[approval.approver_role] }}</span>
      </span>
      <span class="node-id">#{{ approval.id }}</span>
      <span class="status-tag" :style="{ color: STATUS_META[approval.status].color }">
        {{ STATUS_META[approval.status].icon }} {{ STATUS_META[approval.status].label }}
      </span>
    </div>

    <div v-if="taskTitle" class="task-ref">
      📋 任务: {{ taskTitle }}
    </div>

    <div class="reason">
      💬 {{ approval.reason }}
    </div>

    <div v-if="approval.decision_comment" class="decision-comment">
      <span class="comment-label">决策意见:</span>
      <span class="comment-text">{{ approval.decision_comment }}</span>
    </div>

    <div v-if="approval.status === 'pending' && canDecide" class="actions">
      <div v-if="!showComment" class="quick-actions">
        <button class="btn btn-approve" :disabled="submitting" @click="handleDecide(true)">
          ✅ 批准
        </button>
        <button class="btn btn-reject" :disabled="submitting" @click="handleDecide(false)">
          ❌ 拒绝
        </button>
      </div>
      <div v-else class="comment-area">
        <textarea
          v-model="comment"
          class="comment-input"
          placeholder="决策意见（可选）..."
          rows="2"
        />
        <div class="comment-actions">
          <button class="btn btn-approve" :disabled="submitting" @click="submit(true)">
            ✅ 确认批准
          </button>
          <button class="btn btn-reject" :disabled="submitting" @click="submit(false)">
            ❌ 确认拒绝
          </button>
          <button class="btn btn-cancel" @click="showComment = false">取消</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.approval-node {
  background: #1e293b;
  border: 1px solid #334155;
  border-left: 3px solid #f59e0b;
  border-radius: 6px;
  padding: 10px 12px;
  margin-bottom: 8px;
}

.approval-node.status-approved {
  border-left-color: #10b981;
  opacity: 0.7;
}

.approval-node.status-rejected {
  border-left-color: #ef4444;
  opacity: 0.7;
}

.node-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.approver {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary, #f1f5f9);
}

.approver-emoji {
  font-size: 14px;
}

.node-id {
  font-family: monospace;
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
  margin-left: auto;
}

.status-tag {
  font-size: 11px;
  font-weight: 500;
}

.task-ref {
  font-size: 11px;
  color: var(--text-tertiary, #64748b);
  margin-bottom: 4px;
}

.reason {
  font-size: 12px;
  color: var(--text-secondary, #cbd5e1);
  background: #0f172a;
  padding: 6px 8px;
  border-radius: 4px;
  margin-bottom: 6px;
  white-space: pre-wrap;
  word-break: break-word;
}

.decision-comment {
  font-size: 11px;
  color: var(--text-tertiary, #94a3b8);
  margin-bottom: 6px;
  padding-left: 8px;
  border-left: 2px solid #334155;
}

.comment-label {
  font-weight: 600;
  margin-right: 4px;
}

.actions {
  margin-top: 6px;
}

.quick-actions {
  display: flex;
  gap: 6px;
}

.comment-area {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.comment-input {
  width: 100%;
  background: #0f172a;
  border: 1px solid #334155;
  border-radius: 4px;
  padding: 6px 8px;
  color: var(--text-primary, #f1f5f9);
  font-size: 12px;
  font-family: inherit;
  resize: vertical;
}

.comment-input:focus {
  outline: none;
  border-color: var(--accent-color, #3b82f6);
}

.comment-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.btn {
  font-size: 11px;
  padding: 4px 10px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-approve {
  background: #10b981;
  color: #fff;
}

.btn-approve:hover:not(:disabled) {
  background: #059669;
}

.btn-reject {
  background: #ef4444;
  color: #fff;
}

.btn-reject:hover:not(:disabled) {
  background: #dc2626;
}

.btn-cancel {
  background: #475569;
  color: #f1f5f9;
}

.btn-cancel:hover {
  background: #64748b;
}
</style>
