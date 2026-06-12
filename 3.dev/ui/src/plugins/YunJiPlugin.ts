import { registerPlugin } from '@capacitor/core'

export interface YunJiPlugin {
  aiChat(options: {
    prompt: string
    model: string
    provider: string
    api_key: string
    base_url?: string
    session_id?: string
    system_prompt?: string
    temperature?: number
    max_tokens?: number
  }): Promise<{ ok: boolean }>
  aiStop(options: { session_id?: string }): Promise<{ ok: boolean }>
  aiGetModels(options: { provider: string; api_key: string; base_url?: string }): Promise<{ models: string }>
  aiCheckProvider(options: { provider: string; api_key: string; base_url?: string }): Promise<{ ok: boolean; error?: string }>

  projectList(): Promise<{ projects: string }>
  projectCreate(options: { name: string; workspace_path?: string }): Promise<{ project: string }>
  projectSwitch(options: { id: string }): Promise<{ ok: boolean }>
  projectDelete(options: { id: string }): Promise<{ ok: boolean }>

  conversationList(options: { project_id: string }): Promise<{ conversations: string }>
  conversationLoad(options: { project_id: string; session_id: string }): Promise<{ conversation: string }>
  conversationSave(options: { project_id: string; session_id: string; messages: string; title?: string }): Promise<{ ok: boolean }>
  conversationDelete(options: { project_id: string; session_id: string }): Promise<{ ok: boolean }>

  secureGet(options: { key: string }): Promise<{ value: string | null }>
  secureSet(options: { key: string; value: string }): Promise<{ ok: boolean }>
  secureDelete(options: { key: string }): Promise<{ ok: boolean }>

  systemGetInfo(): Promise<{ version: string; platform: string; device: string }>
}

const YunJiPlugin = registerPlugin<YunJiPlugin>('YunJi')

export default YunJiPlugin
