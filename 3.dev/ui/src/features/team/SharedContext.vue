<!--
  SharedContext.vue
  2026-06-09 TASK-3.10 引入：共享上下文面板
  显示团队共享的事实/决策/约定，支持新增
-->
<script setup lang="ts">
import { ref } from 'vue'
import { showToast } from 'vant'
import type { TeamSharedContext } from '@/api'

interface Props {
  items: TeamSharedContext[]
  canEdit?: boolean
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
const props = withDefaults(defineProps<Props>(), {
  canEdit: true,
})

const emit = defineEmits<{
  (e: 'add', key: string, value: string, source: string): void
}>()

const ROLE_EMOJI: Record<string, string> = {
  coordinator: '🎯',
  architect: '🏗️',
  developer: '💻',
  tester: '🧪',
  documenter: '📝',
}

const showAdd = ref(false)
const newKey = ref('')
const newValue = ref('')
const newSource = ref('coordinator')

function handleAdd() {
  if (!newKey.value.trim() || !newValue.value.trim()) {
    showToast('key 和 value 必填')
    return
  }
  emit('add', newKey.value.trim(), newValue.value.trim(), newSource.value)
  newKey.value = ''
  newValue.value = ''
  showAdd.value = false
}

function timeAgo(ts: number): string {
  const diff = Date.now() / 1000 - ts
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  return `${Math.floor(diff / 86400)} 天前`
}
</script>

<template>
  <div class="shared-context">
    <div class="ctx-header">
      <span class="ctx-title">📌 共享上下文</span>
      <span class="ctx-count">{{ items.length }}</span>
      <button v-if="canEdit && !showAdd" class="btn-add" @click="showAdd = true">+ 新增</button>
    </div>

    <div v-if="showAdd" class="add-form">
      <input v-model="newKey" class="ctx-input" placeholder="key（如 api_url）" />
      <textarea
        v-model="newValue"
        class="ctx-textarea"
        placeholder="value（如 https://api.example.com）"
        rows="2"
      />
      <select v-model="newSource" class="ctx-select">
        <option value="coordinator">🎯 主管</option>
        <option value="architect">🏗️ 架构</option>
        <option value="developer">💻 开发</option>
        <option value="tester">🧪 测试</option>
        <option value="documenter">📝 文档</option>
      </select>
      <div class="form-actions">
        <button class="btn btn-primary" @click="handleAdd">保存</button>
        <button class="btn btn-cancel" @click="showAdd = false">取消</button>
      </div>
    </div>

    <div class="ctx-list">
      <div v-for="item in items" :key="item.key" class="ctx-item">
        <div class="ctx-item-header">
          <span class="ctx-key">{{ item.key }}</span>
          <span class="ctx-source">
            {{ ROLE_EMOJI[item.source] }} {{ timeAgo(item.updated_at) }}
          </span>
        </div>
        <div class="ctx-value">{{ item.value }}</div>
      </div>
      <div v-if="items.length === 0 && !showAdd" class="empty">
        暂无共享上下文
      </div>
    </div>
  </div>
</template>

<style scoped>
.shared-context {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.ctx-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-color, #1e293b);
  flex-shrink: 0;
}

.ctx-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary, #f1f5f9);
}

.ctx-count {
  font-size: 11px;
  background: #334155;
  color: #f1f5f9;
  padding: 1px 8px;
  border-radius: 10px;
}

.btn-add {
  margin-left: auto;
  font-size: 11px;
  padding: 3px 10px;
  background: var(--accent-color, #3b82f6);
  color: #fff;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

.btn-add:hover {
  background: #2563eb;
}

.add-form {
  padding: 10px 12px;
  background: #0f172a;
  border-bottom: 1px solid var(--border-color, #1e293b);
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex-shrink: 0;
}

.ctx-input,
.ctx-textarea,
.ctx-select {
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 4px;
  padding: 5px 8px;
  color: var(--text-primary, #f1f5f9);
  font-size: 12px;
  font-family: inherit;
}

.ctx-input:focus,
.ctx-textarea:focus,
.ctx-select:focus {
  outline: none;
  border-color: var(--accent-color, #3b82f6);
}

.form-actions {
  display: flex;
  gap: 6px;
}

.btn {
  font-size: 11px;
  padding: 4px 12px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

.btn-primary {
  background: #10b981;
  color: #fff;
}

.btn-primary:hover {
  background: #059669;
}

.btn-cancel {
  background: #475569;
  color: #f1f5f9;
}

.btn-cancel:hover {
  background: #64748b;
}

.ctx-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
  min-height: 0;
}

.ctx-item {
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 6px;
  padding: 8px 10px;
  margin-bottom: 6px;
}

.ctx-item-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.ctx-key {
  font-family: monospace;
  font-size: 11px;
  font-weight: 600;
  color: var(--accent-color, #60a5fa);
}

.ctx-source {
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
}

.ctx-value {
  font-size: 12px;
  color: var(--text-secondary, #cbd5e1);
  word-break: break-word;
  white-space: pre-wrap;
}

.empty {
  text-align: center;
  font-size: 12px;
  color: var(--text-tertiary, #64748b);
  padding: 20px 0;
  font-style: italic;
}
</style>
