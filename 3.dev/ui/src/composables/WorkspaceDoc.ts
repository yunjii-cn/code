/**
 * WorkspaceDoc — Y.js 工作区领域模型
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 设计动机：
 *  Y.Doc 是一棵 Y.Map/Y.Array/Y.Text 嵌套树，直接操作很危险。
 *  这里把"工作区协作文档"封装成强类型的领域对象 + 操作助手，
 *  让业务层（AI Agent / 任务看板 / 分支管理）只看见领域类型，不看见 CRDT。
 *
 * Schema 概览（根 Y.Doc）：
 *  ├─ meta            : Y.Map<WorkspaceMeta>
 *  ├─ conversations   : Y.Map<conversationId, Y.Map<Conversation>>
 *  ├─ messages        : Y.Map<messageId, Y.Map<Message>>
 *  ├─ codePatches     : Y.Array<Y.Map<CodePatch>>     // 顺序敏感
 *  └─ reviews         : Y.Map<reviewId, Y.Map<Review>>
 *
 * Conversation 内部：
 *  - title: Y.Text（协同编辑标题）
 *  - messageIds: Y.Array<messageId>（消息顺序）
 *  - branches: Y.Map<branchId, Y.Map<BranchInfo>>（分支元信息）
 *  - headSnapshot: snapshotId（合并点，详见 branching-conversations.ts）
 *
 * 字段命名规范：
 *  - 复杂对象用 Y.Map（部分字段可独立 update）
 *  - 顺序敏感列表用 Y.Array
 *  - 大块文本用 Y.Text（避免全量重传）
 *  - 简单标量直接 set 到 Y.Map
 */

import * as Y from 'yjs'
import type { Awareness } from 'y-protocols/awareness'
import type { YjsClientHandle } from './yjs-client'

// ──────────── 领域类型（plain TS） ────────────

export interface WorkspaceMeta {
  workspace_id: string
  name: string
  schema_version: number
  created_at: number
  updated_at: number
  /** 工作区所有者 user id */
  owner_id: string
  /** 描述 */
  description: string
}

export type MessageRole = 'user' | 'assistant' | 'system' | 'ai-agent'

export interface Message {
  id: string
  conversation_id: string
  role: MessageRole
  /** 文本内容（用 Y.Text 存放） */
  content: string
  /** 父消息 id（用于线程/分支） */
  parent_id: string | null
  author_id: string
  author_name: string
  created_at: number
  /** 编辑/重生成次数 */
  edit_count: number
  /** 关联 codePatches（按顺序） */
  code_patch_ids: string[]
  /** 是否正在被 AI 流式写入（前端用 awareness 同步，这里存最终态） */
  is_streaming: boolean
  /** 该消息关联的 review id 列表 */
  review_ids: string[]
}

export interface Conversation {
  id: string
  /** 标题（用 Y.Text 存） */
  title: string
  created_at: number
  updated_at: number
  creator_id: string
  /** 关联任务看板中的任务 id（可空） */
  task_id: string | null
  /** 分支来源快照 */
  branch_of: string | null
  /** 已合并的源分支 id 列表 */
  merged_from: string[]
  /** 当前 head snapshot id */
  head_snapshot: string | null
  /** 是否归档 */
  archived: boolean
}

export interface CodePatch {
  id: string
  conversation_id: string
  message_id: string
  author_id: string
  file_path: string
  language: string
  /** patch 内容（用 Y.Text 存，支持协同） */
  content: string
  /** unified diff，可空 */
  diff: string
  status: 'proposed' | 'applied' | 'rejected' | 'reviewing'
  created_at: number
  /** 关联 review id */
  review_id: string | null
}

export interface Review {
  id: string
  conversation_id: string
  /** 被 review 的 message / codePatch id */
  target_id: string
  target_type: 'message' | 'codePatch'
  author_id: string
  author_name: string
  /** 评论内容（Y.Text） */
  content: string
  created_at: number
  status: 'open' | 'resolved' | 'wontfix'
  /** 线程父 review id（用于嵌套回复） */
  parent_id: string | null
}

