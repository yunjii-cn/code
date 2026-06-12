/**
 * AI Agent as Collaborator — 让 AI 像人一样"在线"在协作里
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 设计动机：
 *  在多人协作里，AI 不应该只是"被动响应"，而应该像同事一样：
 *   - 有头像 / 颜色（presence）
 *   - 有当前活动（typing / thinking / reviewing / idle）
 *   - 会"注视"某条消息（cursor 概念）
 *   - 会主动发起消息、回复、合并分支
 *
 * 这一切都通过 Y.js Awareness 协议同步：所有客户端（包括人类）都能看到
 * "AI 正在为这条消息生成代码补丁"这种实时状态。
 *
 * 触发模式：
 *   - watchAndReply()     监听新消息，自动生成回复（仅当被 @ 提及时）
 *   - autoReview()         监听新增的 codePatch，自动写 review
 *   - activeAssistant()    被动调用：作为 SSE/streaming 的回调接口，把 chunk 写入 Y.Doc
 *
 * 架构：
 *   - 一个 workspace 一个 AICollaborator 实例
 *   - 内部用 AbortController 控制"停止 AI"
 *   - 与 aiApi.streamChat() / agentApi.runTask() 等业务流解耦（注入 LLMStreamFn）
 */

import type { Awareness } from 'y-protocols/awareness'
import type { WorkspaceDoc, MessageRole, CodePatch, Review } from './WorkspaceDoc'
import { Fields } from './WorkspaceDoc'

// ──────────── 类型 ────────────

export interface AIIdentity {
  id: string
  name: string
  /** 头像 emoji 或 URL */
  avatar: string
  /** 协作色（用于 awareness 标识） */
  color: string
  /** 角色：编码员 / 评审员 / 总编 */
  role: 'coder' | 'reviewer' | 'editor' | 'orchestrator'
}

export type AIActivityType =
  | 'idle'
  | 'thinking'
  | 'streaming'
  | 'reviewing'
  | 'patching'
  | 'branching'
  | 'merging'

export interface AIActivity {
  type: AIActivityType
  /** 当前正在处理的目标 */
  target: {
    conversationId: string
    messageId?: string
    codePatchId?: string
  } | null
  /** 进度 0-100（仅 patching/reviewing 有意义） */
  progress: number
  /** 本次活动开始时间 */
  startedAt: number
  /** 给人类看的一句话说明 */
  description: string
}

export interface AIAwarenessState {
  user: AIIdentity & { isAI: true }
  activity: AIActivity
  /** AI 正在"注视"的光标 */
  cursor: {
    conversationId: string
    messageId: string | null
  } | null
}

/**
 * LLM 流式调用抽象（业务层注入）。
 *  - input.messages: 上下文
 *  - signal: 中断信号
 *  - onChunk: 每个 token 触发
 *  - 返回完整文本
 */
export type LLMStreamFn = (opts: {
  messages: { role: 'user' | 'assistant' | 'system'; content: string }[]
  signal: AbortSignal
  onChunk: (chunk: string) => void
}) => Promise<string>

export interface AICollaboratorOptions {
  ws: WorkspaceDoc
  identity: AIIdentity
  llm: LLMStreamFn
  /** 是否自动回复（默认 false，避免噪声） */
  autoReply?: boolean
  /** 触发自动回复的关键字（如 "@ai"），不传则不自动 */
  replyTriggers?: RegExp[]
  /** 是否自动 review 新增 codePatches */
  autoReview?: boolean
  /** 自动 review 的概率 0-1（避免每个 patch 都 review） */
  reviewProbability?: number
  /** 注入到 AI 回复的系统提示 */
  systemPrompt?: string
}

// ──────────── 主类 ────────────

export class AICollaborator {
  readonly ws: WorkspaceDoc
  readonly awareness: Awareness
  readonly identity: AIIdentity

  private _llm: LLMStreamFn
  private _autoReply: boolean
  private _replyTriggers: RegExp[]
  private _autoReview: boolean
  private _reviewProbability: number
  private _systemPrompt: string

  private _unsubs: Array<() => void> = []
  private _currentAbort: AbortController | null = null
  private _activity: AIActivity = {
    type: 'idle',
    target: null,
    progress: 0,
    startedAt: Date.now(),
    description: '',
  }

