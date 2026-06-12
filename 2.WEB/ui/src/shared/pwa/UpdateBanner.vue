<!--
  UpdateBanner.vue
  2026-06-09 TASK-4.3 引入：PWA 更新横幅 + 离线就绪提示
  出现时机：
    - 新版本 SW 等待激活 → 顶部显示"新版本可用"横幅
    - SW 首次激活完成 → 底部 Toast"已可离线使用"
  设计：暗色 + 蓝色强调色 + 滑入动效
-->
<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { showToast } from 'vant'
import { usePwaStore } from '@shared/pwa/pwa-store'

const store = usePwaStore()

const dismissed = ref(false)

// 监听 offlineReady 变化 → 触发 Toast
watch(
  () => store.offlineReady,
  (ready) => {
    if (ready) {
      showToast({
        type: 'success',
        message: '已可离线使用 🎉',
        duration: 3000,
      })
    }
  }
)

const visible = computed(() => store.shouldShowUpdateBanner && !dismissed.value)

function handleUpdate() {
  store.applyUpdate()
}

function handleDismiss() {
  dismissed.value = true
  // 24 小时内不再提示
  try {
    localStorage.setItem('yj-pwa-update-dismissed', String(Date.now()))
  } catch {}
}

function handleClearCache() {
  store.clearCache()
  showToast({ type: 'success', message: '缓存已清理' })
}

// 检查上次 dismiss 时间
try {
  const last = localStorage.getItem('yj-pwa-update-dismissed')
  if (last) {
    const elapsed = Date.now() - parseInt(last, 10)
    if (elapsed < 24 * 60 * 60 * 1000) {
      dismissed.value = true
    }
  }
} catch {}
</script>

<template>
  <Transition name="update-banner">
    <div v-if="visible" class="update-banner" role="alert">
      <div class="update-content">
        <span class="update-icon">🚀</span>
        <div class="update-text">
          <strong>新版本可用</strong>
          <span class="update-hint">点击刷新即可使用最新功能</span>
        </div>
      </div>
      <div class="update-actions">
        <button class="btn btn-ghost" @click="handleClearCache" title="清理缓存">🗑️</button>
        <button class="btn btn-dismiss" @click="handleDismiss" title="稍后">✕</button>
        <button class="btn btn-primary" @click="handleUpdate">立即更新</button>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.update-banner {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  background: linear-gradient(135deg, #1e3a8a 0%, #1e40af 100%);
  color: #fff;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  font-size: 13px;
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.update-content {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
  min-width: 0;
}

.update-icon {
  font-size: 22px;
  flex-shrink: 0;
}

.update-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.update-text strong {
  font-weight: 600;
  font-size: 14px;
  line-height: 1.3;
}

.update-hint {
  font-size: 11px;
  opacity: 0.85;
  line-height: 1.3;
  margin-top: 2px;
}

.update-actions {
  display: flex;
  gap: 6px;
  align-items: center;
  flex-shrink: 0;
}

.btn {
  border: none;
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
  -webkit-tap-highlight-color: transparent;
  white-space: nowrap;
}

.btn-primary {
  background: #fff;
  color: #1e3a8a;
  padding: 6px 14px;
  font-weight: 600;
}

.btn-primary:hover {
  background: #f0f0f0;
  transform: translateY(-1px);
}

.btn-ghost,
.btn-dismiss {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
  padding: 6px 8px;
  font-size: 14px;
  min-width: 32px;
}

.btn-ghost:hover,
.btn-dismiss:hover {
  background: rgba(255, 255, 255, 0.2);
}

/* 移动端调整 */
@media (max-width: 600px) {
  .update-banner {
    padding: 8px 12px;
    font-size: 12px;
  }
  .update-hint { display: none; }
  .update-text strong { font-size: 13px; }
  .btn-ghost, .btn-dismiss { display: none; }
}

/* 滑入动效 */
.update-banner-enter-active,
.update-banner-leave-active {
  transition: transform 0.3s ease, opacity 0.3s ease;
}
.update-banner-enter-from,
.update-banner-leave-to {
  transform: translateY(-100%);
  opacity: 0;
}
</style>
