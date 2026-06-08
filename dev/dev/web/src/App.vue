<script setup lang="ts">
import { computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
// 2026-06-08 TASK-1.9 引入：设计系统 Toast 挂载
import { YJToast } from '@shared/components'

const router = useRouter()
const route = useRoute()

const navTabs = [
  { name: 'chat', path: '/' },
  { name: 'env', path: '/env' },
  { name: 'version', path: '/version' },
  { name: 'projects', path: '/projects' },
  { name: 'settings', path: '/settings' },
]

const labels: Record<string, string> = {
  chat: '🚀 运行服务',
  env: '⚙️ 部署维护',
  version: '📋 软件更新',
  projects: '📁 项目管理',
  settings: '🔧 系统设置',
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
    </header>

    <main class="app-content">
      <router-view v-slot="{ Component }">
        <keep-alive>
          <component :is="Component" />
        </keep-alive>
      </router-view>
    </main>

    <!-- 2026-06-08 TASK-1.9 引入：全局通知挂载点 -->
    <YJToast />

    <footer class="app-statusbar">
      <span class="statusbar-text">云集智能编程工作站</span>
      <span class="statusbar-version">v{{ new Date().getFullYear() }}.{{ String(new Date().getMonth() + 1).padStart(2, '0') }}.{{ String(new Date().getDate()).padStart(2, '0') }}</span>
    </footer>
  </div>
</template>

<style scoped>
.app-shell {
  width: 100%;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: #0d0d0d;
  color: #f0f0f0;
  font-family: 'Microsoft YaHei', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.app-nav {
  flex-shrink: 0;
  height: 38px;
  background-color: #1a1a1a;
  border-bottom: 1px solid #2a2a2a;
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
</style>