// ──────────── 字段名常量（避免拼写错误） ────────────

export const Fields = {
  meta: 'meta',
  conversations: 'conversations',
  messages: 'messages',
  codePatches: 'codePatches',
  reviews: 'reviews',
  // 通用 ID 字段
  id: 'id',
  // conversation 内部
  title: 'title', // Y.Text
  messageIds: 'messageIds', // Y.Array
  branches: 'branches', // Y.Map
  headSnapshot: 'head_snapshot',
  // message 内部
  content: 'content', // Y.Text
  parentId: 'parent_id',
  authorId: 'author_id',
  authorName: 'author_name',
  createdAt: 'created_at',
  editCount: 'edit_count',
  codePatchIds: 'code_patch_ids',
  isStreaming: 'is_streaming',
  reviewIds: 'review_ids',
  conversationId: 'conversation_id',
  role: 'role',
  // code patch 内部
  filePath: 'file_path',
  language: 'language',
  diff: 'diff',
  status: 'status',
  targetId: 'target_id',
  targetType: 'target_type',
  messageId: 'message_id',
  /** 单数 reviewId（codePatch 关联的 review，最多 1 个） */
  reviewId: 'review_id',
  // meta 内部
  workspaceId: 'workspace_id',
  name: 'name',
  schemaVersion: 'schema_version',
  updatedAt: 'updated_at',
  creatorId: 'creator_id',
  ownerId: 'owner_id',
  description: 'description',
  branchOf: 'branch_of',
  mergedFrom: 'merged_from',
  archived: 'archived',
  taskId: 'task_id',
} as const

export const SCHEMA_VERSION = 1

// ──────────── WorkspaceDoc 类 ────────────

export class WorkspaceDoc {
  readonly doc: Y.Doc
  readonly awareness: Awareness
  readonly workspaceId: string

  readonly meta: Y.Map<unknown>
  readonly conversations: Y.Map<Y.Map<unknown>>
  readonly messages: Y.Map<Y.Map<unknown>>
  readonly codePatches: Y.Array<Y.Map<unknown>>
  readonly reviews: Y.Map<Y.Map<unknown>>

  constructor(doc: Y.Doc, awareness: Awareness, workspaceId: string) {
    this.doc = doc
    this.awareness = awareness
    this.workspaceId = workspaceId

    this.meta = doc.getMap(Fields.meta)
    this.conversations = doc.getMap(Fields.conversations) as Y.Map<Y.Map<unknown>>
    this.messages = doc.getMap(Fields.messages) as Y.Map<Y.Map<unknown>>
    this.codePatches = doc.getArray(Fields.codePatches) as Y.Array<Y.Map<unknown>>
    this.reviews = doc.getMap(Fields.reviews) as Y.Map<Y.Map<unknown>>
  }

  // ────── Meta ──────

  ensureMeta(ownerId: string, name: string, description = ''): void {
    if (this.meta.size === 0) {
      this.doc.transact(() => {
        const now = Date.now()
        this.meta.set(Fields.workspaceId, this.workspaceId)
        this.meta.set(Fields.name, name)
        this.meta.set(Fields.description, description)
        this.meta.set(Fields.ownerId, ownerId)
        this.meta.set(Fields.schemaVersion, SCHEMA_VERSION)
        this.meta.set(Fields.createdAt, now)
        this.meta.set(Fields.updatedAt, now)
      }, 'workspace-init')
    } else {
      this.touch()
    }
  }

  getMeta(): WorkspaceMeta | null {
    if (this.meta.size === 0) return null
    return {
      workspace_id: this.meta.get(Fields.workspaceId) as string,
      name: this.meta.get(Fields.name) as string,
      description: (this.meta.get(Fields.description) as string) || '',
      owner_id: this.meta.get(Fields.ownerId) as string,
      schema_version: this.meta.get(Fields.schemaVersion) as number,
      created_at: this.meta.get(Fields.createdAt) as number,
      updated_at: this.meta.get(Fields.updatedAt) as number,
    }
  }