  constructor(opts: AICollaboratorOptions) {
    this.ws = opts.ws
    this.awareness = opts.ws.awareness
    this.identity = { ...opts.identity, role: opts.identity.role || 'coder' }
    this._llm = opts.llm
    this._autoReply = !!opts.autoReply
    this._replyTriggers = opts.replyTriggers || [/@ai\b/i, /@助手/]
    this._autoReview = !!opts.autoReview
    this._reviewProbability = opts.reviewProbability ?? 0.5
    this._systemPrompt =
      opts.systemPrompt ||
      `你是一名多用户协作的 AI 同事，名字叫 ${this.identity.name}，角色是 ${this.identity.role}。` +
      '请用简洁专业的语言回复，避免冗长寒暄。当看到 review 请求或代码补丁时，主动给出建设性意见。'

    this._announcePresence()
  }

  // ────── 生命周期 ──────

  start(): void {
    if (this._unsubs.length > 0) return // 已经启动

    // 监听所有 conversation 消息变化
    const _messagesHandler = (events: Array<{
      target: unknown
      changes: { keys?: Map<string, { action: string }> }
    }>) => {
      for (const ev of events) {
        // 只关心 messageIds 数组的 append
        if (
          ev.target === this.ws.messages ||
          (ev.target && (ev.target as { parent?: unknown }).parent === this.ws.messages)
        ) {
          // 新增 message
          if (ev.changes.keys) {
            for (const [k, change] of ev.changes.keys) {
              if (change.action === 'add') {
                this._handleNewMessage(k)
              }
            }
          }
        }
        // 顶层 conversations.add（新 conversation）
        if (ev.target === this.ws.conversations && ev.changes.keys) {
          for (const [k, change] of ev.changes.keys) {
            if (change.action === 'add') {
              this._handleNewConversation(k)
            }
          }
        }
      }
    }
    const unsubMessages: () => void = () => {
      this.ws.conversations.unobserveDeep(_messagesHandler)
    }
    this.ws.conversations.observeDeep(_messagesHandler)
    this._unsubs.push(unsubMessages)

    // 监听 codePatches（自动 review）
    if (this._autoReview) {
      const unsubPatches = this.ws.onCodePatchesChange(() => {
        // 不在 change handler 内 await（用 microtask）
        queueMicrotask(() => this._maybeReviewRecentPatch())
      })
      this._unsubs.push(unsubPatches)
    }
  }

  stop(): void {
    for (const u of this._unsubs) {
      try { u() } catch { /* ignore */ }
    }
    this._unsubs = []
    this._abortCurrent('collaborator stopped')
    this._setActivity({
      type: 'idle',
      target: null,
      progress: 0,
      startedAt: Date.now(),
      description: '',
    })
  }

  // ────── 主动调用入口（业务层用） ──────

