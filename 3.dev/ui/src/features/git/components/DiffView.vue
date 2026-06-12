<script setup lang="ts">
/**
 * DiffView.vue - 彩色 diff 渲染组件
 *
 * TASK-2.2 (2026-06-10) Git DiffView 增强
 *
 * 功能:
 *   1. 彩色 diff（红=删，绿=增）
 *   2. 双侧行号（old / new）
 *   3. hunk 折叠/展开（按 @@ 段）
 *   4. 搜索高亮（关键字在所有行中查找）
 *   5. 单文件回退按钮（确认后调 gitApi.restoreFile）
 *   6. 步骤关联（props 传入 step 列表，切换查看不同步骤的 diff）
 */
import { ref, computed, watch } from 'vue'
import { showConfirmDialog, showToast } from 'vant'
import { gitApi, type FileDiffData, type DiffHunk, type DiffLine } from '@/api'
import { useProjectStore } from '@/stores/project'
import { useDevice } from '@/composables/useDevice'

interface Props {
  /** 必填：项目根目录（绝对路径） */
  projectPath: string
  /** 必填：相对项目根的文件路径 */
  filePath: string
  /** 切换"未暂存 / 已暂存" */
  staged?: boolean
  /** 可选：外部注入的 diff 数据（用于离线渲染） */
  initialData?: FileDiffData | null
  /** 步骤关联（多步骤切换） */
  steps?: Array<{ id: string; label: string; file_path?: string }>
  /** 当前激活的步骤 id（v-model） */
  currentStepId?: string
}

const props = withDefaults(defineProps<Props>(), {
  staged: false,
  initialData: null,
  steps: () => [],
  currentStepId: '',
})

const emit = defineEmits<{
  (e: 'restored', payload: { file: string; staged: boolean }): void
  (e: 'step-change', stepId: string): void
}>()

const { isMobile } = useDevice()
const projectStore = useProjectStore()

const diffData = ref<FileDiffData | null>(props.initialData)
const loading = ref(false)
const errorMsg = ref('')
const searchKeyword = ref('')
const collapsedHunks = ref<Set<number>>(new Set())
const actionLoading = ref(false)

const totalHunks = computed(() => diffData.value?.hunks?.length ?? 0)
const additions = computed(() => diffData.value?.stats?.additions ?? 0)
const deletions = computed(() => diffData.value?.stats?.deletions ?? 0)

const hasSearch = computed(() => searchKeyword.value.trim().length > 0)

function lineMatches(line: DiffLine): boolean {
  if (!hasSearch.value) return true
  const kw = searchKeyword.value.toLowerCase()
  return line.text.toLowerCase().includes(kw)
}

function isHunkAllCollapsed(idx: number): boolean {
  return collapsedHunks.value.has(idx)
}

function toggleHunk(idx: number) {
  if (collapsedHunks.value.has(idx)) {
    collapsedHunks.value.delete(idx)
  } else {
    collapsedHunks.value.add(idx)
  }
  // 触发响应式
  collapsedHunks.value = new Set(collapsedHunks.value)
}

async function loadDiff() {
  if (!props.projectPath || !props.filePath) return
  loading.value = true
  errorMsg.value = ''
  try {
    const targetPath = props.currentStepId && props.steps.length
      ? props.steps.find(s => s.id === props.currentStepId)?.file_path || props.filePath
      : props.filePath
    const res: any = await gitApi.getFileDiff({
      project_path: props.projectPath,
      file_path: targetPath,
      staged: props.staged,
      context_lines: 3,
    })
    if (!res?.ok) {
      errorMsg.value = res?.error || '获取 diff 失败'
      diffData.value = null
      return
    }
    diffData.value = res.data as FileDiffData
    collapsedHunks.value = new Set()
  } catch (e: any) {
    errorMsg.value = e?.message || String(e)
    diffData.value = null
  } finally {
    loading.value = false
  }
}

