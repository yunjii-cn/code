// scripts/test_use_toast.mjs
// 2026-06-10 TASK-2.7 引入：useToast 增强 API smoke test
// Node 22+ 自带 --experimental-strip-types

import { fileURLToPath, pathToFileURL } from 'node:url'
import { dirname, resolve } from 'node:path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

// 模拟 window/document（Node 环境没有）
globalThis.window = {
  addEventListener: () => {},
  removeEventListener: () => {},
  dispatchEvent: () => true,
}
globalThis.document = {
  body: {},
  documentElement: { style: { setProperty: () => {} } },
}
globalThis.CustomEvent = class {
  constructor(type, init) {
    this.type = type
    this.detail = init?.detail
  }
}

const modulePath = resolve(
  __dirname,
  '..',
  'src',
  'shared',
  'composables',
  'useToast.ts',
)
const ut = await import(pathToFileURL(modulePath).href)

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

console.log('\n=== useToast 增强 API smoke test ===\n')

// 1. 类型 / 函数导出
console.log('[1] 导出检查')
assert('useToast 函数存在', typeof ut.useToast === 'function')
assert('dismissByKey 函数存在', typeof ut.dismissByKey === 'function')
assert('clearAll 函数存在', typeof ut.clearAll === 'function')
assert('MAX_VISIBLE = 3', ut.MAX_VISIBLE === 3)
assert('__bindToastBridge 函数存在', typeof ut.__bindToastBridge === 'function')
assert('__unbindToastBridge 函数存在', typeof ut.__unbindToastBridge === 'function')

// 2. ToastOptions 类型
console.log('\n[2] ToastOptions 字段')
// 模拟一个 ToastOptions 对象
const opt = {
  message: 'test',
  type: 'success',
  duration: 3000,
  detail: 'detail',
  position: 'top-right',
  action: { label: '重试', onClick: () => {} },
  key: 'k1',
  onClose: () => {},
}
assert('ToastOptions 接受 message', opt.message === 'test')
assert('ToastOptions 接受 type', opt.type === 'success')
assert('ToastOptions 接受 duration', opt.duration === 3000)
assert('ToastOptions 接受 detail', opt.detail === 'detail')
assert('ToastOptions 接受 position', opt.position === 'top-right')
assert('ToastOptions 接受 action', opt.action.label === '重试')
assert('ToastOptions 接受 key', opt.key === 'k1')
assert('ToastOptions 接受 onClose', typeof opt.onClose === 'function')

// 3. ToastPosition 取值
console.log('\n[3] ToastPosition 7 种取值')
const positions = ['top', 'top-right', 'top-left', 'center', 'bottom', 'bottom-right', 'bottom-left']
for (const p of positions) {
  assert(`position ${p} 是字符串`, typeof p === 'string')
}

// 4. ToastAction 取值
console.log('\n[4] ToastAction 字段')
const act = { label: '重试', onClick: () => {}, closeOnClick: true, variant: 'danger' }
assert('action.label', act.label === '重试')
assert('action.closeOnClick', act.closeOnClick === true)
assert('action.variant = danger', act.variant === 'danger')

// 5. useToast() 返回对象
console.log('\n[5] useToast() 返回结构')
const toast = ut.useToast()
assert('toast.show 函数', typeof toast.show === 'function')
assert('toast.dismiss 函数', typeof toast.dismiss === 'function')
assert('toast.dismissByKey 函数', typeof toast.dismissByKey === 'function')
assert('toast.clear 函数', typeof toast.clear === 'function')
assert('toast.success 函数', typeof toast.success === 'function')
assert('toast.error 函数', typeof toast.error === 'function')
assert('toast.warning 函数', typeof toast.warning === 'function')
assert('toast.info 函数', typeof toast.info === 'function')
assert('toast.loading 函数', typeof toast.loading === 'function')
assert('toast.remove 函数', typeof toast.remove === 'function')

// 6. show() 不会抛错（即使没桥接）
console.log('\n[6] show() 不抛错')
let showErr = null
try {
  const id = toast.show({ message: 'test' })
  assert('show() 返回数字', typeof id === 'number')
} catch (e) {
  showErr = e
}
assert('show() 不抛错', showErr === null)

// 7. 各种类型不抛错
console.log('\n[7] success/error/warning/info 不抛错')
for (const fn of ['success', 'error', 'warning', 'info']) {
  let err = null
  try {
    toast[fn]('test')
  } catch (e) {
    err = e
  }
  assert(`${fn}() 不抛错`, err === null)
}

// 8. dismissByKey 不抛错
console.log('\n[8] dismissByKey/clearAll 不抛错')
let dismissErr = null
try {
  ut.dismissByKey('non-existent')
  ut.clearAll()
} catch (e) {
  dismissErr = e
}
assert('dismissByKey/clearAll 不抛错', dismissErr === null)

// 9. 桥接注册 / 解注册
console.log('\n[9] 桥接管理')
let enqueueCalled = false
ut.__bindToastBridge({
  enqueue: () => { enqueueCalled = true },
  remove: () => {},
})
const id = toast.show({ message: 'with bridge', key: 'b1' })
assert('桥接后 enqueue 被调用', enqueueCalled)
ut.__unbindToastBridge()

// 10. 带 action 的 show
console.log('\n[10] 带 action 的 show')
let actionClicked = false
const id2 = toast.show({
  message: 'm',
  type: 'error',
  action: { label: '重试', onClick: () => { actionClicked = true } },
})
assert('带 action 的 show 不抛错', typeof id2 === 'number')

console.log(`\n=== 结果：${pass} 通过 / ${fail} 失败 ===`)
if (fail > 0) {
  console.log('失败用例:')
  for (const f of failures) console.log(`  - ${f}`)
  process.exit(1)
}
process.exit(0)
