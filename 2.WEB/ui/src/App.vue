<script setup lang="ts">
import { computed, ref, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
// 2026-06-08 TASK-1.9 引入：设计系统 Toast 挂载
// 2026-06-09 TASK-2.7 引入：统一通知中心（铃铛 + 未读 badge + 抽屉）
// 2026-06-09 TASK-2.8 引入：命令面板（Ctrl+K）+ 全局快捷键
import { YJToast, YJNotificationCenter, useNotificationStore, CommandPalette, ErrorBoundary } from '@shared/components'
// 2026-06-09 TASK-4.3 引入：PWA 更新横幅
import { UpdateBanner, usePwaStore } from '@shared/pwa'
// 2026-06-09 TASK-4.3 引入：离线同步队列（状态栏指示器 + 详情抽屉）
import { useSyncStore, SyncIndicator, SyncDetailDrawer } from '@shared/sync'
import { useGlobalShortcuts } from '@/composables/useGlobalShortcuts'
// 2026-06-10 TASK-2.6 引入：手机端专用壳布局 + 设备检测
import { useDevice } from '@/composables/useDevice'
import MobileShell from '@/views/MobileShell.vue'

const router = useRouter()
const route = useRoute()

// 2026-06-10 TASK-2.6：设备类型检测
const device = useDevice()
const useMobileLayout = computed(() => device.isMobile.value)

// 2026-06-09 TASK-2.7：通知中心开关 + 未读数量
const showNotificationCenter = ref(false)
const { unreadCount } = useNotificationStore()
const pwaStore = usePwaStore()
// 2026-06-09 TASK-4.3：同步队列
const syncStore = useSyncStore()
syncStore.init()

// 2026-06-09 TASK-2.8：注册全局快捷键 + 暴露命令面板开关
const { showCommandPalette } = useGlobalShortcuts()

// 2026-06-09 TASK-2.8：监听 yj:open-notifications 自定义事件，转为打开通知中心
function onYjOpenNotifications() {
  showNotificationCenter.value = true
}

// 2026-06-16 修复：路由切换时滚动到顶部，让用户能看到新页面的内容
watch(
  () => route.path,
  () => {
    nextTick(() => {
      const main = document.querySelector('.app-content')
      if (main) {
        main.scrollTo({ top: 0, behavior: 'smooth' })
      }
    })
  }
)
onMounted(() => {
  window.addEventListener('yj:open-notifications', onYjOpenNotifications)
})
onUnmounted(() => {
  window.removeEventListener('yj:open-notifications', onYjOpenNotifications)
})

const navTabs = [
  { name: 'chat', path: '/', icon: 'chat-o' },
  { name: 'env', path: '/env', icon: 'setting-o' },
  { name: 'version', path: '/version', icon: 'upgrade' },
  { name: 'projects', path: '/projects', icon: 'folder-o' },
  { name: 'github', path: '/github', icon: 'github-o' },
  { name: 'notifications', path: '__notifications__', icon: 'bell' },
  { name: 'settings', path: '/settings', icon: 'setting-o' },
]

const labels: Record<string, string> = {
  chat: '🚀 运行服务',
  env: '⚙️ 部署维护',
  version: '📋 软件更新',
  projects: '📁 项目管理',
  github: '🐙 GitHub',
  notifications: '🔔 通知',
  settings: '🔧 系统设置',
}

// 2026-06-10 TASK-2.6：mobileLabels 已废弃（mobile tab 移到 MobileShell.vue）

const activeTab = computed(() => {
  // 2026-06-16 修复：activeTab 匹配逻辑增强
  // 处理根路径 / 的边界情况：route.path 可能是 '/' 或 ''
  // 使用 startsWith 处理子路由（如 /projects/123）
  const currentPath = route.path || '/'
  // 优先精确匹配
  const exact = navTabs.find((t) => t.path === currentPath)
  if (exact) return exact.name
  // 其次按前缀匹配（仅对 projects/settings/env/version/github 等非根路径）
  const prefix = navTabs.find(
    (t) => t.path !== '/' && t.path !== '__notifications__' && currentPath.startsWith(t.path)
  )
  if (prefix) return prefix.name
  return 'chat'
})

function onTabClick(name: string) {
  const tab = navTabs.find((t) => t.name === name)
  if (!tab) return
  // 2026-06-09 TASK-2.7：notifications 触发抽屉，不跳转路由
  if (tab.path === '__notifications__') {
    showNotificationCenter.value = true
    return
  }
  // 2026-06-16：避免重复点击同一路由触发无意义的导航
  if (route.path === tab.path) {
    // 即使是当前路由，也滚动到顶部，提示用户已经切换过
    const main = document.querySelector('.app-content')
    if (main) main.scrollTo({ top: 0, behavior: 'smooth' })
    return
  }
  router.push(tab.path).catch((err) => {
    console.warn('[App] 路由跳转失败:', err)
  })
}
</script>

<template>
  <!-- 2026-06-10 TASK-2.6 引入：响应式分流（手机用 MobileShell，桌面用传统布局） -->
  <!-- 全局组件（所有模式都挂载） -->
  <UpdateBanner />
  <YJToast />
  <CommandPalette v-model:show="showCommandPalette" />
  <SyncDetailDrawer />

  <!-- 手机端：MobileShell 自带 nav / content / tab bar / 通知中心 -->
  <MobileShell v-if="useMobileLayout" />

  <!-- 桌面端：传统水平 nav + 状态栏布局 -->
  <div v-else class="app-shell">
    <!-- 桌面端：顶部水平 nav -->
    <header class="app-nav">
      <div class="nav-tabs">
        <button
          v-for="tab in navTabs"
          :key="tab.name"
          class="nav-tab"
          :class="{ 'nav-tab--active': activeTab === tab.name }"
          @click="onTabClick(tab.name)"
        >
          {{ labels[tab.name] }}
        </button>
      </div>
      <!-- 2026-06-09 TASK-2.7：桌面端右上角铃铛按钮 + 未读 badge -->
      <button
        class="nav-bell"
        :class="{ 'nav-bell--has-unread': unreadCount > 0 }"
        title="通知中心"
        @click="showNotificationCenter = true"
      >
        <span class="nav-bell-icon">🔔</span>
        <span v-if="unreadCount > 0" class="nav-bell-badge">{{ unreadCount > 99 ? '99+' : unreadCount }}</span>
      </button>
    </header>

    <main class="app-content">
      <!-- 2026-06-09 TASK-4.7：ErrorBoundary 包裹路由视图 -->
      <ErrorBoundary>
        <router-view v-slot="{ Component }">
          <!-- 2026-06-09 TASK-4.7：路由切换过渡动效 -->
          <transition name="route-fade" mode="out-in">
            <keep-alive>
              <component :is="Component" />
            </keep-alive>
          </transition>
        </router-view>
      </ErrorBoundary>
    </main>

    <!-- 2026-06-09 TASK-2.7 引入：统一通知中心抽屉（仅桌面端，MobileShell 自带） -->
    <YJNotificationCenter v-model:show="showNotificationCenter" />

    <footer class="app-statusbar">
      <span class="statusbar-text">云集智能编程工作站</span>
      <!-- 2026-06-09 TASK-4.3 引入：状态栏 PWA 指示器 -->
      <span
        v-if="pwaStore.isInstalled || pwaStore.canInstall || !pwaStore.isOnline"
        class="statusbar-pwa"
        :title="pwaStore.isOnline ? (pwaStore.isInstalled ? '已安装 PWA' : '可安装 PWA') : '离线模式'"
      >
        <span
          class="statusbar-pwa-dot"
          :class="{
            'is-online': pwaStore.isOnline,
            'is-offline': !pwaStore.isOnline,
            'is-installed': pwaStore.isInstalled,
            'can-install': pwaStore.canInstall && !pwaStore.isInstalled,
          }"
        />
        <span v-if="!pwaStore.isOnline">离线</span>
        <span v-else-if="pwaStore.isInstalled">PWA</span>
        <span v-else-if="pwaStore.canInstall">安装</span>
      </span>
      <span class="statusbar-version">v{{ new Date().getFullYear() }}.{{ String(new Date().getMonth() + 1).padStart(2, '0') }}.{{ String(new Date().getDate()).padStart(2, '0') }}</span>
      <!-- 2026-06-09 TASK-4.3 引入：同步队列状态指示器（点击打开详情抽屉） -->
      <span class="statusbar-sync-wrap">
        <SyncIndicator />
      </span>
    </footer>
  </div>
