<script setup lang="ts">
/**
 * Autocomplete.vue - 弹出式候选列表
 *
 * TASK-2.4 (2026-06-10) Composer Autocomplete
 *
 * 4 类触发:
 *   - `/` command: 系统/自定义命令
 *   - `/review`:  代码审查子模式
 *   - `$` skill:   技能列表（v2.0 技能市场）
 *   - `@` file:    workspace 文件路径
 *
 * 使用方式:
 *   <Autocomplete
 *     :open="autoOpen"
 *     :type="autoType"
 *     :query="autoQuery"
 *     @select="handleSelect"
 *   />
 */
import { computed, ref, watch } from 'vue'
import { systemApi, skillMarketApi } from '@/api'
import { useProjectStore } from '@/stores/project'

export interface AutocompleteItem {
  /** 插入到输入框的值（不含 trigger 符） */
  value: string
  /** 显示文本（可与 value 不同） */
  label: string
  /** 描述/副标题 */
  description?: string
  /** 图标 emoji */
  icon?: string
  /** 候选项类型 */
  kind: 'file' | 'command' | 'review' | 'skill'
}

interface Props {
  open: boolean
  /** 触发类型 */
  type: 'file' | 'command' | 'review' | 'skill' | null
  /** 查询字符串（不含 trigger 符） */
  query: string
  /** 当前输入框的完整文本（用于解析 prefix） */
  fullText: string
  /** 触发符起始位置 */
  triggerStart: number
  /** 加载状态（由父组件控制） */
  loading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
})

const emit = defineEmits<{
  (e: 'select', item: AutocompleteItem): void
  (e: 'close'): void
  (e: 'navigate', direction: 'up' | 'down'): void
}>()

const projectStore = useProjectStore()

const items = ref<AutocompleteItem[]>([])
const activeIndex = ref(0)
const loadingInternal = ref(false)

// 内置命令列表（可以由后端 /api/prompts 覆盖）
const BUILTIN_COMMANDS: AutocompleteItem[] = [
  { value: 'help', label: '/help', description: '显示帮助', icon: '❓', kind: 'command' },
  { value: 'clear', label: '/clear', description: '清空对话', icon: '🧹', kind: 'command' },
  { value: 'compact', label: '/compact', description: '压缩上下文', icon: '🗜️', kind: 'command' },
  { value: 'init', label: '/init', description: '分析当前项目', icon: '🏗️', kind: 'command' },
  { value: 'review', label: '/review', description: '代码审查子模式', icon: '🔍', kind: 'command' },
  { value: 'test', label: '/test', description: '运行测试', icon: '🧪', kind: 'command' },
  { value: 'doc', label: '/doc', description: '生成文档', icon: '📝', kind: 'command' },
  { value: 'git', label: '/git', description: 'Git 助手', icon: '🌿', kind: 'command' },
]

// /review 子模式
const REVIEW_MODES: AutocompleteItem[] = [
  { value: 'security', label: '安全审查', description: '检查漏洞、XSS、注入等', icon: '🔒', kind: 'review' },
  { value: 'performance', label: '性能审查', description: '算法复杂度、N+1、内存', icon: '⚡', kind: 'review' },
  { value: 'style', label: '代码风格', description: '命名/格式/最佳实践', icon: '🎨', kind: 'review' },
  { value: 'tests', label: '测试覆盖', description: '缺失的测试用例', icon: '🧪', kind: 'review' },
  { value: 'all', label: '全面审查', description: '安全+性能+风格+测试', icon: '🧐', kind: 'review' },
]

// 静态搜索匹配
function filterByQuery<T extends { value: string; label: string }>(list: T[], q: string): T[] {
  if (!q) return list
  const lc = q.toLowerCase()
  return list.filter((it) => it.value.toLowerCase().includes(lc) || it.label.toLowerCase().includes(lc))
}

// 加载文件列表
async function loadFileList(query: string) {
  loadingInternal.value = true
  try {
    const projectPath = projectStore.activeProject?.path || ''
    const res: any = await systemApi.searchFiles?.({ path: projectPath, query, limit: 20 })
    if (res?.ok && Array.isArray(res.data)) {
      items.value = res.data.map((f: any) => ({
        value: f.path || f.name,
        label: f.path || f.name,
        description: f.size ? `${(f.size / 1024).toFixed(1)}KB` : undefined,
        icon: '📄',
        kind: 'file' as const,
      }))
    } else {
      items.value = []
    }
  } catch {
    items.value = []
  } finally {
    loadingInternal.value = false
  }
}

// 加载技能列表
async function loadSkillList(query: string) {
  loadingInternal.value = true
  try {
    const res: any = await skillMarketApi.listSkills?.({ query, limit: 20 })
    if (res?.ok && Array.isArray(res.data)) {
      items.value = res.data.map((s: any) => ({
        value: s.id || s.name,
        label: s.name || s.id,
        description: s.description,
        icon: '🧩',
        kind: 'skill' as const,
      }))
    } else {
      items.value = []
    }
  } catch {
    items.value = []
  } finally {
    loadingInternal.value = false
  }
}

