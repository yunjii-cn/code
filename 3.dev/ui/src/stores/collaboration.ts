/**
 * collaboration store — 协作会话的 Pinia 状态
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 职责：
 *  - 管理"当前用户 / 当前工作区"（对接 auth store）
 *  - 持有 YjsClient 单例 + WorkspaceDoc
 *  - 把 awareness 状态（在线用户 / AI 活动）转换为响应式 ref
 *  - 提供 connect/disconnect/refresh 等动作
 *
 * 设计原则：
 *  - 不直接 import yjs-client.ts（避免循环依赖）；通过 getCurrentInstance / inject？
 *    —— 简单方案：动态 import，在 useYjsClient() 中延迟加载
 *  - 业务组件用 computed 从 store 拿数据，不要直接读 awareness
 */

import { defineStore } from 'pinia'
import { ref, computed, shallowRef } from 'vue'

// 复用 yjs-client 的连接状态类型
import type { YjsConnectionStatus } from '@/composables/yjs-client'
export type { YjsConnectionStatus }

export interface OnlineUser {
  clientId: number
  user: {
    id: string
    name: string
    avatar?: string
    color: string
    isAI: boolean
    role?: string
  }
  activity: {
    type: 'idle' | 'thinking' | 'streaming' | 'reviewing' | 'patching' | 'branching' | 'merging'
    target: { conversationId: string; messageId?: string; codePatchId?: string } | null
    progress: number
    description: string
    startedAt: number
  }
  cursor: { conversationId: string; messageId: string | null } | null
}

export interface TokenBundle {
  access_token: string
  refresh_token: string
}

const TOKEN_KEY = 'yunji:auth:tokens'
const USER_KEY = 'yunji:auth:user'
const WS_KEY = 'yunji:auth:active_workspace'

// ──────────── localStorage 工具 ────────────

function _loadTokens(): TokenBundle | null {
  try {
    const raw = localStorage.getItem(TOKEN_KEY)
    if (!raw) return null
    return JSON.parse(raw) as TokenBundle
  } catch {
    return null
  }
}

function _saveTokens(t: TokenBundle | null): void {
  try {
    if (t) localStorage.setItem(TOKEN_KEY, JSON.stringify(t))
    else localStorage.removeItem(TOKEN_KEY)
  } catch { /* ignore */ }
}

function _loadUser<T = unknown>(): T | null {
  try {
    const raw = localStorage.getItem(USER_KEY)
    if (!raw) return null
    return JSON.parse(raw) as T
  } catch {
    return null
  }
}

function _saveUser(u: unknown): void {
  try {
    if (u) localStorage.setItem(USER_KEY, JSON.stringify(u))
    else localStorage.removeItem(USER_KEY)
  } catch { /* ignore */ }
}

function _loadActiveWorkspace(): string | null {
  try {
    return localStorage.getItem(WS_KEY)
  } catch {
    return null
  }
}

function _saveActiveWorkspace(id: string | null): void {
  try {
    if (id) localStorage.setItem(WS_KEY, id)
    else localStorage.removeItem(WS_KEY)
  } catch { /* ignore */ }
}

// ──────────── store ────────────

