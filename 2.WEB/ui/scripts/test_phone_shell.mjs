// scripts/test_phone_shell.mjs
// 2026-06-10 TASK-2.6 引入：phone-shell 工具模块 smoke test
// Node 22+ 自带 --experimental-strip-types，无需 tsx

import { fileURLToPath, pathToFileURL } from 'node:url'
import { dirname, resolve } from 'node:path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
const modulePath = resolve(__dirname, '..', 'src', 'platform', 'phone-shell.ts')

// Windows 必须用 pathToFileURL 转换
const phone = await import(pathToFileURL(modulePath).href)

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

console.log('\n=== phone-shell API smoke test ===\n')

// 1. 类型导出
console.log('[1] 类型导出检查')
assert('getDisplayMode 函数存在', typeof phone.getDisplayMode === 'function')
assert('isInstalledPWA 函数存在', typeof phone.isInstalledPWA === 'function')
assert('isIOSPWA 函数存在', typeof phone.isIOSPWA === 'function')
assert('getOrientation 函数存在', typeof phone.getOrientation === 'function')
assert('isIOSDevice 函数存在', typeof phone.isIOSDevice === 'function')
assert('isAndroidDevice 函数存在', typeof phone.isAndroidDevice === 'function')
assert('isTouchDevice 函数存在', typeof phone.isTouchDevice === 'function')
assert('installVhFix 函数存在', typeof phone.installVhFix === 'function')
assert('installSafeAreaVars 函数存在', typeof phone.installSafeAreaVars === 'function')
assert('lockBodyScroll 函数存在', typeof phone.lockBodyScroll === 'function')
assert('setThemeColor 函数存在', typeof phone.setThemeColor === 'function')
assert('setIOSStatusBarStyle 函数存在', typeof phone.setIOSStatusBarStyle === 'function')

// 2. getDisplayMode 在 SSR/无 window 时返回 browser
console.log('\n[2] getDisplayMode 边界')
assert('getDisplayMode() 返回字符串', typeof phone.getDisplayMode() === 'string')
assert('getDisplayMode() 返回 4 种之一', ['standalone', 'fullscreen', 'minimal-ui', 'browser'].includes(phone.getDisplayMode()))

// 3. isInstalledPWA 返回 boolean
console.log('\n[3] isInstalledPWA 返回类型')
assert('isInstalledPWA() 是 boolean', typeof phone.isInstalledPWA() === 'boolean')

// 4. isIOSPWA 返回 boolean
console.log('\n[4] isIOSPWA 返回类型')
assert('isIOSPWA() 是 boolean', typeof phone.isIOSPWA() === 'boolean')

// 5. getOrientation 返回 portrait/landscape
console.log('\n[5] getOrientation 返回值')
assert('getOrientation() 返回字符串', typeof phone.getOrientation() === 'string')
assert('getOrientation() 返回 portrait/landscape', ['portrait', 'landscape'].includes(phone.getOrientation()))

// 6. installVhFix 返回卸载函数
console.log('\n[6] installVhFix 返回 cleanup')
const cleanup = phone.installVhFix()
assert('cleanup 是函数', typeof cleanup === 'function')
cleanup()  // 立即清理

// 7. lockBodyScroll 不抛错
console.log('\n[7] lockBodyScroll 边界')
let bodyOverflowErr = null
try {
  phone.lockBodyScroll(true)
  phone.lockBodyScroll(false)
} catch (e) {
  bodyOverflowErr = e
}
assert('lockBodyScroll 不抛错', bodyOverflowErr === null)

// 8. setThemeColor 不抛错
console.log('\n[8] setThemeColor 边界')
let themeErr = null
try {
  phone.setThemeColor('#000000', 'dark')
  phone.setThemeColor('#ffffff', 'light')
} catch (e) {
  themeErr = e
}
assert('setThemeColor 不抛错', themeErr === null)

// 9. setIOSStatusBarStyle 不抛错
console.log('\n[9] setIOSStatusBarStyle 边界')
let iosErr = null
try {
  phone.setIOSStatusBarStyle('black-translucent')
  phone.setIOSStatusBarStyle('default')
  phone.setIOSStatusBarStyle('black')
} catch (e) {
  iosErr = e
}
assert('setIOSStatusBarStyle 不抛错', iosErr === null)

console.log(`\n=== 结果：${pass} 通过 / ${fail} 失败 ===`)
if (fail > 0) {
  console.log('失败用例:')
  for (const f of failures) console.log(`  - ${f}`)
  process.exit(1)
}
process.exit(0)
