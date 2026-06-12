<!--
  PwaIndicator.vue
  2026-06-09 TASK-4.3 引入：PWA 状态指示器
  显示：在线/离线、新版本可更新、可安装提示
-->
<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { usePwaStore } from '@shared/pwa/pwa-store'

const store = usePwaStore()

const statusText = computed(() => {
  if (!store.isOnline) return '离线模式'
  if (store.updateAvailable) return '有更新可用'
  if (store.swReady) return '已就绪'
  return '初始化中'
})

const statusColor = computed(() => {
  if (!store.isOnline) return '#f59e0b'
  if (store.updateAvailable) return '#8b5cf6'
  if (store.swReady) return '#10b981'
  return '#94a3b8'
})

async function handleInstall() {
  const result = await store.promptInstall()
  if (result === 'accepted') {
    showToast({ type: 'success', message: '正在安装...' })
  } else if (result === 'dismissed') {
    showToast('已取消安装')
  } else {
    showToast({ type: 'fail', message: '当前不可安装' })
  }
}

async function handleUpdate() {
  try {
    await showConfirmDialog({
      title: '发现新版本',
      message: '是否立即更新？更新后页面将自动刷新。',
    })
  } catch {
    return
  }
  await store.applyUpdate()
}

async function handleClearCache() {
  try {
    await showConfirmDialog({
      title: '清理缓存？',
      message: '将清空所有 PWA 缓存（不影响你的数据）',
    })
  } catch {
    return
  }
  await store.clearCache()
  showToast({ type: 'success', message: '缓存已清理' })
}
</script>

<template>
  <div class="pwa-indicator" :class="{ offline: store.offlineMode }">
    <div class="status-row">
      <span class="dot" :style="{ background: statusColor }" />
      <span class="text" :style="{ color: statusColor }">{{ statusText }}</span>
      <span v-if="store.swVersion" class="version">v{{ store.swVersion }}</span>
    </div>

    <div v-if="store.canInstall" class="action-row">
      <button class="btn btn-primary" @click="handleInstall">
        📥 安装到桌面
      </button>
    </div>

    <div v-if="store.updateAvailable" class="action-row">
      <button class="btn btn-primary" @click="handleUpdate">
        🔄 立即更新
      </button>
    </div>

    <div v-if="store.swReady" class="action-row">
      <button class="btn btn-ghost" @click="handleClearCache">
        🗑️ 清理缓存
      </button>
    </div>
  </div>
</template>

<style scoped>
.pwa-indicator {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 12px;
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 6px;
  font-size: 12px;
}

.pwa-indicator.offline {
  border-color: #f59e0b;
  background: rgba(245, 158, 11, 0.05);
}

.status-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  box-shadow: 0 0 6px currentColor;
}

.text {
  font-weight: 500;
}

.version {
  margin-left: auto;
  font-family: monospace;
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
}

.action-row {
  display: flex;
  gap: 6px;
}

.btn {
  flex: 1;
  font-size: 11px;
  padding: 4px 8px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
}

.btn-primary {
  background: #3b82f6;
  color: #fff;
}

.btn-primary:hover {
  background: #2563eb;
}

.btn-ghost {
  background: #334155;
  color: #cbd5e1;
}

.btn-ghost:hover {
  background: #475569;
}
</style>
