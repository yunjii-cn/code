<!--
  TailscaleSettings.vue
  2026-06-09 TASK-4.2 引入：Tailscale 远程访问设置面板
  显示 Tailscale 状态、配对码、远程访问 URL
-->
<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { tailscaleApi, type TailscaleSummary, type TailscaleInfo } from '@/api'

const summary = ref<TailscaleSummary | null>(null)
const loading = ref(false)
const regenerating = ref(false)

const STATUS_META: Record<string, { label: string; color: string; icon: string }> = {
  unknown:              { label: '未知',       color: '#94a3b8', icon: '❓' },
  not_installed:        { label: '未安装',     color: '#6b7280', icon: '⚠️' },
  installed_stopped:    { label: '已停止',     color: '#f59e0b', icon: '⏸' },
  running_logged_out:   { label: '未登录',     color: '#f59e0b', icon: '🔓' },
  running_online:       { label: '在线',       color: '#10b981', icon: '✅' },
  error:                { label: '异常',       color: '#ef4444', icon: '❌' },
}

const info = computed<TailscaleInfo | null>(() => summary.value?.tailscale || null)
const statusMeta = computed(() => {
  if (!info.value) return null
  return STATUS_META[info.value.status] || STATUS_META.unknown
})
const pairingCode = computed(() => summary.value?.pairing_code || null)
const remoteHint = computed(() => summary.value?.remote_hint || null)

async function load() {
  loading.value = true
  try {
    const resp = await tailscaleApi.summary()
    summary.value = resp.data
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  } finally {
    loading.value = false
  }
}

async function handleRegenerate() {
  try {
    await showConfirmDialog({
      title: '重新生成配对码？',
      message: '旧的配对码将立即失效',
    })
  } catch {
    return
  }
  regenerating.value = true
  try {
    await tailscaleApi.regenerateCode(3600)
    showToast({ type: 'success', message: '配对码已重新生成' })
    await load()
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  } finally {
    regenerating.value = false
  }
}

async function copyUrl(url: string) {
  try {
    await navigator.clipboard.writeText(url)
    showToast('URL 已复制')
  } catch {
    showToast({ type: 'fail', message: '复制失败' })
  }
}

function timeUntilExpiry(): string {
  if (!pairingCode.value) return '—'
  const left = pairingCode.value.generated_at + pairingCode.value.ttl_seconds - Date.now() / 1000
  if (left <= 0) return '已过期'
  const min = Math.floor(left / 60)
  if (min < 60) return `${min} 分钟`
  const h = Math.floor(min / 60)
  return `${h} 小时 ${min % 60} 分`
}

onMounted(load)
</script>

