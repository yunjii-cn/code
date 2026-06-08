<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { useDevice } from '@/composables/useDevice'
import { useProjectStore } from '@/stores/project'
import { projectApi } from '@/api'

const { isMobile } = useDevice()
const projectStore = useProjectStore()

interface Memory {
  filename: string
  type: string
  size: number
  modified: string
  content?: string
}

const memories = ref<Memory[]>([])
const searchKeyword = ref('')
const searchResults = ref<Memory[]>([])
const stats = ref<{ total: number; totalSize: number }>({ total: 0, totalSize: 0 })
const loading = ref(false)

async function loadMemories() {
  const project = projectStore.activeProject
  if (!project) return
  loading.value = true
  try {
    const data: any = await projectApi.listMemories(project.id)
    memories.value = (data?.memories || data || []).map((m: any) => ({
      filename: m.filename || m.name,
      type: m.type || m.mem_type || 'project',
      size: m.size || 0,
      modified: m.modified || m.updated_at || '',
    }))
  } catch {
    memories.value = []
  } finally {
    loading.value = false
  }
}

async function loadStats() {
  const project = projectStore.activeProject
  if (!project) return
  try {
    const data: any = await projectApi.getMemoryStats(project.id)
    stats.value = {
      total: data?.total || memories.value.length,
      totalSize: data?.total_size || 0,
    }
  } catch {
    // 使用默认值
  }
}

async function searchMemories() {
  if (!searchKeyword.value.trim()) return
  const project = projectStore.activeProject
  if (!project) return
  try {
    const data: any = await projectApi.searchMemories(project.id, searchKeyword.value.trim())
    searchResults.value = (data?.memories || data || []).map((m: any) => ({
      filename: m.filename || m.name,
      type: m.type || 'project',
      size: m.size || 0,
      modified: m.modified || '',
    }))
  } catch {
    showToast('搜索失败')
  }
}

async function deleteMemory(filename: string) {
  const project = projectStore.activeProject
  if (!project) return
  try {
    await showConfirmDialog({
      title: '删除记忆',
      message: `确定要删除「${filename}」吗？`,
      confirmButtonColor: 'var(--error)',
    })
    await projectApi.deleteMemory(project.id, filename)
    showToast('已删除')
    await loadMemories()
  } catch {
    // 用户取消
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`
}

onMounted(async () => {
  await loadMemories()
  await loadStats()
})
</script>

<template>
  <div class="memory-view" :class="{ mobile: isMobile }">
    <div class="page-header">
      <h2 class="page-title">记忆管理</h2>
      <div class="stats-badge">
        {{ stats.total }} 条 · {{ formatSize(stats.totalSize) }}
      </div>
    </div>

    <div v-if="!projectStore.activeProject" class="empty-state">
      <p>请先选择一个项目</p>
    </div>

    <template v-else>
      <div class="search-row">
        <van-field
          v-model="searchKeyword"
          placeholder="搜索记忆..."
          class="dark-field"
          @keydown.enter="searchMemories"
        />
        <van-button size="small" type="primary" @click="searchMemories">搜索</van-button>
      </div>

      <van-loading v-if="loading" class="page-loading" color="var(--accent)" vertical>加载中...</van-loading>

      <div v-else class="memory-list">
        <div v-for="m in memories" :key="m.filename" class="memory-item">
          <div class="memory-info">
            <span class="memory-name">{{ m.filename }}</span>
            <div class="memory-meta">
              <span class="memory-type">{{ m.type }}</span>
              <span v-if="m.size" class="memory-size">{{ formatSize(m.size) }}</span>
            </div>
          </div>
          <van-button size="mini" plain color="var(--error)" @click="deleteMemory(m.filename)">
            删除
          </van-button>
        </div>
        <div v-if="memories.length === 0" class="empty-hint">暂无记忆文件</div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.memory-view {
  height: 100%;
  overflow-y: auto;
  padding: 24px;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.memory-view.mobile {
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

.stats-badge {
  font-size: 13px;
  color: var(--accent);
  background: rgba(66, 165, 245, 0.1);
  padding: 4px 12px;
  border-radius: 12px;
}

.empty-state,
.empty-hint {
  text-align: center;
  padding: 30px 0;
  color: var(--text-muted);
}

.search-row {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.search-row .dark-field {
  flex: 1;
}

.dark-field {
  background: var(--bg-card);
  border-radius: 8px;
}

:deep(.dark-field .van-field__control) {
  color: var(--text-primary);
}

.memory-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.memory-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
}

.memory-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  flex: 1;
}

.memory-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.memory-meta {
  display: flex;
  gap: 10px;
}

.memory-type,
.memory-size {
  font-size: 12px;
  color: var(--text-muted);
}

:deep(.van-button--primary) {
  background: var(--accent);
  border-color: var(--accent);
}
</style>
