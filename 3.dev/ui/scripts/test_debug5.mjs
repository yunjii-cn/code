import { parseTriggerAtCursor } from '../src/features/chat/composables/triggerParser.ts'
const text = 'Type /hel'
const cursor = 9
console.log('text:', JSON.stringify(text), 'len:', text.length, 'cursor:', cursor)
for (let i = 0; i < text.length; i++) console.log(`  text[${i}]:`, JSON.stringify(text[i]))
const r = parseTriggerAtCursor(text, cursor)
console.log('result:', r)