  /**
   * 主动生成一条 AI 回复（不依赖 trigger 检测）。
   * 会把流式输出写进 Y.Doc，过程中持续更新 awareness。
   */
  async activeAssistant(opts: {
    conversationId: string
    parentMessageId: string | null
    /** 上下文（不含 system prompt） */
    history: { role: 'user' | 'assistant' | 'system'; content: string }[]
    /** AI 在这条消息中的角色（默认 'assistant'） */
    role?: MessageRole
  }): Promise<string> {
    const messageId = this._genId('m-')
    this._abortCurrent('starting new active assistant turn')
    this._currentAbort = new AbortController()

    // 1. 先放一个空 message 进去
    this.ws.appendMessage({
      id: messageId,
      conversationId: opts.conversationId,
      role: opts.role || 'ai-agent',
      content: '',
      authorId: this.identity.id,
      authorName: this.identity.name,
      parentId: opts.parentMessageId,
    })
    this.ws.setMessageStreaming(messageId, true)
    this._setActivity({
      type: 'streaming',
      target: { conversationId: opts.conversationId, messageId },
      progress: 0,
      startedAt: Date.now(),
      description: '正在生成回复…',
    })

    // 2. 调 LLM 流式
    const messages = [
      { role: 'system' as const, content: this._systemPrompt },
      ...opts.history,
    ]
    let buffer = ''
    try {
      const text = await this._llm({
        messages,
        signal: this._currentAbort.signal,
        onChunk: (chunk) => {
          buffer += chunk
          this.ws.appendTextToMessage(messageId, chunk)
        },
      })
      buffer = text || buffer
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e)
      this.ws.appendTextToMessage(messageId, `\n\n[AI 中断] ${err}`)
    } finally {
      this.ws.setMessageStreaming(messageId, false)
      this._setActivity({
        type: 'idle',
        target: null,
        progress: 0,
        startedAt: Date.now(),
        description: '',
      })
    }
    return buffer
  }

  /** 主动生成一个 codePatch 提议 */
  async proposeCodePatch(opts: {
    conversationId: string
    messageId: string
    filePath: string
    language: string
    content: string
    diff?: string
  }): Promise<string> {
    const id = this._genId('p-')
    this._setActivity({
      type: 'patching',
      target: {
        conversationId: opts.conversationId,
        messageId: opts.messageId,
        codePatchId: id,
      },
      progress: 50,
      startedAt: Date.now(),
      description: `正在写代码补丁：${opts.filePath}`,
    })
    this.ws.pushCodePatch({
      id,
      conversationId: opts.conversationId,
      messageId: opts.messageId,
      authorId: this.identity.id,
      filePath: opts.filePath,
      language: opts.language,
      content: opts.content,
      diff: opts.diff,
    })
    this._setActivity({
      type: 'idle',
      target: null,
      progress: 0,
      startedAt: Date.now(),
      description: '',
    })
    return id
  }

  /** 主动写一条 review */
  async writeReview(opts: {
    conversationId: string
    targetId: string
    targetType: 'message' | 'codePatch'
    content: string
  }): Promise<string> {
    const id = this._genId('r-')
    this._setActivity({
      type: 'reviewing',
      target: { conversationId: opts.conversationId, codePatchId: opts.targetId },
      progress: 100,
      startedAt: Date.now(),
      description: '正在写评审意见…',
    })
    this.ws.addReview({
      id,
      conversationId: opts.conversationId,
      targetId: opts.targetId,
      targetType: opts.targetType,
      authorId: this.identity.id,
      authorName: this.identity.name,
      content: opts.content,
    })
    this._setActivity({
      type: 'idle',
      target: null,
      progress: 0,
      startedAt: Date.now(),
      description: '',
    })
    return id
  }

  /** 移动 AI "光标"（注视某条消息） */
  setCursor(target: { conversationId: string; messageId: string | null } | null): void {
    this._setAwareness({
      user: { ...this.identity, isAI: true },
      activity: this._activity,
      cursor: target,
    })
  }

  /** 停止当前正在进行的活动 */
  abort(reason = 'aborted by user'): void {
    this._abortCurrent(reason)
  }

  // ────── 内部：自动触发 ──────

  private _handleNewMessage(messageId: string): void {
    if (!this._autoReply) return
    const m = this.ws.messages.get(messageId)
    if (!m) return
    const convId = m.get(Fields.conversationId) as string
    const content = ((m.get(Fields.content) as { toString: () => string } | undefined)?.toString?.() || '').toString()
    const authorId = m.get(Fields.authorId) as string

    // 不回复自己
    if (authorId === this.identity.id) return
    // 必须是 user 消息
    if (m.get(Fields.role) !== 'user') return
    // 检查 trigger
    if (!this._matchesTrigger(content)) return

    // 异步触发（不阻塞 Y.js 事件循环）
    queueMicrotask(() => {
      void this._autoReplyToMessage(convId, messageId, content)
    })
  }

  private _handleNewConversation(_conversationId: string): void {
    // 未来：AI 主动发起新对话（e.g. 总结）
  }

  private async _autoReplyToMessage(
    conversationId: string,
    parentMessageId: string,
    userContent: string,
  ): Promise<void> {
    // 拼装上下文：取最近 N 条消息
    const history = this.ws.listMessages(conversationId).slice(-10).map((m) => ({
      role: m.role === 'user' ? 'user' as const : 'assistant' as const,
      content: m.content,
    }))
    if (history.length === 0) {
      history.push({ role: 'user', content: userContent })
    }
    await this.activeAssistant({
      conversationId,
      parentMessageId,
      history,
    })
  }

  private async _maybeReviewRecentPatch(): Promise<void> {
    if (Math.random() > this._reviewProbability) return
    // 找最新的一条 proposed patch
    const patches = this.ws.codePatches.toArray() as Array<{
      get: (k: string) => unknown
    }>
    let latest: CodePatch | null = null
    for (const p of patches) {
      const status = p.get(Fields.status) as string
      if (status !== 'proposed') continue
      const createdAt = p.get(Fields.createdAt) as number
      if (!latest || createdAt > (latest as CodePatch).created_at) {
        latest = this._patchToPlain(p)
      }
    }
    if (!latest) return

    // 简单启发式：调用 LLM 写一句评审
    this._currentAbort = new AbortController()
    try {
      const verdict = await this._llm({
        messages: [
          { role: 'system', content: '你是资深代码评审员。请用 1-2 句中文给出简短评审意见（接受/拒绝 + 理由）。' },
          {
            role: 'user',
            content: `文件：${latest.file_path}\n语言：${latest.language}\n代码：\n${latest.content.slice(0, 800)}`,
          },
        ],
        signal: this._currentAbort.signal,
        onChunk: () => {},
      })
      await this.writeReview({
        conversationId: latest.conversation_id,
        targetId: latest.id,
        targetType: 'codePatch',
        content: verdict.trim(),
      })
    } catch {
      // 静默
    }
  }

  private _matchesTrigger(content: string): boolean {
    return this._replyTriggers.some((re) => re.test(content))
  }

  // ────── 内部：awareness 维护 ──────

  private _announcePresence(): void {
    this._setAwareness({
      user: { ...this.identity, isAI: true },
      activity: this._activity,
      cursor: null,
    })
  }

  private _setActivity(activity: AIActivity): void {
    this._activity = activity
    this._setAwareness({
      user: { ...this.identity, isAI: true },
      activity,
      cursor:
        activity.target && activity.target.messageId
          ? {
              conversationId: activity.target.conversationId,
              messageId: activity.target.messageId,
            }
          : null,
    })
  }

  private _setAwareness(state: AIAwarenessState): void {
    try {
      this.awareness.setLocalState(state as unknown as Record<string, unknown>)
    } catch {
      // 某些环境下 setLocalState 行为不同
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      ;(this.awareness as any).setLocalState?.(state)
    }
  }

  private _abortCurrent(reason: string): void {
    if (this._currentAbort) {
      try { this._currentAbort.abort(reason) } catch { /* ignore */ }
      this._currentAbort = null
    }
  }

  private _genId(prefix: string): string {
    const c = (globalThis as { crypto?: { randomUUID?: () => string } }).crypto
    if (c && c.randomUUID) return prefix + c.randomUUID()
    return prefix + Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
  }

  private _patchToPlain(p: { get: (k: string) => unknown }): CodePatch {
    const t = p.get(Fields.content) as { toString: () => string } | undefined
    return {
      id: p.get(Fields.id) as string,
      conversation_id: p.get(Fields.conversationId) as string,
      message_id: p.get(Fields.messageId) as string,
      author_id: p.get(Fields.authorId) as string,
      file_path: p.get(Fields.filePath) as string,
      language: (p.get(Fields.language) as string) || 'text',
      content: t ? t.toString() : '',
      diff: (p.get(Fields.diff) as string) || '',
      status: (p.get(Fields.status) as CodePatch['status']) || 'proposed',
      created_at: p.get(Fields.createdAt) as number,
      review_id: (p.get(Fields.reviewId) as string) || null,
    }
  }
}

// ──────────── 工厂：基于 WorkspaceDoc 创建 ────────────

export function createAICollaborator(
  ws: WorkspaceDoc,
  identity: AIIdentity,
  llm: LLMStreamFn,
  extra?: Partial<AICollaboratorOptions>,
): AICollaborator {
  return new AICollaborator({
    ws,
    identity,
    llm,
    ...extra,
  })
}
