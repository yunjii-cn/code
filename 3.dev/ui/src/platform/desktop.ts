import * as api from '@/api'

export async function chat(
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
    // 2026-06-08 TASK-2.3 引入：图片附件
    images?: string[]
  },
  signal: AbortSignal,
  onChunk: (chunk: string) => void,
) {
  const response = await fetch('/api/ai/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
    signal,
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
        if (data === '[DONE]') return
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
}

export async function stopChat(sessionId?: string) {
  await api.aiApi.stop(sessionId)
}

export async function loadSession(projectId: string, sessionId: string) {
  const data = await api.projectApi.loadConversation(projectId, sessionId) as unknown as { messages: Array<{ id: string; role: string; content: string; timestamp: number }> }
  return data
}

export async function getProviders() {
  const data = await api.aiApi.getProviders() as unknown as Array<{ id: string; name: string; type: string; enabled: boolean }>
  return data
}

export async function getModels(provider: string) {
  const data = await api.aiApi.getModels(provider) as unknown as Array<{ id: string; name: string; provider: string; size?: string; quantization?: string }>
  return data
}

export async function listProjects() {
  const data = await api.projectApi.list() as unknown as Array<{ id: string; name: string; path: string; description?: string; createdAt: number }>
  return data
}

export async function createProject(params: { name: string; workspace_path?: string }) {
  const data = await api.projectApi.create(params) as unknown as { id: string; name: string; path: string; description?: string; createdAt: number }
  return data
}

export async function switchProject(projectId: string) {
  await api.projectApi.switch(projectId)
}

export async function deleteProject(projectId: string) {
  await api.projectApi.delete(projectId)
}

export async function getActiveProject() {
  const data = await api.projectApi.getActive() as unknown as { id: string; name: string; path: string; description?: string; createdAt: number } | null
  return data
}

export async function getConversations(projectId: string) {
  const data = await api.projectApi.getConversations(projectId) as unknown as Array<{ id: string; title: string; projectId: string; createdAt: number; updatedAt: number }>
  return data
}

export async function getSettings() {
  const data = await api.systemApi.getSettings() as unknown as Record<string, unknown>
  return data
}

export async function saveSettings(settings: Record<string, unknown>) {
  await api.systemApi.saveSettings(settings as Record<string, string>)
}

export async function getSystemInfo() {
  const data = await api.systemApi.getHardware() as unknown as { os: string; cpu: string; memory: number; gpu?: string; pythonVersion?: string; nodeVersion?: string }
  return data
}
