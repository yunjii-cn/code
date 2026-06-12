<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { showToast, showDialog } from 'vant'
import { useModelStore } from '@/stores/model'
import { useChatStore } from '@/stores/chat'
import { useDevice } from '@/composables/useDevice'
import * as platform from '@/platform'
import type { Model } from '@/stores/model'

const modelStore = useModelStore()
const chatStore = useChatStore()

const { isMobile } = useDevice()
const loading = ref(false)
const detecting = ref(false)
const connectionStatus = ref<Record<string, 'online' | 'offline' | 'checking' | 'idle'>>({})

onMounted(async () => {
  await loadProviders()
})

async function loadProviders() {
  try {
    await modelStore.fetchProviders()
    if (modelStore.currentProvider) {
      await loadModels()
    }
  } catch {
    showToast('加载提供商失败')
  }
}

async function loadModels() {
  loading.value = true
  try {
    await modelStore.fetchModels()
  } catch {
    showToast('加载模型失败')
  } finally {
    loading.value = false
  }
}

async function selectProvider(providerId: string) {
  if (modelStore.currentProvider === providerId) return
  await modelStore.switchProvider(providerId)
}

function selectModel(model: Model) {
  modelStore.switchModel(model.id)
  chatStore.switchModel(model.provider, model.id)
  showToast(`已切换到 ${model.name}`)
}

async function detectConnection(providerId: string) {
  detecting.value = true
  connectionStatus.value[providerId] = 'checking'
  try {
    const models = await platform.getModels(providerId)
    connectionStatus.value[providerId] = models.length > 0 ? 'online' : 'offline'
    if (models.length > 0) {
      showToast('连接正常')
    } else {
      showToast('连接成功但无可用模型')
    }
  } catch {
    connectionStatus.value[providerId] = 'offline'
    showToast('连接失败')
  } finally {
    detecting.value = false
  }
}

async function checkOllamaHealth() {
  try {
    const models = await platform.getModels('ollama')
    if (models.length > 0) {
      showDialog({
        title: 'Ollama 健康检测',
        message: `Ollama 运行正常\n可用模型: ${models.length} 个`,
        confirmButtonColor: '#42A5F5',
      })
    } else {
      showDialog({
        title: 'Ollama 健康检测',
        message: 'Ollama 运行中，但无可用模型',
        confirmButtonColor: '#42A5F5',
      })
    }
  } catch {
    showDialog({
      title: 'Ollama 健康检测',
      message: 'Ollama 无法连接，请确认服务已启动',
      confirmButtonColor: '#42A5F5',
    })
  }
}

async function queryOllamaCapabilities() {
  try {
    const models = await platform.getModels('ollama')
    const info = models.map(m => {
      const caps = getCapabilities(m)
      return `${m.name}: ${caps.length > 0 ? caps.join('、') : '基础对话'}`
    }).join('\n')
    showDialog({
      title: 'Ollama 能力查询',
      message: info || '无可用模型',
      confirmButtonColor: '#42A5F5',
    })
  } catch {
    showToast('查询失败')
  }
}

const providerIcons: Record<string, string> = {
  ollama: '🦙',
  anthropic: '🤖',
  openrouter: '🌐',
  zhipu: '🧠',
  api: '🔗',
}

function getProviderIcon(providerId: string): string {
  return providerIcons[providerId] || '📦'
}

function getModelDescription(model: Model): string {
  const name = model.name.toLowerCase()
  if (name.includes('coder') || name.includes('code')) return '代码生成与编程助手'
  if (name.includes('vision') || name.includes('vl')) return '多模态视觉理解模型'
  if (name.includes('embed')) return '文本嵌入模型'
  if (name.includes('chat') || name.includes('instruct')) return '对话与指令遵循模型'
  if (name.includes('reason') || name.includes('think')) return '深度推理模型'
  if (name.includes('math')) return '数学计算专用模型'
  if (model.size) return `参数量 ${model.size}`
  return '通用大语言模型'
}

function getCapabilities(model: Model): string[] {
  const caps: string[] = []
  const name = model.name.toLowerCase()
  if (name.includes('vision') || name.includes('vl') || name.includes('llava')) caps.push('视觉')
  if (name.includes('coder') || name.includes('code') || name.includes('deepseek')) caps.push('工具')
  caps.push('流式')
  if (name.includes('reason') || name.includes('think') || name.includes('o1') || name.includes('o3')) caps.push('推理')
  if (name.includes('embed')) caps.push('嵌入')
  if (name.includes('math')) caps.push('数学')
  return caps
}

function isCurrentModel(model: Model): boolean {
  return chatStore.currentModel === model.id && chatStore.currentProvider === model.provider
}

const tagColors: Record<string, string> = {
  '视觉': '#E91E63',
  '工具': '#FF9800',
  '流式': '#42A5F5',
  '推理': '#9C27B0',
  '嵌入': '#4CAF50',
  '数学': '#00BCD4',
}

const activeTab = computed({
  get: () => modelStore.currentProvider,
  set: (val: string) => selectProvider(val),
})
</script>

