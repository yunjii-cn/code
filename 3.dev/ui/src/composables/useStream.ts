import { ref } from 'vue'

export function useStream() {
  const isStreaming = ref(false)
  let abortController: AbortController | null = null

  async function streamChat(
    params: {
      prompt: string
      model?: string
      provider?: string
      session_id?: string
      workspace_path?: string
      ai_language?: string
      ai_temperature?: number
      ai_max_tokens?: number
      system_prompt?: string
      auto_approve?: boolean
    },
    onChunk: (content: string) => void,
    onDone?: () => void,
    onError?: (error: Error) => void,
  ) {
    isStreaming.value = true
    abortController = new AbortController()

    try {
      const response = await fetch('/api/ai/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(params),
        signal: abortController.signal,
      })

      if (!response.ok) {
        throw new Error(`请求失败: ${response.status}`)
      }

      const reader = response.body?.getReader()
      if (!reader) throw new Error('无法读取响应流')

      const decoder = new TextDecoder()

      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        const text = decoder.decode(value, { stream: true })
        const lines = text.split('\n')

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6)
            if (data === '[DONE]') {
              onDone?.()
              return
            }
            try {
              const parsed = JSON.parse(data)
              if (parsed.content) {
                onChunk(parsed.content)
              }
            } catch {
              onChunk(data)
            }
          }
        }
      }

      onDone?.()
    } catch (e) {
      if ((e as Error).name !== 'AbortError') {
        onError?.(e as Error)
      }
    } finally {
      isStreaming.value = false
      abortController = null
    }
  }

  function stopStream() {
    abortController?.abort()
    isStreaming.value = false
  }

  return {
    isStreaming,
    streamChat,
    stopStream,
  }
}