  setName(name: string): void {
    this.doc.transact(() => {
      this.meta.set(Fields.name, name)
      this.touch()
    }, 'set-name')
  }

  setDescription(desc: string): void {
    this.doc.transact(() => {
      this.meta.set(Fields.description, desc)
      this.touch()
    }, 'set-description')
  }

  touch(): void {
    this.meta.set(Fields.updatedAt, Date.now())
  }

  // ────── Conversation ──────

  createConversation(opts: {
    id: string
    title: string
    creatorId: string
    taskId?: string | null
    branchOf?: string | null
  }): Y.Map<unknown> {
    let result: Y.Map<unknown> | null = null
    this.doc.transact(() => {
      const conv = new Y.Map<unknown>()
      const now = Date.now()
      const titleText = new Y.Text()
      titleText.insert(0, opts.title)
      conv.set(Fields.id, opts.id)
      conv.set(Fields.title, titleText)
      conv.set(Fields.messageIds, new Y.Array<string>())
      conv.set(Fields.branches, new Y.Map())
      conv.set(Fields.createdAt, now)
      conv.set(Fields.updatedAt, now)
      conv.set(Fields.creatorId, opts.creatorId)
      conv.set(Fields.taskId, opts.taskId ?? null)
      conv.set(Fields.branchOf, opts.branchOf ?? null)
      conv.set(Fields.mergedFrom, [] as string[])
      conv.set(Fields.headSnapshot, null)
      conv.set(Fields.archived, false)
      this.conversations.set(opts.id, conv)
      this.touch()
      result = conv
    }, 'create-conversation')
    return result!
  }

  getConversation(id: string): Y.Map<unknown> | null {
    return this.conversations.get(id) || null
  }

  listConversations(opts: { includeArchived?: boolean } = {}): Conversation[] {
    const result: Conversation[] = []
    this.conversations.forEach((conv) => {
      const archived = conv.get(Fields.archived) === true
      if (!opts.includeArchived && archived) return
      result.push(_convToPlain(conv))
    })
    // 按 updatedAt 降序
    result.sort((a, b) => b.updated_at - a.updated_at)
    return result
  }

  setConversationTitle(id: string, newTitle: string): void {
    const conv = this.conversations.get(id)
    if (!conv) return
    this.doc.transact(() => {
      const titleText = conv.get(Fields.title) as Y.Text
      titleText.delete(0, titleText.length)
      titleText.insert(0, newTitle)
      conv.set(Fields.updatedAt, Date.now())
      this.touch()
    }, 'set-conv-title')
  }

  archiveConversation(id: string, archived = true): void {
    const conv = this.conversations.get(id)
    if (!conv) return
    this.doc.transact(() => {
      conv.set(Fields.archived, archived)
      conv.set(Fields.updatedAt, Date.now())
      this.touch()
    }, 'archive-conv')
  }

  // ────── Message ──────

  appendMessage(opts: {
    id: string
    conversationId: string
    role: MessageRole
    content: string
    authorId: string
    authorName: string
    parentId?: string | null
  }): Y.Map<unknown> | null {
    const conv = this.conversations.get(opts.conversationId)
    if (!conv) return null
    let msg: Y.Map<unknown> | null = null
    this.doc.transact(() => {
      const m = new Y.Map<unknown>()
      const text = new Y.Text()
      text.insert(0, opts.content)
      const now = Date.now()
      m.set(Fields.id, opts.id)
      m.set(Fields.conversationId, opts.conversationId)
      m.set(Fields.role, opts.role)
      m.set(Fields.content, text)
      m.set(Fields.parentId, opts.parentId ?? null)
      m.set(Fields.authorId, opts.authorId)
      m.set(Fields.authorName, opts.authorName)
      m.set(Fields.createdAt, now)
      m.set(Fields.editCount, 0)
      m.set(Fields.codePatchIds, [] as string[])
      m.set(Fields.isStreaming, false)
      m.set(Fields.reviewIds, [] as string[])
      this.messages.set(opts.id, m)
      const ids = conv.get(Fields.messageIds) as Y.Array<string>
      ids.push([opts.id])
      conv.set(Fields.updatedAt, now)
      this.touch()
      msg = m
    }, 'append-message')
    return msg
  }

