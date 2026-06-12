import { parseTriggerAtCursor } from '../src/features/chat/composables/triggerParser.ts'
const text = '@src'
const cursor = 4
console.log('text len:', text.length, 'cursor:', cursor)
const r = parseTriggerAtCursor(text, cursor)
console.log('result:', r)
