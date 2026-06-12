<script setup lang="ts">
/**
 * CollaborationView — 协作主页
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 布局：
 *   ┌────────────────────────────────────────────┐
 *   │  顶栏：工作区切换 | 用户菜单 | AI 状态     │
 *   ├──────────┬─────────────────┬───────────────┤
 *   │ 侧边栏   │  消息流          │  右栏         │
 *   │ - 会话   │  (当前对话)      │  - 在线        │
 *   │ - 分支   │                  │  - AI 活动    │
 *   │ - 任务   │  [输入框]        │               │
 *   │ - 成员   │                  │               │
 *   └──────────┴─────────────────┴───────────────┘
 */
import { ref, computed, onMounted, watch } from 'vue'
import { showToast } from 'vant'
import { useCollaborationStore } from '@/stores/collaboration'
import { useWorkspaceDoc } from './composables/useWorkspaceDoc'
import { createBranch } from '@/composables/branching-conversations'
import { aiApi } from '@/api'
import type { WorkspaceDoc as WorkspaceDocType } from '@/composables/WorkspaceDoc'

import AwarenessPresence from './AwarenessPresence.vue'
import WorkspaceSwitcher from './WorkspaceSwitcher.vue'
import LoginPanel from './LoginPanel.vue'
import ConversationList from './ConversationList.vue'
import SharedTaskBoard from './SharedTaskBoard.vue'
import BranchExplorer from './BranchExplorer.vue'
import MessageStream from './MessageStream.vue'
import AiActivityPanel from './AiActivityPanel.vue'

const store = useCollaborationStore()

// 把 store 里的非响应式 ref 转成 WorkspaceDoc 引用
const workspaceDocRef = computed<WorkspaceDocType | null>(
  () => (store.workspaceDoc as unknown as WorkspaceDocType) || null,
)

const {
  meta,
  conversations,
  activeConversation,
  messages,
  setActiveConversation,
} = useWorkspaceDoc(workspaceDocRef)

const sidebarTab = ref<'conversations' | 'branches' | 'tasks'>('conversations')
const showRightPanel = ref(true)

// 创建对话弹窗
const showCreateDialog = ref(false)
const newConvTitle = ref('')
const showBranchDialog = ref(false)

// ────── 初始化连接 ──────

onMounted(async () => {
  if (store.isAuthenticated && store.activeWorkspaceId) {
    await store.connect()
  }
})

// 连接成功后默认选第一条对话
watch(
  () => conversations.value.length,
  (n) => {
    if (n > 0 && !activeConversation.value) {
      setActiveConversation(conversations.value[0].id)
    }
  },
)

// ────── 动作 ──────

function onCreateConversation() {
  if (!workspaceDocRef.value || !store.currentUser) return
  newConvTitle.value = ''
  showCreateDialog.value = true
}

function onConfirmCreate() {
  if (!workspaceDocRef.value || !store.currentUser) return
  const title = newConvTitle.value.trim()
  if (!title) return
  const id = 'c-' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6)
  workspaceDocRef.value.createConversation({
    id,
    title,
    creatorId: store.currentUser.id,
  })
  setActiveConversation(id)
  showCreateDialog.value = false
  showToast({ type: 'success', message: '已创建' })
}

// AI 协作者实例（懒创建）
let _ai: import('@/composables/ai-agent-as-collaborator').AICollaborator | null = null

async function onAskAI(content: string) {
  if (!workspaceDocRef.value || !activeConversation.value) return
  try {
    // 第一次调用时懒创建 AI 协作者
    if (!_ai) {
      _ai = await store.getAICollaborator({
        identity: {
          id: 'ai-assistant',
          name: 'AI 助手',
          avatar: '🤖',
          color: '#7c3aed',
          role: 'editor',
        },
        llm: async ({ messages, signal, onChunk }) => {
          // 把对话历史拼成 prompt（简化：未来可对接真正的流式 SSE）
          const prompt = messages.map((m) => `${m.role}: ${m.content}`).join('\n')
          try {
            // aiApi.chat() 返回的是 AxiosResponse，要取 .data
            const resp = await aiApi.chat({ prompt } as { prompt: string })
            const r = resp?.data || resp
            const text = (r && (r.text || r.content)) || ''
            // 分块模拟流式
            const chunkSize = 10
            for (let i = 0; i < text.length; i += chunkSize) {
              if (signal.aborted) break
              onChunk(text.slice(i, i + chunkSize))
              await new Promise((resolve) => setTimeout(resolve, 30))
            }
            return text
          } catch (e) {
            throw e
          }
        },
        autoReply: false,
        autoReview: false,
      })
    }

    // 先把用户消息写进 Y.Doc
    workspaceDocRef.value.appendMessage({
      id: 'm-' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
      conversationId: activeConversation.value.id,
      role: 'user',
      content: content + ' @ai',
      authorId: store.currentUser?.id || 'anonymous',
      authorName: store.currentUser?.display_name || '我',
    })

    await _ai.activeAssistant({
      conversationId: activeConversation.value.id,
      parentMessageId: null,
      history: [{ role: 'user', content }],
    })
  } catch (e) {
    const err = e instanceof Error ? e.message : String(e)
    showToast({ type: 'fail', message: `AI 调用失败: ${err}` })
  }
}

