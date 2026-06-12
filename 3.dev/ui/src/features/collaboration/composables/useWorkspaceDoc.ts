/**
 * useWorkspaceDoc — 把 Y.js WorkspaceDoc 暴露为响应式 Vue 数据
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 为什么不用 store 直接管？
 *  - WorkspaceDoc 内部是 Y.Map/Y.Array，不能整体替换（会破坏 CRDT 引用）
 *  - 需要在"加载完成后"+"每次 deep observe 触发"时拉取快照到 ref
 *  - 业务组件 watch ref 即可，CRDT 细节完全隔离
 *
 * 用法：
 *   const { meta, conversations, messages, ... } = useWorkspaceDoc(workspaceDoc)
 */

import { ref, watch, onUnmounted, type Ref, shallowRef } from 'vue'
import type { WorkspaceDoc, WorkspaceMeta, Conversation, Message, CodePatch } from '@/composables/WorkspaceDoc'
import { Fields } from '@/composables/WorkspaceDoc'

export interface UseWorkspaceDocReturn {
  meta: Ref<WorkspaceMeta | null>
  conversations: Ref<Conversation[]>
  activeConversation: Ref<Conversation | null>
  messages: Ref<Message[]>
  codePatches: Ref<CodePatch[]>
  reviews: Ref<unknown[]>
  setActiveConversation: (id: string | null) => void
  refresh: () => void
}

export function useWorkspaceDoc(wsRef: Ref<WorkspaceDoc | null>): UseWorkspaceDocReturn {
  const meta = ref<WorkspaceMeta | null>(null)
  const conversations = ref<Conversation[]>([])
  const activeConvId = ref<string | null>(null)
  const activeConversation = ref<Conversation | null>(null)
  const messages = ref<Message[]>([])
  const codePatches = ref<CodePatch[]>([])
  const reviews = ref<unknown[]>([])

  const unsubs = shallowRef<Array<() => void>>([])

  function _detach() {
    for (const u of unsubs.value) {
      try { u() } catch { /* ignore */ }
    }
    unsubs.value = []
  }

  function _attach(ws: WorkspaceDoc) {
    _detach()
    const u1 = ws.onMetaChange(() => {
      meta.value = ws.getMeta()
    })
    const u2 = ws.onConversationsChange(() => {
      const list = ws.listConversations({ includeArchived: false })
      conversations.value = list
      if (activeConvId.value) {
        activeConversation.value = list.find((c) => c.id === activeConvId.value) || null
        if (activeConversation.value) {
          messages.value = ws.listMessages(activeConvId.value)
        }
      }
    })
    const u3 = ws.onCodePatchesChange(() => {
      if (activeConvId.value) {
        codePatches.value = ws.listCodePatches(activeConvId.value)
      }
    })
    const u4 = ws.onReviewsChange(() => {
      const out: unknown[] = []
      ws.reviews.forEach((r) => out.push(r.toJSON?.() || r))
      reviews.value = out
    })
    unsubs.value = [u1, u2, u3, u4]

    // 初次拉取
    meta.value = ws.getMeta()
    conversations.value = ws.listConversations({ includeArchived: false })
  }

  function _attachMessageObserver(ws: WorkspaceDoc, convId: string) {
    // 先取消旧订阅
    const old = unsubs.value
    unsubs.value = unsubs.value.filter((u, i) => i < 4) // 保留前 4 个
    old.slice(4).forEach((u) => { try { u() } catch { /* ignore */ } })
    const u = ws.onMessagesChange(convId, () => {
      messages.value = ws.listMessages(convId)
    })
    unsubs.value.push(u)
  }

  function setActiveConversation(id: string | null) {
    activeConvId.value = id
    if (!id || !wsRef.value) {
      activeConversation.value = null
      messages.value = []
      codePatches.value = []
      return
    }
    const conv = wsRef.value.listConversations().find((c) => c.id === id) || null
    activeConversation.value = conv
    messages.value = wsRef.value.listMessages(id)
    codePatches.value = wsRef.value.listCodePatches(id)
    _attachMessageObserver(wsRef.value, id)
  }

  function refresh() {
    const ws = wsRef.value
    if (!ws) return
    meta.value = ws.getMeta()
    conversations.value = ws.listConversations({ includeArchived: false })
    if (activeConvId.value) {
      activeConversation.value =
        conversations.value.find((c) => c.id === activeConvId.value) || null
      messages.value = ws.listMessages(activeConvId.value)
      codePatches.value = ws.listCodePatches(activeConvId.value)
    }
  }

  // 监听 wsRef 变化（连接/断开）
  watch(
    wsRef,
    (newWs) => {
      if (newWs) _attach(newWs)
      else {
        _detach()
        meta.value = null
        conversations.value = []
        activeConversation.value = null
        messages.value = []
        codePatches.value = []
        reviews.value = []
      }
    },
    { immediate: true },
  )

  onUnmounted(() => {
    _detach()
  })

  return {
    meta,
    conversations,
    activeConversation,
    messages,
    codePatches,
    reviews,
    setActiveConversation,
    refresh,
  }
}

// 重新导出 Fields 方便组件用
export { Fields }
