/**
 * usePullRefresh — 下拉刷新 composable
 *
 * 2026-06-09 TASK-4.7 引入
 *
 * 支持：
 *  - 下拉阈值（默认 60px）
 *  - 释放触发回调
 *  - 状态：idle / pulling / ready / refreshing / success / error
 *  - 自动回弹动画
 *
 * 用法：
 *  const { pullState, pullY, onRefresh } = usePullRefresh(target, { threshold: 60 })
 *  onRefresh(async () => { await store.refreshAll() })
 */
import { ref, onMounted, onUnmounted, type Ref } from 'vue'

export type PullState = 'idle' | 'pulling' | 'ready' | 'refreshing' | 'success' | 'error'

export interface PullRefreshOptions {
  threshold?: number     // 触发刷新的下拉距离（px），默认 60
  maxPull?: number       // 最大下拉距离（px），默认 120
  successDuration?: number // 成功状态显示时长（ms），默认 800
  errorDuration?: number   // 错误状态显示时长（ms），默认 1500
}

export function usePullRefresh(
  target: Ref<HTMLElement | null | undefined>,
  options: PullRefreshOptions = {},
) {
  const threshold = options.threshold ?? 60
  const maxPull = options.maxPull ?? 120
  const successDuration = options.successDuration ?? 800
  const errorDuration = options.errorDuration ?? 1500

  const pullState = ref<PullState>('idle')
  const pullY = ref(0)

  let startY = 0
  let refreshCallback: (() => Promise<void>) | null = null
  let isLocked = false  // 防止重复触发

  function onRefresh(cb: () => Promise<void>) {
    refreshCallback = cb
  }

  function onTouchStart(e: TouchEvent) {
    if (isLocked) return
    // 仅在滚动到顶部时触发
    const el = target.value
    if (!el) return
    if (el.scrollTop > 0) return
    if (!e.touches.length) return
    startY = e.touches[0].clientY
    pullState.value = 'pulling'
  }

  function onTouchMove(e: TouchEvent) {
    if (pullState.value === 'refreshing' || isLocked) return
    if (!e.touches.length) return
    const dy = e.touches[0].clientY - startY
    if (dy <= 0) {
      pullY.value = 0
      pullState.value = 'idle'
      return
    }
    // 阻尼效果：越拉越难拉
    const dampened = Math.min(maxPull, dy * 0.5)
    pullY.value = dampened
    pullState.value = dampened >= threshold ? 'ready' : 'pulling'
  }

  async function onTouchEnd() {
    if (pullState.value === 'refreshing' || isLocked) return
    if (pullState.value !== 'ready') {
      // 未达阈值，回弹
      pullY.value = 0
      pullState.value = 'idle'
      return
    }
    // 触发刷新
    pullY.value = threshold
    pullState.value = 'refreshing'
    isLocked = true
    try {
      if (refreshCallback) {
        await refreshCallback()
      }
      pullState.value = 'success'
      setTimeout(() => {
        pullY.value = 0
        pullState.value = 'idle'
        isLocked = false
      }, successDuration)
    } catch {
      pullState.value = 'error'
      setTimeout(() => {
        pullY.value = 0
        pullState.value = 'idle'
        isLocked = false
      }, errorDuration)
    }
  }

  onMounted(() => {
    const el = target.value
    if (!el) return
    el.addEventListener('touchstart', onTouchStart, { passive: true })
    el.addEventListener('touchmove', onTouchMove, { passive: true })
    el.addEventListener('touchend', onTouchEnd, { passive: true })
  })

  onUnmounted(() => {
    const el = target.value
    if (!el) return
    el.removeEventListener('touchstart', onTouchStart)
    el.removeEventListener('touchmove', onTouchMove)
    el.removeEventListener('touchend', onTouchEnd)
  })

  return {
    pullState,
    pullY,
    onRefresh,
  }
}
