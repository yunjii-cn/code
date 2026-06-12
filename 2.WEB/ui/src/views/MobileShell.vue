<!--
  MobileShell.vue
  2026-06-10 TASK-2.6 引入：iOS Phone Layout — 手机浏览器专用壳布局
  位置：dev/web/src/views/MobileShell.vue
  位置来源：plan "TASK-2.6 iOS Phone Layout"

  与 App.vue 中的 .mobile-only tab bar 不同：
  - App.vue 的 tab bar 是混合布局（桌面 + 移动端），简化版
  - MobileShell 是 **手机专用** 完整壳：
    * 顶部 fake 状态栏（时间 + 信号 + 电池）
    * viewport-safe 内容区（env(safe-area-inset-*)）
    * 5 个主 tab + 通知 + 设置 = 7 个底部按钮
    * 左右滑手势切换 tab（useSwipe）
    * 路由切换过渡动效（slide）
    * vh 修正（解决 100vh 含地址栏偏差）
    * 100dvh 优先（Dynamic Viewport Height，2023+ 浏览器支持）

  使用：
    // App.vue
    <MobileShell v-if="device.isMobile.value" />
    <DesktopLayout v-else />
-->
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useDevice } from '@/composables/useDevice'
import { useSwipe } from '@/composables/useSwipe'
import { installVhFix, isInstalledPWA, isIOSDevice, type Orientation } from '@/platform/phone-shell'
import { YJNotificationCenter, useNotificationStore } from '@shared/components'

const router = useRouter()
const route = useRoute()
const device = useDevice()
const { unreadCount } = useNotificationStore()

// === 1. Viewport vh 修正（地址栏 100vh 偏差） ===
let vhCleanup: (() => void) | null = null
onMounted(() => {
  vhCleanup = installVhFix()
})

// === 2. PWA / 设备 检测 ===
const isPWA = computed(() => isInstalledPWA())
const isIOS = ref(false)
onMounted(() => {
  isIOS.value = isIOSDevice()
})

// === 3. 方向检测（响应式） ===
const orientation = computed<Orientation>(() =>
  device.width.value > device.height.value ? 'landscape' : 'portrait',
)

// === 4. 状态栏时间（每 30s 刷新） ===
const statusTime = ref('')
let timeTimer: number | null = null
function updateTime() {
  const d = new Date()
  const h = String(d.getHours()).padStart(2, '0')
  const m = String(d.getMinutes()).padStart(2, '0')
  statusTime.value = `${h}:${m}`
}
onMounted(() => {
  updateTime()
  timeTimer = window.setInterval(updateTime, 30000)
})

// === 5. 底部 Tab 配置（5 主 + 通知 + 设置） ===
interface Tab {
  key: string
  label: string
  icon: string
  path: string
  /** 标记特殊 tab（不跳转路由，而是触发抽屉） */
  special?: 'notifications'
}

const tabs: Tab[] = [
  { key: 'chat', label: '运行', icon: '🚀', path: '/' },
  { key: 'env', label: '部署', icon: '⚙️', path: '/env' },
  { key: 'projects', label: '项目', icon: '📁', path: '/projects' },
  { key: 'github', label: 'GitHub', icon: '🐙', path: '/github' },
  { key: 'version', label: '更新', icon: '📋', path: '/version' },
]

const activeIndex = computed(() => {
  const i = tabs.findIndex((t) => t.path === route.path)
  return i >= 0 ? i : 0
})

function onTabClick(tab: Tab) {
  if (tab.special === 'notifications') {
    showNotifications.value = true
    return
  }
  if (route.path === tab.path) return
  router.push(tab.path)
}

// === 6. 左右滑切换 tab（useSwipe） ===
const contentRef = ref<HTMLElement | null>(null)
const { onSwipe } = useSwipe(contentRef, { minDistance: 60, maxDuration: 400 })

onSwipe((dir) => {
  if (dir === 'left') {
    const next = activeIndex.value + 1
    if (next < tabs.length) {
      const target = tabs[next]
      if (target) router.push(target.path)
    }
  } else if (dir === 'right') {
    const prev = activeIndex.value - 1
    if (prev >= 0) {
      const target = tabs[prev]
      if (target) router.push(target.path)
    }
  }
})

// === 7. 通知中心 + 设置入口 ===
const showNotifications = ref(false)

function openSettings() {
  if (route.path === '/settings') return
  router.push('/settings')
}

// === 8. 卸载清理 ===
onUnmounted(() => {
  if (vhCleanup) vhCleanup()
  if (timeTimer !== null) {
    clearInterval(timeTimer)
    timeTimer = null
  }
})
</script>

