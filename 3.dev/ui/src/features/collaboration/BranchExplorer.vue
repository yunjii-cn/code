<script setup lang="ts">
/**
 * BranchExplorer — 分支树视图 + 合并 UI
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 */
import { ref, computed } from 'vue'
import { showDialog, showToast } from 'vant'
import type { Conversation, WorkspaceDoc } from '@/composables/WorkspaceDoc'
import {
  listBranches,
  diffBranch,
  mergeBranch,
  isBranch,
  type BranchInfo,
  type DiffSummary,
  type MergeResult,
} from '@/composables/branching-conversations'

const props = defineProps<{
  workspaceDoc: WorkspaceDoc | null
  conversations: Conversation[]
  activeId: string | null
  currentUserId: string | null
}>()

const emit = defineEmits<{
  (e: 'select', id: string): void
}>()

interface BranchTreeNode {
  conv: Conversation
  branches: BranchInfo[]
  isBranch: boolean
  depth: number
  children: BranchTreeNode[]
}

const tree = computed<BranchTreeNode[]>(() => {
  if (!props.workspaceDoc) return []
  const byId = new Map<string, Conversation>()
  for (const c of props.conversations) byId.set(c.id, c)

  // 主干 = branchOf 为空的对话
  const roots = props.conversations.filter((c) => !c.branch_of)
  return roots.map((r) => _buildTree(r, byId, 0))
})

function _buildTree(conv: Conversation, byId: Map<string, Conversation>, depth: number): BranchTreeNode {
  if (!props.workspaceDoc) return { conv, branches: [], isBranch: false, depth, children: [] }
  const branches = listBranches(props.workspaceDoc, conv.id)
  return {
    conv,
    branches,
    isBranch: isBranch(props.workspaceDoc, conv.id),
    depth,
    children: branches
      .map((b) => byId.get(b.id))
      .filter((c): c is Conversation => !!c)
      .map((c) => _buildTree(c, byId, depth + 1)),
  }
}

const showMergeDialog = ref(false)
const diffCache = ref<{ diff: DiffSummary; source: Conversation; target: Conversation } | null>(null)

async function onMerge(sourceConv: Conversation) {
  if (!props.workspaceDoc || !props.currentUserId) return
  // 找主干（向上找 branch_of 链）
  let parentId = sourceConv.branch_of
  // 简化：直接弹窗让用户选目标
  const candidates = props.conversations.filter(
    (c) => !c.archived && c.id !== sourceConv.id,
  )
  if (candidates.length === 0) {
    showToast('没有可合并的目标')
    return
  }
  // 选第一个主干作为默认
  const targetCandidates = candidates.filter((c) => !c.branch_of)
  let target = targetCandidates[0]
  if (!target) target = candidates[0]

  try {
    await showDialog({
      title: '合并到',
      message: `将 "${sourceConv.title}" 合并到 "${target.title}"？`,
      showCancelButton: true,
      confirmButtonText: '合并',
    })
  } catch { return }

  try {
    const diff = diffBranch(props.workspaceDoc, sourceConv.id)
    diffCache.value = { diff, source: sourceConv, target }
    showMergeDialog.value = true
  } catch (e) {
    const err = e instanceof Error ? e.message : String(e)
    showToast({ type: 'fail', message: err })
  }
}

async function onConfirmMerge() {
  if (!props.workspaceDoc || !props.currentUserId || !diffCache.value) return
  try {
    const r: MergeResult = mergeBranch(props.workspaceDoc, {
      sourceConversationId: diffCache.value.source.id,
      targetConversationId: diffCache.value.target.id,
      mergerId: props.currentUserId,
      archiveSource: true,
    })
    showMergeDialog.value = false
    showToast({
      type: 'success',
      message: `已合并：${r.appliedMessages} 条消息, ${r.appliedCodePatches} 个补丁, ${r.appliedReviews} 条评审`,
    })
    if (r.conflictWarnings.length > 0) {
      showDialog({
        title: '合并警告',
        message: r.conflictWarnings.join('\n'),
      })
    }
    diffCache.value = null
  } catch (e) {
    const err = e instanceof Error ? e.message : String(e)
    showToast({ type: 'fail', message: err })
  }
}

