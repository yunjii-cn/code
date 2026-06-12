/**
 * useSwipe — 触屏滑动手势 composable
 *
 * 2026-06-09 TASK-4.7 引入
 *
 * 支持：
 *  - 左右滑动（swipeLeft / swipeRight）
 *  - 上下滑动（swipeUp / swipeDown）
 *  - 最小距离阈值（默认 50px）
 *  - 最大时间窗口（默认 300ms，防止慢拖误判）
 *  - 可绑定到任意 DOM 元素
 *
 * 用法：
 *  const { onSwipeLeft, onSwipeRight } = useSwipe(element, { minDistance: 60 })
 *  onSwipeLeft(() => router.next())
 *  onSwipeRight(() => router.back())
 */
import { ref, onMounted, onUnmounted, type Ref } from 'vue'

export interface SwipeOptions {
  minDistance?: number   // 最小滑动距离（px），默认 50
  maxDuration?: number   // 最大时间窗口（ms），默认 300
  preventDefault?: boolean // 是否阻止默认行为，默认 false
}

export type SwipeDirection = 'left' | 'right' | 'up' | 'down'
export type SwipeHandler = (direction: SwipeDirection, delta: { dx: number; dy: number }) => void

export function useSwipe(
  target: Ref<HTMLElement | null | undefined>,
  options: SwipeOptions = {},
) {
  const minDistance = options.minDistance ?? 50
  const maxDuration = options.maxDuration ?? 300
  const preventDefault = options.preventDefault ?? false

  const isSwiping = ref(false)
  const direction = ref<SwipeDirection | null>(null)

  let startX = 0
  let startY = 0
  let startTime = 0

  const handlers: SwipeHandler[] = []

  function onSwipe(handler: SwipeHandler) {
    handlers.push(handler)
  }

  function emit(dir: SwipeDirection, dx: number, dy: number) {
    direction.value = dir
    for (const h of handlers) {
      h(dir, { dx, dy })
    }
  }

  function onTouchStart(e: TouchEvent) {
    if (!e.touches.length) return
    const touch = e.touches[0]
    startX = touch.clientX
    startY = touch.clientY
    startTime = Date.now()
    isSwiping.value = true
    direction.value = null
  }

  function onTouchEnd(e: TouchEvent) {
    if (!isSwiping.value) return
    isSwiping.value = false

    if (!e.changedTouches.length) return
    const touch = e.changedTouches[0]
    const dx = touch.clientX - startX
    const dy = touch.clientY - startY
    const elapsed = Date.now() - startTime

    if (elapsed > maxDuration) return

    const absDx = Math.abs(dx)
    const absDy = Math.abs(dy)

    if (absDx > absDy && absDx >= minDistance) {
      if (preventDefault) e.preventDefault()
      emit(dx > 0 ? 'right' : 'left', dx, dy)
    } else if (absDy > absDx && absDy >= minDistance) {
      if (preventDefault) e.preventDefault()
      emit(dy > 0 ? 'down' : 'up', dx, dy)
    }
  }

  function onTouchMove(e: TouchEvent) {
    if (!isSwiping.value || !preventDefault) return
    // 如果已确定方向且是水平滑动，阻止浏览器回退手势
    if (!e.touches.length) return
    const dx = Math.abs(e.touches[0].clientX - startX)
    const dy = Math.abs(e.touches[0].clientY - startY)
    if (dx > dy && dx > 10) {
      e.preventDefault()
    }
  }

  onMounted(() => {
    const el = target.value
    if (!el) return
    el.addEventListener('touchstart', onTouchStart, { passive: true })
    el.addEventListener('touchmove', onTouchMove, { passive: !preventDefault })
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
    isSwiping,
    direction,
    onSwipe,
  }
}
