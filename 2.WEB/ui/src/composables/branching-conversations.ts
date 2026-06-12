/**
 * branching-conversations — 对话分支/合并
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 模型说明：
 *  - "分支" = 在某条对话的某个状态点 fork 出新对话，两边可独立演化
 *  - "合并" = 把分支的差异（branchOf 之后的新消息/codePatches/reviews）应用到主干
 *  - 实现基于 Y.js Y.snapshot（不可变状态切点），用 base64 字符串作为快照 ID
 *
 * 关键不变量：
 *  - 分支点（branchOf）记录的是父对话创建分支那一刻的 Y.snapshot
 *  - 合并后，目标对话的 head_snapshot 推进到新位置
 *  - 合并不删除源分支，只在目标的 mergedFrom 数组里追加 sourceId
 *  - 同一对话链可以反复合并/分支（A→B→A→C 都合法）
 */

import * as Y from 'yjs'
import type { WorkspaceDoc, Message, CodePatch, Review } from './WorkspaceDoc'
import { Fields } from './WorkspaceDoc'

// ──────────── 类型 ────────────

export interface BranchInfo {
  /** 分支 id（= 目标 conversation id） */
  id: string
  /** 父 conversation id（被 fork 的那条） */
  parent_id: string
  /** 父端的 fork 点 Y.snapshot 编码 */
  fork_snapshot: string
  /** 父端 fork 时的 messageIds 拷贝 */
  fork_message_ids: string[]
  /** 分支创建时间 */
  created_at: number
  /** 创建者 */
  creator_id: string
  /** 分支名（用户可命名） */
  label: string
  /** 分支是否已被合并到主干 */
  merged: boolean
  /** 合并到主干的 conversation id（== parent_id） */
  merged_into: string | null
}

export interface DiffSummary {
  added_messages: Message[]
  added_code_patches: CodePatch[]
  added_reviews: Review[]
}

export interface CreateBranchOptions {
  /** 父 conversation id */
  parentConversationId: string
  /** 新分支 id（调用方生成，UUID/ULID） */
  newConversationId: string
  /** 新分支标题 */
  title: string
  /** 创建者 */
  creatorId: string
  /** 新分支所属任务 id（可继承父） */
  taskId?: string | null
  /** 分支标签（"试一下 GPT-5"、"做单元测试"） */
  label?: string
  /** 在父对话的哪个 messageId 之后 fork；不传则 fork 整条对话 */
  forkAtMessageId?: string
}

export interface MergeOptions {
  /** 源（被合并的）conversation id */
  sourceConversationId: string
  /** 目标 conversation id（= 主干） */
  targetConversationId: string
  /** 合并者 */
  mergerId: string
  /** 合并后是否归档源 */
  archiveSource?: boolean
  /** 合并时跳过某些 codePatches（review 没通过） */
  skipCodePatchIds?: string[]
}

export interface CreateBranchResult {
  branchInfo: BranchInfo
  newConversationId: string
  forkedMessages: Message[]
}

export interface MergeResult {
  appliedMessages: number
  appliedCodePatches: number
  appliedReviews: number
  skippedCodePatchIds: string[]
  conflictWarnings: string[]
}

// ──────────── 工具：snapshot 编码/解码 ────────────

/**
 * Y.snapshot 是一个 Y.Snapshot 实例（不可直接序列化）。
 * 我们用 Y.encodeSnapshot 转 Uint8Array，再 base64 存到 Y.Map。
 */
function encodeSnapshot(snap: Y.Snapshot): string {
  const bytes = Y.encodeSnapshot(snap)
  return _uint8ToBase64(bytes)
}

function decodeSnapshot(b64: string): Y.Snapshot {
  const bytes = _base64ToUint8(b64)
  return Y.decodeSnapshot(bytes)
}

function _uint8ToBase64(bytes: Uint8Array): string {
  // 浏览器环境（btoa 是全局）
  if (typeof btoa === 'function') {
    let bin = ''
    for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i])
    return btoa(bin)
  }
  // Node/非浏览器：构造二进制字符串再走 btoa 等价
  // 注：纯前端项目，理论不会走到这里；保留兜底
  let bin = ''
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i])
  return bin
}

