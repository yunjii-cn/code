// scripts/test_debug.mjs
import { parseTriggerAtCursor } from '../src/features/chat/composables/triggerParser.ts'

const text = 'See @src/utils/index.ts'
const cursor = 25
console.log('text:', text, 'len:', text.length, 'cursor:', cursor)
console.log('text[24]:', text[24], 'text[23]:', text[23])
const r = parseTriggerAtCursor(text, cursor)
console.log('result:', r)
