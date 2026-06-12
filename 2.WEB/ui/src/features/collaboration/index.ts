/**
 * features/collaboration — 协作模块入口
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 */
export { default as CollaborationView } from './CollaborationView.vue'
export { default as AwarenessPresence } from './AwarenessPresence.vue'
export { default as WorkspaceSwitcher } from './WorkspaceSwitcher.vue'
export { default as LoginPanel } from './LoginPanel.vue'
export { default as ConversationList } from './ConversationList.vue'
export { default as SharedTaskBoard } from './SharedTaskBoard.vue'
export { default as BranchExplorer } from './BranchExplorer.vue'
export { default as MessageStream } from './MessageStream.vue'
export { default as AiActivityPanel } from './AiActivityPanel.vue'

export * from './types'
export { useWorkspaceDoc } from './composables/useWorkspaceDoc'
