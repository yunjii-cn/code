import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as platform from '@/platform'

export interface Message {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: number
  // 2026-06-08 TASK-2.3 引入：图片附件（base64 数据 URL 列表）
  images?: string[]
}

export const useChatStore = defineStore('chat', () => {
  const messages = ref<Message[]>([])
  const isStreaming = ref(false)
  const currentModel = ref('')
  const currentProvider = ref('')
  const currentSessionId = ref('')

  let abortController: AbortController | null = null

  async function sendMessage(content: string, images?: string[]) {
    const userMsg: Message = {
      id: crypto.randomUUID(),
      role: 'user',
      content,
      timestamp: Date.now(),
      // 2026-06-08 TASK-2.3 引入：图片附件（base64 数据 URL 列表，渲染时显示缩略图）
      images: images && images.length > 0 ? images : undefined,
    }
    messages.value.push(userMsg)

    const assistantMsg: Message = {
      id: crypto.randomUUID(),
      role: 'assistant',
      content: '',
      timestamp: Date.now(),
    }
    messages.value.push(assistantMsg)
    isStreaming.value = true

    abortController = new AbortController()

    try {
      await platform.chat(
        {
          prompt: content,
          model: currentModel.value || undefined,
          provider: currentProvider.value || undefined,
          session_id: currentSessionId.value || undefined,
          // 2026-06-08 TASK-2.3: 透传图片
          images: images && images.length > 0 ? images : undefined,
        },
        abortController.signal,
        (chunk: string) => {
          assistantMsg.content += chunk
        },
      )
    } catch (e: unknown) {
      if (e instanceof Error && e.name !== 'AbortError') {
        assistantMsg.content += `\n\n[错误] ${(e as Error).message}`
      }
    } finally {
      isStreaming.value = false
      abortController = null
    }
  }

  function stopStream() {
    if (abortController) {
      abortController.abort()
      abortController = null
    }
    isStreaming.value = false
  }

  function clearChat() {
    messages.value = []
    currentSessionId.value = ''
  }

  async function loadSession(projectId: string, sessionId: string) {
    const session = await platform.loadSession(projectId, sessionId)
    messages.value = session.messages.map((m: { id: string; role: string; content: string; timestamp: number }) => ({
      ...m,
      role: m.role as 'user' | 'assistant' | 'system',
    }))
    currentSessionId.value = sessionId
  }

  function switchModel(provider: string, model: string) {
    currentProvider.value = provider
    currentModel.value = model
  }

  return {
    messages,
    isStreaming,
    currentModel,
    currentProvider,
    currentSessionId,
    sendMessage,
    stopStream,
    clearChat,
    loadSession,
    switchModel,
  }
})
