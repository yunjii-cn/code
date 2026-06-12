<!--
  KnowledgePanel.vue
  2026-06-09 TASK-3.3 引入：自进化知识系统 UI 面板
  功能：
    - 4 层知识分组展示（L1 纠正 / L2 模式 / L3 事实 / L4 偏好）
    - 强度可视化（weak/medium/strong 进度条 + 颜色）
    - 一键确认 / 否定 / 提升全局 / 删除
    - 搜索过滤 / 强度 / 作用域过滤
    - 新增知识（弹窗）
    - 统计概览
-->
<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { knowledgeApi, type KnowledgeItem, type KnowledgeStats } from '@/api'
import { useDevice } from '@/composables/useDevice'

const { isMobile } = useDevice()

const items = ref<KnowledgeItem[]>([])
const stats = ref<KnowledgeStats | null>(null)
const loading = ref(false)
const searchQuery = ref('')
const layerFilter = ref<string>('')  // '' = all, 'L1'/'L2'/'L3'/'L4'
const strengthFilter = ref<string>('')
const scopeFilter = ref<string>('')
const showAddDialog = ref(false)

// 添加表单
const newContent = ref('')
const newLayer = ref<'L1' | 'L2' | 'L3' | 'L4'>('L1')
const newScope = ref<'project' | 'global'>('project')
const newStrength = ref<'weak' | 'medium' | 'strong'>('weak')
const newSource = ref('')
const adding = ref(false)

const LAYER_META: Record<string, { label: string; icon: string; color: string; hint: string }> = {
  L1: { label: '纠正', icon: '✏️', color: 'var(--danger)',  hint: '"不要用 var，用 const"' },
  L2: { label: '模式', icon: '🔁', color: 'var(--warning)', hint: '"用户连续 3 次手动测试→提交→部署"' },
  L3: { label: '事实', icon: '📌', color: 'var(--accent)',  hint: '"这个项目用 PostgreSQL"' },
  L4: { label: '偏好', icon: '⭐', color: 'var(--success)', hint: '"我喜欢 Tab 缩进"' },
}

const STRENGTH_META: Record<string, { label: string; color: string; progress: number }> = {
  weak:   { label: '弱',   color: '#94a3b8', progress: 0.33 },
  medium: { label: '中',   color: '#3b82f6', progress: 0.66 },
  strong: { label: '强',   color: '#10b981', progress: 1.0 },
}

const filteredItems = computed(() => {
  let arr = items.value
  if (layerFilter.value) arr = arr.filter(k => k.layer === layerFilter.value)
  if (strengthFilter.value) arr = arr.filter(k => k.strength === strengthFilter.value)
  if (scopeFilter.value) arr = arr.filter(k => k.scope === scopeFilter.value)
  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase()
    arr = arr.filter(k => k.content.toLowerCase().includes(q))
  }
  return arr
})

const groupedByLayer = computed(() => {
  const groups: Record<string, KnowledgeItem[]> = { L1: [], L2: [], L3: [], L4: [] }
  for (const k of filteredItems.value) {
    if (groups[k.layer]) groups[k.layer].push(k)
  }
  return groups
})

async function loadAll() {
  loading.value = true
  try {
    const [listRes, statsRes] = await Promise.all([
      knowledgeApi.list(),
      knowledgeApi.stats(),
    ])
    items.value = (listRes as { data: KnowledgeItem[] }).data
    stats.value = (statsRes as { data: KnowledgeStats }).data
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e)
    showToast(`加载失败: ${msg}`)
  } finally {
    loading.value = false
  }
}

