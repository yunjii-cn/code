<script setup lang="ts">
/**
 * SkillMarketView — 技能市场主视图
 * 2026-06-09 TASK-4.5
 *
 * 布局：
 *   ┌─────────────────────────┬──────────────────┐
 *   │ 搜索 + 分类 + 卡片网格   │ 已安装侧栏         │
 *   │ 统计 + 上传按钮          │                  │
 *   └─────────────────────────┴──────────────────┘
 */
import { ref, computed, onMounted, watch } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { useSkillMarketStore } from '@/stores/skill-market'
import type { Skill, InstalledSkill } from '@/api'
import SkillCard from './SkillCard.vue'
import SkillDetailDialog from './SkillDetailDialog.vue'
import UploadSkillDialog from './UploadSkillDialog.vue'
import InstalledSkillsPanel from './InstalledSkillsPanel.vue'

const store = useSkillMarketStore()

const searchInput = ref('')
const selectedCategory = ref('all')
const showDetail = ref(false)
const detailSkill = ref<Skill | null>(null)
const showUpload = ref(false)
const busyIds = ref(new Set<string>())
const activeTab = ref<'discover' | 'installed'>('discover')

const stats = computed(() => store.stats)
const featured = computed(() => store.featuredSkills)
const filtered = computed(() => store.filteredCatalog)
const installedGlobal = computed(() => store.installedGlobal)
const installedProject = computed(() => store.installedProject)
const loading = computed(() => store.loading)
const error = computed(() => store.error)
const categories = computed(() => store.categories)

const isInstalled = (id: string) => store.installedIdSet.has(id)
const isBusy = (id: string) => busyIds.value.has(id)

const tagFilter = ref('')

// 监听 store filter 变化
watch(() => store.filter, (f) => {
  selectedCategory.value = f.category || 'all'
  searchInput.value = f.search || ''
  tagFilter.value = f.tag || ''
}, { immediate: true })

function applyFilter() {
  store._setFilter({
    category: selectedCategory.value,
    search: searchInput.value,
    tag: tagFilter.value,
  })
}

function onCategoryClick(catId: string) {
  selectedCategory.value = catId
  applyFilter()
}

function onSearchChange() {
  applyFilter()
}

function clearFilters() {
  selectedCategory.value = 'all'
  searchInput.value = ''
  tagFilter.value = ''
  applyFilter()
}

