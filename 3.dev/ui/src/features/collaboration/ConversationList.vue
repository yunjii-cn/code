<script setup lang="ts">
/**
 * ConversationList — 对话列表（含分支指示）
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 */
import { computed, ref } from 'vue'
import { showToast } from 'vant'
import type { Conversation, WorkspaceDoc } from '@/composables/WorkspaceDoc'
import { Fields } from '@/composables/WorkspaceDoc'
import { createBranch, isBranch, getParentId } from '@/composables/branching-conversations'

const props = defineProps<{
  conversations: Conversation[]
  activeId: string | null
  workspaceDoc: WorkspaceDoc | null
  currentUserId: string | null
}>()

const emit = defineEmits<{
  (e: 'select', id: string): void
  (e: 'create'): void
}>()

const search = ref('')
const filterArchived = ref(false)

// 分支弹窗
const showBranchDialog = ref(false)
const branchTitle = ref('')
const branchParentId = ref<string | null>(null)

const filtered = computed(() => {
  let list = props.conversations
  if (!filterArchived.value) list = list.filter((c) => !c.archived)
  if (search.value) {
    const q = search.value.toLowerCase()
    list = list.filter((c) => c.title.toLowerCase().includes(q))
  }
  return list
})

function openForkDialog(conv: Conversation) {
  branchParentId.value = conv.id
  branchTitle.value = ''
  showBranchDialog.value = true
}

function confirmFork() {
  if (!props.workspaceDoc || !props.currentUserId || !branchParentId.value) return
  const title = branchTitle.value.trim()
  if (!title) return
  const parentId = branchParentId.value
  const newId = 'c-' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6)
  const parent = props.workspaceDoc.getConversation(parentId)
  if (!parent) return
  // title 字段是 Y.Text
  const titleYText = parent.get(Fields.title) as { toString: () => string } | undefined
  const parentTitle = titleYText ? titleYText.toString() : '对话'
  const r = createBranch(props.workspaceDoc, {
    parentConversationId: parentId,
    newConversationId: newId,
    title: `${parentTitle} · ${title}`,
    creatorId: props.currentUserId,
    label: title,
  })
  showBranchDialog.value = false
  showToast({ type: 'success', message: `已创建分支：${title}` })
  emit('select', r.newConversationId)
}

function _isBranch(conv: Conversation): boolean {
  return !!conv.branch_of
}

function _parent(conv: Conversation): string | null {
  if (!props.workspaceDoc) return null
  return getParentId(props.workspaceDoc, conv.id)
}
</script>

<template>
  <div class="conv-list">
    <div class="header">
      <span class="title">对话</span>
      <van-button size="mini" type="primary" icon="plus" @click="emit('create')">新建</van-button>
    </div>
    <van-search v-model="search" placeholder="搜索对话" />

    <div v-if="filtered.length === 0" class="empty">
      暂无对话，点击右上角"新建"开始
    </div>
    <div v-else class="items">
      <div
        v-for="c in filtered"
        :key="c.id"
        class="conv-item"
        :class="{ active: c.id === activeId, branch: _isBranch(c) }"
        @click="emit('select', c.id)"
      >
        <div class="conv-icon">
          <van-icon :name="_isBranch(c) ? 'fork' : 'chat-o'" />
        </div>
        <div class="conv-meta">
          <div class="conv-title">
            {{ c.title || '（无标题）' }}
          </div>
          <div class="conv-sub">
            <span v-if="_isBranch(c)">分支 · {{ _parent(c) ? (_parent(c) || '').slice(0, 6) : '?' }}</span>
            <span v-else>{{ c.branch_of ? '有源分支' : '主干' }}</span>
            <span class="dot-sep">·</span>
            <span>{{ new Date(c.updated_at).toLocaleDateString() }}</span>
          </div>
        </div>
        <van-icon
          name="plus"
          class="fork-btn"
          @click.stop="openForkDialog(c)"
          title="创建分支"
        />
      </div>
    </div>

    <!-- 分支弹窗 -->
    <van-dialog
      v-model:show="showBranchDialog"
      title="创建分支"
      show-cancel-button
      @confirm="confirmFork"
    >
      <div style="padding: 12px;">
        <van-field
          v-model="branchTitle"
          label="分支名"
          placeholder="如：试一下 GPT-5"
          required
          clearable
          @keydown.enter="confirmFork"
        />
      </div>
    </van-dialog>
  </div>
</template>

<style scoped>
.conv-list {
  height: 100%;
  display: flex; flex-direction: column;
  background: var(--yj-sidebar-bg, #141414);
}
.header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 12px 16px; border-bottom: 1px solid var(--yj-border, #2a2a2a);
}
.header .title { font-size: 14px; font-weight: 600; }
.empty { padding: 24px; text-align: center; color: var(--yj-text-muted, #666); font-size: 13px; }
.items { flex: 1; overflow-y: auto; }
.conv-item {
  display: flex; align-items: center; gap: 10px;
  padding: 10px 16px;
  cursor: pointer;
  border-bottom: 1px solid rgba(255,255,255,0.03);
  transition: background 0.15s;
}
.conv-item:hover { background: var(--yj-hover, rgba(255,255,255,0.04)); }
.conv-item.active { background: var(--yj-active, rgba(124,58,237,0.12)); }
.conv-item.branch { border-left: 2px solid #7c3aed; }
.conv-icon { color: var(--yj-text-muted, #666); font-size: 18px; }
.conv-meta { flex: 1; min-width: 0; }
.conv-title {
  font-size: 13px; font-weight: 500; color: var(--yj-text, #eee);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.conv-sub {
  font-size: 11px; color: var(--yj-text-muted, #888);
  display: flex; gap: 4px; align-items: center;
}
.dot-sep { opacity: 0.5; }
.fork-btn {
  opacity: 0; transition: opacity 0.15s;
  color: var(--yj-text-secondary, #aaa); font-size: 16px; padding: 4px;
}
.conv-item:hover .fork-btn { opacity: 1; }
</style>
