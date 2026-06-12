import { ref, computed, onMounted, onUnmounted } from 'vue'

/**
 * useDevice — 设备检测 composable
 *
 * 2026-06-09 TASK-4.7 增强：增加平板断点 + 触屏检测 + 横竖屏
 *
 * 断点体系（与 tokens.css --bp-mobile / --bp-tablet 对齐）：
 *   - mobile:  < 768px
 *   - tablet:  768px - 1023px
 *   - desktop: >= 1024px
 */
export function useDevice() {
  const width = ref(window.innerWidth)
  const height = ref(window.innerHeight)

  const isMobile = computed(() => width.value < 768)
  const isTablet = computed(() => width.value >= 768 && width.value < 1024)
  const isDesktop = computed(() => width.value >= 1024)
  const isTouch = computed(() => 'ontouchstart' in window || navigator.maxTouchPoints > 0)
  const isLandscape = computed(() => width.value > height.value)
  const isPortrait = computed(() => !isLandscape.value)

  /** 小屏手机（< 375px，如 iPhone SE） */
  const isSmallMobile = computed(() => width.value < 375)

  /** 大屏桌面（>= 1440px，可展示三栏） */
  const isLargeDesktop = computed(() => width.value >= 1440)

  /** 侧栏是否应该折叠（平板 + 小桌面） */
  const sidebarCollapsed = computed(() => isMobile.value || isTablet.value)

  function onResize() {
    width.value = window.innerWidth
    height.value = window.innerHeight
  }

  onMounted(() => window.addEventListener('resize', onResize))
  onUnmounted(() => window.removeEventListener('resize', onResize))

  return {
    width,
    height,
    isMobile,
    isTablet,
    isDesktop,
    isTouch,
    isLandscape,
    isPortrait,
    isSmallMobile,
    isLargeDesktop,
    sidebarCollapsed,
  }
}