// 卡片操作
async function onInstall(skill: Skill) {
  busyIds.value.add(skill.id)
  try {
    await store.install(skill.id, 'global')
    showToast(`已安装 ${skill.name}`)
  } catch (e) {
    showToast(`安装失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    busyIds.value.delete(skill.id)
  }
}

async function onUninstall(skill: Skill) {
  busyIds.value.add(skill.id)
  try {
    await store.uninstall(skill.id, 'global')
    showToast(`已卸载 ${skill.name}`)
  } catch (e) {
    showToast(`卸载失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    busyIds.value.delete(skill.id)
  }
}

function onView(skill: Skill) {
  detailSkill.value = skill
  showDetail.value = true
}

async function onRate(skill: Skill, score: number) {
  busyIds.value.add(skill.id)
  try {
    const result = await store.rate(skill.id, score)
    showToast(`已评分 ${score} 星（平均 ${result.average}，共 ${result.count} 次）`)
  } catch (e) {
    showToast(`评分失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    busyIds.value.delete(skill.id)
  }
}

function onApply(skill: Skill) {
  // 这里仅做演示：把模板的占位符信息展示
  showToast(`已应用 ${skill.name} 模板到主聊天（演示）`)
}

// 已安装侧栏
async function onUninstallFromPanel(skill: InstalledSkill, scope: 'global' | 'project') {
  busyIds.value.add(skill.id)
  try {
    await store.uninstall(skill.id, scope)
    showToast(`已卸载 ${skill.id}`)
  } catch (e) {
    showToast(`卸载失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    busyIds.value.delete(skill.id)
  }
}

function onApplyFromPanel(skill: InstalledSkill, _scope: 'global' | 'project') {
  showToast(`已应用 ${skill.name || skill.id}`)
}

async function onExportFromPanel(skill: InstalledSkill, scope: 'global' | 'project') {
  busyIds.value.add(skill.id)
  try {
    const result = await store.exportSkill(skill.id, scope)
    // 触发下载
    const blob = new Blob(
      [Uint8Array.from(atob(result.base64), c => c.charCodeAt(0))],
      { type: 'application/zip' },
    )
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = result.filename
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    showToast(`已导出 ${result.filename}（${(result.size / 1024).toFixed(1)}KB）`)
  } catch (e) {
    showToast(`导出失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    busyIds.value.delete(skill.id)
  }
}

function onRefreshInstalled() {
  store.fetchInstalled('global')
  store.fetchInstalled('project')
}

// 上传
async function onUploadSubmit(meta: Partial<Skill> & { id: string; name: string; version: string; category: string; prompt_template: string }) {
  busyIds.value.add(meta.id)
  try {
    await store.upload(meta, 'global')
    showToast(`已上传 ${meta.name}`)
  } catch (e) {
    showToast(`上传失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    busyIds.value.delete(meta.id)
  }
}

onMounted(async () => {
  await store.refreshAll()
})
</script>

<template>
  <div class="skill-market-view">
    <!-- 顶部统计 -->
    <div v-if="stats" class="stats-bar">
      <div class="stat-card">
        <div class="stat-value">{{ stats.official_count }}</div>
        <div class="stat-label">官方技能</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stats.global_installed }}</div>
        <div class="stat-label">已安装</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">⭐ {{ stats.avg_rating.toFixed(1) }}</div>
        <div class="stat-label">平均评分</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ (stats.total_downloads / 1000).toFixed(1) }}k</div>
        <div class="stat-label">总下载</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stats.categories }}</div>
        <div class="stat-label">分类数</div>
      </div>
    </div>

    <!-- Tab 切换 -->
    <div class="tab-bar">
      <div
        class="tab-item"
        :class="{ active: activeTab === 'discover' }"
        @click="activeTab = 'discover'"
      >
        🔍 发现
      </div>
      <div
        class="tab-item"
        :class="{ active: activeTab === 'installed' }"
        @click="activeTab = 'installed'"
      >
        📦 已安装 ({{ installedGlobal.length + installedProject.length }})
      </div>
      <div class="tab-spacer" />
      <button class="btn-upload" @click="showUpload = true">+ 上传技能</button>
    </div>

    <!-- 主体两栏 -->
    <div class="main">
      <!-- 左：发现 -->
      <div v-if="activeTab === 'discover'" class="discover-pane">
        <!-- 搜索 + 标签 -->
        <div class="search-bar">
          <input
            v-model="searchInput"
            class="search-input"
            placeholder="搜索技能 / 标签 / 描述…"
            @input="onSearchChange"
          />
          <button v-if="searchInput || tagFilter || selectedCategory !== 'all'" class="btn-clear" @click="clearFilters">清除</button>
        </div>

        <!-- 分类 -->
        <div class="category-strip">
          <button
            v-for="c in categories"
            :key="c.id"
            class="cat-chip"
            :class="{ active: selectedCategory === c.id }"
            @click="onCategoryClick(c.id)"
          >
            {{ c.label }} <span class="cat-count">{{ c.count }}</span>
          </button>
        </div>

        <!-- 错误 -->
        <div v-if="error" class="error-banner">⚠️ {{ error }}</div>

        <!-- 加载 -->
        <div v-if="loading" class="loading">加载中…</div>

        <!-- 推荐位（仅在没搜索时显示） -->
        <div v-if="!searchInput && selectedCategory === 'all' && !tagFilter && featured.length" class="featured">
          <div class="section-title">⭐ 推荐技能</div>
          <div class="card-grid">
            <SkillCard
              v-for="s in featured"
              :key="`feat-${s.id}`"
              :skill="s"
              :is-installed="isInstalled(s.id)"
              :is-installing="isBusy(s.id)"
              @install="onInstall"
              @uninstall="onUninstall"
              @view="onView"
            />
          </div>
        </div>

        <!-- 全部 -->
        <div v-if="filtered.length" class="all-skills">
          <div class="section-title-row">
            <div class="section-title">
              {{ searchInput || selectedCategory !== 'all' || tagFilter ? '搜索结果' : '全部技能' }}
              <span class="result-count">({{ filtered.length }})</span>
            </div>
          </div>
          <div class="card-grid">
            <SkillCard
              v-for="s in filtered"
              :key="s.id"
              :skill="s"
              :is-installed="isInstalled(s.id)"
              :is-installing="isBusy(s.id)"
              @install="onInstall"
              @uninstall="onUninstall"
              @view="onView"
            />
          </div>
        </div>

        <div v-else-if="!loading" class="empty-result">
          <div class="empty-icon">🔍</div>
          <div class="empty-text">没有匹配的技能</div>
          <button class="btn-clear-large" @click="clearFilters">清除筛选</button>
        </div>
      </div>

      <!-- 已安装 -->
      <div v-else class="installed-pane">
        <InstalledSkillsPanel
          :global-list="installedGlobal"
          :project-list="installedProject"
          :busy-ids="busyIds"
          @uninstall="onUninstallFromPanel"
          @apply="onApplyFromPanel"
          @export-skill="onExportFromPanel"
          @refresh="onRefreshInstalled"
        />
      </div>
    </div>

    <!-- 详情弹窗 -->
    <SkillDetailDialog
      v-model:show="showDetail"
      :skill="detailSkill"
      :is-installed="detailSkill ? isInstalled(detailSkill.id) : false"
      :is-busy="detailSkill ? isBusy(detailSkill.id) : false"
      @install="onInstall"
      @uninstall="onUninstall"
      @rate="onRate"
      @apply="onApply"
    />

    <!-- 上传弹窗 -->
    <UploadSkillDialog
      v-model:show="showUpload"
      @submit="onUploadSubmit"
    />
  </div>