  appendTextToMessage(id: string, text: string): void {
    const m = this.messages.get(id)
    if (!m) return
    this.doc.transact(() => {
      const t = m.get(Fields.content) as Y.Text
      t.insert(t.length, text)
    }, 'append-msg-text')
  }

  setMessageStreaming(id: string, streaming: boolean): void {
    const m = this.messages.get(id)
    if (!m) return
    this.doc.transact(() => {
      m.set(Fields.isStreaming, streaming)
    }, 'set-msg-streaming')
  }

  listMessages(conversationId: string): Message[] {
    const conv = this.conversations.get(conversationId)
    if (!conv) return []
    const ids = conv.get(Fields.messageIds) as Y.Array<string>
    const out: Message[] = []
    ids.forEach((mid) => {
      const m = this.messages.get(mid)
      if (m) out.push(_msgToPlain(m))
    })
    return out
  }

  // ────── CodePatch ──────

  pushCodePatch(opts: {
    id: string
    conversationId: string
    messageId: string
    authorId: string
    filePath: string
    language: string
    content: string
    diff?: string
  }): Y.Map<unknown> | null {
    const m = this.messages.get(opts.messageId)
    if (!m) return null
    let patch: Y.Map<unknown> | null = null
    this.doc.transact(() => {
      const p = new Y.Map<unknown>()
      const text = new Y.Text()
      text.insert(0, opts.content)
      p.set(Fields.id, opts.id)
      p.set(Fields.conversationId, opts.conversationId)
      p.set(Fields.messageId, opts.messageId)
      p.set(Fields.authorId, opts.authorId)
      p.set(Fields.filePath, opts.filePath)
      p.set(Fields.language, opts.language)
      p.set(Fields.content, text)
      p.set(Fields.diff, opts.diff ?? '')
      p.set(Fields.status, 'proposed')
      p.set(Fields.createdAt, Date.now())
      p.set(Fields.reviewId, null)
      this.codePatches.push([p])
      const patchIds = m.get(Fields.codePatchIds) as string[]
      m.set(Fields.codePatchIds, [...patchIds, opts.id])
      patch = p
    }, 'push-code-patch')
    return patch
  }

  setCodePatchStatus(id: string, status: CodePatch['status']): void {
    this.codePatches.forEach((p) => {
      if (p.get(Fields.id) === id) {
        this.doc.transact(() => {
          p.set(Fields.status, status)
        }, 'set-patch-status')
      }
    })
  }

  listCodePatches(conversationId: string): CodePatch[] {
    const out: CodePatch[] = []
    this.codePatches.forEach((p) => {
      if (p.get(Fields.conversationId) === conversationId) {
        out.push(_patchToPlain(p))
      }
    })
    return out
  }

  // ────── Review ──────

  addReview(opts: {
    id: string
    conversationId: string
    targetId: string
    targetType: Review['target_type']
    authorId: string
    authorName: string
    content: string
    parentId?: string | null
  }): Y.Map<unknown> {
    let rv: Y.Map<unknown> | null = null
    this.doc.transact(() => {
      const r = new Y.Map<unknown>()
      const text = new Y.Text()
      text.insert(0, opts.content)
      r.set(Fields.id, opts.id)
      r.set(Fields.conversationId, opts.conversationId)
      r.set(Fields.targetId, opts.targetId)
      r.set(Fields.targetType, opts.targetType)
      r.set(Fields.authorId, opts.authorId)
      r.set(Fields.authorName, opts.authorName)
      r.set(Fields.content, text)
      r.set(Fields.createdAt, Date.now())
      r.set(Fields.status, 'open')
      r.set(Fields.parentId, opts.parentId ?? null)
      this.reviews.set(opts.id, r)

      // 关联到目标对象
      if (opts.targetType === 'message') {
        const m = this.messages.get(opts.targetId)
        if (m) {
          const list = (m.get(Fields.reviewIds) as string[]) || []
          m.set(Fields.reviewIds, [...list, opts.id])
        }
      } else {
        this.codePatches.forEach((p) => {
          if (p.get(Fields.id) === opts.targetId) {
            p.set(Fields.reviewId, opts.id)
          }
        })
      }
      rv = r
    }, 'add-review')
    return rv!
  }

