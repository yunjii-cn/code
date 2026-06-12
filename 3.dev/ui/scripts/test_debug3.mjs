import { parseTriggerAtCursor } from '../src/features/chat/composables/triggerParser.ts'
const text = '@src'
const cursor = 4
console.log('cursor <= 0:', cursor <= 0, 'cursor > text.length:', cursor > text.length)
console.log('text[cursor-1]:', text[cursor - 1])
const r = parseTriggerAtCursor(text, cursor)
console.log('result:', r)