// 加载 prompt（命令）列表
async function loadPromptList() {
  loadingInternal.value = true
  try {
    // 尝试从后端 /api/prompts 拉取，失败回退到内置
    const res: any = await (systemApi as any).listPrompts?.()
    let all = BUILTIN_COMMANDS
    if (res?.ok && Array.isArray(res.data)) {
      const userPrompts: AutocompleteItem[] = res.data.map((p: any) => ({
        value: p.name,
        label: p.name,
        description: p.description,
        icon: '📜',
        kind: 'command' as const,
      }))
      all = [...userPrompts, ...BUILTIN_COMMANDS]
    }
    items.value = filterByQuery(all, props.query)
  } catch {
    items.value = filterByQuery(BUILTIN_COMMANDS, props.query)
  } finally {
    loadingInternal.value = false
  }
}

// 根据 type 加载
async function loadItems() {
  if (!props.open || !props.type) {
    items.value = []
    return
  }
  activeIndex.value = 0
  if (props.type === 'file') {
    await loadFileList(props.query)
  } else if (props.type === 'skill') {
    await loadSkillList(props.query)
  } else if (props.type === 'command') {
    await loadPromptList()
  } else if (props.type === 'review') {
    items.value = filterByQuery(REVIEW_MODES, props.query)
  }
}

watch(
  () => [props.open, props.type, props.query],
  () => loadItems(),
  { immediate: true },
)

const isLoading = computed(() => props.loading || loadingInternal.value)

const titleText = computed(() => {
  switch (props.type) {
    case 'file': return '📁 文件'
    case 'command': return '⚡ 命令'
    case 'review': return '🔍 审查模式'
    case 'skill': return '🧩 技能'
    default: return ''
  }
})

function onItemClick(idx: number) {
  const item = items.value[idx]
  if (item) emit('select', item)
}

function onItemHover(idx: number) {
  activeIndex.value = idx
}

function onKeyDown(e: KeyboardEvent): boolean {
  if (!props.open || items.value.length === 0) return false
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    activeIndex.value = (activeIndex.value + 1) % items.value.length
    return true
  }
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    activeIndex.value = (activeIndex.value - 1 + items.value.length) % items.value.length
    return true
  }
  if (e.key === 'Enter' || e.key === 'Tab') {
    e.preventDefault()
    const item = items.value[activeIndex.value]
    if (item) {
      emit('select', item)
      return true
    }
  }
  if (e.key === 'Escape') {
    e.preventDefault()
    emit('close')
    return true
  }
  return false
}

defineExpose({ onKeyDown })
</script>

<template>
  <div v-if="open && type" class="autocomplete" role="listbox">
    <div class="autocomplete__header">
      <span class="autocomplete__title">{{ titleText }}</span>
      <span v-if="query" class="autocomplete__query">{{ query }}</span>
    </div>
    <div v-if="isLoading" class="autocomplete__loading">
      <span>加载中…</span>
    </div>
    <div v-else-if="items.length === 0" class="autocomplete__empty">
      无匹配项
    </div>
    <ul v-else class="autocomplete__list">
      <li
        v-for="(item, idx) in items"
        :key="`${item.kind}-${item.value}-${idx}`"
        class="autocomplete__item"
        :class="{ 'is-active': idx === activeIndex }"
        role="option"
        :aria-selected="idx === activeIndex"
        @mousedown.prevent="onItemClick(idx)"
        @mouseenter="onItemHover(idx)"
      >
        <span class="autocomplete__icon">{{ item.icon || '•' }}</span>
        <div class="autocomplete__content">
          <div class="autocomplete__label">{{ item.label }}</div>
          <div v-if="item.description" class="autocomplete__desc">{{ item.description }}</div>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.autocomplete {
  position: absolute;
  bottom: 100%;
  left: 8px;
  right: 8px;
  margin-bottom: 4px;
  max-height: 280px;
  background: var(--bg-elevated, #2a2a2a);
  border: 1px solid var(--border, #444);
  border-radius: 6px;
  box-shadow: 0 -4px 12px rgba(0, 0, 0, 0.4);
  z-index: 100;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.autocomplete__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 10px;
  background: var(--bg-secondary, #252525);
  border-bottom: 1px solid var(--border, #333);
  font-size: 12px;
  color: var(--text-secondary, #999);
}

.autocomplete__title {
  font-weight: 500;
}

.autocomplete__query {
  font-family: ui-monospace, monospace;
  color: var(--accent, #42a5f5);
}

.autocomplete__loading,
.autocomplete__empty {
  padding: 16px;
  text-align: center;
  color: var(--text-secondary, #999);
  font-size: 12px;
}

.autocomplete__list {
  list-style: none;
  margin: 0;
  padding: 4px 0;
  overflow-y: auto;
  flex: 1;
}

.autocomplete__item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  cursor: pointer;
  transition: background 0.1s;
}

.autocomplete__item.is-active {
  background: var(--accent-bg, rgba(66, 165, 245, 0.2));
}

.autocomplete__icon {
  flex: 0 0 24px;
  font-size: 16px;
  text-align: center;
}

.autocomplete__content {
  flex: 1;
  min-width: 0;
}

.autocomplete__label {
  font-size: 13px;
  color: var(--text-primary, #e0e0e0);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.autocomplete__desc {
  font-size: 11px;
  color: var(--text-tertiary, #888);
  margin-top: 1px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