</template>

<script lang="ts">
// 2026-06-10 TASK-2.6：emojiForTab 已废弃（mobile tab bar 移到 MobileShell）
// 2026-06-10 TASK-2.6：mobileLabels 已废弃（mobileLabels 移到 MobileShell）
// 占位：原 <script> 块保留以避免 Vue 编译器报错
export default {}
</script>

<style scoped>
.app-shell {
  width: 100%;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: var(--bg-primary);
  color: var(--text-primary);
  font-family: 'Microsoft YaHei', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

/* ============ 2026-06-10 TASK-2.6 已重构：响应式可见性 ============ */
/* 旧的 .desktop-only / .mobile-only / .tablet-only CSS 类已废弃，
   现在用 v-if="useMobileLayout" 在模板层分流（App.vue 管桌面，MobileShell 管手机） */

/* 2026-06-10 TASK-2.6：移除旧 mobile tab bar CSS（已迁到 MobileShell.vue）
   包括 .app-tabbar / .tabbar-item / .tabbar-icon / .tabbar-label /
   .tabbar-item--active / .tabbar-item--bell / .tabbar-bell-badge */

.app-nav {
  flex-shrink: 0;
  height: 38px;
  background-color: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: stretch;
  justify-content: space-between;
  padding: 0 8px;
}

.nav-tabs {
  display: flex;
  align-items: stretch;
  gap: 1px;
  height: 100%;
}

/* ============ 2026-06-09 TASK-2.7 引入：桌面端右上角铃铛按钮 ============ */
.nav-bell {
  position: relative;
  background: transparent;
  color: #999;
  border: none;
  border-radius: 4px;
  padding: 4px 8px;
  font-size: 14px;
  cursor: pointer;
  transition: color 0.15s, background-color 0.15s;
  display: flex;
  align-items: center;
  justify-content: center;
  align-self: center;
  -webkit-tap-highlight-color: transparent;
  font-family: inherit;
}

.nav-bell:hover {
  color: #fff;
  background-color: #252525;
}

.nav-bell-icon {
  font-size: 16px;
  line-height: 1;
}

.nav-bell-badge {
  position: absolute;
  top: -2px;
  right: -4px;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  background: #ef4444;
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  line-height: 16px;
  text-align: center;
  border-radius: 8px;
  box-shadow: 0 0 0 2px var(--bg-secondary);
  pointer-events: none;
}

.nav-bell--has-unread .nav-bell-icon {
  animation: nav-bell-shake 0.6s ease-in-out;
}

@keyframes nav-bell-shake {
  0%, 100% { transform: rotate(0deg); }
  20% { transform: rotate(-15deg); }
  40% { transform: rotate(12deg); }
  60% { transform: rotate(-8deg); }
  80% { transform: rotate(4deg); }
}

.nav-tab {
  background: transparent;
  color: #999;
  border: none;
  border-bottom: 3px solid transparent;
  border-radius: 0;
  padding: 6px 14px 8px;
  font-size: 12px;
  font-weight: normal;
  cursor: pointer;
  transition: color 0.15s, background-color 0.15s, border-color 0.15s;
  white-space: nowrap;
  font-family: inherit;
  position: relative;
}

.nav-tab:hover {
  color: #fff;
  background-color: #252525;
}

.nav-tab--active {
  color: #fff;
  background-color: rgba(59, 130, 246, 0.08);
  border-bottom-color: #3b82f6;
  font-weight: 600;
}

.nav-tab--active:hover {
  background-color: rgba(59, 130, 246, 0.12);
}

.app-content {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.app-statusbar {
  flex-shrink: 0;
  height: 26px;
  background-color: #1a1a1a;
  border-top: 1px solid #2a2a2a;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
}

.statusbar-text {
  color: #666;
  font-size: 10px;
}

.statusbar-version {
  color: #555;
  font-size: 10px;
  font-family: Consolas, monospace;
}

/* ============ 2026-06-09 TASK-4.3 引入：状态栏 PWA 指示器 ============ */
.statusbar-pwa {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  color: #888;
  font-family: Consolas, monospace;
  user-select: none;
}

.statusbar-pwa-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  display: inline-block;
  transition: background-color 0.2s, box-shadow 0.2s;
}

.statusbar-pwa-dot.is-online.is-installed {
  background: #10b981;
  box-shadow: 0 0 4px rgba(16, 185, 129, 0.6);
}

.statusbar-pwa-dot.is-online.can-install {
  background: #42a5f5;
  box-shadow: 0 0 4px rgba(66, 165, 245, 0.6);
  animation: pwa-pulse 2s ease-in-out infinite;
}

.statusbar-pwa-dot.is-offline {
  background: #f59e0b;
  box-shadow: 0 0 4px rgba(245, 158, 11, 0.6);
}

@keyframes pwa-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* 2026-06-10 TASK-2.6：mobile tab bar 样式已迁到 MobileShell.vue */

</style>