function _base64ToUint8(b64: string): Uint8Array {
  if (typeof atob === 'function') {
    const bin = atob(b64)
    const out = new Uint8Array(bin.length)
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i)
    return out
  }
  // 非浏览器：手写 base64 解码
  // 这里简化兜底（前端项目不会用）
  return new Uint8Array(0)
}

function _genId(prefix = ''): string {
  // 简单 ID 生成：crypto.randomUUID 优先，回退 timestamp
  const c = (globalThis as { crypto?: { randomUUID?: () => string } }).crypto
  if (c && c.randomUUID) return prefix + c.randomUUID()
  return prefix + Date.now().toString(36) + Math.random().toString(36).slice(2, 10)
}

// ──────────── 创建分支 ────────────

/**
 * 在 parentConversationId 的当前（或指定 message）状态下 fork 一条新对话。
 *
 * 行为：
 *  1. 取父端 messageIds 列表（可选截断到 forkAtMessageId）
 *  2. 创建新 conversation，messageIds 初始为空
 *  3. 把截断的 messageIds 复制进新 conversation（保持 ID 引用，CRDT 共享消息体）
 *  4. 在新 conversation 的 branches.Y.Map 中登记 BranchInfo
 *  5. 父 conversation 暂不动（branchOf 由分支侧自己记）
 */
export function createBranch(ws: WorkspaceDoc, opts: CreateBranchOptions): CreateBranchResult {
  const parent = ws.getConversation(opts.parentConversationId)
  if (!parent) {
    throw new Error(`[branch] parent conversation not found: ${opts.parentConversationId}`)
  }

  // 1. 父端 messageIds
  const parentIds = (parent.get(Fields.messageIds) as Y.Array<string>).toArray()
  let forkAtIdx = parentIds.length
  if (opts.forkAtMessageId) {
    const idx = parentIds.indexOf(opts.forkAtMessageId)
    if (idx === -1) {
      throw new Error(`[branch] forkAtMessageId not in parent: ${opts.forkAtMessageId}`)
    }
    forkAtIdx = idx + 1 // 含这条
  }
  const forkedIds = parentIds.slice(0, forkAtIdx)

  // 2. 取当前 doc 快照（用于记录 fork 点）
  const snap = Y.snapshot(ws.doc)
  const snapB64 = encodeSnapshot(snap)

  // 3. 创建新 conversation
  const newConv = ws.createConversation({
    id: opts.newConversationId,
    title: opts.title,
    creatorId: opts.creatorId,
    taskId: opts.taskId ?? ((parent.get(Fields.taskId) as string) || null),
    branchOf: snapB64,
  })

  // 4. 把 fork 点的消息 ID 写入新对话
  //    直接复用 messages 中的同一 Y.Map（CRDT 多引用安全）
  ws.doc.transact(() => {
    const ids = newConv.get(Fields.messageIds) as Y.Array<string>
    if (forkedIds.length > 0) ids.push(forkedIds)
  }, 'branch-copy-messages')

  // 5. 登记 BranchInfo
  const branchInfo: BranchInfo = {
    id: opts.newConversationId,
    parent_id: opts.parentConversationId,
    fork_snapshot: snapB64,
    fork_message_ids: forkedIds,
    created_at: Date.now(),
    creator_id: opts.creatorId,
    label: opts.label || `分支 @ ${new Date().toLocaleString()}`,
    merged: false,
    merged_into: null,
  }
  ws.doc.transact(() => {
    const branches = newConv.get(Fields.branches) as Y.Map<Y.Map<unknown>>
    const self = new Y.Map<unknown>()
    self.set('id', branchInfo.id)
    self.set('parent_id', branchInfo.parent_id)
    self.set('fork_snapshot', branchInfo.fork_snapshot)
    self.set('fork_message_ids', branchInfo.fork_message_ids)
    self.set('created_at', branchInfo.created_at)
    self.set('creator_id', branchInfo.creator_id)
    self.set('label', branchInfo.label)
    self.set('merged', false)
    self.set('merged_into', null)
    branches.set(branchInfo.id, self)
  }, 'branch-register')

  // 返回结果
  const forkedMessages = forkedIds
    .map((mid) => ws.messages.get(mid))
    .filter((m): m is Y.Map<unknown> => !!m)
    .map((m) => _msgToPlain(m))

  return { branchInfo, newConversationId: opts.newConversationId, forkedMessages }
}

