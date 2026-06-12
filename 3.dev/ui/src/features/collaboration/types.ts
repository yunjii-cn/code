/**
 * collaboration 模块类型
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 与 composables/ 下的 Y.js 领域类型对应，但侧重 UI 渲染：
 *  - composables/WorkspaceDoc.ts 是 Y.Map 结构 + 操作
 *  - 这里 types 是给 Vue 组件用的扁平化、可序列化的视图模型
 */

import type {
  WorkspaceMeta,
  Conversation,
  Message,
  CodePatch,
  Review,
} from '@/composables/WorkspaceDoc'
import type { OnlineUser } from '@/stores/collaboration'

// 重新导出核心类型
export type { WorkspaceMeta, Conversation, Message, CodePatch, Review }
export type { OnlineUser }

// ──────────── 任务看板（共享任务 Y.Map） ────────────

export type SharedTaskStatus = 'todo' | 'in_progress' | 'blocked' | 'review' | 'done'

export interface SharedTask {
  id: string
  title: string
  description: string
  status: SharedTaskStatus
  assignee_id: string | null
  assignee_name: string | null
  created_by: string
  created_at: number
  updated_at: number
  due_at: number | null
  /** 关联 conversation id（点击可跳到对话） */
  conversation_id: string | null
  tags: string[]
}

// ──────────── 视图状态 ────────────

export interface CollaborationViewState {
  /** 当前选中的 conversation id */
  activeConversationId: string | null
  /** 侧边栏选中项 */
  sidebarTab: 'conversations' | 'branches' | 'tasks' | 'reviews' | 'members'
  /** 任务看板过滤 */
  taskFilter: {
    status: SharedTaskStatus | 'all'
    assignee: string | 'all'
    search: string
  }
  /** 是否显示 AI 活动面板 */
  showAIPanel: boolean
}

// ──────────── 分支 UI 节点 ────────────

export interface BranchNodeUI {
  conversation: Conversation
  isBranch: boolean
  parentId: string | null
  branches: BranchNodeUI[]
  /** 嵌套深度（用于缩进） */
  depth: number
}
