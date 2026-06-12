import YunJiPlugin from '@/plugins/YunJiPlugin'

const PROVIDERS = [
  { id: 'anthropic', name: 'Anthropic', type: 'cloud', enabled: true },
  { id: 'openrouter', name: 'OpenRouter', type: 'cloud', enabled: true },
  { id: 'zhipu', name: '智谱', type: 'cloud', enabled: true },
]

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
  _signal: AbortSignal,
  onChunk: (chunk: string) => void,
) {
  const provider = params.provider || 'anthropic'
  const apiKeyResult = await YunJiPlugin.secureGet({ key: `api_key_${provider}` })
  const apiKey = apiKeyResult.value
  if (!apiKey) {
    throw new Error(`未配置 ${provider} 的 API Key`)
  }

  const baseUrlResult = await YunJiPlugin.secureGet({ key: `base_url_${provider}` })
  const baseUrl = baseUrlResult.value || undefined

  // @ts-ignore - Capacitor 插件事件监听
  const chunkListener = await (YunJiPlugin as any).addListener('ai:chunk', (event: { content?: string; error?: string }) => {
    if (event.content) {
      onChunk(event.content)
    }
    if (event.error) {
      throw new Error(event.error)
    }
  })

  try {
    await YunJiPlugin.aiChat({
      prompt: params.prompt,
      model: params.model || '',
      provider,
      api_key: apiKey,
      base_url: baseUrl,
      session_id: params.session_id || '',
      system_prompt: params.system_prompt || '',
      temperature: params.ai_temperature ?? 0.7,
      max_tokens: params.ai_max_tokens ?? 4096,
    })
  } finally {
    chunkListener.remove()
  }
}

export async function stopChat(sessionId?: string) {
  await YunJiPlugin.aiStop({ session_id: sessionId || '' })
}

export async function loadSession(projectId: string, sessionId: string) {
  const result = await YunJiPlugin.conversationLoad({ project_id: projectId, session_id: sessionId })
  const data = JSON.parse(result.conversation)
  return {
    messages: (data.messages || []).map((m: { id?: string; role: string; content: string; timestamp?: number }) => ({
      id: m.id || crypto.randomUUID(),
      role: m.role,
      content: m.content,
      timestamp: m.timestamp || Date.now(),
    })),
  }
}

export async function getProviders() {
  return PROVIDERS
}

export async function getModels(provider: string) {
  const apiKeyResult = await YunJiPlugin.secureGet({ key: `api_key_${provider}` })
  const apiKey = apiKeyResult.value
  if (!apiKey) {
    return []
  }

  const baseUrlResult = await YunJiPlugin.secureGet({ key: `base_url_${provider}` })
  const baseUrl = baseUrlResult.value || undefined

  try {
    const result = await YunJiPlugin.aiGetModels({
      provider,
      api_key: apiKey,
      base_url: baseUrl,
    })
    const data = JSON.parse(result.models)

    let models: Array<{ id: string; name: string; provider: string; size?: string; quantization?: string }> = []

    if (provider === 'anthropic') {
      const items = data.data || []
      models = items.map((m: { id: string; display_name?: string }) => ({
        id: m.id,
        name: m.display_name || m.id,
        provider,
      }))
    } else {
      const items = data.data || []
      models = items.map((m: { id: string; name?: string }) => ({
        id: m.id,
        name: m.name || m.id,
        provider,
      }))
    }

    return models
  } catch {
    return []
  }
}

export async function listProjects() {
  const result = await YunJiPlugin.projectList()
  return JSON.parse(result.projects) as Array<{
    id: string
    name: string
    path: string
    description?: string
    createdAt: number
  }>
}

export async function createProject(params: { name: string; workspace_path?: string }) {
  const result = await YunJiPlugin.projectCreate({
    name: params.name,
    workspace_path: params.workspace_path || '',
  })
  return JSON.parse(result.project) as {
    id: string
    name: string
    path: string
    description?: string
    createdAt: number
  }
}

export async function switchProject(projectId: string) {
  await YunJiPlugin.projectSwitch({ id: projectId })
}

export async function deleteProject(projectId: string) {
  await YunJiPlugin.projectDelete({ id: projectId })
}

export async function getActiveProject() {
  const projects = await listProjects()
  const activeProjectId = await getActiveProjectId()
  if (!activeProjectId) return null
  return projects.find((p) => p.id === activeProjectId) || null
}

export async function getConversations(projectId: string) {
  const result = await YunJiPlugin.conversationList({ project_id: projectId })
  return JSON.parse(result.conversations) as Array<{
    id: string
    title: string
    projectId: string
    createdAt: number
    updatedAt: number
  }>
}

export async function getSettings() {
  const settingsKeys = ['theme', 'language', 'fontSize', 'autoSave', 'defaultProvider', 'defaultModel', 'temperature', 'maxTokens', 'systemPrompt']
  const settings: Record<string, unknown> = {}

  for (const key of settingsKeys) {
    const result = await YunJiPlugin.secureGet({ key: `setting_${key}` })
    if (result.value !== null) {
      try {
        settings[key] = JSON.parse(result.value)
      } catch {
        settings[key] = result.value
      }
    }
  }

  if (!settings.theme) settings.theme = 'dark'
  if (!settings.language) settings.language = 'zh-CN'
  if (!settings.fontSize) settings.fontSize = 14
  if (!settings.autoSave) settings.autoSave = true

  return settings
}

export async function saveSettings(settings: Record<string, unknown>) {
  for (const [key, value] of Object.entries(settings)) {
    await YunJiPlugin.secureSet({
      key: `setting_${key}`,
      value: JSON.stringify(value),
    })
  }
}

export async function getSystemInfo() {
  const result = await YunJiPlugin.systemGetInfo()
  return {
    os: result.platform,
    cpu: '',
    memory: 0,
    gpu: result.device,
  }
}

async function getActiveProjectId(): Promise<string | null> {
  const result = await YunJiPlugin.secureGet({ key: 'active_project_id' })
  return result.value as string | null
}
