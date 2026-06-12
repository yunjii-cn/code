// features/chat/index.ts
// 2026-06-08 TASK-1.4 引入：feature-sliced 统一导出
// 2026-06-10 TASK-2.3 引入：Composer 组件导出
// 2026-06-10 TASK-2.4 引入：Autocomplete 组件 + triggerParser 导出

export { default } from './ChatView.vue'
export { default as ChatView } from './ChatView.vue'
export { default as Composer } from './components/Composer.vue'
export type { AttachedImage } from './components/Composer.vue'
export { default as Autocomplete } from './components/Autocomplete.vue'
export { default as DictationButton } from './components/DictationButton.vue'
export { parseTriggerAtCursor } from './composables/triggerParser'
export type { AutocompleteType, ParsedTrigger } from './composables/triggerParser'
