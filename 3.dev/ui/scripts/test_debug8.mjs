import { parseTriggerAtCursor } from '../src/features/chat/composables/triggerParser.ts'
console.log(parseTriggerAtCursor('@a/b/c/d', 8))
console.log(parseTriggerAtCursor('See @src/utils/index.ts', 23))
console.log(parseTriggerAtCursor('/review sec', 11))
