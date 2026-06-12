// composables/useShortcut.ts
// 2026-06-09 TASK-2.8 引入：通用快捷键 composable
// 2026-06-10 TASK-2.8 升级：
//   - 多 combo 支持（一个 useShortcut 注册多个快捷键）
//   - 命令面板描述注册（getAllShortcuts 收集所有注册的快捷键）
//   - 修饰键归一化（Ctrl/Cmd 自动跨平台）

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
  /** 作用域：用于分组（如 'global' / 'chat' / 'editor'） */
  scope?: string
  /** 分组：用于命令面板的二级分类 */
  group?: string
}

export interface ShortcutEntry {
  combo: string
  description: string
  scope: string
  group: string
  registeredAt: number
}

interface ParsedCombo {
  ctrl: boolean
  meta: boolean
  shift: boolean
  alt: boolean
  key: string
  raw: string
}

/** 全局快捷键注册表（命令面板查询用） */
const registry = new Map<string, ShortcutEntry[]>()

/** 获取所有已注册的快捷键 */
export function getAllShortcuts(): ShortcutEntry[] {
  const result: ShortcutEntry[] = []
  for (const arr of registry.values()) {
    result.push(...arr)
  }
  return result
}

/** 按 scope 获取快捷键 */
export function getShortcutsByScope(scope: string): ShortcutEntry[] {
  return registry.get(scope) || []
}

/** 清空注册表（用于热重载 / 测试） */
export function clearShortcutRegistry(): void {
  registry.clear()
}

/** 键盘 key 别名映射（用户输入 'Esc' → KeyboardEvent 'Escape'） */
const KEY_ALIAS: Record<string, string> = {
  esc: 'escape',
  escape: 'escape',
  enter: 'enter',
  return: 'enter',
  tab: 'tab',
  space: ' ',
  spacebar: ' ',
  up: 'arrowup',
  down: 'arrowdown',
  left: 'arrowleft',
  right: 'arrowright',
  del: 'delete',
  ins: 'insert',
  pgup: 'pageup',
  pgdn: 'pagedown',
  '?': '?',
  '/': '/',
}

/**
 * 解析快捷键字符串
 *   "Ctrl+K"     → { ctrl: true, key: "k" }
 *   "Cmd+Shift+P"→ { meta: true, shift: true, key: "p" }
 *   "Esc"        → { key: "escape" }（自动映射到 KeyboardEvent key）
 *   "Shift+/?"   → { shift: true, key: "?" }
 */
function parseShortcut(combo: string): ParsedCombo {
  const parts = combo.toLowerCase().split('+').map((s) => s.trim())
  const result: ParsedCombo = { ctrl: false, meta: false, shift: false, alt: false, key: '', raw: combo }
  for (const part of parts) {
    if (part === 'ctrl' || part === 'control') result.ctrl = true
    else if (part === 'cmd' || part === 'meta' || part === 'command') result.meta = true
    else if (part === 'shift') result.shift = true
    else if (part === 'alt' || part === 'option') result.alt = true
    else result.key = KEY_ALIAS[part] ?? part
  }
  return result
}

function matchEvent(evt: KeyboardEvent, parsed: ParsedCombo): boolean {
  if (parsed.ctrl !== evt.ctrlKey) return false
  if (parsed.meta !== evt.metaKey) return false
  if (parsed.shift !== evt.shiftKey) return false
  if (parsed.alt !== evt.altKey) return false
  return evt.key.toLowerCase() === parsed.key
}

/**
 * 内部 API：解析快捷键字符串（供测试使用）
 * @internal
 */
export function _parseShortcut(combo: string): ParsedCombo {
  return parseShortcut(combo)
}

/**
 * 内部 API：判断 KeyboardEvent 是否匹配已解析的快捷键（供测试使用）
 * @internal
 */
export function _matchEvent(evt: { ctrlKey: boolean; metaKey: boolean; shiftKey: boolean; altKey: boolean; key: string }, parsed: ParsedCombo): boolean {
  if (parsed.ctrl !== evt.ctrlKey) return false
  if (parsed.meta !== evt.metaKey) return false
  if (parsed.shift !== evt.shiftKey) return false
  if (parsed.alt !== evt.altKey) return false
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
 * 注册一个键盘快捷键（支持多 combo）
 * @param comboOrCombos  快捷键字符串或字符串数组
 *   "Ctrl+K" / ["Ctrl+K", "Cmd+K"]
 * @param handler 触发时执行的回调
 * @param options 路由限定 / 输入框禁用等
 */
export function useShortcut(
  comboOrCombos: string | string[],
  handler: (e: KeyboardEvent) => void,
  options: ShortcutOptions = {},
) {
  const route = useRoute()
  const combos = Array.isArray(comboOrCombos) ? comboOrCombos : [comboOrCombos]
  const parsed = combos.map(parseShortcut)
  const { disableInInputs = true, routes, preventDefault = true, description = '', scope = 'global', group = 'general' } = options

  // 注册到全局注册表
  const entries: ShortcutEntry[] = []
  for (const c of combos) {
    entries.push({ combo: c, description, scope, group, registeredAt: Date.now() })
  }
  const existing = registry.get(scope) || []
  registry.set(scope, [...existing, ...entries])

  function onKeyDown(e: KeyboardEvent) {
    if (disableInInputs && isEditableTarget(e.target)) return
    if (routes && routes.length > 0 && !routes.includes(route.path)) return
    const matched = parsed.some((p) => matchEvent(e, p))
    if (!matched) return
    if (preventDefault) e.preventDefault()
    handler(e)
  }

  onMounted(() => {
    window.addEventListener('keydown', onKeyDown)
  })

  onUnmounted(() => {
    window.removeEventListener('keydown', onKeyDown)
    // 从注册表移除
    const current = registry.get(scope)
    if (current) {
      const filtered = current.filter((e) => !entries.some((x) => x.combo === e.combo && x.registeredAt === e.registeredAt))
      if (filtered.length === 0) {
        registry.delete(scope)
      } else {
        registry.set(scope, filtered)
      }
    }
  })
}
