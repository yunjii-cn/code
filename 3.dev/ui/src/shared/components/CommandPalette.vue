<!--
  CommandPalette.vue
  2026-06-09 TASK-2.8 引入：命令面板（Ctrl+K 触发）
  用法：
    <CommandPalette v-model:show="show" />
  内置命令：
    - 切换到 6 个 tab（chat / env / version / projects / github / settings）
    - 打开通知中心
    - 跳转到任意 view
    - 触发 toast 自检
-->
<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { showToast } from 'vant'

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ (e: 'update:show', v: boolean): void }>()

const router = useRouter()
const query = ref('')
const activeIndex = ref(0)
const inputRef = ref<HTMLInputElement | null>(null)

interface Command {
  id: string
  label: string
  hint?: string
  category: '导航' | '操作' | '设置'
  shortcut?: string
  action: () => void
}

const commands = computed<Command[]>(() => {
  const cmds: Command[] = [
    { id: 'goto-chat', label: '运行服务', category: '导航', hint: '主对话', shortcut: '1', action: () => router.push('/') },
    { id: 'goto-env', label: '部署维护', category: '导航', hint: '环境与依赖', shortcut: '2', action: () => router.push('/env') },
    { id: 'goto-version', label: '软件更新', category: '导航', hint: '版本管理', shortcut: '3', action: () => router.push('/version') },
    { id: 'goto-projects', label: '项目管理', category: '导航', hint: '项目列表', shortcut: '4', action: () => router.push('/projects') },
    { id: 'goto-github', label: 'GitHub', category: '导航', hint: 'PR/Issue', shortcut: '5', action: () => router.push('/github') },
    { id: 'goto-settings', label: '系统设置', category: '导航', hint: '偏好与系统', shortcut: '6', action: () => router.push('/settings') },
    { id: 'open-notif', label: '打开通知中心', category: '操作', hint: '查看历史通知', shortcut: 'Ctrl+Shift+N', action: () => window.dispatchEvent(new CustomEvent('yj:open-notifications')) },
    { id: 'reload-page', label: '刷新当前页', category: '操作', hint: '重载路由', action: () => window.location.reload() },
    { id: 'show-toast', label: '测试 toast 通知', category: '设置', hint: '触发一条成功 toast', action: () => showToast('🎉 命令面板 toast 测试') },
    { id: 'show-keys', label: '查看快捷键列表', category: '设置', hint: '跳转设置页', action: () => router.push('/settings') },
  ]
  const q = query.value.trim().toLowerCase()
  if (!q) return cmds
  return cmds.filter(c =>
    c.label.toLowerCase().includes(q) ||
    (c.hint && c.hint.toLowerCase().includes(q)) ||
    c.category.toLowerCase().includes(q)
  )
})

const groupedCommands = computed(() => {
  const groups: Record<string, Command[]> = {}
  for (const c of commands.value) {
    if (!groups[c.category]) groups[c.category] = []
    groups[c.category].push(c)
  }
  return groups
})

watch(() => props.show, async (v) => {
  if (v) {
    query.value = ''
    activeIndex.value = 0
    await nextTick()
    inputRef.value?.focus()
  }
})

function close() {
  emit('update:show', false)
}

function runCommand(cmd: Command) {
  cmd.action()
  close()
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    close()
    return
  }
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    activeIndex.value = Math.min(activeIndex.value + 1, commands.value.length - 1)
    return
  }
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    activeIndex.value = Math.max(activeIndex.value - 1, 0)
    return
  }
  if (e.key === 'Enter') {
    e.preventDefault()
    const cmd = commands.value[activeIndex.value]
    if (cmd) runCommand(cmd)
  }
}
</script>

<template>
  <van-popup
    :show="props.show"
    @update:show="emit('update:show', $event)"
    position="top"
    :style="{ background: 'transparent' }"
    :overlay="true"
  >
    <div class="cp-shell" @keydown="onKeyDown">
      <div class="cp-input-wrap">
        <span class="cp-search-icon">🔍</span>
        <input
          ref="inputRef"
          v-model="query"
          class="cp-input"
          type="text"
          placeholder="输入命令、页面或操作..."
          @input="activeIndex = 0"
        />
        <span class="cp-esc-hint">Esc</span>
      </div>

      <div class="cp-results">
        <div v-if="commands.length === 0" class="cp-empty">
          <div class="cp-empty-icon">🔍</div>
          <div class="cp-empty-text">没有匹配的命令</div>
        </div>
        <template v-else>
          <template v-for="(group, cat) in groupedCommands" :key="cat">
            <div class="cp-section-title">{{ cat }}</div>
            <div
              v-for="cmd in group"
              :key="cmd.id"
              class="cp-item"
              :class="{ 'cp-item--active': commands.indexOf(cmd) === activeIndex }"
              @click="runCommand(cmd)"
              @mouseenter="activeIndex = commands.indexOf(cmd)"
            >
              <div class="cp-item-body">
                <div class="cp-item-label">{{ cmd.label }}</div>
                <div v-if="cmd.hint" class="cp-item-hint">{{ cmd.hint }}</div>
              </div>
              <span v-if="cmd.shortcut" class="cp-item-shortcut">{{ cmd.shortcut }}</span>
            </div>
          </template>
        </template>
      </div>
    </div>
  </van-popup>
</template>

<style scoped>
.cp-shell {
  width: min(640px, 92vw);
  max-height: 70vh;
  margin: 14vh auto 0;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  font-family: inherit;
}

.cp-input-wrap {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
}

.cp-search-icon {
  font-size: 16px;
  color: var(--text-muted);
  flex-shrink: 0;
}

.cp-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  font-size: var(--font-lg);
  color: var(--text-primary);
  font-family: inherit;
}

.cp-input::placeholder {
  color: var(--text-muted);
}

.cp-esc-hint {
  font-size: 10px;
  color: var(--text-muted);
  border: 1px solid var(--border);
  border-radius: 3px;
  padding: 1px 5px;
  font-family: Consolas, monospace;
}

.cp-results {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-2) 0;
}

.cp-section-title {
  padding: var(--space-2) var(--space-4) var(--space-1);
  font-size: var(--font-xs);
  color: var(--text-muted);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.cp-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-4);
  margin: 0 var(--space-2);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.cp-item--active,
.cp-item:hover {
  background: var(--bg-card-hover);
}

.cp-item-body {
  flex: 1;
  min-width: 0;
}

.cp-item-label {
  font-size: var(--font-base);
  color: var(--text-primary);
  font-weight: 500;
}

.cp-item-hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
  margin-top: 2px;
}

.cp-item-shortcut {
  font-size: 11px;
  color: var(--text-muted);
  border: 1px solid var(--border);
  border-radius: 3px;
  padding: 1px 6px;
  font-family: Consolas, monospace;
  background: var(--bg-secondary);
  white-space: nowrap;
}

.cp-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-8);
  color: var(--text-muted);
}

.cp-empty-icon {
  font-size: 36px;
  margin-bottom: var(--space-2);
  opacity: 0.5;
}

.cp-empty-text {
  font-size: var(--font-sm);
}
</style>