async function handleRestore() {
  try {
    await showConfirmDialog({
      title: '确认回退？',
      message: `将丢弃 ${props.filePath} 的所有未保存改动（git restore）`,
      confirmButtonText: '确认回退',
      cancelButtonText: '取消',
    })
  } catch {
    return
  }
  actionLoading.value = true
  try {
    const res: any = await gitApi.restoreFile({
      project_path: props.projectPath,
      file_path: props.filePath,
      staged: false,
    })
    if (!res?.ok) {
      showToast({ type: 'fail', message: res?.error || '回退失败' })
      return
    }
    showToast({ type: 'success', message: '回退成功' })
    emit('restored', { file: props.filePath, staged: false })
    await loadDiff()
  } catch (e: any) {
    showToast({ type: 'fail', message: e?.message || '回退失败' })
  } finally {
    actionLoading.value = false
  }
}

async function handleUnstage() {
  actionLoading.value = true
  try {
    const res: any = await gitApi.restoreFile({
      project_path: props.projectPath,
      file_path: props.filePath,
      staged: true,
    })
    if (!res?.ok) {
      showToast({ type: 'fail', message: res?.error || '取消暂存失败' })
      return
    }
    showToast({ type: 'success', message: '已取消暂存' })
    emit('restored', { file: props.filePath, staged: true })
    await loadDiff()
  } catch (e: any) {
    showToast({ type: 'fail', message: e?.message || '取消暂存失败' })
  } finally {
    actionLoading.value = false
  }
}

function selectStep(stepId: string) {
  emit('step-change', stepId)
}

watch(
  () => [props.projectPath, props.filePath, props.staged, props.currentStepId],
  () => {
    loadDiff()
  },
  { immediate: true },
)
</script>

