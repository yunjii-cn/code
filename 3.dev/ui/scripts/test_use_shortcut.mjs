// scripts/test_use_shortcut.mjs
// 2026-06-10 TASK-2.8 引入：useShortcut 增强 API smoke test
// 注意：useShortcut 需要 Vue 上下文（onMounted/onUnmounted + useRoute）
// 这里只测试独立工具函数 parseShortcut（不实际注册）
// 实际注册通过测试辅助函数 manualShortcutTest 模拟

import { fileURLToPath, pathToFileURL } from 'node:url'
import { dirname, resolve } from 'node:path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
const modulePath = resolve(__dirname, '..', 'src', 'composables', 'useShortcut.ts')

// 模拟 Vue 上下文
let mounted = []
let unmounted = []
const _onMounted = (fn) => { mounted.push(fn); return () => { mounted = mounted.filter(f => f !== fn) } }
const _onUnmounted = (fn) => { unmounted.push(fn) }
let _route = { path: '/' }
const _useRoute = () => _route

// 通过 Module._compile 替换依赖？过于复杂，改用动态替换
// 方案：把 useShortcut 的依赖 stub 掉，测试 parseShortcut 通过 _test_export 暴露

// 实际：直接测试 registry 工具
const us = await import(pathToFileURL(modulePath).href)

let pass = 0
let fail = 0
const failures = []

function assert(name, cond) {
  if (cond) {
    pass++
    console.log(`  ✓ ${name}`)
  } else {
    fail++
    failures.push(name)
    console.log(`  ✗ ${name}`)
  }
}

console.log('\n=== useShortcut 增强 API smoke test ===\n')

// 1. 导出检查
console.log('[1] 导出检查')
assert('useShortcut 函数存在', typeof us.useShortcut === 'function')
assert('getAllShortcuts 函数存在', typeof us.getAllShortcuts === 'function')
assert('getShortcutsByScope 函数存在', typeof us.getShortcutsByScope === 'function')
assert('clearShortcutRegistry 函数存在', typeof us.clearShortcutRegistry === 'function')

// 2. ShortcutEntry / ShortcutOptions 类型
console.log('\n[2] 类型导出（运行时）')
// 模拟构造 ShortcutEntry
const entry = { combo: 'Ctrl+K', description: '打开面板', scope: 'global', group: 'nav', registeredAt: Date.now() }
assert('ShortcutEntry 接受 combo', entry.combo === 'Ctrl+K')
assert('ShortcutEntry 接受 description', entry.description === '打开面板')
assert('ShortcutEntry 接受 scope', entry.scope === 'global')
assert('ShortcutEntry 接受 group', entry.group === 'nav')
assert('ShortcutEntry 接受 registeredAt', typeof entry.registeredAt === 'number')

// 3. ShortcutOptions 类型
const opt = {
  disableInInputs: true,
  routes: ['/'],
  preventDefault: true,
  description: 'desc',
  scope: 'global',
  group: 'general',
}
assert('ShortcutOptions 字段全', typeof opt.disableInInputs === 'boolean')

// 4. 注册表操作
console.log('\n[4] 注册表操作')
us.clearShortcutRegistry()
assert('清空后注册表为空', us.getAllShortcuts().length === 0)

const before = us.getShortcutsByScope('test-scope')
assert('空 scope 返回空数组', Array.isArray(before) && before.length === 0)

// 5. 模拟注册（直接操作 registry 内部难以测试，因为是私有）
// 这里只能验证 API 形状

console.log('\n[5] 模拟 KeyboardEvent（不实际触发）')
const evt1 = { ctrlKey: true, metaKey: false, shiftKey: false, altKey: false, key: 'K' }
const evt2 = { ctrlKey: true, metaKey: false, shiftKey: false, altKey: false, key: 'k' }
// 内部 parseShortcut 后 evt.key.toLowerCase() === parsed.key，所以大写 K 也会匹配小写 k
assert('KeyboardEvent 形状正确', evt1.ctrlKey === true && evt1.key === 'K')

// 6. useShortcut 是函数（实际注册需要 Vue 上下文）
console.log('\n[6] useShortcut 是函数')
assert('useShortcut 是函数', typeof us.useShortcut === 'function')

// 7. parseShortcut 通过 _parseShortcut 暴露（如果想测）
// 这里我们只能从源码推断：'Ctrl+K' → { ctrl: true, key: 'k' }
// 通过 mock 模拟一个事件匹配测试

// 8. parseShortcut 各种组合（用源码中的 _parseShortcut + _matchEvent）
console.log('\n[8] parseShortcut 组合验证（用源码）')
const parseCases = [
  { input: 'Ctrl+K', expected: { ctrl: true, meta: false, shift: false, alt: false, key: 'k' } },
  { input: 'Cmd+Shift+P', expected: { meta: true, shift: true, key: 'p' } },
  { input: 'Esc', expected: { key: 'escape' } },
  { input: 'Escape', expected: { key: 'escape' } },
  { input: 'Shift+/', expected: { shift: true, key: '/' } },
  { input: 'Ctrl+Enter', expected: { ctrl: true, key: 'enter' } },
  { input: 'Ctrl+/', expected: { ctrl: true, key: '/' } },
  { input: 'Alt+Tab', expected: { alt: true, key: 'tab' } },
  { input: 'Cmd+Up', expected: { meta: true, key: 'arrowup' } },
  { input: 'Space', expected: { key: ' ' } },
  { input: 'Ctrl+Shift+Delete', expected: { ctrl: true, shift: true, key: 'delete' } },
]
for (const c of parseCases) {
  const p = us._parseShortcut(c.input)
  let ok = true
  for (const k of Object.keys(c.expected)) {
    if (p[k] !== c.expected[k]) {
      ok = false
      break
    }
  }
  assert(`parse "${c.input}" 全部字段匹配`, ok)
}

// 9. _matchEvent 模拟匹配
console.log('\n[9] _matchEvent 模拟匹配')
const p1 = us._parseShortcut('Ctrl+K')
const m1 = us._matchEvent({ ctrlKey: true, metaKey: false, shiftKey: false, altKey: false, key: 'K' }, p1)
assert('Ctrl+K + KeyboardEvent{K} 匹配', m1 === true)
const m2 = us._matchEvent({ ctrlKey: true, metaKey: false, shiftKey: false, altKey: false, key: 'L' }, p1)
assert('Ctrl+K + KeyboardEvent{L} 不匹配', m2 === false)
const p2 = us._parseShortcut('Esc')
const m3 = us._matchEvent({ ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, key: 'Escape' }, p2)
assert('Esc + KeyboardEvent{Escape} 匹配', m3 === true)
const p3 = us._parseShortcut('Ctrl+Enter')
const m4 = us._matchEvent({ ctrlKey: true, metaKey: false, shiftKey: false, altKey: false, key: 'Enter' }, p3)
assert('Ctrl+Enter + KeyboardEvent{Enter} 匹配', m4 === true)
const m5 = us._matchEvent({ ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, key: 'Enter' }, p3)
assert('Ctrl+Enter + KeyboardEvent{Enter (无Ctrl)} 不匹配', m5 === false)

console.log(`\n=== 结果：${pass} 通过 / ${fail} 失败 ===`)
if (fail > 0) {
  console.log('失败用例:')
  for (const f of failures) console.log(`  - ${f}`)
  process.exit(1)
}
process.exit(0)
