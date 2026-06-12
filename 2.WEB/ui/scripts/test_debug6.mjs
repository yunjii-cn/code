import { parseTriggerAtCursor } from '../src/features/chat/composables/triggerParser.ts'
console.log(parseTriggerAtCursor('See @src/utils/index.ts', 23))
console.log(parseTriggerAtCursor('/review sec', 11))
console.log(parseTriggerAtCursor('a/@', 3))
console.log(parseTriggerAtCursor('@a/b/c/d', 8))