// ──────────── 合并分支 ────────────

/**
 * 把 sourceConversationId 自 fork 之后的新内容合并到 targetConversationId。
 *
 * 行为：
 *  1. 计算 source 相对 fork 点的差异（新增 messageIds、codePatches、reviews）
 *  2. 排除 target 中已存在的 messageId（避免重复追加）
 *  3. 把新增的 message/codePatch/review 追加到 target（按时间顺序）
 *  4. 更新 target.head_snapshot = 当前源快照
 *  5. 标记 BranchInfo.merged = true，merged_into = targetId
 *  6. 可选：归档 source
 */
export function mergeBranch(ws: WorkspaceDoc, opts: MergeOptions): MergeResult {
  const source = ws.getConversation(opts.sourceConversationId)
  const target = ws.getConversation(opts.targetConversationId)
  if (!source) throw new Error(`[merge] source not found: ${opts.sourceConversationId}`)
  if (!target) throw new Error(`[merge] target not found: ${opts.targetConversationId}`)

  // 1. 差异
  const diff = diffBranch(ws, opts.sourceConversationId)
  const skipSet = new Set(opts.skipCodePatchIds || [])

  // 2. target 已有的 messageId（去重）
  const targetIds = new Set((target.get(Fields.messageIds) as Y.Array<string>).toArray())

  // 3. 过滤要追加的 messages（按 createdAt 排序）
  const toAppend = diff.added_messages
    .filter((m) => !targetIds.has(m.id))
    .sort((a, b) => a.created_at - b.created_at)

  const targetMessageIds = target.get(Fields.messageIds) as Y.Array<string>
  const targetCodePatches: { id: string; conversationId: string; messageId: string; authorId: string; filePath: string; language: string; content: string; diff?: string }[] = []
  let appliedMessages = 0
  let appliedCodePatches = 0
  const conflictWarnings: string[] = []

  ws.doc.transact(() => {
    for (const m of toAppend) {
      // 重新构造（避免跨 conversation_id 引用错乱）
      ws.appendMessage({
        id: _genId('m-'),
        conversationId: opts.targetConversationId,
        role: m.role,
        content: m.content,
        authorId: m.author_id,
        authorName: m.author_name,
        parentId: m.parent_id,
      })
      appliedMessages++
    }
  }, 'merge-messages')

  // 4. codePatches（按 createdAt 顺序；按 target conversation 重新登记）
  const patchesToApply = diff.added_code_patches
    .filter((p) => !skipSet.has(p.id))
    .sort((a, b) => a.created_at - b.created_at)

  ws.doc.transact(() => {
    for (const p of patchesToApply) {
      // 找到合并到 target 后对应的新 messageId（按原 message_id 的 createdAt 找到最接近的）
      const targetMsgId = _findMessageByOriginalAuthor(
        ws,
        opts.targetConversationId,
        p.message_id,
        p.author_id,
      )
      const finalMsgId = targetMsgId || _genId('m-')
      if (!targetMsgId) {
        // 找不到对应 message（合并时原 message 被裁掉了）—— 创建一个 placeholder
        ws.appendMessage({
          id: finalMsgId,
          conversationId: opts.targetConversationId,
          role: 'assistant',
          content: `[合并自 ${opts.sourceConversationId}] ${p.file_path} 的代码补丁`,
          authorId: p.author_id,
          authorName: 'merge-bot',
        })
        appliedMessages++
        conflictWarnings.push(
          `CodePatch ${p.id} 找不到对应 message（${p.message_id}），已创建占位消息`,
        )
      }
      ws.pushCodePatch({
        id: _genId('p-'),
        conversationId: opts.targetConversationId,
        messageId: finalMsgId,
        authorId: p.author_id,
        filePath: p.file_path,
        language: p.language,
        content: p.content,
        diff: p.diff,
      })
      targetCodePatches.push({ id: p.id, conversationId: opts.targetConversationId, messageId: finalMsgId, authorId: p.author_id, filePath: p.file_path, language: p.language, content: p.content, diff: p.diff })
      appliedCodePatches++
    }
  }, 'merge-patches')

  // 5. reviews
  const reviewsToApply = diff.added_reviews.sort((a, b) => a.created_at - b.created_at)
  let appliedReviews = 0
  ws.doc.transact(() => {
    for (const r of reviewsToApply) {
      ws.addReview({
        id: _genId('r-'),
        conversationId: opts.targetConversationId,
        targetId: r.target_id,
        targetType: r.target_type,
        authorId: r.author_id,
        authorName: r.author_name,
        content: r.content,
        parentId: r.parent_id,
      })
      appliedReviews++
    }
  }, 'merge-reviews')

  // 6. 更新 head_snapshot + BranchInfo
  const newSnap = Y.snapshot(ws.doc)
  const newSnapB64 = encodeSnapshot(newSnap)
  ws.doc.transact(() => {
    target.set(Fields.headSnapshot, newSnapB64)
    target.set(Fields.updatedAt, Date.now())
    const targetMergedFrom = (target.get(Fields.mergedFrom) as string[]) || []
    if (!targetMergedFrom.includes(opts.sourceConversationId)) {
      target.set(Fields.mergedFrom, [...targetMergedFrom, opts.sourceConversationId])
    }
    // 标记源端的 BranchInfo
    const sourceBranches = source.get(Fields.branches) as Y.Map<Y.Map<unknown>>
    const selfBranch = sourceBranches.get(opts.sourceConversationId)
    if (selfBranch) {
      selfBranch.set('merged', true)
      selfBranch.set('merged_into', opts.targetConversationId)
    }
  }, 'merge-meta')

  // 7. 归档源（可选）
  if (opts.archiveSource) {
    ws.archiveConversation(opts.sourceConversationId, true)
  }

  return {
    appliedMessages,
    appliedCodePatches,
    appliedReviews,
    skippedCodePatchIds: [...skipSet],
    conflictWarnings,
  }
}