async function onLogout() {
  await store.logout()
  showToast('已登出')
}
</script>

<template>
  <div class="collaboration-view">
    <!-- 顶栏 -->
    <div class="topbar">
      <div class="left">
        <span class="brand">🤝 协作</span>
        <WorkspaceSwitcher v-if="store.isAuthenticated" />
      </div>
      <div class="right">
        <van-button
          v-if="!store.isConnected && store.isAuthenticated"
          size="small"
          type="success"
          icon="reconnect"
          @click="store.connect()"
        >
          重连
        </van-button>
        <van-button
          v-if="store.isAuthenticated"
          size="small"
          plain
          @click="onLogout"
        >
          登出
        </van-button>
      </div>
    </div>

    <!-- 未登录时显示登录面板 -->
    <LoginPanel v-if="!store.isAuthenticated" />

    <!-- 已登录 -->
    <div v-else class="main">
      <!-- 左：Awareness + 侧边栏 -->
      <aside class="left-panel">
        <AwarenessPresence />
        <div class="tabs">
          <div
            v-for="t in [
              { key: 'conversations', label: '会话', icon: 'chat-o' },
              { key: 'branches', label: '分支', icon: 'fork' },
              { key: 'tasks', label: '任务', icon: 'todo-list-o' },
            ]"
            :key="t.key"
            class="tab"
            :class="{ active: sidebarTab === t.key }"
            @click="sidebarTab = t.key as any"
          >
            <van-icon :name="t.icon" />
            <span>{{ t.label }}</span>
          </div>
        </div>
        <div class="tab-content">
          <ConversationList
            v-if="sidebarTab === 'conversations'"
            :conversations="conversations"
            :active-id="activeConversation?.id || null"
            :workspace-doc="workspaceDocRef"
            :current-user-id="store.currentUser?.id || null"
            @select="setActiveConversation"
            @create="onCreateConversation"
          />
          <BranchExplorer
            v-else-if="sidebarTab === 'branches'"
            :workspace-doc="workspaceDocRef"
            :conversations="conversations"
            :active-id="activeConversation?.id || null"
            :current-user-id="store.currentUser?.id || null"
            @select="setActiveConversation"
          />
          <SharedTaskBoard
            v-else-if="sidebarTab === 'tasks'"
            :workspace-doc="workspaceDocRef"
            :conversations="conversations"
            :current-user-id="store.currentUser?.id || null"
            @open-conversation="setActiveConversation"
          />
        </div>
      </aside>

      <!-- 中：消息流 -->
      <main class="center">
        <MessageStream
          v-if="activeConversation"
          :workspace-doc="workspaceDocRef"
          :conversation="activeConversation"
          :messages="messages"
          :current-user-id="store.currentUser?.id || null"
          @ask-ai="onAskAI"
        />
        <div v-else class="empty-center">
          <van-icon name="chat-o" size="48" />
          <p>选择或创建一个对话开始协作</p>
        </div>
      </main>

      <!-- 右：AI 活动面板 -->
      <aside v-if="showRightPanel" class="right-panel">
        <AiActivityPanel
          :workspace-doc="workspaceDocRef"
          :conversation="activeConversation"
        />
      </aside>
    </div>

    <!-- 创建对话弹窗 -->
    <van-dialog
      v-model:show="showCreateDialog"
      title="新建对话"
      show-cancel-button
      @confirm="onConfirmCreate"
    >
      <div style="padding: 12px;">
        <van-field
          v-model="newConvTitle"
          label="标题"
          placeholder="如：实现登录功能"
          required
          clearable
          @keydown.enter="onConfirmCreate"
        />
      </div>
    </van-dialog>
  </div>
</template>

<style scoped>
.collaboration-view {
  display: flex; flex-direction: column;
  height: 100%;
  background: var(--yj-page-bg, #0a0a0a);
  color: var(--yj-text, #e5e5e5);
}
.topbar {
  display: flex; align-items: center; justify-content: space-between;
  padding: 12px 20px;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
  background: var(--yj-topbar-bg, #141414);
}
.topbar .left, .topbar .right {
  display: flex; align-items: center; gap: 12px;
}
.brand { font-size: 16px; font-weight: 700; }
.main {
  flex: 1; display: flex; min-height: 0;
}
.left-panel {
  width: 320px;
  border-right: 1px solid var(--yj-border, #2a2a2a);
  display: flex; flex-direction: column;
  background: var(--yj-sidebar-bg, #141414);
}
.tabs {
  display: flex;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
}
.tab {
  flex: 1;
  display: flex; flex-direction: column; align-items: center; gap: 4px;
  padding: 10px 0;
  font-size: 11px; color: var(--yj-text-secondary, #aaa);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.15s;
}
.tab:hover { color: var(--yj-text, #e5e5e5); }
.tab.active { color: #7c3aed; border-bottom-color: #7c3aed; }
.tab-content { flex: 1; overflow: hidden; }
.center {
  flex: 1; display: flex; flex-direction: column;
  min-width: 0;
}
.empty-center {
  flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center;
  color: var(--yj-text-muted, #666);
  gap: 12px;
}
.right-panel {
  width: 300px;
  border-left: 1px solid var(--yj-border, #2a2a2a);
  background: var(--yj-sidebar-bg, #141414);
  overflow-y: auto;
}
</style>
