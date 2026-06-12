import { parseTriggerAtCursor } from '../src/features/chat/composables/triggerParser.ts'
const text = 'See @src/utils/index.ts'
const cursor = 23
console.log('text length:', text.length, 'cursor:', cursor)
console.log('text[5]:', JSON.stringify(text[5]))
console.log('text[4]:', JSON.stringify(text[4]))
const r = parseTriggerAtCursor(text, cursor)
console.log('result:', r)

// 简化 case
console.log('---')
console.log(parseTriggerAtCursor('@src', 4))
console.log(parseTriggerAtCursor('@a/b', 4))