export const useCollaborationStore = defineStore('collaboration', () => {
  // ─── 状态 ───
  const currentUser = ref<{ id: string; username: string; display_name: string; email?: string } | null>(
    _loadUser() as { id: string; username: string; display_name: string; email?: string } | null,
  )
  const tokens = ref<TokenBundle | null>(_loadTokens())
  const activeWorkspaceId = ref<string | null>(_loadActiveWorkspace())

  const connectionStatus = ref<YjsConnectionStatus>('disconnected')
  const onlineUsers = ref<OnlineUser[]>([])
  const aiActivities = ref<OnlineUser[]>([]) // isAI = true
  const error = ref<string | null>(null)

  // Yjs 客户端句柄（不响应式，只持有引用）
  const clientHandle = shallowRef<unknown>(null)
  // WorkspaceDoc 同上
  const workspaceDoc = shallowRef<unknown>(null)

  // ─── 计算属性 ───
  const isAuthenticated = computed(() => !!tokens.value?.access_token && !!currentUser.value)
  const isConnected = computed(() => connectionStatus.value === 'connected')
  const humanUsers = computed(() => onlineUsers.value.filter((u) => !u.user.isAI))
  const hasAI = computed(() => aiActivities.value.length > 0)

  function _setTokens(t: TokenBundle | null) {
    tokens.value = t
    _saveTokens(t)
  }

  function _setUser(u: typeof currentUser.value) {
    currentUser.value = u
    _saveUser(u)
  }

  function _setActiveWorkspace(id: string | null) {
    activeWorkspaceId.value = id
    _saveActiveWorkspace(id)
  }

  // ─── 鉴权动作 ───

  /**
   * 用户登录（用 api/index.ts 的 authApi.login()）
   * 调用方负责捕获异常
   */
  async function login(username: string, password: string) {
    error.value = null
    const { authApi } = await import('@/api')
    const result = await authApi.login({ username, password })
    _setUser(result.user)
    _setTokens({ access_token: result.access_token, refresh_token: result.refresh_token })
    return result.user
  }

  /** 用户注册 */
  async function register(payload: { username: string; password: string; email?: string; display_name?: string }) {
    error.value = null
    const { authApi } = await import('@/api')
    const result = await authApi.register(payload)
    _setUser(result.user)
    _setTokens({ access_token: result.access_token, refresh_token: result.refresh_token })
    _setActiveWorkspace(result.workspace_id)
    return result
  }

  /** 刷新 access token */
  async function refreshAccessToken(): Promise<string | null> {
    if (!tokens.value?.refresh_token) return null
    try {
      const { authApi } = await import('@/api')
      const newTokens = await authApi.refresh(tokens.value.refresh_token)
      _setTokens({
        access_token: newTokens.access_token,
        refresh_token: newTokens.refresh_token || tokens.value.refresh_token,
      })
      return newTokens.access_token
    } catch (e) {
      // refresh 失败 → 登出
      const reason = e instanceof Error ? e.message : String(e)
      error.value = `Token 刷新失败: ${reason}`
      logout()
      return null
    }
  }

  /** 登出 */
  async function logout() {
    if (tokens.value?.refresh_token) {
      try {
        const { authApi } = await import('@/api')
        await authApi.logout(tokens.value.refresh_token)
      } catch { /* ignore */ }
    }
    await disconnect()
    _setTokens(null)
    _setUser(null)
    _setActiveWorkspace(null)
  }

  /** 设备配对（用配对码换 device_id + user_id） */
  async function pairDevice(pairCode: string, deviceName?: string) {
    const { authApi } = await import('@/api')
    return authApi.pairDevice(pairCode, deviceName)
  }

  // ─── 工作区动作 ───

  async function listWorkspaces() {
    if (!tokens.value?.access_token) throw new Error('未登录')
    const { workspaceApi } = await import('@/api')
    const r = await workspaceApi.list(tokens.value.access_token)
    return r.data
  }

  async function createWorkspace(name: string, description: string = '') {
    if (!tokens.value?.access_token) throw new Error('未登录')
    const { workspaceApi } = await import('@/api')
    const r = await workspaceApi.create(tokens.value.access_token, name, description)
    return r.data
  }

  async function switchWorkspace(workspaceId: string) {
    if (activeWorkspaceId.value === workspaceId) return
    await disconnect()
    _setActiveWorkspace(workspaceId)
  }

  // ─── Y.js 连接动作 ───

  /**
   * 连接到当前 activeWorkspaceId 的协作房间
   */
  async function connect(workspaceId?: string) {
    if (workspaceId) _setActiveWorkspace(workspaceId)
    if (!activeWorkspaceId.value) {
      error.value = '未选择工作区'
      return
    }
    if (!tokens.value?.access_token) {
      error.value = '未登录'
      return
    }
    error.value = null
    connectionStatus.value = 'connecting'

    // 动态 import 避免循环依赖
    const { getYjsClient } = await import('@/composables/yjs-client')
    const { createWorkspaceDoc } = await import('@/composables/WorkspaceDoc')

    const handle = getYjsClient({
      workspaceId: activeWorkspaceId.value,
      getToken: () => tokens.value?.access_token ?? null,
      onAuthError: async () => {
        const newToken = await refreshAccessToken()
        return newToken
      },
    })

    clientHandle.value = handle
    workspaceDoc.value = createWorkspaceDoc(handle)

    // 等待 IndexedDB 加载
    await handle.whenSynced()

    // 订阅状态变化
    handle.onStatusChange((s) => {
      connectionStatus.value = s
    })

    // 启动 awareness 观察
    _startAwarenessObserver(handle.awareness as { on: (ev: string, cb: () => void) => void; getStates: () => Map<number, Record<string, unknown>> })
  }

  async function disconnect() {
    const handle = clientHandle.value as { disconnect?: () => void; destroy?: () => void } | null
    if (handle) {
      try { handle.disconnect?.() } catch { /* ignore */ }
    }
    clientHandle.value = null
    workspaceDoc.value = null
    onlineUsers.value = []
    aiActivities.value = []
    connectionStatus.value = 'disconnected'
  }

  // ─── awareness 观察 ───

  function _startAwarenessObserver(awareness: {
    on: (ev: string, cb: () => void) => void
    getStates: () => Map<number, Record<string, unknown>>
  }) {
    const _refresh = () => {
      const states = awareness.getStates()
      const users: OnlineUser[] = []
      const ais: OnlineUser[] = []
      states.forEach((s, clientId) => {
        // 仅接受有完整 user/activity 的状态（AI 和人）
        if (!s || typeof s !== 'object') return
        const u = s.user as { id: string; name: string; color: string; isAI: boolean; avatar?: string; role?: string } | undefined
        if (!u) return
        const a = (s.activity as OnlineUser['activity']) || {
          type: 'idle',
          target: null,
          progress: 0,
          description: '',
          startedAt: Date.now(),
        }
        const c = (s.cursor as OnlineUser['cursor']) || null
        const record: OnlineUser = {
          clientId,
          user: {
            id: u.id,
            name: u.name,
            color: u.color,
            isAI: !!u.isAI,
            avatar: u.avatar,
            role: u.role,
          },
          activity: a,
          cursor: c,
        }
        users.push(record)
        if (record.user.isAI) ais.push(record)
      })
      onlineUsers.value = users
      aiActivities.value = ais
    }
    awareness.on('change', _refresh)
    _refresh()
  }

  // ─── AI 协作者快捷入口（业务层用） ───

  /**
   * 获取（或创建）一个 AI 协作者实例
   */
  async function getAICollaborator(opts: {
    identity: {
      id: string
      name: string
      avatar?: string
      color?: string
      role?: 'coder' | 'reviewer' | 'editor' | 'orchestrator'
    }
    llm: (opts: {
      messages: { role: 'user' | 'assistant' | 'system'; content: string }[]
      signal: AbortSignal
      onChunk: (chunk: string) => void
    }) => Promise<string>
    autoReply?: boolean
    autoReview?: boolean
  }) {
    if (!workspaceDoc.value) {
      throw new Error('未连接到工作区，请先 connect()')
    }
    const { createAICollaborator } = await import('@/composables/ai-agent-as-collaborator')
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const ai = createAICollaborator(workspaceDoc.value as any, {
      id: opts.identity.id,
      name: opts.identity.name,
      avatar: opts.identity.avatar || '🤖',
      color: opts.identity.color || '#7c3aed',
      role: opts.identity.role || 'coder',
    }, opts.llm, {
      autoReply: opts.autoReply,
      autoReview: opts.autoReview,
    })
    ai.start()
    return ai
  }

  return {
    // 状态
    currentUser,
    tokens,
    activeWorkspaceId,
    connectionStatus,
    onlineUsers,
    aiActivities,
    error,
    // refs
    clientHandle,
    workspaceDoc,
    // 计算
    isAuthenticated,
    isConnected,
    humanUsers,
    hasAI,
    // 动作
    login,
    register,
    refreshAccessToken,
    logout,
    pairDevice,
    listWorkspaces,
    createWorkspace,
    switchWorkspace,
    connect,
    disconnect,
    getAICollaborator,
  }
})

// ──────────── 类型导出 ────────────

export type CollaborationStore = ReturnType<typeof useCollaborationStore>
