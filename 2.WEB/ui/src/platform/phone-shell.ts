// platform/phone-shell.ts
// 2026-06-10 TASK-2.6 引入：iOS Phone Layout — 手机浏览器布局工具集
//
// 与 platform/mobile.ts（Capacitor 原生 App 平台实现）区分：
// 本文件针对 **PWA / 手机浏览器** 场景，提供：
//   1. 100vh 修正（解决地址栏导致的 vh 不准）
//   2. PWA display-mode 检测
//   3. orientation / safe-area 写入 CSS 变量
//   4. iOS standalone 判定
//   5. 主题色动态切换
//   6. body 滚动锁

/**
 * PWA display-mode 取值
 *   standalone: 已添加到主屏幕，全屏启动（iOS / Android PWA）
 *   fullscreen: 浏览器请求的全屏模式
 *   minimal-ui: 最小浏览器 chrome
 *   browser:    普通浏览器标签
 */
export type DisplayMode = 'standalone' | 'fullscreen' | 'minimal-ui' | 'browser'

/**
 * 获取当前 PWA display-mode
 * 参考：https://developer.mozilla.org/en-US/docs/Web/API/Window/matchMedia
 */
export function getDisplayMode(): DisplayMode {
  if (typeof window === 'undefined' || !window.matchMedia) return 'browser'
  if (window.matchMedia('(display-mode: standalone)').matches) return 'standalone'
  if (window.matchMedia('(display-mode: fullscreen)').matches) return 'fullscreen'
  if (window.matchMedia('(display-mode: minimal-ui)').matches) return 'minimal-ui'
  return 'browser'
}

/** 是否已作为 PWA 安装 */
export function isInstalledPWA(): boolean {
  const mode = getDisplayMode()
  return mode === 'standalone' || mode === 'fullscreen'
}

/** iOS PWA standalone 判定（通过 navigator.standalone） */
export function isIOSPWA(): boolean {
  if (typeof navigator === 'undefined') return false
  return 'standalone' in navigator && Boolean((navigator as Navigator & { standalone?: boolean }).standalone)
}

/** 屏幕方向 */
export type Orientation = 'portrait' | 'landscape'

export function getOrientation(): Orientation {
  if (typeof window === 'undefined') return 'portrait'
  return window.innerWidth > window.innerHeight ? 'landscape' : 'portrait'
}

/** iOS 设备检测（iPhone / iPad） */
export function isIOSDevice(): boolean {
  if (typeof navigator === 'undefined') return false
  const ua = navigator.userAgent
  return /iPad|iPhone|iPod/.test(ua) || (ua.includes('Mac') && 'ontouchend' in document)
}

/** Android 设备检测 */
export function isAndroidDevice(): boolean {
  if (typeof navigator === 'undefined') return false
  return /Android/i.test(navigator.userAgent)
}

/** 触屏检测 */
export function isTouchDevice(): boolean {
  if (typeof window === 'undefined') return false
  return 'ontouchstart' in window || (navigator.maxTouchPoints || 0) > 0
}

/**
 * 修正移动浏览器 100vh 偏差
 * 写入 CSS 变量：
 *   --viewport-height = 1vh
 *   --viewport-width  = 1vw
 * 用法：height: calc(var(--viewport-height) * 100)
 *
 * 监听 resize / orientationchange，自动更新
 * @returns 卸载函数
 */
export function installVhFix(): () => void {
  if (typeof window === 'undefined' || typeof document === 'undefined') return () => {}

  function setVh() {
    const vh = window.innerHeight * 0.01
    const vw = window.innerWidth * 0.01
    const root = document.documentElement
    root.style.setProperty('--viewport-height', `${vh}px`)
    root.style.setProperty('--viewport-width', `${vw}px`)
  }

  setVh()
  window.addEventListener('resize', setVh, { passive: true })
  window.addEventListener('orientationchange', setVh, { passive: true })

  return () => {
    window.removeEventListener('resize', setVh)
    window.removeEventListener('orientationchange', setVh)
  }
}

/**
 * 把 safe-area 实时值写入 CSS 变量
 * 已有 --safe-top / --safe-bottom / --safe-left / --safe-right
 * 这里补充：--safe-top-fallback / --safe-bottom-fallback 给非 env() 兼容环境用
 */
export function installSafeAreaVars(): () => void {
  if (typeof window === 'undefined') return () => {}
  // CSS 已处理 env() fallback，这里是占位，未来需要时扩展
  return () => {}
}

/**
 * 锁定 / 解锁 body 滚动（用于打开抽屉/模态时）
 */
export function lockBodyScroll(lock: boolean): void {
  if (typeof document === 'undefined') return
  const body = document.body
  if (lock) {
    body.style.overflow = 'hidden'
    body.style.touchAction = 'none'
  } else {
    body.style.overflow = ''
    body.style.touchAction = ''
  }
}

/**
 * 动态切换 theme-color meta 标签
 * 用于适配浅色/深色模式或状态栏变色
 */
export function setThemeColor(color: string, scheme: 'dark' | 'light' = 'dark'): void {
  if (typeof document === 'undefined') return
  const meta = document.querySelector(
    `meta[name="theme-color"][media*="${scheme}"]`,
  ) as HTMLMetaElement | null
  if (meta) {
    meta.content = color
  }
}

/**
 * 触发 iOS PWA 状态栏风格（修改 apple-mobile-web-app-status-bar-style）
 * 可选：default / black / black-translucent
 */
export function setIOSStatusBarStyle(style: 'default' | 'black' | 'black-translucent'): void {
  if (typeof document === 'undefined') return
  const meta = document.querySelector('meta[name="apple-mobile-web-app-status-bar-style"]') as HTMLMetaElement | null
  if (meta) {
    meta.content = style
  }
}