<template>
  <div class="diff-view" :class="{ 'is-mobile': isMobile }">
    <!-- 头部：文件名 + 操作按钮 -->
    <div class="diff-view__header">
      <div class="diff-view__file">
        <span class="diff-view__icon">📄</span>
        <span class="diff-view__filename">{{ filePath }}</span>
      </div>
      <div v-if="diffData" class="diff-view__stats">
        <span class="diff-view__add">+{{ additions }}</span>
        <span class="diff-view__del">-{{ deletions }}</span>
      </div>
    </div>

    <!-- 步骤切换器（多步骤关联） -->
    <div v-if="steps.length > 1" class="diff-view__steps">
      <button
        v-for="step in steps"
        :key="step.id"
        class="diff-view__step"
        :class="{ 'is-active': step.id === currentStepId }"
        @click="selectStep(step.id)"
      >
        {{ step.label }}
      </button>
    </div>

    <!-- 工具栏：搜索 + 回退 -->
    <div class="diff-view__toolbar">
      <input
        v-model="searchKeyword"
        type="text"
        class="diff-view__search"
        placeholder="🔍 搜索关键字（高亮匹配行）"
      />
      <button
        v-if="!staged"
        class="diff-view__btn diff-view__btn--danger"
        :disabled="actionLoading"
        @click="handleRestore"
      >
        ↩ 回退
      </button>
      <button
        v-else
        class="diff-view__btn"
        :disabled="actionLoading"
        @click="handleUnstage"
      >
        取消暂存
      </button>
    </div>

    <!-- 加载 / 错误状态 -->
    <div v-if="loading" class="diff-view__loading">
      <div class="diff-view__spinner" />
      <span>正在加载 diff…</span>
    </div>
    <div v-else-if="errorMsg" class="diff-view__error">
      <span>❌ {{ errorMsg }}</span>
      <button class="diff-view__btn" @click="loadDiff">重试</button>
    </div>
    <div v-else-if="!diffData || totalHunks === 0" class="diff-view__empty">
      ✨ 无改动
    </div>

    <!-- Hunk 列表 -->
    <div v-else class="diff-view__hunks">
      <div
        v-for="(hunk, idx) in diffData.hunks"
        :key="idx"
        class="diff-view__hunk"
      >
        <button
          class="diff-view__hunk-header"
          @click="toggleHunk(idx)"
        >
          <span class="diff-view__hunk-toggle">
            {{ isHunkAllCollapsed(idx) ? '▶' : '▼' }}
          </span>
          <span class="diff-view__hunk-info">
            @@ -{{ hunk.oldStart }},{{ hunk.oldLines }}
            +{{ hunk.newStart }},{{ hunk.newLines }} @@
            <span v-if="hunk.header" class="diff-view__hunk-section">
              {{ hunk.header }}
            </span>
          </span>
        </button>

        <div
          v-if="!isHunkAllCollapsed(idx)"
          class="diff-view__lines"
        >
          <div
            v-for="(line, lineIdx) in hunk.lines"
            :key="lineIdx"
            class="diff-view__line"
            :class="[
              `diff-view__line--${line.type}`,
              { 'is-match': hasSearch && lineMatches(line) },
            ]"
          >
            <span class="diff-view__lineno diff-view__lineno--old">
              {{ line.oldLineNo ?? '' }}
            </span>
            <span class="diff-view__lineno diff-view__lineno--new">
              {{ line.newLineNo ?? '' }}
            </span>
            <span class="diff-view__sign">
              <template v-if="line.type === 'add'">+</template>
              <template v-else-if="line.type === 'remove'">-</template>
              <template v-else> </template>
            </span>
            <span class="diff-view__text">{{ line.text || ' ' }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.diff-view {
  --diff-font: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  --diff-add-bg: rgba(46, 160, 67, 0.15);
  --diff-add-border: rgba(46, 160, 67, 0.4);
  --diff-del-bg: rgba(248, 81, 73, 0.15);
  --diff-del-border: rgba(248, 81, 73, 0.4);
  --diff-context-bg: transparent;
  --diff-match-bg: rgba(255, 235, 59, 0.3);

  display: flex;
  flex-direction: column;
  height: 100%;
  font-family: var(--diff-font);
  font-size: 13px;
  line-height: 1.5;
  background: var(--yj-bg-elevated, #1e1e1e);
  color: var(--yj-text-primary, #e0e0e0);
  border: 1px solid var(--yj-border, #333);
  border-radius: 6px;
  overflow: hidden;
}

.diff-view__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--yj-bg-secondary, #252525);
  border-bottom: 1px solid var(--yj-border, #333);
}

.diff-view__file {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 500;
}

.diff-view__filename {
  font-family: var(--diff-font);
  word-break: break-all;
}

.diff-view__stats {
  display: flex;
  gap: 8px;
  font-weight: 600;
  white-space: nowrap;
}

.diff-view__add {
  color: #2ea043;
}

.diff-view__del {
  color: #f85149;
}

.diff-view__steps {
  display: flex;
  gap: 4px;
  padding: 6px 12px;
  background: var(--yj-bg-tertiary, #2a2a2a);
  border-bottom: 1px solid var(--yj-border, #333);
  overflow-x: auto;
}

.diff-view__step {
  padding: 4px 10px;
  background: transparent;
  border: 1px solid var(--yj-border, #444);
  border-radius: 4px;
  color: var(--yj-text-secondary, #999);
  cursor: pointer;
  font-size: 12px;
  white-space: nowrap;
  transition: all 0.15s;
}

.diff-view__step:hover {
  background: var(--yj-bg-hover, #333);
}

.diff-view__step.is-active {
  background: var(--yj-accent, #42a5f5);
  color: white;
  border-color: var(--yj-accent, #42a5f5);
}

.diff-view__toolbar {
  display: flex;
  gap: 6px;
  padding: 6px 12px;
  background: var(--yj-bg-tertiary, #2a2a2a);
  border-bottom: 1px solid var(--yj-border, #333);
}

.diff-view__search {
  flex: 1;
  padding: 4px 8px;
  background: var(--yj-bg-primary, #1a1a1a);
  border: 1px solid var(--yj-border, #444);
  border-radius: 4px;
  color: inherit;
  font-family: inherit;
  font-size: 12px;
}

.diff-view__search:focus {
  outline: none;
  border-color: var(--yj-accent, #42a5f5);
}

.diff-view__btn {
  padding: 4px 10px;
  background: var(--yj-bg-secondary, #252525);
  border: 1px solid var(--yj-border, #444);
  border-radius: 4px;
  color: inherit;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;
}

.diff-view__btn:hover:not(:disabled) {
  background: var(--yj-bg-hover, #333);
}

.diff-view__btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.diff-view__btn--danger {
  color: #f85149;
  border-color: rgba(248, 81, 73, 0.4);
}

.diff-view__btn--danger:hover:not(:disabled) {
  background: rgba(248, 81, 73, 0.1);
}

.diff-view__loading,
.diff-view__error,
.diff-view__empty {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px;
  color: var(--yj-text-secondary, #999);
  font-size: 13px;
}

.diff-view__spinner {
  width: 14px;
  height: 14px;
  border: 2px solid var(--yj-border, #444);
  border-top-color: var(--yj-accent, #42a5f5);
  border-radius: 50%;
  animation: diff-spin 0.8s linear infinite;
}

@keyframes diff-spin {
  to { transform: rotate(360deg); }
}

.diff-view__error {
  color: #f85149;
}

.diff-view__hunks {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.diff-view__hunk {
  border-bottom: 1px solid var(--yj-border, #333);
}

.diff-view__hunk:last-child {
  border-bottom: none;
}

.diff-view__hunk-header {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 4px 12px;
  background: var(--yj-bg-tertiary, #2a2a2a);
  border: none;
  color: var(--yj-text-secondary, #999);
  cursor: pointer;
  font-family: var(--diff-font);
  font-size: 12px;
  text-align: left;
}

.diff-view__hunk-header:hover {
  background: var(--yj-bg-hover, #333);
}

.diff-view__hunk-toggle {
  font-size: 10px;
  width: 12px;
}

.diff-view__hunk-info {
  font-weight: 500;
}

.diff-view__hunk-section {
  color: var(--yj-text-tertiary, #777);
  font-style: italic;
  margin-left: 4px;
}

.diff-view__lines {
  background: var(--diff-context-bg);
}

.diff-view__line {
  display: grid;
  grid-template-columns: 50px 50px 20px 1fr;
  align-items: start;
  font-size: 12px;
  min-height: 20px;
}

.diff-view__line--add {
  background: var(--diff-add-bg);
  border-left: 3px solid var(--diff-add-border);
}

.diff-view__line--remove {
  background: var(--diff-del-bg);
  border-left: 3px solid var(--diff-del-border);
}

.diff-view__line--context {
  border-left: 3px solid transparent;
}

.diff-view__line.is-match {
  background: var(--diff-match-bg) !important;
  font-weight: 500;
}

.diff-view__lineno {
  text-align: right;
  padding: 0 6px;
  color: var(--yj-text-tertiary, #666);
  user-select: none;
  font-size: 11px;
}

.diff-view__lineno--old {
  border-right: 1px solid var(--yj-border, #333);
}

.diff-view__lineno--new {
  border-right: 1px solid var(--yj-border, #333);
}

.diff-view__sign {
  text-align: center;
  user-select: none;
  color: var(--yj-text-secondary, #888);
  font-weight: 600;
}

.diff-view__line--add .diff-view__sign {
  color: #2ea043;
}

.diff-view__line--remove .diff-view__sign {
  color: #f85149;
}

.diff-view__text {
  padding: 0 6px;
  white-space: pre-wrap;
  word-break: break-all;
}

/* 移动端：减小行号列 */
.diff-view.is-mobile .diff-view__line {
  grid-template-columns: 36px 36px 16px 1fr;
  font-size: 11px;
}

.diff-view.is-mobile .diff-view__lineno {
  font-size: 10px;
  padding: 0 4px;
}
</style>
