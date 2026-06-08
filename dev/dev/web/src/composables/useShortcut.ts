// composables/useShortcut.ts
// 2026-06-09 TASK-2.8 引入：通用快捷键 composable
// 设计：key 字符串解析 → KeyboardEvent 匹配；自动清理（onUnmounted）
// 支持：修饰键（Ctrl/Cmd/Shift/Alt）+ 单字母/功能键；可设置 route 限定

import { onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'

export interface ShortcutOptions {
  /** 是否在 input/textarea/contenteditable 中禁用（默认 true） */
  disableInInputs?: boolean
  /** 仅在指定路由路径触发（数组） */
  routes?: string[]
  /** 是否阻止默认行为（默认 true） */
  preventDefault?: boolean
  /** 描述（用于显示在命令面板） */
  description?: string
}

/**
 * 解析快捷键字符串为可比较的 key combo
 *   "Ctrl+K"     → { ctrl: true, key: "k" }
 *   "Cmd+Shift+P"→ { meta: true, shift: true, key: "p" }  (Mac alias)
 *   "Esc"        → { key: "escape" }
 *   "?"          → { key: "?" }
 */
function parseShortcut(combo: string): { ctrl: boolean; meta: boolean; shift: boolean; alt: boolean; key: string } {
  const parts = combo.toLowerCase().split('+').map(s => s.trim())
  const result = { ctrl: false, meta: false, shift: false, alt: false, key: '' }
  for (const part of parts) {
    if (part === 'ctrl' || part === 'control') result.ctrl = true
    else if (part === 'cmd' || part === 'meta' || part === 'command') result.meta = true
    else if (part === 'shift') result.shift = true
    else if (part === 'alt' || part === 'option') result.alt = true
    else result.key = part
  }
  return result
}

function matchEvent(evt: KeyboardEvent, parsed: ReturnType<typeof parseShortcut>): boolean {
  if (parsed.ctrl !== evt.ctrlKey) return false
  if (parsed.meta !== evt.metaKey) return false
  if (parsed.shift !== evt.shiftKey) return false
  if (parsed.alt !== evt.altKey) return false
  // KeyboardEvent.key 是大小写敏感（"K" vs "k"），统一小写
  return evt.key.toLowerCase() === parsed.key
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  const tag = target.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true
  if (target.isContentEditable) return true
  return false
}

/**
 * 注册一个键盘快捷键
 * @param combo  快捷键字符串，如 "Ctrl+K" / "Esc" / "Shift+?"
 * @param handler 触发时执行的回调
 * @param options 路由限定 / 输入框禁用等
 */
export function useShortcut(combo: string, handler: (e: KeyboardEvent) => void, options: ShortcutOptions = {}) {
  const route = useRoute()
  const parsed = parseShortcut(combo)
  const { disableInInputs = true, routes, preventDefault = true } = options

  function onKeyDown(e: KeyboardEvent) {
    if (disableInInputs && isEditableTarget(e.target)) return
    if (routes && routes.length > 0 && !routes.includes(route.path)) return
    if (!matchEvent(e, parsed)) return
    if (preventDefault) e.preventDefault()
    handler(e)
  }

  onMounted(() => {
    window.addEventListener('keydown', onKeyDown)
  })

  onUnmounted(() => {
    window.removeEventListener('keydown', onKeyDown)
  })
}
