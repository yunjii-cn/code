<!--
  PushNotificationSettings.vue
  2026-06-09 TASK-4.3 引入：Web Push 通知设置 UI
  嵌入位置：SettingsView.vue（"通知"分类下）

  功能：
    - 显示当前推送能力
    - 订阅 / 取消订阅
    - 发送测试推送
    - 显示订阅统计
-->
<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { showToast } from 'vant'
import { usePushStore } from './push-store'

const store = usePushStore()
onMounted(async () => {
  await store.detect()
  await store.refreshServerStats()
})

const statusColor = computed(() => {
  switch (store.status) {
    case 'subscribed': return '#10b981'
    case 'denied': return '#ef4444'
    case 'not-subscribed': return '#f59e0b'
    case 'default': return '#888'
    case 'unsupported': return '#666'
  }
  return '#888'
})

const statusIcon = computed(() => {
  switch (store.status) {
    case 'subscribed': return '✓'
    case 'denied': return '✕'
    case 'not-subscribed': return '○'
    case 'default': return '○'
    case 'unsupported': return '∅'
  }
  return '○'
})

async function handleSubscribe() {
  if (store.permission === 'denied') {
    showToast('通知权限已被浏览器拒绝，请在浏览器设置中允许')
    return
  }
  const result = await store.subscribe()
  if (result.ok) {
    showToast({ type: 'success', message: '推送订阅已启用 🎉' })
  } else {
    showToast({ type: 'fail', message: result.error || '订阅失败' })
  }
}

async function handleUnsubscribe() {
  const result = await store.unsubscribe()
  if (result.ok) {
    showToast({ type: 'success', message: '已取消订阅' })
  } else {
    showToast({ type: 'fail', message: result.error || '取消失败' })
  }
}

async function handleTest() {
  showToast({ type: 'loading', message: '正在发送测试…', duration: 0 })
  const result = await store.sendTest()
  if (result) {
    showToast({
      type: result.delivered > 0 ? 'success' : 'fail',
      message: result.delivered > 0
        ? `测试推送成功（${result.delivered} 个设备${result.cleaned ? `，清理 ${result.cleaned} 个失效订阅` : ''}）`
        : '没有可用的订阅设备',
    })
  } else {
    showToast({ type: 'fail', message: store.errorMessage || '测试失败' })
  }
}
</script>

<template>
  <div class="push-settings">
    <h3 class="section-title">🔔 Web 推送通知</h3>
    <p class="section-desc">
      启用后，当感知引擎检测到重要事件时，会向你的设备发送推送通知。
    </p>

    <!-- 不支持 -->
    <div v-if="!store.supported" class="status-card unsupported">
      <div class="status-icon" :style="{ color: statusColor }">{{ statusIcon }}</div>
      <div class="status-content">
        <strong>当前浏览器不支持推送</strong>
        <p>请使用 Chrome / Edge / Firefox 浏览器；iOS 用户需要先将应用添加到主屏幕。</p>
        <p v-if="store.reason" class="status-hint">原因：{{ store.reason }}</p>
      </div>
    </div>

    <!-- 状态卡片 -->
    <div v-else class="status-card" :class="`is-${store.status}`">
      <div class="status-icon" :style="{ color: statusColor }">{{ statusIcon }}</div>
      <div class="status-content">
        <strong>状态：{{ store.statusText }}</strong>
        <p v-if="store.status === 'subscribed'">你将收到 warning / error 级别的感知通知。</p>
        <p v-else-if="store.status === 'denied'">通知权限已被浏览器拒绝。点击浏览器地址栏的锁图标 → 站点设置 → 通知 → 允许。</p>
        <p v-else-if="store.status === 'not-subscribed'">已获得通知权限，但未注册推送订阅。</p>
        <p v-else>点击下方按钮开启通知，浏览器会询问你的授权。</p>
      </div>
    </div>

    <!-- 操作按钮 -->
    <div v-if="store.supported" class="actions">
      <button
        v-if="store.status !== 'subscribed'"
        class="btn btn-primary"
        :disabled="store.isProcessing || store.status === 'denied'"
        @click="handleSubscribe"
      >
        {{ store.isProcessing ? '处理中…' : '🔔 启用推送' }}
      </button>
      <button
        v-else
        class="btn btn-danger"
        :disabled="store.isProcessing"
        @click="handleUnsubscribe"
      >
        🔕 关闭推送
      </button>
      <button
        v-if="store.status === 'subscribed'"
        class="btn btn-secondary"
        :disabled="store.isProcessing"
        @click="handleTest"
      >
        ✉️ 发送测试
      </button>
    </div>

    <!-- 服务端统计 -->
    <div v-if="store.serverStats" class="server-stats">
      <h4>服务端统计</h4>
      <div class="stats-grid">
        <div class="stat">
          <div class="stat-value">{{ store.serverStats.subscription_count }}</div>
          <div class="stat-label">总订阅</div>
        </div>
        <div class="stat">
          <div class="stat-value">{{ store.serverStats.active_count }}</div>
          <div class="stat-label">活跃订阅</div>
        </div>
        <div class="stat">
          <div class="stat-value">{{ store.serverStats.vapid_initialized ? '✓' : '✕' }}</div>
          <div class="stat-label">VAPID</div>
        </div>
      </div>
    </div>

    <!-- 最近测试结果 -->
    <div v-if="store.lastTestResult" class="test-result">
      <strong>最近测试：</strong>
      投递 {{ store.lastTestResult.delivered }} 个设备
      <span v-if="store.lastTestResult.failed > 0">，失败 {{ store.lastTestResult.failed }}</span>
      <span v-if="store.lastTestResult.cleaned > 0">，清理 {{ store.lastTestResult.cleaned }} 个失效订阅</span>
    </div>

    <!-- 提示信息 -->
    <div class="tips">
      <h4>使用提示</h4>
      <ul>
        <li>推送走标准 Web Push 协议（RFC 8030），无需第三方服务</li>
        <li>iOS 16.4+ 需要将站点"添加到主屏幕"后才能接收推送</li>
        <li>Safari 桌面版支持有限，建议使用 Chrome / Edge</li>
        <li>感知通知级别（warning / error）会触发推送，info 级别不会</li>
        <li>关闭浏览器后，仍可接收推送（依赖后台 SW）</li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.push-settings {
  padding: 16px;
  background: #1a1a2e;
  border: 1px solid #2a2a44;
  border-radius: 8px;
  margin-bottom: 16px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 6px;
  color: #e0e0e0;
}