async function handleConfirm(id: string, forceStrong: boolean) {
  try {
    await knowledgeApi.confirm(id, forceStrong)
    showToast(forceStrong ? '已强制确认为强' : '已确认 +1')
    await loadAll()
  } catch (e: unknown) {
    showToast(`确认失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handleDeny(id: string) {
  try {
    await knowledgeApi.deny(id)
    showToast('已否定，重置为弱')
    await loadAll()
  } catch (e: unknown) {
    showToast(`否定失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handleDelete(id: string) {
  try {
    await showConfirmDialog({
      title: '删除知识',
      message: '确认删除这条知识？此操作不可撤销。',
    })
  } catch {
    return
  }
  try {
    await knowledgeApi.remove(id)
    showToast('已删除')
    await loadAll()
  } catch (e: unknown) {
    showToast(`删除失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handlePromote(id: string) {
  try {
    await showConfirmDialog({
      title: '提升为全局',
      message: '将这条项目级知识提升为全局知识（~/.yunji/knowledge/）？',
    })
  } catch {
    return
  }
  try {
    await knowledgeApi.promote(id)
    showToast('已提升为全局')
    await loadAll()
  } catch (e: unknown) {
    showToast(`提升失败: ${e instanceof Error ? e.message : e}`)
  }
}

function openAddDialog() {
  newContent.value = ''
  newLayer.value = 'L1'
  newScope.value = 'project'
  newStrength.value = 'weak'
  newSource.value = ''
  showAddDialog.value = true
}

async function handleAdd() {
  if (!newContent.value.trim()) {
    showToast('请输入知识内容')
    return
  }
  adding.value = true
  try {
    await knowledgeApi.add({
      content: newContent.value.trim(),
      layer: newLayer.value,
      scope: newScope.value,
      strength: newStrength.value,
      source: newSource.value.trim() || undefined,
    })
    showToast('已添加')
    showAddDialog.value = false
    await loadAll()
  } catch (e: unknown) {
    showToast(`添加失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    adding.value = false
  }
}

function formatDate(ts: number): string {
  const d = new Date(ts * 1000)
  const now = Date.now()
  const diff = now - d.getTime()
  if (diff < 60_000) return '刚刚'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`
  if (diff < 7 * 86_400_000) return `${Math.floor(diff / 86_400_000)} 天前`
  return d.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
}

onMounted(loadAll)
</script>

<template>
  <div class="kp-shell">
    <header class="kp-header">
      <div class="kp-title-row">
        <h2 class="kp-title">🧠 知识面板</h2>
        <van-button type="primary" size="small" @click="openAddDialog">
          + 新增知识
        </van-button>
      </div>
      <p class="kp-subtitle">自进化知识系统 · 四层分类 · 强度演化</p>
    </header>

    <!-- 统计 -->
    <section v-if="stats" class="kp-stats">
      <div class="kp-stat">
        <div class="kp-stat-value">{{ stats.total }}</div>
        <div class="kp-stat-label">总条数</div>
      </div>
      <div class="kp-stat">
        <div class="kp-stat-value" :style="{ color: LAYER_META.L1.color }">
          {{ stats.by_layer.L1 || 0 }}
        </div>
        <div class="kp-stat-label">L1 纠正</div>
      </div>
      <div class="kp-stat">
        <div class="kp-stat-value" :style="{ color: LAYER_META.L2.color }">
          {{ stats.by_layer.L2 || 0 }}
        </div>
        <div class="kp-stat-label">L2 模式</div>
      </div>
      <div class="kp-stat">
        <div class="kp-stat-value" :style="{ color: LAYER_META.L3.color }">
          {{ stats.by_layer.L3 || 0 }}
        </div>
        <div class="kp-stat-label">L3 事实</div>
      </div>
      <div class="kp-stat">
        <div class="kp-stat-value" :style="{ color: LAYER_META.L4.color }">
          {{ stats.by_layer.L4 || 0 }}
        </div>
        <div class="kp-stat-label">L4 偏好</div>
      </div>
    </section>

    <!-- 过滤栏 -->
    <section class="kp-filters">
      <input
        v-model="searchQuery"
        class="kp-search"
        type="text"
        placeholder="🔍 搜索内容..."
      />
      <div class="kp-filter-row">
        <span class="kp-filter-label">层级</span>
        <button
          v-for="l in ['', 'L1', 'L2', 'L3', 'L4']"
          :key="l || 'all-layer'"
          class="kp-chip"
          :class="{ 'kp-chip--active': layerFilter === l }"
          @click="layerFilter = l"
        >
          {{ l || '全部' }}
        </button>
      </div>
      <div class="kp-filter-row">
        <span class="kp-filter-label">强度</span>
        <button
          v-for="s in ['', 'weak', 'medium', 'strong']"
          :key="s || 'all-strength'"
          class="kp-chip"
          :class="{ 'kp-chip--active': strengthFilter === s }"
          @click="strengthFilter = s"
        >
          {{ s ? STRENGTH_META[s].label : '全部' }}
        </button>
      </div>
      <div class="kp-filter-row">
        <span class="kp-filter-label">作用域</span>
        <button
          v-for="sc in ['', 'project', 'global']"
          :key="sc || 'all-scope'"
          class="kp-chip"
          :class="{ 'kp-chip--active': scopeFilter === sc }"
          @click="scopeFilter = sc"
        >
          {{ sc === 'project' ? '项目' : sc === 'global' ? '全局' : '全部' }}
        </button>
      </div>
    </section>

    <!-- 列表 -->
    <section v-if="loading" class="kp-loading">
      <div class="kp-loading-text">加载中...</div>
    </section>
    <section v-else-if="filteredItems.length === 0" class="kp-empty">
      <div class="kp-empty-icon">📭</div>
      <div class="kp-empty-text">没有匹配的知识</div>
    </section>
    <section v-else class="kp-groups">
      <template v-for="(group, layer) in groupedByLayer" :key="layer">
        <div v-if="group.length > 0" class="kp-group">
          <div class="kp-group-header" :style="{ borderLeftColor: LAYER_META[layer].color }">
            <span class="kp-group-icon">{{ LAYER_META[layer].icon }}</span>
            <span class="kp-group-label">L{{ layer }} · {{ LAYER_META[layer].label }}</span>
            <span class="kp-group-count">{{ group.length }}</span>
          </div>
          <div
            v-for="k in group"
            :key="k.id"
            class="kp-card"
            :class="{ 'kp-card--strong': k.strength === 'strong' }"
          >
            <div class="kp-card-content">{{ k.content }}</div>
            <div class="kp-card-meta">
              <div class="kp-strength">
                <div class="kp-strength-bar">
                  <div
                    class="kp-strength-fill"
                    :style="{
                      width: STRENGTH_META[k.strength].progress * 100 + '%',
                      background: STRENGTH_META[k.strength].color,
                    }"
                  ></div>
                </div>
                <span class="kp-strength-label" :style="{ color: STRENGTH_META[k.strength].color }">
                  {{ STRENGTH_META[k.strength].label }} · 确认 {{ k.confirm_count }} 次
                </span>
              </div>
              <div class="kp-tags">
                <span class="kp-tag" :class="`kp-tag--${k.scope}`">
                  {{ k.scope === 'project' ? '📁 项目' : '🌐 全局' }}
                </span>
                <span v-if="k.source" class="kp-tag kp-tag--source">来源: {{ k.source }}</span>
                <span class="kp-tag kp-tag--time">{{ formatDate(k.updated_at) }}</span>
              </div>
            </div>
            <div class="kp-card-actions">
              <van-button size="mini" plain type="primary" @click="handleConfirm(k.id, false)">
                ✓ 确认
              </van-button>
              <van-button size="mini" plain @click="handleConfirm(k.id, true)">
                ⭐ 强确
              </van-button>
              <van-button v-if="k.scope === 'project'" size="mini" plain type="success" @click="handlePromote(k.id)">
                ⬆ 提升
              </van-button>
              <van-button size="mini" plain type="warning" @click="handleDeny(k.id)">
                ✕ 否定
              </van-button>
              <van-button size="mini" plain type="danger" @click="handleDelete(k.id)">
                🗑 删除
              </van-button>
            </div>
          </div>
        </div>
      </template>
    </section>

    <!-- 新增知识弹窗 -->
    <van-popup
      v-model:show="showAddDialog"
      position="bottom"
      :style="{ height: isMobile ? '90vh' : '560px', maxWidth: '600px', margin: isMobile ? '0' : '10vh auto 0', borderTopLeftRadius: '12px', borderTopRightRadius: '12px' }"
    >
      <div class="kp-add">
        <div class="kp-add-header">
          <h3 class="kp-add-title">新增知识</h3>
          <van-button size="mini" plain icon="cross" @click="showAddDialog = false" />
        </div>
        <div class="kp-add-form">
          <label class="kp-field-label">内容</label>
          <textarea
            v-model="newContent"
            class="kp-textarea"
            placeholder="例如：不要用 var 声明变量，统一用 const"
            rows="3"
          />

          <label class="kp-field-label">层级</label>
          <div class="kp-chip-row">
            <button
              v-for="l in ['L1', 'L2', 'L3', 'L4'] as const"
              :key="l"
              class="kp-chip"
              :class="{ 'kp-chip--active': newLayer === l }"
              @click="newLayer = l"
            >
              {{ LAYER_META[l].icon }} {{ l }} {{ LAYER_META[l].label }}
            </button>
          </div>
          <div class="kp-hint">{{ LAYER_META[newLayer].hint }}</div>

          <label class="kp-field-label">作用域</label>
          <div class="kp-chip-row">
            <button
              v-for="sc in ['project', 'global'] as const"
              :key="sc"
              class="kp-chip"
              :class="{ 'kp-chip--active': newScope === sc }"
              @click="newScope = sc"
            >
              {{ sc === 'project' ? '📁 项目级' : '🌐 全局级' }}
            </button>
          </div>

          <label class="kp-field-label">初始强度</label>
          <div class="kp-chip-row">
            <button
              v-for="st in ['weak', 'medium', 'strong'] as const"
              :key="st"
              class="kp-chip"
              :class="{ 'kp-chip--active': newStrength === st }"
              @click="newStrength = st"
            >
              {{ STRENGTH_META[st].label }}
            </button>
          </div>

          <label class="kp-field-label">来源（可选）</label>
          <input
            v-model="newSource"
            class="kp-input"
            type="text"
            placeholder="user / ai / error_correction / 任意标识"
          />

          <van-button
            type="primary"
            block
            :loading="adding"
            :disabled="!newContent.trim()"
            @click="handleAdd"
          >
            添加
          </van-button>
        </div>
      </div>
    </van-popup>
  </div>
</template>

<style scoped>
.kp-shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: inherit;
  overflow-y: auto;
  padding: var(--space-4);
  padding-bottom: var(--tab-bar-total, 24px);
  box-sizing: border-box;
}

.kp-header {
  margin-bottom: var(--space-3);
}

.kp-title-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
}

.kp-title {
  font-size: var(--font-xl);
  font-weight: 600;
  margin: 0;
  color: var(--text-primary);
}

.kp-subtitle {
  margin: 4px 0 0;
  font-size: var(--font-sm);
  color: var(--text-muted);
}

.kp-stats {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}

.kp-stat {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-2);
  text-align: center;
}

.kp-stat-value {
  font-size: var(--font-xl);
  font-weight: 700;
  color: var(--text-primary);
  line-height: 1;
}

.kp-stat-label {
  font-size: var(--font-xs);
  color: var(--text-muted);
  margin-top: 2px;
}

.kp-filters {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
}

.kp-search {
  width: 100%;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  color: var(--text-primary);
  font-size: var(--font-base);
  font-family: inherit;
  box-sizing: border-box;
}

.kp-search:focus {
  outline: none;
  border-color: var(--accent);
}

.kp-filter-row {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex-wrap: wrap;
}

.kp-filter-label {
  font-size: var(--font-xs);
  color: var(--text-muted);
  margin-right: 4px;
  min-width: 36px;
}

.kp-chip {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 2px 10px;
  font-size: var(--font-xs);
  color: var(--text-secondary);
  cursor: pointer;
  font-family: inherit;
  transition: all 0.15s;
}

.kp-chip:hover {
  border-color: var(--accent);
  color: var(--text-primary);
}

.kp-chip--active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.kp-loading,
.kp-empty {
  padding: var(--space-8);
  text-align: center;
  color: var(--text-muted);
}

.kp-empty-icon {
  font-size: 48px;
  margin-bottom: var(--space-2);
  opacity: 0.5;
}

.kp-groups {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.kp-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.kp-group-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: 6px 10px;
  border-left: 3px solid;
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--font-sm);
  font-weight: 600;
  color: var(--text-primary);
}

.kp-group-count {
  margin-left: auto;
  font-size: var(--font-xs);
  color: var(--text-muted);
  background: var(--bg-card);
  padding: 1px 6px;
  border-radius: 8px;
}

.kp-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  transition: border-color 0.15s;
}

.kp-card--strong {
  border-color: var(--success);
  box-shadow: 0 0 0 1px var(--success) inset;
}

.kp-card-content {
  font-size: var(--font-base);
  color: var(--text-primary);
  line-height: 1.5;
  word-break: break-word;
}

.kp-card-meta {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.kp-strength {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.kp-strength-bar {
  flex: 1;
  height: 4px;
  background: var(--bg-secondary);
  border-radius: 2px;
  overflow: hidden;
}

.kp-strength-fill {
  height: 100%;
  transition: width 0.3s, background 0.3s;
}

.kp-strength-label {
  font-size: var(--font-xs);
  white-space: nowrap;
  font-weight: 500;
}

.kp-tags {
  display: flex;
  gap: var(--space-1);
  flex-wrap: wrap;
}

.kp-tag {
  font-size: var(--font-xs);
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--bg-secondary);
  color: var(--text-muted);
  border: 1px solid var(--border);
}

.kp-tag--project { color: var(--accent); border-color: var(--accent); }
.kp-tag--global { color: var(--success); border-color: var(--success); }

.kp-card-actions {
  display: flex;
  gap: var(--space-1);
  flex-wrap: wrap;
  padding-top: var(--space-1);
  border-top: 1px solid var(--border);
}

/* 新增弹窗 */
.kp-add {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
}

.kp-add-header {
  flex-shrink: 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}

.kp-add-title {
  margin: 0;
  font-size: var(--font-lg);
  font-weight: 600;
}

.kp-add-form {
  flex: 1;
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  overflow-y: auto;
}

.kp-field-label {
  font-size: var(--font-sm);
  color: var(--text-muted);
  font-weight: 500;
  margin-top: var(--space-2);
}

.kp-textarea,
.kp-input {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  color: var(--text-primary);
  font-size: var(--font-base);
  font-family: inherit;
  resize: vertical;
  box-sizing: border-box;
}

.kp-textarea:focus,
.kp-input:focus {
  outline: none;
  border-color: var(--accent);
}

.kp-chip-row {
  display: flex;
  gap: var(--space-1);
  flex-wrap: wrap;
}

.kp-hint {
  font-size: var(--font-xs);
  color: var(--text-muted);
  font-style: italic;
  padding: 0 4px;
}
</style>