function _renderTree(_nodes: BranchTreeNode[], _depth = 0) {
  // 占位：当前在 computed 中已经直接构建 depth
  return []
}
</script>

<template>
  <div class="branch-explorer">
    <div class="header">
      <span>分支与合并</span>
    </div>

    <div v-if="tree.length === 0" class="empty">
      暂无对话
    </div>

    <div v-else class="tree">
      <template v-for="root in tree" :key="root.conv.id">
        <div
          class="tree-row root"
          :class="{ active: root.conv.id === activeId }"
          :style="{ paddingLeft: 12 + root.depth * 16 + 'px' }"
        >
          <van-icon name="cluster-o" />
          <span class="row-title" @click="emit('select', root.conv.id)">
            {{ root.conv.title || '（无标题）' }}
          </span>
        </div>
        <template v-for="b in root.branches" :key="b.id">
          <div
            v-if="props.conversations.find(c => c.id === b.id)"
            class="tree-row branch"
            :class="{ active: b.id === activeId, merged: b.merged }"
            :style="{ paddingLeft: 12 + (root.depth + 1) * 16 + 'px' }"
          >
            <van-icon :name="b.merged ? 'success' : 'fork'" />
            <span class="row-title" @click="emit('select', b.id)">
              {{ props.conversations.find(c => c.id === b.id)?.title || b.label }}
            </span>
            <van-tag v-if="b.merged" type="success" style="margin-left: 8px;">已合并</van-tag>
            <van-button
              v-else
              size="mini"
              plain
              type="primary"
              style="margin-left: auto;"
              @click="onMerge(props.conversations.find(c => c.id === b.id)!)"
            >
              合并
            </van-button>
          </div>
        </template>
      </template>
    </div>

    <!-- 合并确认对话框 -->
    <van-dialog
      v-model:show="showMergeDialog"
      title="合并预览"
      :show-confirm-button="true"
      :show-cancel-button="true"
      confirm-button-text="执行合并"
      @confirm="onConfirmMerge"
    >
      <div v-if="diffCache" class="diff-preview">
        <p><strong>{{ diffCache.source.title }}</strong> → <strong>{{ diffCache.target.title }}</strong></p>
        <ul>
          <li>新增消息：<b>{{ diffCache.diff.added_messages.length }}</b></li>
          <li>代码补丁：<b>{{ diffCache.diff.added_code_patches.length }}</b></li>
          <li>评审：<b>{{ diffCache.diff.added_reviews.length }}</b></li>
        </ul>
        <p style="font-size: 12px; color: #888;">合并后源分支会被归档。</p>
      </div>
    </van-dialog>
  </div>
</template>

<style scoped>
.branch-explorer {
  display: flex; flex-direction: column;
  height: 100%;
  background: var(--yj-content-bg, #0f0f0f);
}
.header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
  font-size: 14px; font-weight: 600;
}
.empty {
  padding: 40px; text-align: center;
  color: var(--yj-text-muted, #666); font-size: 13px;
}
.tree { flex: 1; overflow-y: auto; padding: 8px 0; }
.tree-row {
  display: flex; align-items: center; gap: 8px;
  padding: 8px 12px; cursor: pointer;
  font-size: 13px;
  border-bottom: 1px solid rgba(255,255,255,0.02);
}
.tree-row:hover { background: var(--yj-hover, rgba(255,255,255,0.04)); }
.tree-row.active { background: var(--yj-active, rgba(124,58,237,0.12)); }
.tree-row.root .row-title { font-weight: 500; }
.tree-row.branch .row-title { color: var(--yj-text-secondary, #aaa); }
.tree-row.merged { opacity: 0.6; }
.row-title { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.diff-preview { padding: 16px; }
.diff-preview ul { padding-left: 20px; margin: 8px 0; }
.diff-preview li { margin: 4px 0; font-size: 13px; }
</style>
