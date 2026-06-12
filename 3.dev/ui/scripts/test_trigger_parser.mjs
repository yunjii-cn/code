// scripts/test_trigger_parser.mjs
// Quick smoke test for triggerParser (TASK-2.4)
import { parseTriggerAtCursor } from '../src/features/chat/composables/triggerParser.ts'

const cases = [
  ['@src', 4, { type: 'file', query: 'src', start: 0 }],
  ['See @src/utils/index.ts', 'See @src/utils/index.ts'.length, { type: 'file', query: 'src/utils/index.ts', start: 4 }],
  ['Type /hel', 'Type /hel'.length, { type: 'command', query: 'hel', start: 5 }],
  ['/review sec', '/review sec'.length, { type: 'review', query: 'sec', start: 0 }],
  ['Use $react', 'Use $react'.length, { type: 'skill', query: 'react', start: 4 }],
  ['hello@src', 'hello@src'.length, null],
  ['a/@', 'a/@'.length, null],
  ['$python.dev', '$python.dev'.length, { type: 'skill', query: 'python.dev', start: 0 }],
  ['text @', 'text @'.length, { type: 'file', query: '', start: 5 }],
  ['hi @main', 'hi @main'.length, { type: 'file', query: 'main', start: 3 }],
  ['$react ', '$react '.length, null],
  ['@a/b/c/d', '@a/b/c/d'.length, { type: 'file', query: 'a/b/c/d', start: 0 }],
  ['@my-test_file', '@my-test_file'.length, { type: 'file', query: 'my-test_file', start: 0 }],
  ['@component.tsx', '@component.tsx'.length, { type: 'file', query: 'component.tsx', start: 0 }],
  ['hello world', 'hello world'.length, null],
  ['', 0, null],
  ['@', 1, { type: 'file', query: '', start: 0 }],
]

let pass = 0, fail = 0
for (const [text, cursor, expected] of cases) {
  const got = parseTriggerAtCursor(text, cursor)
  const matches = (() => {
    if (expected === null) return got === null
    if (got === null) return false
    if (expected.type !== undefined && got.type !== expected.type) return false
    if (expected.query !== undefined && got.query !== expected.query) return false
    if (expected.start !== undefined && got.start !== expected.start) return false
    return true
  })()
  if (matches) {
    pass++
    console.log('PASS', JSON.stringify(text), '->', JSON.stringify(got))
  } else {
    fail++
    console.log('FAIL', JSON.stringify(text), 'cursor=', cursor, 'expected=', JSON.stringify(expected), 'got=', JSON.stringify(got))
  }
}
console.log(`\n${pass}/${pass + fail} passed`)
process.exit(fail > 0 ? 1 : 0)
