<script setup lang="ts">
import { computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
// 2026-06-08 TASK-1.9 引入：设计系统 Toast 挂载
import { YJToast } from '@shared/components'

const router = useRouter()
const route = useRoute()

const navTabs = [
  { name: 'chat', path: '/', icon: 'chat-o' },
  { name: 'env', path: '/env', icon: 'setting-o' },
  { name: 'version', path: '/version', icon: 'upgrade' },
  { name: 'projects', path: '/projects', icon: 'folder-o' },
  { name: 'github', path: '/github', icon: 'github-o' },
  { name: 'settings', path: '/settings', icon: 'setting-o' },
]

const labels: Record<string, string> = {
  chat: '🚀 运行服务',
  env: '⚙️ 部署维护',
  version: '📋 软件更新',
  projects: '📁 项目管理',
  github: '🐙 GitHub',
  settings: '🔧 系统设置',
}

const mobileLabels: Record<string, string> = {
  chat: '运行',
  env: '部署',
  version: '更新',
  projects: '项目',
  github: 'GitHub',
  settings: '设置',
}

const activeTab = computed(() => {
  const found = navTabs.find((t) => t.path === route.path)
  return found?.name || 'chat'
})

function onTabClick(name: string) {
  const tab = navTabs.find((t) => t.name === name)
  if (tab) router.push(tab.path)
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
    </header>

    <!-- 移动端：底部 iOS 风格 tab bar -->
    <nav class="app-tabbar mobile-only" role="navigation">
      <button
        v-for="tab in navTabs"
        :key="tab.name"
        class="tabbar-item"
        :class="{ 'tabbar-item--active': activeTab === tab.name }"
        @click="onTabClick(tab.name)"
      >
        <span class="tabbar-icon">{{ emojiForTab(tab.name) }}</span>
        <span class="tabbar-label">{{ mobileLabels[tab.name] }}</span>
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

    <footer class="app-statusbar desktop-only">
      <span class="statusbar-text">云集智能编程工作站</span>
      <span class="statusbar-version">v{{ new Date().getFullYear() }}.{{ String(new Date().getMonth() + 1).padStart(2, '0') }}.{{ String(new Date().getDate()).padStart(2, '0') }}</span>
    </footer>
  </div>
</template>

<script lang="ts">
// 2026-06-08 TASK-2.6：移动端 tab bar emoji 映射
function emojiForTab(name: string): string {
  switch (name) {
    case 'chat': return '🚀'
    case 'env': return '⚙️'
    case 'version': return '📋'
    case 'projects': return '📁'
    case 'github': return '🐙'
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
  padding: 0 8px;
}

.nav-tabs {
  display: flex;
  align-items: stretch;
  gap: 1px;
  height: 100%;
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