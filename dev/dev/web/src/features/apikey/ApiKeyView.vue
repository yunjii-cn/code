<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { showToast } from 'vant'
import { useDevice } from '@/composables/useDevice'
import { aiApi, systemApi } from '@/api'

const { isMobile } = useDevice()

const providers = ref<Array<{
  id: string
  name: string
  apiKey: string
  baseUrl: string
  connected: boolean | null
  checking: boolean
}>>([
  { id: 'ollama', name: 'Ollama', apiKey: '', baseUrl: 'http://localhost:11434', connected: null, checking: false },
  { id: 'anthropic', name: 'Anthropic', apiKey: '', baseUrl: 'https://api.anthropic.com', connected: null, checking: false },
  { id: 'openrouter', name: 'OpenRouter', apiKey: '', baseUrl: 'https://openrouter.ai/api/v1', connected: null, checking: false },
  { id: 'zhipu', name: '智谱', apiKey: '', baseUrl: 'https://open.bigmodel.cn/api/paas/v4', connected: null, checking: false },
  { id: 'deepseek', name: 'DeepSeek', apiKey: '', baseUrl: 'https://api.deepseek.com', connected: null, checking: false },
  { id: 'siliconflow', name: '硅基流动', apiKey: '', baseUrl: 'https://api.siliconflow.cn/v1', connected: null, checking: false },
])

const showKeyMap = ref<Record<string, boolean>>({})

async function loadSettings() {
  try {
    const data: any = await systemApi.getSettings()
    if (data) {
      for (const p of providers.value) {
        if (data[`api_key_${p.id}`]) p.apiKey = data[`api_key_${p.id}`]
        if (data[`base_url_${p.id}`]) p.baseUrl = data[`base_url_${p.id}`]
      }
    }
  } catch {
    // 使用默认值
  }
}

async function saveProvider(provider: typeof providers.value[0]) {
  try {
    const data: Record<string, string> = {
      [`api_key_${provider.id}`]: provider.apiKey,
      [`base_url_${provider.id}`]: provider.baseUrl,
    }
    await systemApi.saveSettings(data)
    showToast('保存成功')
  } catch {
    showToast('保存失败')
  }
}

async function checkConnection(provider: typeof providers.value[0]) {
  provider.checking = true
  provider.connected = null
  try {
    if (provider.id === 'ollama') {
      await aiApi.getModels('ollama', { base_url: provider.baseUrl })
    } else {
      await aiApi.checkProvider({
        provider: provider.id,
        base_url: provider.baseUrl || undefined,
        api_key: provider.apiKey || undefined,
      })
    }
    provider.connected = true
    showToast('连接成功')
  } catch {
    provider.connected = false
    showToast('连接失败')
  } finally {
    provider.checking = false
  }
}

function toggleKeyVisibility(id: string) {
  showKeyMap.value[id] = !showKeyMap.value[id]
}

function maskKey(key: string): string {
  if (!key) return ''
  if (key.length <= 8) return '•'.repeat(key.length)
  return key.slice(0, 4) + '•'.repeat(key.length - 8) + key.slice(-4)
}

onMounted(loadSettings)
</script>

<template>
  <div class="apikey-view" :class="{ mobile: isMobile }">
    <div class="page-header">
      <h2 class="page-title">API 密钥管理</h2>
      <span class="page-desc">配置各AI提供商的API密钥和访问地址</span>
    </div>

    <div class="provider-list">
      <div v-for="p in providers" :key="p.id" class="provider-card">
        <div class="card-header">
          <div class="provider-info">
            <span class="provider-name">{{ p.name }}</span>
            <span
              class="status-dot"
              :class="{
                online: p.connected === true,
                offline: p.connected === false,
                unknown: p.connected === null,
              }"
            />
          </div>
          <div class="card-actions">
            <van-button
              size="mini"
              plain
              :loading="p.checking"
              @click="checkConnection(p)"
            >
              检测连接
            </van-button>
            <van-button
              size="mini"
              type="primary"
              @click="saveProvider(p)"
            >
              保存
            </van-button>
          </div>
        </div>

        <div class="card-body">
          <div class="form-row">
            <label class="form-label">API Key</label>
            <div class="key-input-wrap">
              <van-field
                :model-value="showKeyMap[p.id] ? p.apiKey : maskKey(p.apiKey)"
                :type="showKeyMap[p.id] ? 'text' : 'password'"
                placeholder="输入API密钥..."
                class="dark-field"
                @update:model-value="(val: string) => p.apiKey = val"
                @focus="showKeyMap[p.id] = true"
                @blur="showKeyMap[p.id] = false"
              />
              <van-icon
                :name="showKeyMap[p.id] ? 'eye-o' : 'closed-eye'"
                class="key-toggle"
                @click="toggleKeyVisibility(p.id)"
              />
            </div>
          </div>

          <div class="form-row">
            <label class="form-label">Base URL</label>
            <van-field
              v-model="p.baseUrl"
              placeholder="API地址"
              class="dark-field"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.apikey-view {
  height: 100%;
  overflow-y: auto;
  padding: 24px;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.apikey-view.mobile {
  padding: 16px;
}

.page-header {
  margin-bottom: 24px;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 4px;
}

.page-desc {
  font-size: 13px;
  color: var(--text-muted);
}

.provider-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.provider-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
}

.provider-info {
  display: flex;
  align-items: center;
  gap: 10px;
}

.provider-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-dot.online {
  background: var(--success);
  box-shadow: 0 0 6px rgba(76, 175, 80, 0.4);
}

.status-dot.offline {
  background: var(--error);
  box-shadow: 0 0 6px rgba(244, 67, 54, 0.4);
}

.status-dot.unknown {
  background: var(--text-muted);
}

.card-actions {
  display: flex;
  gap: 8px;
}

.card-body {
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.form-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-label {
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 500;
}

.key-input-wrap {
  position: relative;
  display: flex;
  align-items: center;
}

.key-input-wrap .dark-field {
  flex: 1;
}

.key-toggle {
  position: absolute;
  right: 12px;
  color: var(--text-muted);
  cursor: pointer;
  z-index: 1;
}

.dark-field {
  background: var(--bg-primary);
  border-radius: 8px;
}

:deep(.dark-field .van-field__control) {
  color: var(--text-primary);
  font-size: 13px;
}

:deep(.dark-field .van-field__control::placeholder) {
  color: var(--text-muted);
}

:deep(.van-button--primary) {
  background: var(--accent);
  border-color: var(--accent);
}

:deep(.van-button--plain) {
  background: transparent;
  border-color: var(--border);
  color: var(--text-secondary);
}
</style>