  setReviewStatus(id: string, status: Review['status']): void {
    const r = this.reviews.get(id)
    if (!r) return
    this.doc.transact(() => {
      r.set(Fields.status, status)
    }, 'set-review-status')
  }

  // ────── 观察者（订阅） ──────

  /** 监听 conversations 列表变化（新增/删除/归档） */
  onConversationsChange(cb: () => void): () => void {
    const handler = () => cb()
    this.conversations.observeDeep(handler)
    return () => this.conversations.unobserveDeep(handler)
  }

  /** 监听某条对话的消息列表变化（追加/编辑/删除） */
  onMessagesChange(conversationId: string, cb: () => void): () => void {
    const conv = this.conversations.get(conversationId)
    if (!conv) return () => {}
    const handler = () => cb()
    conv.observeDeep(handler)
    return () => conv.unobserveDeep(handler)
  }

  /** 监听 meta 变化（名称/描述） */
  onMetaChange(cb: () => void): () => void {
    const handler = () => cb()
    this.meta.observe(handler)
    return () => this.meta.unobserve(handler)
  }

  /** 监听 codePatches 变化 */
  onCodePatchesChange(cb: () => void): () => void {
    const handler = () => cb()
    this.codePatches.observeDeep(handler)
    return () => this.codePatches.unobserveDeep(handler)
  }

  /** 监听 reviews 变化 */
  onReviewsChange(cb: () => void): () => void {
    const handler = () => cb()
    this.reviews.observeDeep(handler)
    return () => this.reviews.unobserveDeep(handler)
  }
}

// ──────────── 内部：Y.Map → plain ────────────

function _convToPlain(m: Y.Map<unknown>): Conversation {
  const titleText = m.get(Fields.title) as Y.Text
  return {
    id: m.get(Fields.id) as string,
    title: titleText ? titleText.toString() : '',
    created_at: m.get(Fields.createdAt) as number,
    updated_at: m.get(Fields.updatedAt) as number,
    creator_id: m.get(Fields.creatorId) as string,
    task_id: (m.get(Fields.taskId) as string) || null,
    branch_of: (m.get(Fields.branchOf) as string) || null,
    merged_from: (m.get(Fields.mergedFrom) as string[]) || [],
    head_snapshot: (m.get(Fields.headSnapshot) as string) || null,
    archived: m.get(Fields.archived) === true,
  }
}

function _msgToPlain(m: Y.Map<unknown>): Message {
  const t = m.get(Fields.content) as Y.Text
  return {
    id: m.get(Fields.id) as string,
    conversation_id: m.get(Fields.conversationId) as string,
    role: m.get(Fields.role) as MessageRole,
    content: t ? t.toString() : '',
    parent_id: (m.get(Fields.parentId) as string) || null,
    author_id: m.get(Fields.authorId) as string,
    author_name: m.get(Fields.authorName) as string,
    created_at: m.get(Fields.createdAt) as number,
    edit_count: (m.get(Fields.editCount) as number) || 0,
    code_patch_ids: (m.get(Fields.codePatchIds) as string[]) || [],
    is_streaming: m.get(Fields.isStreaming) === true,
    review_ids: (m.get(Fields.reviewIds) as string[]) || [],
  }
}

function _patchToPlain(p: Y.Map<unknown>): CodePatch {
  const t = p.get(Fields.content) as Y.Text
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

// ──────────── 工厂：基于 yjs-client 句柄创建 WorkspaceDoc ────────────

export function createWorkspaceDoc(client: YjsClientHandle): WorkspaceDoc {
  return new WorkspaceDoc(client.doc, client.awareness, client.workspaceId)
}