<template>
  <div class="models-view" :class="{ mobile: isMobile }">
    <!-- 桌面端：左右分栏 -->
    <template v-if="!isMobile">
      <div class="provider-sidebar">
        <div class="sidebar-title">提供商</div>
        <div class="provider-list">
          <div
            v-for="p in modelStore.providers"
            :key="p.id"
            class="provider-item"
            :class="{ active: modelStore.currentProvider === p.id }"
            @click="selectProvider(p.id)"
          >
            <span class="provider-icon">{{ getProviderIcon(p.id) }}</span>
            <span class="provider-name">{{ p.name }}</span>
            <span
              v-if="connectionStatus[p.id]"
              class="status-dot"
              :class="connectionStatus[p.id]"
            />
          </div>
        </div>
        <div class="sidebar-actions">
          <van-button
            size="small"
            plain
            :loading="detecting"
            @click="detectConnection(modelStore.currentProvider)"
          >
            检测连接
          </van-button>
          <template v-if="modelStore.currentProvider === 'ollama'">
            <van-button size="small" plain @click="checkOllamaHealth">
              健康检测
            </van-button>
            <van-button size="small" plain @click="queryOllamaCapabilities">
              能力查询
            </van-button>
          </template>
        </div>
      </div>
      <div class="models-content">
        <div class="content-header">
          <h3 class="content-title">
            {{ modelStore.providers.find(p => p.id === modelStore.currentProvider)?.name || '模型' }}
          </h3>
          <span class="model-count">{{ modelStore.models.length }} 个模型</span>
        </div>
        <van-loading v-if="loading" class="page-loading" color="#42A5F5" size="36px" vertical>
          加载中...
        </van-loading>
        <div v-else-if="modelStore.models.length === 0" class="empty-state">
          <van-icon name="search" size="48" color="#555" />
          <p>暂无可用模型</p>
        </div>
        <div v-else class="model-grid">
          <div
            v-for="model in modelStore.models"
            :key="model.id"
            class="model-card"
            :class="{ current: isCurrentModel(model) }"
            @click="selectModel(model)"
          >
            <div class="card-header">
              <span class="model-name">{{ model.name }}</span>
              <van-icon v-if="isCurrentModel(model)" name="success" color="#42A5F5" size="18" />
            </div>
            <p class="model-desc">{{ getModelDescription(model) }}</p>
            <div class="model-meta">
              <span v-if="model.size" class="meta-item">{{ model.size }}</span>
              <span v-if="model.quantization" class="meta-item">{{ model.quantization }}</span>
            </div>
            <div class="capability-tags">
              <span
                v-for="cap in getCapabilities(model)"
                :key="cap"
                class="cap-tag"
                :style="{ background: tagColors[cap] + '22', color: tagColors[cap], borderColor: tagColors[cap] + '44' }"
              >
                {{ cap }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- 手机端：标签页 + 列表 -->
    <template v-else>
      <div class="models-mobile">
        <van-tabs
          v-model:active="activeTab"
          animated
          swipeable
          background="#0d0d0d"
          color="#42A5F5"
          title-active-color="#42A5F5"
          title-inactive-color="#8888aa"
          line-width="20"
          line-height="2"
        >
          <van-tab
            v-for="p in modelStore.providers"
            :key="p.id"
            :name="p.id"
            :title="p.name"
          >
            <div class="mobile-toolbar">
              <van-button
                size="mini"
                plain
                :loading="detecting"
                @click="detectConnection(p.id)"
              >
                检测连接
              </van-button>
              <template v-if="p.id === 'ollama'">
                <van-button size="mini" plain @click="checkOllamaHealth">
                  健康检测
                </van-button>
                <van-button size="mini" plain @click="queryOllamaCapabilities">
                  能力查询
                </van-button>
              </template>
            </div>
            <van-loading v-if="loading" class="page-loading" color="#42A5F5" size="36px" vertical>
              加载中...
            </van-loading>
            <div v-else-if="modelStore.models.length === 0" class="empty-state">
              <van-icon name="search" size="48" color="#555" />
              <p>暂无可用模型</p>
            </div>
            <div v-else class="mobile-model-list">
              <div
                v-for="model in modelStore.models"
                :key="model.id"
                class="model-card mobile-card"
                :class="{ current: isCurrentModel(model) }"
                @click="selectModel(model)"
              >
                <div class="card-header">
                  <span class="model-name">{{ model.name }}</span>
                  <van-icon v-if="isCurrentModel(model)" name="success" color="#42A5F5" size="18" />
                </div>
                <p class="model-desc">{{ getModelDescription(model) }}</p>
                <div class="model-meta">
                  <span v-if="model.size" class="meta-item">{{ model.size }}</span>
                  <span v-if="model.quantization" class="meta-item">{{ model.quantization }}</span>
                </div>
                <div class="capability-tags">
                  <span
                    v-for="cap in getCapabilities(model)"
                    :key="cap"
                    class="cap-tag"
                    :style="{ background: tagColors[cap] + '22', color: tagColors[cap], borderColor: tagColors[cap] + '44' }"
                  >
                    {{ cap }}
                  </span>
                </div>
              </div>
            </div>
          </van-tab>
        </van-tabs>
      </div>
    </template>
  </div>
</template>

<style scoped>
.models-view {
  height: 100vh;
  background: #0d0d0d;
  color: #e0e0e0;
  display: flex;
}

.models-view.mobile {
  flex-direction: column;
}

/* 桌面端侧边栏 */
.provider-sidebar {
  width: 220px;
  background: #111122;
  border-right: 1px solid #1e1e3a;
  padding: 20px 0;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.sidebar-title {
  font-size: 12px;
  font-weight: 600;
  color: #6666aa;
  text-transform: uppercase;
  letter-spacing: 1.5px;
  padding: 0 20px;
  margin-bottom: 12px;
}

.provider-list {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.provider-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 20px;
  cursor: pointer;
  transition: all 0.2s;
  color: #8888aa;
  border-left: 3px solid transparent;
  font-size: 14px;
}

.provider-item:hover {
  background: #1a1a2e;
  color: #c0c0e0;
}

.provider-item.active {
  background: #1a1a2e;
  border-left-color: #42A5F5;
  color: #e0e0f0;
}

.provider-icon {
  font-size: 18px;
  width: 24px;
  text-align: center;
}

.provider-name {
  flex: 1;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-dot.online {
  background: #4CAF50;
  box-shadow: 0 0 6px #4CAF5066;
}

.status-dot.offline {
  background: #f44336;
  box-shadow: 0 0 6px #f4433666;
}

.status-dot.checking {
  background: #FF9800;
  animation: pulse 1s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.sidebar-actions {
  padding: 16px 16px 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  border-top: 1px solid #1e1e3a;
  padding-top: 16px;
}

:deep(.sidebar-actions .van-button) {
  border-color: #42A5F544;
  color: #42A5F5;
  background: transparent;
}

:deep(.sidebar-actions .van-button:active) {
  background: #42A5F511;
}

/* 桌面端内容区 */
.models-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 28px;
}

.content-header {
  display: flex;
  align-items: baseline;
  gap: 12px;
  margin-bottom: 20px;
}

.content-title {
  font-size: 18px;
  font-weight: 600;
  color: #e0e0f0;
  margin: 0;
}

.model-count {
  font-size: 13px;
  color: #6666aa;
}

.page-loading {
  display: flex;
  justify-content: center;
  padding: 60px 0;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 0;
  color: #555;
  gap: 12px;
}

.empty-state p {
  margin: 0;
  font-size: 14px;
}

/* 模型卡片网格 */
.model-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 16px;
}

.model-card {
  background: #1a1a2e;
  border: 1px solid #2a2a4a;
  border-radius: 12px;
  padding: 16px;
  cursor: pointer;
  transition: all 0.25s;
}

.model-card:hover {
  border-color: #42A5F544;
  transform: translateY(-2px);
  box-shadow: 0 4px 20px #00000066;
}

.model-card.current {
  border-color: #42A5F5;
  box-shadow: 0 0 0 1px #42A5F5, 0 4px 20px #42A5F522;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.model-name {
  font-size: 14px;
  font-weight: 600;
  color: #e0e0f0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 85%;
}

.model-desc {
  font-size: 12px;
  color: #8888aa;
  margin: 0 0 10px;
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.model-meta {
  display: flex;
  gap: 8px;
  margin-bottom: 10px;
}

.meta-item {
  font-size: 11px;
  color: #6666aa;
  background: #222244;
  padding: 2px 8px;
  border-radius: 4px;
}

.capability-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.cap-tag {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  border: 1px solid;
  line-height: 1.6;
}

/* 手机端 */
.models-mobile {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

:deep(.van-tabs) {
  flex: 1;
  display: flex;
  flex-direction: column;
}

:deep(.van-tabs__content) {
  flex: 1;
  overflow-y: auto;
}

:deep(.van-tab__panel) {
  padding: 12px 16px;
}

:deep(.van-tabs__nav) {
  border-bottom: 1px solid #1e1e3a;
}

.mobile-toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

:deep(.mobile-toolbar .van-button) {
  border-color: #42A5F544;
  color: #42A5F5;
  background: transparent;
}

:deep(.mobile-toolbar .van-button:active) {
  background: #42A5F511;
}

.mobile-model-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.mobile-card {
  border-radius: 10px;
}

/* 全局 Vant 暗黑覆盖 */
:deep(.van-loading__text) {
  color: #8888aa;
}

:deep(.van-dialog) {
  background: #1a1a2e;
  border: 1px solid #2a2a4a;
}

:deep(.van-dialog__header) {
  color: #e0e0f0;
}

:deep(.van-dialog__message) {
  color: #a0a0c0;
  white-space: pre-line;
}

:deep(.van-dialog__confirm) {
  color: #42A5F5;
}
</style>
