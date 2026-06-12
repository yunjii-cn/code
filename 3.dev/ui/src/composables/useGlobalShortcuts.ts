// composables/useGlobalShortcuts.ts
// 2026-06-09 TASK-2.8 引入：全局快捷键注册
// 2026-06-10 TASK-2.8 升级：增加 5 个默认快捷键
//   - Ctrl+K 命令面板（已有）
//   - Ctrl+, 设置（已有）
//   - Ctrl+Shift+N 通知中心（已有）
//   - Shift+/ ? 帮助（已有）
//   - Ctrl+Enter 发送消息（新增）— 派发 yj:shortcut-send
//   - Ctrl+N 新建对话（新增）— 派发 yj:shortcut-new-chat
//   - Ctrl+S 保存当前编辑（新增）— 派发 yj:shortcut-save
//   - Ctrl+/ 切换模式（独行/团队）（新增）— 派发 yj:shortcut-toggle-mode
//   - Esc 关闭弹窗（新增）— 派发 yj:shortcut-esc

import { ref, type Ref } from 'vue'
import { useRouter } from 'vue-router'
import { useShortcut } from './useShortcut.ts'

export interface GlobalShortcuts {
  showCommandPalette: Ref<boolean>
}

export function useGlobalShortcuts(): GlobalShortcuts {
  const router = useRouter()

  const showCommandPalette: Ref<boolean> = ref(false)

  // 1. Ctrl+K / Cmd+K → 打开命令面板
  useShortcut('Ctrl+K', () => {
    showCommandPalette.value = true
  }, { description: '打开命令面板' })

  // 2. Ctrl+, → 打开设置
  useShortcut('Ctrl+,', () => {
    router.push('/settings')
  }, { description: '打开系统设置' })

  // 3. Ctrl+Shift+N → 打开通知中心
  useShortcut('Ctrl+Shift+N', () => {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('yj:open-notifications'))
    }
  }, { description: '打开通知中心' })

  // 4. Shift+/ (?) → 帮助（打开命令面板）
  useShortcut('Shift+/', () => {
    showCommandPalette.value = true
  }, { description: '查看快捷键帮助' })

  // === 2026-06-10 TASK-2.8 新增：5 个默认快捷键 ===

  // 5. Ctrl+Enter / Cmd+Enter → 发送消息（路由限定 chat view）
  useShortcut('Ctrl+Enter', () => {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('yj:shortcut-send'))
    }
  }, {
    routes: ['/', '/chat'],
    description: '发送消息',
  })

  // 6. Ctrl+N / Cmd+N → 新建对话（全局可用；chat view 监听事件执行）
  useShortcut('Ctrl+N', () => {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('yj:shortcut-new-chat'))
    }
  }, { description: '新建对话' })

  // 7. Ctrl+S / Cmd+S → 保存当前编辑（全局可用；各 view 监听事件执行）
  useShortcut('Ctrl+S', () => {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('yj:shortcut-save'))
    }
  }, { description: '保存当前编辑' })

  // 8. Ctrl+/ → 切换模式（独行/团队）
  useShortcut('Ctrl+/', () => {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('yj:shortcut-toggle-mode'))
    }
  }, { description: '切换模式（独行/团队）' })

  // 9. Esc → 关闭弹窗（最顶层抽屉/模态/菜单）
  useShortcut('Escape', () => {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('yj:shortcut-esc'))
    }
  }, {
    description: '关闭弹窗',
    disableInInputs: false,  // 在 input 内也允许（清空焦点）
  })

  return {
    showCommandPalette,
  }
}
