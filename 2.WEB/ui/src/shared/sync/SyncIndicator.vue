<!--
  SyncIndicator.vue
  2026-06-09 TASK-4.3 引入：同步状态徽标（带 pending 计数）
  位置：状态栏右侧
  状态：
    - 无 pending：隐藏
    - 同步中：旋转图标 + "正在同步 N 项"
    - 有 pending：徽标数字 + 离线黄色 / 在线蓝色
    - 有 failed：红色徽标 + 警告
  行为：
    - 点击 → 打开 SyncDetailDrawer
-->
<script setup lang="ts">
import { computed } from 'vue'
import { useSyncStore } from './sync-store'

const store = useSyncStore()

const totalCount = computed(() => store.stats.pending + store.stats.inFlight)
const status = computed(() => {
  if (store.isSyncing) return 'syncing'
  if (!store.isOnline) return 'offline'
  if (store.hasFailed) return 'failed'
  if (store.hasPending) return 'pending'
  return 'idle'
})

const title = computed(() => {
  switch (status.value) {
    case 'syncing':
      return `正在同步 ${store.stats.inFlight} 项`
    case 'offline':
      return `离线 - ${totalCount.value} 项待同步`
    case 'failed':
      return `${store.stats.failed} 项同步失败`
    case 'pending':
      return `${totalCount.value} 项待同步`
    default:
      return '已同步'
  }
})

function handleClick() {
  store.openDetail()
}
</script>

<template>
  <Transition name="sync-indicator">
    <button
      v-if="store.showIndicator"
      class="sync-indicator"
      :class="`is-${status}`"
      :title="title"
      @click="handleClick"
    >
      <!-- 同步中：旋转图标 -->
      <svg
        v-if="status === 'syncing'"
        class="sync-icon spinning"
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M21 12a9 9 0 11-6.219-8.56"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
        />
      </svg>

      <!-- 离线：云斜杠 -->
      <svg
        v-else-if="status === 'offline'"
        class="sync-icon"
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M7 18a5 5 0 010-10 7 7 0 0113 2.5"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
        />
        <path
          d="M2 2l20 20"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
        />
      </svg>

      <!-- 失败：警告 -->
      <svg
        v-else-if="status === 'failed'"
        class="sync-icon"
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M12 9v4m0 4h.01M5.07 19h13.86a2 2 0 001.74-3L13.74 4a2 2 0 00-3.48 0l-6.93 12a2 2 0 001.74 3z"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>

      <!-- 待同步：云上传 -->
      <svg
        v-else
        class="sync-icon"
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M7 16a4 4 0 01-.88-7.9 5 5 0 019.79 1.7M16 16l-4-4-4 4M12 12v8"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>

      <!-- 计数徽标 -->
      <span v-if="totalCount > 0 || store.stats.failed > 0" class="sync-badge">
        {{ store.stats.failed > 0 ? store.stats.failed : totalCount }}
      </span>
    </button>
  </Transition>
</template>

<style scoped>
.sync-indicator {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: #888;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
  -webkit-tap-highlight-color: transparent;
}

.sync-indicator:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #ccc;
}

.sync-icon {
  width: 16px;
  height: 16px;
}

.sync-indicator.is-syncing {
  color: #42a5f5;
}
.sync-indicator.is-syncing .spinning {
  animation: sync-rotate 1s linear infinite;
}

.sync-indicator.is-offline {
  color: #f59e0b;
}

.sync-indicator.is-failed {
  color: #ef4444;
}

.sync-indicator.is-pending {
  color: #42a5f5;
}

.sync-badge {
  position: absolute;
  top: -2px;
  right: -2px;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  font-size: 10px;
  font-weight: 600;
  line-height: 16px;
  text-align: center;
  border-radius: 8px;
  background: #42a5f5;
  color: #fff;
  box-shadow: 0 0 0 2px #0d0d0d;
}

.sync-indicator.is-failed .sync-badge {
  background: #ef4444;
}

.sync-indicator.is-offline .sync-badge {
  background: #f59e0b;
}

@keyframes sync-rotate {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.sync-indicator-enter-active,
.sync-indicator-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}
.sync-indicator-enter-from,
.sync-indicator-leave-to {
  opacity: 0;
  transform: scale(0.8);
}
</style>
