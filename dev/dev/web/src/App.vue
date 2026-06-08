<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
// 2026-06-08 TASK-1.9 引入：设计系统 Toast 挂载
// 2026-06-09 TASK-2.7 引入：统一通知中心（铃铛 + 未读 badge + 抽屉）
// 2026-06-09 TASK-2.8 引入：命令面板（Ctrl+K）+ 全局快捷键
import { YJToast, YJNotificationCenter, useNotificationStore, CommandPalette } from '@shared/components'
import { useGlobalShortcuts } from '@/composables/useGlobalShortcuts'

const router = useRouter()
const route = useRoute()

// 2026-06-09 TASK-2.7：通知中心开关 + 未读数量
const showNotificationCenter = ref(false)
const { unreadCount } = useNotificationStore()

// 2026-06-09 TASK-2.8：注册全局快捷键 + 暴露命令面板开关
const { showCommandPalette } = useGlobalShortcuts()

// 2026-06-09 TASK-2.8：监听 yj:open-notifications 自定义事件，转为打开通知中心
function onYjOpenNotifications() {
  showNotificationCenter.value = true
}
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

const mobileLabels: Record<string, string> = {
  chat: '运行',
  env: '部署',
  version: '更新',
  projects: '项目',
  github: 'GitHub',
  notifications: '通知',
  settings: '设置',
}

const activeTab = computed(() => {
  // 2026-06-09 TASK-2.7：notifications 不是真实路由，路由层不会匹配到
  const found = navTabs.find((t) => t.path === route.path)
  return found?.name || 'chat'
})

function onTabClick(name: string) {
  const tab = navTabs.find((t) => t.name === name)
  if (!tab) return
  // 2026-06-09 TASK-2.7：notifications 触发抽屉，不跳转路由
  if (tab.path === '__notifications__') {
    showNotificationCenter.value = true
    return
  }
  router.push(tab.path)
}
</script>

<template>
  <div class="app-shell">
    <!-- 桌面端：顶部水平 nav -->
    <header class="app-nav desktop-only">
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
        class="nav-bell desktop-only"
        :class="{ 'nav-bell--has-unread': unreadCount > 0 }"
        title="通知中心"
        @click="showNotificationCenter = true"
      >
        <span class="nav-bell-icon">🔔</span>
        <span v-if="unreadCount > 0" class="nav-bell-badge">{{ unreadCount > 99 ? '99+' : unreadCount }}</span>
      </button>
    </header>

    <!-- 移动端：底部 iOS 风格 tab bar -->
    <nav class="app-tabbar mobile-only" role="navigation">
      <button
        v-for="tab in navTabs"
        :key="tab.name"
        class="tabbar-item"
        :class="{ 'tabbar-item--active': activeTab === tab.name, 'tabbar-item--bell': tab.name === 'notifications' }"
        @click="onTabClick(tab.name)"
      >
        <span class="tabbar-icon">{{ emojiForTab(tab.name) }}</span>
        <span class="tabbar-label">{{ mobileLabels[tab.name] }}</span>
        <!-- 2026-06-09 TASK-2.7：移动端通知 tab 的未读小红点 -->
        <span
          v-if="tab.name === 'notifications' && unreadCount > 0"
          class="tabbar-bell-badge"
        >{{ unreadCount > 99 ? '99+' : unreadCount }}</span>
      </button>
    </nav>

    <main class="app-content">
      <router-view v-slot="{ Component }">
        <keep-alive>
          <component :is="Component" />
        </keep-alive>
      </router-view>
    </main>

    <!-- 2026-06-08 TASK-1.9 引入：全局通知挂载点 -->
    <YJToast />

    <!-- 2026-06-09 TASK-2.7 引入：统一通知中心抽屉 -->
    <YJNotificationCenter v-model:show="showNotificationCenter" />

    <!-- 2026-06-09 TASK-2.8 引入：命令面板（Ctrl+K 触发） -->
    <CommandPalette v-model:show="showCommandPalette" />

    <footer class="app-statusbar desktop-only">
      <span class="statusbar-text">云集智能编程工作站</span>
      <span class="statusbar-version">v{{ new Date().getFullYear() }}.{{ String(new Date().getMonth() + 1).padStart(2, '0') }}.{{ String(new Date().getDate()).padStart(2, '0') }}</span>
    </footer>
  </div>
</template>

<script lang="ts">
// 2026-06-08 TASK-2.6：移动端 tab bar emoji 映射
// 2026-06-09 TASK-2.7：增加 notifications
function emojiForTab(name: string): string {
  switch (name) {
    case 'chat': return '🚀'
    case 'env': return '⚙️'
    case 'version': return '📋'
    case 'projects': return '📁'
    case 'github': return '🐙'
    case 'notifications': return '🔔'
    case 'settings': return '🔧'
    default: return '•'
  }
}
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

/* ============ 2026-06-08 TASK-2.6 引入：响应式可见性 ============ */
.desktop-only { display: flex; }
.mobile-only { display: none; }

@media (max-width: 768px) {
  .desktop-only { display: none !important; }
  .mobile-only { display: flex; }
}

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
  padding: 4px 14px 6px;
  font-size: 12px;
  font-weight: normal;
  cursor: pointer;
  transition: color 0.15s, background-color 0.15s;
  white-space: nowrap;
  font-family: inherit;
}

.nav-tab:hover {
  color: #fff;
  background-color: #252525;
}

.nav-tab--active {
  color: #fff;
  border-bottom-color: #3b82f6;
}

.nav-tab--active:hover {
  background-color: #252525;
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

/* ============ 2026-06-08 TASK-2.6 引入：移动端 iOS 风格底部 tab bar ============ */
.app-tabbar {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  height: var(--tab-bar-total);
  background: rgba(20, 20, 20, 0.92);
  backdrop-filter: saturate(180%) blur(20px);
  -webkit-backdrop-filter: saturate(180%) blur(20px);
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: space-around;
  align-items: stretch;
  z-index: var(--z-fixed);
  padding-bottom: var(--safe-bottom);
  font-family: inherit;
}

.tabbar-item {
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
  padding: 6px 4px;
  transition: color var(--transition-fast);
  position: relative;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
}

.tabbar-item:active {
  opacity: 0.6;
}

.tabbar-icon {
  font-size: 22px;
  line-height: 1;
}

.tabbar-label {
  font-size: 10px;
  line-height: 1;
  font-weight: 500;
}

.tabbar-item--active {
  color: var(--accent);
}

.tabbar-item--active .tabbar-icon {
  transform: scale(1.1);
  transition: transform var(--transition-fast);
}

/* ============ 2026-06-09 TASK-2.7 引入：移动端通知 tab 右上角小红点 ============ */
.tabbar-item--bell {
  position: relative;
}

.tabbar-bell-badge {
  position: absolute;
  top: 4px;
  right: 18%;
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

/* 移动端：主内容区需要为底部 tab bar 留出空间 */
@media (max-width: 768px) {
  .app-shell {
    padding-bottom: var(--tab-bar-total);
  }
  .app-content {
    /* 内容不被 tab bar 遮挡 */
    padding-bottom: var(--tab-bar-total);
  }
}

/* 移动端：顶部 safe-area 留白 */
@media (max-width: 768px) {
  .app-content::before {
    content: '';
    display: block;
    height: var(--safe-top);
    background: var(--bg-primary);
  }
}
</style>