<template>
  <div class="tailscale-settings">
    <div class="header">
      <h3 class="title">🌐 远程访问（Tailscale）</h3>
      <button class="btn-refresh" :disabled="loading" @click="load">🔄 刷新</button>
    </div>

    <div v-if="!summary" class="loading">加载中...</div>

    <template v-else>
      <!-- Tailscale 状态卡片 -->
      <div class="status-card" :class="info?.status">
        <div class="status-header">
          <span class="status-icon">{{ statusMeta?.icon }}</span>
          <div class="status-info">
            <div class="status-label" :style="{ color: statusMeta?.color }">
              Tailscale: {{ statusMeta?.label }}
            </div>
            <div v-if="info?.error" class="status-error">{{ info.error }}</div>
            <div v-else-if="info?.installed" class="status-sub">
              {{ info.running ? '服务运行中' : '服务已停止' }}
              {{ info.logged_in ? '· 已登录' : '· 未登录' }}
            </div>
            <div v-else class="status-sub">未安装 Tailscale</div>
          </div>
        </div>

        <div v-if="info?.online" class="tailscale-info">
          <div v-if="info.ipv4" class="info-row">
            <span class="info-label">IPv4</span>
            <code class="info-value">{{ info.ipv4 }}</code>
          </div>
          <div v-if="info.hostname && info.tailnet" class="info-row">
            <span class="info-label">Hostname</span>
            <code class="info-value">{{ info.hostname }}.{{ info.tailnet }}</code>
          </div>
          <div v-if="info.tailnet" class="info-row">
            <span class="info-label">Tailnet</span>
            <code class="info-value">{{ info.tailnet }}</code>
          </div>
        </div>

        <div v-if="!info?.installed" class="install-hint">
          💡 安装 Tailscale：访问 <a href="https://tailscale.com/download" target="_blank" class="link">tailscale.com/download</a>
        </div>
      </div>

      <!-- 远程访问 URL -->
      <div v-if="remoteHint" class="section">
        <div class="section-title">🔗 远程访问 URL</div>

        <div v-if="remoteHint.note" class="note">
          {{ remoteHint.note }}
        </div>

        <div class="url-group">
          <div class="url-label">推荐</div>
          <div class="url-row primary">
            <code class="url-text">{{ remoteHint.primary_url || '—' }}</code>
            <button
              v-if="remoteHint.primary_url"
              class="btn-copy"
              @click="copyUrl(remoteHint.primary_url)"
            >
              📋
            </button>
          </div>
        </div>

        <div v-if="remoteHint.backup_urls.length" class="url-group">
          <div class="url-label">备选</div>
          <div
            v-for="(url, idx) in remoteHint.backup_urls"
            :key="idx"
            class="url-row"
          >
            <code class="url-text">{{ url }}</code>
            <button class="btn-copy" @click="copyUrl(url)">📋</button>
          </div>
        </div>
      </div>

      <!-- 配对码 -->
      <div v-if="pairingCode" class="section">
        <div class="section-title">
          🔑 配对码
          <span class="expiry">有效期: {{ timeUntilExpiry() }}</span>
        </div>
        <div class="code-row">
          <div class="code-display">
            <span v-for="(ch, idx) in pairingCode.value" :key="idx" :class="['ch', idx === 2 ? 'ch-sep' : '']">
              {{ ch }}
            </span>
          </div>
          <button
            class="btn-regen"
            :disabled="regenerating"
            @click="handleRegenerate"
          >
            🔄 重新生成
          </button>
        </div>
        <div class="code-hint">
          💡 手机访问时，在 URL 后加 <code>?token={{ pairingCode.value }}</code>，或输入 6 位配对码
        </div>
      </div>

      <!-- 平台信息 -->
      <div class="section">
        <div class="section-title">💻 平台信息</div>
        <div class="info-row">
          <span class="info-label">操作系统</span>
          <code class="info-value">{{ summary.platform }}</code>
        </div>
        <div v-if="summary.lan_ip" class="info-row">
          <span class="info-label">本机 IP</span>
          <code class="info-value">{{ summary.lan_ip }}</code>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.tailscale-settings {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px;
  background: var(--card-bg, #0f172a);
  border: 1px solid var(--border-color, #1e293b);
  border-radius: 8px;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary, #f1f5f9);
}

.btn-refresh {
  font-size: 11px;
  padding: 3px 10px;
  background: #334155;
  color: #f1f5f9;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

.btn-refresh:hover:not(:disabled) {
  background: #475569;
}

.btn-refresh:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.loading {
  text-align: center;
  font-size: 12px;
  color: var(--text-tertiary, #64748b);
  padding: 20px 0;
}

.status-card {
  background: #1e293b;
  border: 1px solid #334155;
  border-left: 3px solid;
  border-radius: 6px;
  padding: 12px;
}

.status-card.running_online { border-left-color: #10b981; }
.status-card.not_installed { border-left-color: #6b7280; }
.status-card.installed_stopped,
.status-card.running_logged_out { border-left-color: #f59e0b; }
.status-card.error { border-left-color: #ef4444; }

.status-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.status-icon {
  font-size: 24px;
}

.status-info {
  flex: 1;
}

.status-label {
  font-size: 14px;
  font-weight: 600;
}

.status-sub {
  font-size: 11px;
  color: var(--text-tertiary, #94a3b8);
  margin-top: 2px;
}

.status-error {
  font-size: 11px;
  color: #ef4444;
  margin-top: 2px;
}

.tailscale-info {
  background: #0f172a;
  border-radius: 4px;
  padding: 8px;
  margin-top: 6px;
}

.info-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  margin-bottom: 4px;
}

.info-label {
  color: var(--text-tertiary, #94a3b8);
  min-width: 80px;
  font-size: 11px;
}

.info-value {
  font-family: monospace;
  background: #1e293b;
  padding: 1px 6px;
  border-radius: 3px;
  color: var(--text-primary, #f1f5f9);
  word-break: break-all;
}

.install-hint {
  font-size: 11px;
  color: var(--text-tertiary, #94a3b8);
  margin-top: 8px;
  padding: 6px 8px;
  background: rgba(245, 158, 11, 0.05);
  border-radius: 4px;
}

.link {
  color: #60a5fa;
  text-decoration: underline;
}

.section {
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 6px;
  padding: 10px 12px;
}

.section-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary, #cbd5e1);
  margin-bottom: 8px;
}

.expiry {
  font-size: 10px;
  font-weight: 400;
  color: var(--text-tertiary, #64748b);
}

.note {
  font-size: 11px;
  color: var(--text-secondary, #cbd5e1);
  background: #0f172a;
  padding: 6px 8px;
  border-radius: 4px;
  margin-bottom: 8px;
}

.url-group {
  margin-bottom: 8px;
}

.url-label {
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
  margin-bottom: 3px;
  text-transform: uppercase;
}

.url-row {
  display: flex;
  align-items: center;
  gap: 6px;
  background: #0f172a;
  border: 1px solid #334155;
  border-radius: 4px;
  padding: 6px 8px;
  margin-bottom: 4px;
}

.url-row.primary {
  border-color: #10b981;
  background: rgba(16, 185, 129, 0.05);
}

.url-text {
  flex: 1;
  font-family: monospace;
  font-size: 11px;
  color: var(--text-primary, #f1f5f9);
  word-break: break-all;
  user-select: all;
}

.btn-copy {
  font-size: 12px;
  padding: 2px 8px;
  background: #334155;
  color: #f1f5f9;
  border: none;
  border-radius: 3px;
  cursor: pointer;
  flex-shrink: 0;
}

.btn-copy:hover {
  background: #475569;
}

.code-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 6px;
}

.code-display {
  display: flex;
  align-items: center;
  gap: 2px;
  background: #0f172a;
  padding: 6px 12px;
  border-radius: 6px;
  font-family: monospace;
  font-size: 18px;
  font-weight: 700;
  color: #f59e0b;
  letter-spacing: 1px;
  user-select: all;
}

.ch {
  display: inline-block;
  min-width: 16px;
  text-align: center;
}

.ch-sep {
  margin: 0 6px;
  color: var(--text-tertiary, #64748b);
}

.btn-regen {
  font-size: 11px;
  padding: 4px 10px;
  background: #3b82f6;
  color: #fff;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

.btn-regen:hover:not(:disabled) {
  background: #2563eb;
}

.btn-regen:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.code-hint {
  font-size: 11px;
  color: var(--text-tertiary, #94a3b8);
  margin-top: 4px;
}

.code-hint code {
  background: #0f172a;
  padding: 1px 4px;
  border-radius: 2px;
  font-family: monospace;
  color: #f59e0b;
}
</style>