</template>

<style scoped>
.skill-market-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 16px;
  gap: 14px;
  background: var(--yj-bg-base, #0d0d0d);
  overflow-y: auto;
}

/* 统计 */
.stats-bar {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 10px;
}

.stat-card {
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 8px;
  padding: 12px;
  text-align: center;
}

.stat-value {
  font-size: 20px;
  font-weight: 700;
  color: var(--yj-accent, #a855f7);
  font-family: 'Cascadia Code', 'Consolas', monospace;
}

.stat-label {
  font-size: 11px;
  color: var(--yj-text-secondary, #888);
  margin-top: 2px;
}

/* Tab */
.tab-bar {
  display: flex;
  gap: 4px;
  align-items: center;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
  padding-bottom: 0;
}

.tab-item {
  padding: 8px 14px;
  font-size: 13px;
  font-weight: 500;
  color: var(--yj-text-secondary, #888);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
  transition: all 0.15s;
}

.tab-item:hover {
  color: var(--yj-text-primary, #ddd);
}

.tab-item.active {
  color: var(--yj-accent, #a855f7);
  border-bottom-color: var(--yj-accent, #7c3aed);
}

.tab-spacer {
  flex: 1;
}

.btn-upload {
  background: var(--yj-accent, #7c3aed);
  color: #fff;
  border: none;
  padding: 6px 14px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}

.btn-upload:hover {
  background: #6d28d9;
}

/* 主区 */
.main {
  flex: 1;
  min-height: 0;
}

.discover-pane {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.search-bar {
  display: flex;
  gap: 8px;
}

.search-input {
  flex: 1;
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--yj-text-primary, #ddd);
  font-size: 13px;
}

.search-input:focus {
  outline: none;
  border-color: var(--yj-accent, #7c3aed);
}

.btn-clear {
  background: transparent;
  border: 1px solid var(--yj-border, #2a2a2a);
  color: var(--yj-text-secondary, #aaa);
  font-size: 12px;
  padding: 6px 12px;
  border-radius: 6px;
  cursor: pointer;
}

.btn-clear:hover {
  border-color: var(--yj-accent, #7c3aed);
}

.category-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.cat-chip {
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  color: var(--yj-text-secondary, #aaa);
  font-size: 12px;
  padding: 5px 12px;
  border-radius: 14px;
  cursor: pointer;
  transition: all 0.15s;
}

.cat-chip:hover {
  border-color: var(--yj-accent, #7c3aed);
  color: var(--yj-text-primary, #ddd);
}

.cat-chip.active {
  background: var(--yj-accent, #7c3aed);
  border-color: var(--yj-accent, #7c3aed);
  color: #fff;
}

.cat-count {
  display: inline-block;
  margin-left: 4px;
  font-size: 10px;
  padding: 0 5px;
  background: rgba(0, 0, 0, 0.2);
  border-radius: 6px;
}

.cat-chip.active .cat-count {
  background: rgba(255, 255, 255, 0.25);
}

.error-banner {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid #ef4444;
  color: #fca5a5;
  padding: 10px 14px;
  border-radius: 6px;
  font-size: 13px;
}

.loading {
  text-align: center;
  padding: 30px;
  color: var(--yj-text-secondary, #888);
  font-size: 14px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--yj-text-primary, #f5f5f5);
  margin-bottom: 10px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.section-title-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.result-count {
  font-size: 12px;
  color: var(--yj-text-secondary, #888);
  font-weight: 400;
}

.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 12px;
}

.empty-result {
  text-align: center;
  padding: 60px 20px;
  color: var(--yj-text-secondary, #888);
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 12px;
  opacity: 0.4;
}

.empty-text {
  font-size: 14px;
  margin-bottom: 14px;
}

.btn-clear-large {
  background: var(--yj-accent, #7c3aed);
  color: #fff;
  border: none;
  padding: 6px 18px;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
}

.btn-clear-large:hover {
  background: #6d28d9;
}

.installed-pane {
  height: 100%;
}
</style>
