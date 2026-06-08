// composables/useGlobalShortcuts.ts
// 2026-06-09 TASK-2.8 引入：全局快捷键注册
// 在 App.vue 启动时调用一次，定义一组全局快捷键
// 内置：Ctrl+K 命令面板 / Esc 关闭弹窗 / Ctrl+, 设置 / Ctrl+Shift+N 通知中心 / ? 帮助

import { ref, type Ref } from 'vue'
import { useRouter } from 'vue-router'
import { useShortcut } from './useShortcut'

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
    window.dispatchEvent(new CustomEvent('yj:open-notifications'))
  }, { description: '打开通知中心' })

  // 4. ? → 帮助（打开命令面板）
  useShortcut('Shift+/', () => {
    showCommandPalette.value = true
  }, { description: '查看快捷键帮助' })

  return {
    showCommandPalette,
  }
}