.section-desc {
  font-size: 12px;
  color: #888;
  margin: 0 0 12px;
  line-height: 1.5;
}

.status-card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px;
  background: #0d0d0d;
  border: 1px solid #2a2a44;
  border-radius: 8px;
  margin-bottom: 12px;
}

.status-card.is-subscribed {
  border-color: rgba(16, 185, 129, 0.3);
  background: rgba(16, 185, 129, 0.05);
}

.status-card.is-denied {
  border-color: rgba(239, 68, 68, 0.3);
  background: rgba(239, 68, 68, 0.05);
}

.status-card.unsupported {
  opacity: 0.6;
}

.status-icon {
  font-size: 24px;
  line-height: 1;
  flex-shrink: 0;
}

.status-content {
  flex: 1;
  min-width: 0;
}

.status-content strong {
  display: block;
  font-size: 13px;
  margin-bottom: 4px;
  color: #e0e0e0;
}

.status-content p {
  font-size: 11px;
  color: #888;
  margin: 4px 0 0;
  line-height: 1.5;
}

.status-hint {
  font-style: italic;
  opacity: 0.8;
}

.actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.btn {
  flex: 1;
  min-width: 120px;
  padding: 10px 16px;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
  -webkit-tap-highlight-color: transparent;
}

.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn-primary {
  background: #42a5f5;
  color: #fff;
}

.btn-primary:hover:not(:disabled) {
  background: #1565c0;
}

.btn-secondary {
  background: #1a1a2e;
  color: #e0e0e0;
  border: 1px solid #2a2a44;
}

.btn-secondary:hover:not(:disabled) {
  background: #222240;
}

.btn-danger {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
  border: 1px solid rgba(239, 68, 68, 0.3);
}

.btn-danger:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.2);
}

.server-stats {
  margin-top: 12px;
  padding: 12px;
  background: #0d0d0d;
  border: 1px solid #2a2a44;
  border-radius: 6px;
}

.server-stats h4 {
  font-size: 12px;
  color: #888;
  margin: 0 0 8px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

.stat {
  text-align: center;
}

.stat-value {
  font-size: 18px;
  font-weight: 600;
  color: #42a5f5;
  line-height: 1.2;
}

.stat-label {
  font-size: 10px;
  color: #888;
  margin-top: 2px;
}

.test-result {
  margin-top: 12px;
  padding: 8px 12px;
  background: rgba(66, 165, 245, 0.05);
  border: 1px solid rgba(66, 165, 245, 0.2);
  border-radius: 6px;
  font-size: 11px;
  color: #ccc;
}

.tips {
  margin-top: 16px;
  padding: 12px;
  background: #0d0d0d;
  border: 1px dashed #2a2a44;
  border-radius: 6px;
}

.tips h4 {
  font-size: 12px;
  color: #888;
  margin: 0 0 8px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.tips ul {
  margin: 0;
  padding-left: 18px;
  font-size: 11px;
  color: #888;
  line-height: 1.7;
}

.tips li {
  margin-bottom: 4px;
}
</style>
