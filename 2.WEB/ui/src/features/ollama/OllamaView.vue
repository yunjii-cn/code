<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { showToast, showDialog } from 'vant'
import { useDevice } from '@/composables/useDevice'
import { aiApi, systemApi } from '@/api'

const { isMobile } = useDevice()

const ollamaBaseUrl = ref('http://localhost:11434')
const localModels = ref<Array<{ name: string; size?: string; modified?: string }>>([])
const searchQuery = ref('')
const searchResults = ref<Array<{ name: string; description?: string; pulls?: string }>>([])
const recommendedModels = ref<Array<{ name: string; desc: string; size: string }>>([])
const loading = ref(false)
const pulling = ref<string | null>(null)
const pullProgress = ref('')

async function loadLocalModels() {
  loading.value = true
  try {
    const data: any = await aiApi.getModels('ollama', { base_url: ollamaBaseUrl.value })
    localModels.value = (data?.models || data?.data || []).map((m: any) => ({
      name: m.name || m.id || m.model,
      size: m.size ? `${(m.size / 1e9).toFixed(1)}GB` : undefined,
      modified: m.modified_at || m.modified,
    }))
  } catch {
    localModels.value = []
    showToast('无法连接Ollama')
  } finally {
    loading.value = false
  }
}

async function checkHealth() {
  try {
    await aiApi.checkOllamaHealth({
      model_name: localModels.value[0]?.name || 'llama3',
      base_url: ollamaBaseUrl.value || undefined,
    })
    showToast('Ollama 运行正常')
  } catch {
    showToast('Ollama 无法连接')
  }
}

async function searchModels() {
  if (!searchQuery.value.trim()) return
  loading.value = true
  try {
    const data: any = await systemApi.searchOllamaLibrary(searchQuery.value.trim())
    searchResults.value = (data?.models || []).map((m: any) => ({
      name: m.name,
      description: m.description,
      pulls: m.pulls,
    }))
  } catch {
    showToast('搜索失败')
  } finally {
    loading.value = false
  }
}

async function pullModel(name: string) {
  pulling.value = name
  pullProgress.value = '正在拉取...'
  try {
    await systemApi.pullModel(name)
    pullProgress.value = '拉取完成'
    showToast(`${name} 拉取成功`)
    await loadLocalModels()
  } catch (e: any) {
    pullProgress.value = `拉取失败: ${e.message}`
    showToast('拉取失败')
  } finally {
    setTimeout(() => {
      pulling.value = null
      pullProgress.value = ''
    }, 2000)
  }
}

async function deleteModel(name: string) {
  try {
    await showDialog({
      title: '删除模型',
      message: `确定要删除模型「${name}」吗？`,
      confirmButtonColor: 'var(--error)',
      confirmButtonText: '删除',
    })
    await systemApi.deleteModel(name)
    showToast('已删除')
    await loadLocalModels()
  } catch {
    // 用户取消
  }
}

async function loadRecommended() {
  try {
    const data: any = await systemApi.recommendModels()
    recommendedModels.value = data?.models || []
  } catch {
    recommendedModels.value = []
  }
}

onMounted(async () => {
  await loadLocalModels()
  await loadRecommended()
})
</script>

<template>
  <div class="ollama-view" :class="{ mobile: isMobile }">
    <div class="page-header">
      <h2 class="page-title">Ollama 管理</h2>
      <div class="header-actions">
        <van-button size="small" plain @click="checkHealth">健康检测</van-button>
        <van-button size="small" type="primary" @click="loadLocalModels">刷新</van-button>
      </div>
    </div>

    <div class="config-row">
      <van-field
        v-model="ollamaBaseUrl"
        label="Ollama 地址"
        placeholder="http://localhost:11434"
        class="dark-field"
      />
    </div>

    <div class="section">
      <div class="section-header">
        <h3 class="section-title">本地模型 ({{ localModels.length }})</h3>
      </div>
      <van-loading v-if="loading" class="page-loading" color="var(--accent)" size="24px" vertical>
        加载中...
      </van-loading>
      <div v-else-if="localModels.length === 0" class="empty-state">
        <p>暂无本地模型，从下方搜索并拉取</p>
      </div>
      <div v-else class="model-list">
        <div v-for="m in localModels" :key="m.name" class="model-item">
          <div class="model-info">
            <span class="model-name">{{ m.name }}</span>
            <span v-if="m.size" class="model-size">{{ m.size }}</span>
          </div>
          <van-button
            size="mini"
            plain
            color="var(--error)"
            @click="deleteModel(m.name)"
          >
            删除
          </van-button>
        </div>
      </div>
    </div>

    <div class="section">
      <div class="section-header">
        <h3 class="section-title">搜索模型库</h3>
      </div>
      <div class="search-row">
        <van-field
          v-model="searchQuery"
          placeholder="搜索模型..."
          class="dark-field search-input"
          @keydown.enter="searchModels"
        />
        <van-button size="small" type="primary" @click="searchModels">搜索</van-button>
      </div>
      <div v-if="searchResults.length" class="model-list">
        <div v-for="m in searchResults" :key="m.name" class="model-item">
          <div class="model-info">
            <span class="model-name">{{ m.name }}</span>
            <span v-if="m.pulls" class="model-pulls">{{ m.pulls }} 次拉取</span>
          </div>
          <van-button
            size="mini"
            type="primary"
            :loading="pulling === m.name"
            @click="pullModel(m.name)"
          >
            {{ pulling === m.name ? pullProgress || '拉取中' : '拉取' }}
          </van-button>
        </div>
      </div>
    </div>

    <div v-if="recommendedModels.length" class="section">
      <div class="section-header">
        <h3 class="section-title">推荐模型</h3>
      </div>
      <div class="model-list">
        <div v-for="m in recommendedModels" :key="m.name" class="model-item recommend-item">
          <div class="model-info">
            <span class="model-name">{{ m.name }}</span>
            <span class="model-desc">{{ m.desc }}</span>
          </div>
          <van-button
            size="mini"
            plain
            :loading="pulling === m.name"
            @click="pullModel(m.name)"
          >
            拉取
          </van-button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ollama-view {
  height: 100%;
  overflow-y: auto;
  padding: 24px;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.ollama-view.mobile {
  padding: 16px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.config-row {
  margin-bottom: 20px;
}

.dark-field {
  background: var(--bg-card);
  border-radius: 8px;
}

:deep(.dark-field .van-field__control) {
  color: var(--text-primary);
}

:deep(.dark-field .van-field__label) {
  color: var(--text-secondary);
}

.section {
  margin-bottom: 24px;
}

.section-header {
  margin-bottom: 12px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
  color: var(--text-primary);
}

.page-loading {
  display: flex;
  justify-content: center;
  padding: 40px 0;
}

.empty-state {
  text-align: center;
  padding: 30px 0;
  color: var(--text-muted);
  font-size: 14px;
}

.model-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.model-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
}

.model-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.model-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.model-size,
.model-pulls,
.model-desc {
  font-size: 12px;
  color: var(--text-muted);
}

.search-row {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.search-input {
  flex: 1;
}

.recommend-item {
  border-color: rgba(66, 165, 245, 0.2);
}

:deep(.van-button--primary) {
  background: var(--accent);
  border-color: var(--accent);
}
</style>