<template>
  <div
    class="mobile-shell"
    :class="{
      'is-pwa': isPWA,
      'is-ios': isIOS,
      'is-landscape': orientation === 'landscape',
    }"
  >
    <!-- 顶部 fake 状态栏（iOS 风格） -->
    <header class="status-bar">
      <span class="status-time">{{ statusTime }}</span>
      <span class="status-spacer" />
      <span class="status-icons">
        <span class="status-icon" title="信号">📶</span>
        <span class="status-icon" title="电池">🔋</span>
      </span>
    </header>

    <!-- 主内容区（带左右滑切换 tab） -->
    <main ref="contentRef" class="mobile-content">
      <router-view v-slot="{ Component }">
        <transition name="tab-slide" mode="out-in">
          <keep-alive>
            <component :is="Component" />
          </keep-alive>
        </transition>
      </router-view>
    </main>

    <!-- 底部 Tab Bar（iOS 风格毛玻璃） -->
    <nav class="tab-bar" role="navigation" aria-label="主导航">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="tab-item"
        :class="{ 'tab-item--active': route.path === tab.path }"
        @click="onTabClick(tab)"
      >
        <span class="tab-icon">{{ tab.icon }}</span>
        <span class="tab-label">{{ tab.label }}</span>
      </button>

      <!-- 通知按钮（特殊：触发抽屉） -->
      <button
        class="tab-item tab-item--bell"
        :class="{ 'tab-item--active': showNotifications }"
        @click="showNotifications = true"
      >
        <span class="tab-icon">🔔</span>
        <span class="tab-label">通知</span>
        <span
          v-if="unreadCount > 0"
          class="tab-badge"
        >{{ unreadCount > 99 ? '99+' : unreadCount }}</span>
      </button>

      <!-- 设置按钮 -->
      <button
        class="tab-item"
        :class="{ 'tab-item--active': route.path === '/settings' }"
        @click="openSettings"
      >
        <span class="tab-icon">🔧</span>
        <span class="tab-label">设置</span>
      </button>
    </nav>

    <!-- 通知中心抽屉 -->
    <YJNotificationCenter v-model:show="showNotifications" />
  </div>
</template>

<style scoped>
/* ============ 基础布局 ============ */
.mobile-shell {
  width: 100%;
  /* 100dvh 优先（2023+ 浏览器：动态 viewport 高度，自动跟随地址栏） */
  height: 100vh;
  height: 100dvh;
  /* 兼容 vh-fix 工具：fallback to --viewport-height * 100 */
  height: calc(var(--viewport-height, 1vh) * 100);
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: 'Microsoft YaHei', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  /* 防止 iOS Safari 整体下拉/上滑橡皮筋 */
  overscroll-behavior: none;
  -webkit-overflow-scrolling: touch;
}

/* ============ 顶部状态栏（fake iOS status bar） ============ */
.status-bar {
  flex-shrink: 0;
  height: 24px;
  padding: 0 12px;
  padding-top: var(--safe-top);
  /* 状态栏实际高度 = 24px + safe-top；用 box-sizing: content-box 避免 padding 影响 */
  box-sizing: content-box;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  user-select: none;
}

.status-time {
  font-variant-numeric: tabular-nums;
  font-family: var(--font-mono);
  letter-spacing: 0.02em;
}

.status-spacer {
  flex: 1;
}

.status-icons {
  display: flex;
  align-items: center;
  gap: 4px;
}

.status-icon {
  font-size: 12px;
  line-height: 1;
}

/* PWA standalone 模式：不需要 fake 状态栏（系统已经有） */
.mobile-shell.is-pwa .status-bar {
  display: none;
}

/* ============ 主内容区 ============ */
.mobile-content {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  position: relative;
  /* 适配 iOS Safari 底部 home indicator 干扰 */
  padding-bottom: var(--safe-bottom);
}

/* ============ 底部 Tab Bar（iOS 风格毛玻璃） ============ */
.tab-bar {
  flex-shrink: 0;
  height: var(--tab-bar-height);
  padding-bottom: var(--safe-bottom);
  background: rgba(20, 20, 20, 0.92);
  backdrop-filter: saturate(180%) blur(20px);
  -webkit-backdrop-filter: saturate(180%) blur(20px);
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: space-around;
  align-items: stretch;
  z-index: var(--z-fixed);
  font-family: inherit;
}

.tab-item {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 6px 2px;
  transition: color var(--transition-fast);
  position: relative;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  min-width: 0;
}

.tab-item:active {
  opacity: 0.6;
}

.tab-icon {
  font-size: 20px;
  line-height: 1;
  display: block;
}

.tab-label {
  font-size: 10px;
  line-height: 1;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}

.tab-item--active {
  color: var(--accent);
}

.tab-item--active .tab-icon {
  transform: scale(1.12);
  transition: transform var(--transition-fast);
}

/* 通知按钮的小红点 */
.tab-item--bell {
  position: relative;
}

.tab-badge {
  position: absolute;
  top: 2px;
  right: calc(50% - 18px);
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  background: var(--danger);
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  line-height: 16px;
  text-align: center;
  border-radius: 8px;
  box-shadow: 0 0 0 2px var(--bg-secondary);
  pointer-events: none;
}

/* ============ 路由切换过渡（tab 滑动） ============ */
.tab-slide-enter-active {
  transition: transform 220ms ease, opacity 220ms ease;
}
.tab-slide-leave-active {
  transition: transform 180ms ease, opacity 180ms ease;
}
.tab-slide-enter-from {
  opacity: 0;
  transform: translateX(20px);
}
.tab-slide-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}

/* ============ 横屏适配：tab 横向排列，保持 7 列 ============ */
.mobile-shell.is-landscape .status-bar {
  height: 18px;
  font-size: 11px;
}

.mobile-shell.is-landscape .tab-bar {
  height: 48px;
}

.mobile-shell.is-landscape .tab-icon {
  font-size: 18px;
}

.mobile-shell.is-landscape .tab-label {
  font-size: 9px;
}

/* ============ 减弱动画偏好（a11y） ============ */
@media (prefers-reduced-motion: reduce) {
  .tab-slide-enter-active,
  .tab-slide-leave-active {
    transition: opacity 80ms ease;
  }
  .tab-slide-enter-from,
  .tab-slide-leave-to {
    transform: none;
  }
}

/* ============ iOS PWA：隐藏 fake 状态栏（系统状态栏已存在） ============ */
.mobile-shell.is-ios.is-pwa .mobile-content {
  /* 给系统状态栏留空间 */
  padding-top: var(--safe-top);
}
</style>
