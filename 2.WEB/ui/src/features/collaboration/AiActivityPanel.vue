<script setup lang="ts">
/**
 * AiActivityPanel — AI 协作者活动详情面板
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 显示：
 *  - 每个 AI 当前在做什么（来自 awareness.activity）
 *  - 进度条（patching / reviewing）
 *  - 快速动作：让 AI 总结当前对话 / 让 AI 评审最近 patch
 */
import { ref, computed } from 'vue'
import { showToast } from 'vant'
import { useCollaborationStore } from '@/stores/collaboration'
import type { WorkspaceDoc, Conversation } from '@/composables/WorkspaceDoc'

const props = defineProps<{
  workspaceDoc: WorkspaceDoc | null
  conversation: Conversation | null
}>()

const store = useCollaborationStore()

const aiList = computed(() => store.aiActivities)

const showConfig = ref(false)
const customPrompt = ref('')

function _activityLabel(type: string): string {
  return {
    idle: '空闲',
    thinking: '思考中',
    streaming: '生成回复中',
    reviewing: '评审中',
    patching: '写代码补丁',
    branching: '创建分支',
    merging: '合并中',
  }[type] || type
}

async function onSummarize() {
  if (!props.workspaceDoc || !props.conversation) {
    showToast('请先选择对话')
    return
  }
  const msgs = props.workspaceDoc.listMessages(props.conversation.id)
  if (msgs.length === 0) {
    showToast('当前对话为空')
    return
  }
  // 取最近 10 条
  const last = msgs.slice(-10).map((m) => `${m.author_name}: ${m.content}`).join('\n')
  showPrompt(`总结这段对话：\n\n${last}`)
}

async function onReviewAll() {
  if (!props.workspaceDoc || !props.conversation) {
    showToast('请先选择对话')
    return
  }
  const patches = props.workspaceDoc.listCodePatches(props.conversation.id)
  const proposed = patches.filter((p) => p.status === 'proposed')
  if (proposed.length === 0) {
    showToast('没有待评审的补丁')
    return
  }
  showToast(`有 ${proposed.length} 个待评审补丁（评审功能开发中）`)
}

function showPrompt(p: string) {
  customPrompt.value = p
  showConfig.value = true
}
</script>

<template>
  <div class="ai-panel">
    <div class="header">
      <span>AI 协作者</span>
      <van-button size="mini" type="primary" plain @click="onSummarize">让 AI 总结</van-button>
    </div>

    <div v-if="aiList.length === 0" class="empty">
      暂无 AI 在线。在下方"AI 协作者"卡片中添加。
    </div>

    <div v-else class="ai-list">
      <div v-for="ai in aiList" :key="ai.clientId" class="ai-item">
        <div class="ai-avatar" :style="{ background: ai.user.color }">
          {{ ai.user.avatar || ai.user.name.slice(0, 1) }}
        </div>
        <div class="ai-body">
          <div class="ai-name">
            {{ ai.user.name }}
            <van-tag plain type="primary">{{ ai.user.role || 'coder' }}</van-tag>
          </div>
          <div class="ai-status">
            <span class="status-dot" :class="ai.activity.type"></span>
            {{ _activityLabel(ai.activity.type) }}
            <span v-if="ai.activity.description" class="description"> · {{ ai.activity.description }}</span>
          </div>
          <div v-if="ai.activity.progress > 0 && ai.activity.progress < 100" class="ai-progress">
            <van-progress
              :percentage="ai.activity.progress"
              :show-pivot="false"
              stroke-width="3"
              color="#7c3aed"
            />
          </div>
        </div>
      </div>
    </div>

    <div class="actions">
      <van-button block size="small" plain @click="onReviewAll">评审所有待审补丁</van-button>
    </div>

    <van-dialog v-model:show="showConfig" title="AI 提示" :show-confirm-button="false">
      <div class="prompt-box">
        <pre>{{ customPrompt }}</pre>
      </div>
    </van-dialog>
  </div>
</template>

<style scoped>
.ai-panel {
  padding: 12px;
  border-top: 1px solid var(--yj-border, #2a2a2a);
}
.header {
  display: flex; align-items: center; justify-content: space-between;
  margin-bottom: 12px;
  font-size: 13px; font-weight: 600;
}
.empty { font-size: 12px; color: var(--yj-text-muted, #666); padding: 12px 0; }
.ai-list { display: flex; flex-direction: column; gap: 10px; }
.ai-item {
  display: flex; gap: 10px; align-items: flex-start;
  background: rgba(124, 58, 237, 0.05);
  border-radius: 8px;
  padding: 10px;
}
.ai-avatar {
  width: 36px; height: 36px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  color: white; font-size: 14px; font-weight: 600;
  flex-shrink: 0;
}
.ai-body { flex: 1; min-width: 0; }
.ai-name {
  display: flex; align-items: center; gap: 6px;
  font-size: 13px; font-weight: 500;
  margin-bottom: 4px;
}
.ai-status {
  display: flex; align-items: center; gap: 6px;
  font-size: 12px; color: var(--yj-text-secondary, #aaa);
}
.status-dot {
  width: 6px; height: 6px; border-radius: 50%;
  background: #22c55e;
}
.status-dot.idle { background: #6b7280; }
.status-dot.streaming, .status-dot.thinking, .status-dot.patching, .status-dot.reviewing {
  background: #eab308; animation: pulse 1.5s infinite;
}
.description { color: var(--yj-text-muted, #888); }
.ai-progress { margin-top: 6px; }
.actions { margin-top: 12px; }
.prompt-box {
  padding: 16px; max-height: 60vh; overflow: auto;
}
.prompt-box pre {
  font-size: 12px; line-height: 1.5;
  white-space: pre-wrap; word-break: break-word;
  color: var(--yj-text, #e5e5e5);
}
@keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.4; } }
</style>