// ──────────── 差异计算 ────────────

/**
 * 计算 source 相对其 fork 点的差异。
 * 如果 source 没有 fork 记录（旧数据 / 直接创建），则返回整条对话。
 */
export function diffBranch(ws: WorkspaceDoc, sourceConversationId: string): DiffSummary {
  const source = ws.getConversation(sourceConversationId)
  if (!source) {
    return { added_messages: [], added_code_patches: [], added_reviews: [] }
  }

  const allMessages = ws.listMessages(sourceConversationId)
  const allPatches = ws.listCodePatches(sourceConversationId)

  // 找到 fork 点消息 ID
  const branchOf = source.get(Fields.branchOf) as string | null
  let forkPointMsgCount = 0
  if (branchOf) {
    // branchOf 是 base64 snapshot，无法直接解码为消息计数
    // 用"被复制的 fork_message_ids 长度"近似（创建分支时记录在 BranchInfo.fork_message_ids）
    const branches = source.get(Fields.branches) as Y.Map<Y.Map<unknown>>
    const self = branches.get(sourceConversationId)
    if (self) {
      const forkIds = (self.get('fork_message_ids') as string[]) || []
      forkPointMsgCount = forkIds.length
    }
  }

  const addedMessages = allMessages.slice(forkPointMsgCount)
  const forkPointMsgIds = new Set(allMessages.slice(0, forkPointMsgCount).map((m) => m.id))
  const addedPatches = allPatches.filter((p) => !forkPointMsgIds.has(p.message_id))

  // reviews：拿全部（review 不会因为分支丢失）
  const addedReviews: Review[] = []
  ws.reviews.forEach((rv) => {
    if (rv.get(Fields.conversationId) === sourceConversationId) {
      const t = rv.get(Fields.content) as Y.Text
      addedReviews.push({
        id: rv.get(Fields.id) as string,
        conversation_id: sourceConversationId,
        target_id: rv.get(Fields.targetId) as string,
        target_type: rv.get(Fields.targetType) as Review['target_type'],
        author_id: rv.get(Fields.authorId) as string,
        author_name: rv.get(Fields.authorName) as string,
        content: t ? t.toString() : '',
        created_at: rv.get(Fields.createdAt) as number,
        status: (rv.get(Fields.status) as Review['status']) || 'open',
        parent_id: (rv.get(Fields.parentId) as string) || null,
      })
    }
  })

  return {
    added_messages: addedMessages,
    added_code_patches: addedPatches,
    added_reviews: addedReviews,
  }
}

// ──────────── 列出分支 ────────────

/**
 * 列出 conversation 的所有分支（仅在 conversation 是主干时调用）。
 * 判断"主干"：没有 branchOf（自己不是被 fork 出来的）。
 */
export function listBranches(ws: WorkspaceDoc, rootConversationId: string): BranchInfo[] {
  const conv = ws.getConversation(rootConversationId)
  if (!conv) return []
  const result: BranchInfo[] = []
  ws.conversations.forEach((c) => {
    const branches = c.get(Fields.branches) as Y.Map<Y.Map<unknown>>
    branches.forEach((b) => {
      if (b.get('parent_id') === rootConversationId) {
        result.push({
          id: b.get('id') as string,
          parent_id: b.get('parent_id') as string,
          fork_snapshot: b.get('fork_snapshot') as string,
          fork_message_ids: (b.get('fork_message_ids') as string[]) || [],
          created_at: b.get('created_at') as number,
          creator_id: b.get('creator_id') as string,
          label: (b.get('label') as string) || '',
          merged: b.get('merged') === true,
          merged_into: (b.get('merged_into') as string) || null,
        })
      }
    })
  })
  return result
}

/** 判断某条对话是否是分支 */
export function isBranch(ws: WorkspaceDoc, conversationId: string): boolean {
  const conv = ws.getConversation(conversationId)
  if (!conv) return false
  return !!conv.get(Fields.branchOf)
}

/** 获取某条对话的父主干 id（如果不是分支则返回 null） */
export function getParentId(ws: WorkspaceDoc, branchConversationId: string): string | null {
  const conv = ws.getConversation(branchConversationId)
  if (!conv) return null
  // BranchInfo 在自己 conversation.branches 里
  const branches = conv.get(Fields.branches) as Y.Map<Y.Map<unknown>>
  const self = branches.get(branchConversationId)
  if (self) {
    return (self.get('parent_id') as string) || null
  }
  return null
}

// ──────────── 内部 helper ────────────

function _msgToPlain(m: Y.Map<unknown>): Message {
  const t = m.get(Fields.content) as Y.Text
  return {
    id: m.get(Fields.id) as string,
    conversation_id: m.get(Fields.conversationId) as string,
    role: m.get(Fields.role) as Message['role'],
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

function _findMessageByOriginalAuthor(
  ws: WorkspaceDoc,
  targetConvId: string,
  _originalMsgId: string,
  authorId: string,
): string | null {
  // 简化实现：返回 target 中按时间最近、authorId 匹配的最新 message
  // 真实场景需要更精细的"重映射"策略，先用启发式
  const msgs = ws.listMessages(targetConvId)
  if (msgs.length === 0) return null
  for (let i = msgs.length - 1; i >= 0; i--) {
    if (msgs[i].author_id === authorId) return msgs[i].id
  }
  return msgs[msgs.length - 1].id
}
