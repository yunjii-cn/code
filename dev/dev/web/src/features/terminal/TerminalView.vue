<script setup lang="ts">
import { ref, nextTick, computed, watch } from 'vue'
import { useDevice } from '@/composables/useDevice'
import { systemApi } from '@/api'
import { showToast } from 'vant'

const { isMobile } = useDevice()

interface OutputLine {
  text: string
  type: 'cmd' | 'stdout' | 'stderr' | 'error'
  timestamp?: number
  duration?: number
}

interface TerminalSession {
  id: string
  name: string
  output: OutputLine[]
  cwd: string
  command: string
  running: boolean
  history: string[]
  historyIndex: number
}

let sessionIdCounter = 0

function createSession(name?: string): TerminalSession {
  sessionIdCounter++
  return {
    id: `session-${sessionIdCounter}`,
    name: name || `终端 ${sessionIdCounter}`,
    output: [],
    cwd: '',
    command: '',
    running: false,
    history: [],
    historyIndex: -1,
  }
}

const sessions = ref<TerminalSession[]>([createSession()])
const activeSessionId = ref(sessions.value[0].id)

const activeSession = computed(() =>
  sessions.value.find((s) => s.id === activeSessionId.value) || sessions.value[0],
)

const outputEl = ref<HTMLElement>()
const autoScroll = ref(true)
const showSearch = ref(false)
const searchQuery = ref('')
const showHistoryPopup = ref(false)

const MAX_HISTORY = 100

const filteredOutput = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return activeSession.value.output
  return activeSession.value.output.filter((line) =>
    line.text.toLowerCase().includes(q),
  )
})

watch(activeSessionId, () => {
  showSearch.value = false
  searchQuery.value = ''
  showHistoryPopup.value = false
  scrollToBottom()
})

watch(
  () => activeSession.value.output.length,
  () => {
    if (autoScroll.value) {
      scrollToBottom()
    }
  },
)

function switchSession(id: string) {
  activeSessionId.value = id
}

function addSession() {
  const s = createSession()
  sessions.value.push(s)
  activeSessionId.value = s.id
}

function closeSession(id: string) {
  if (sessions.value.length <= 1) return
  const idx = sessions.value.findIndex((s) => s.id === id)
  sessions.value = sessions.value.filter((s) => s.id !== id)
  if (activeSessionId.value === id) {
    const newIdx = Math.min(idx, sessions.value.length - 1)
    activeSessionId.value = sessions.value[newIdx].id
  }
}

async function executeCommand() {
  const session = activeSession.value
  const cmd = session.command.trim()
  if (!cmd || session.running) return

  session.history.push(cmd)
  if (session.history.length > MAX_HISTORY) {
    session.history = session.history.slice(-MAX_HISTORY)
  }
  session.historyIndex = -1

  const startTime = Date.now()
  session.output.push({ text: cmd, type: 'cmd', timestamp: startTime })
  session.command = ''
  session.running = true
  showHistoryPopup.value = false
  scrollToBottom()

  try {
    const data: any = await systemApi.runCommand({
      cmd,
      cwd: session.cwd || undefined,
    })
    const duration = Date.now() - startTime
    if (data?.stdout) {
      session.output.push({ text: data.stdout, type: 'stdout', timestamp: Date.now(), duration })
    }
    if (data?.stderr) {
      session.output.push({ text: data.stderr, type: 'stderr', timestamp: Date.now(), duration })
    }
    if (data?.returncode !== undefined && data.returncode !== 0) {
      session.output.push({
        text: `退出码: ${data.returncode}`,
        type: 'error',
        timestamp: Date.now(),
        duration,
      })
    }
    if (!data?.stdout && !data?.stderr && data?.returncode === 0) {
      session.output.push({ text: '(命令执行完成，无输出)', type: 'stdout', timestamp: Date.now(), duration })
    }
  } catch (e: any) {
    session.output.push({
      text: `错误: ${e.message}`,
      type: 'error',
      timestamp: Date.now(),
      duration: Date.now() - startTime,
    })
  } finally {
    session.running = false
    scrollToBottom()
  }
}

function handleKeydown(e: KeyboardEvent) {
  const session = activeSession.value

  if (e.ctrlKey && e.key === 'f') {
    e.preventDefault()
    showSearch.value = !showSearch.value
    if (!showSearch.value) searchQuery.value = ''
    return
  }

  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    executeCommand()
    return
  }

  if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (session.command === '' && session.history.length > 0) {
      showHistoryPopup.value = true
    }
    if (session.history.length > 0) {
      if (session.historyIndex === -1) {
        session.historyIndex = session.history.length - 1
      } else if (session.historyIndex > 0) {
        session.historyIndex--
      }
      session.command = session.history[session.historyIndex]
    }
    return
  }

  if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (session.historyIndex !== -1) {
      if (session.historyIndex < session.history.length - 1) {
        session.historyIndex++
        session.command = session.history[session.historyIndex]
      } else {
        session.historyIndex = -1
        session.command = ''
      }
    }
    showHistoryPopup.value = false
    return
  }

  if (e.key === 'Escape') {
    showSearch.value = false
    searchQuery.value = ''
    showHistoryPopup.value = false
  }
}

function selectHistoryItem(cmd: string) {
  activeSession.value.command = cmd
  showHistoryPopup.value = false
}

function clearOutput() {
  activeSession.value.output = []
}

function scrollToBottom() {
  nextTick(() => {
    if (outputEl.value) {
      outputEl.value.scrollTop = outputEl.value.scrollHeight
    }
  })
}

function handleScroll() {
  if (!outputEl.value) return
  const el = outputEl.value
  const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40
  if (autoScroll.value && !atBottom) {
    autoScroll.value = false
  }
}

function formatTime(ts?: number) {
  if (!ts) return ''
  const d = new Date(ts)
  return d.toLocaleTimeString('zh-CN', { hour12: false })
}

function formatDuration(ms?: number) {
  if (ms === undefined || ms === null) return ''
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function getPromptPrefix() {
  const cwd = activeSession.value.cwd
  if (!cwd) return '~/'
  const parts = cwd.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || '~/'
}

const ANSI_COLORS: Record<string, string> = {
  '30': '#4a4a4a',
  '31': '#ff5252',
  '32': '#69f0ae',
  '33': '#ffd740',
  '34': '#448aff',
  '35': '#e040fb',
  '36': '#18ffff',
  '37': '#e0e0e0',
  '90': '#757575',
  '91': '#ff8a80',
  '92': '#b9f6ca',
  '93': '#ffe57f',
  '94': '#82b1ff',
  '95': '#ea80fc',
  '96': '#84ffff',
  '97': '#ffffff',
  '1': '',
}

function parseAnsi(text: string): Array<{ text: string; color?: string; bold?: boolean }> {
  const segments: Array<{ text: string; color?: string; bold?: boolean }> = []
  const ansiRegex = /\x1b\[([0-9;]*)m/g
  let lastIndex = 0
  let currentColor: string | undefined
  let isBold = false
  let match: RegExpExecArray | null

  while ((match = ansiRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      segments.push({
        text: text.slice(lastIndex, match.index),
        color: currentColor,
        bold: isBold,
      })
    }
    const codes = match[1].split(';')
    for (const code of codes) {
      if (code === '0') {
        currentColor = undefined
        isBold = false
      } else if (code === '1') {
        isBold = true
      } else if (ANSI_COLORS[code]) {
        currentColor = ANSI_COLORS[code]
      } else if (code === '39') {
        currentColor = undefined
      }
    }
    lastIndex = match.index + match[0].length
  }

  if (lastIndex < text.length) {
    segments.push({
      text: text.slice(lastIndex),
      color: currentColor,
      bold: isBold,
    })
  }

  return segments.length > 0 ? segments : [{ text }]
}

function highlightText(text: string): string {
  const q = searchQuery.value.trim()
  if (!q) return escapeHtml(text)
  const escaped = escapeHtml(text)
  const escapedQ = escapeHtml(q)
  const regex = new RegExp(`(${escapeRegex(escapedQ)})`, 'gi')
  return escaped.replace(regex, '<mark class="search-highlight">$1</mark>')
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

async function copyAllOutput() {
  const lines = activeSession.value.output.map((line) => {
    const prefix = line.type === 'cmd' ? '> ' : ''
    return prefix + line.text.replace(/\x1b\[[0-9;]*m/g, '')
  })
  const text = lines.join('\n')
  try {
    await navigator.clipboard.writeText(text)
    showToast('已复制到剪贴板')
  } catch {
    showToast('复制失败')
  }
}

function onOutputClick() {
  showHistoryPopup.value = false
}
</script>

<template>
  <div class="terminal-view" :class="{ mobile: isMobile }">
    <div class="terminal-header">
      <div class="terminal-tabs">
        <div
          v-for="session in sessions"
          :key="session.id"
          class="terminal-tab"
          :class="{ active: session.id === activeSessionId }"
          @click="switchSession(session.id)"
        >
          <span class="tab-name">{{ session.name }}</span>
          <span
            v-if="sessions.length > 1"
            class="tab-close"
            @click.stop="closeSession(session.id)"
          >&times;</span>
        </div>
        <div class="terminal-tab tab-add" @click="addSession">+</div>
      </div>
      <div class="terminal-actions">
        <van-button
          size="mini"
          plain
          :type="autoScroll ? 'primary' : 'default'"
          @click="autoScroll = !autoScroll"
        >
          {{ autoScroll ? '自动滚动' : '固定位置' }}
        </van-button>
        <van-button size="mini" plain @click="copyAllOutput">复制全部</van-button>
        <van-button size="mini" plain @click="clearOutput">清空</van-button>
      </div>
    </div>

    <div v-if="showSearch" class="terminal-search">
      <van-field
        v-model="searchQuery"
        placeholder="搜索输出内容..."
        class="search-field"
        clearable
        autofocus
      />
      <span class="search-count">
        {{ searchQuery.trim() ? `${filteredOutput.length} 条匹配` : '' }}
      </span>
    </div>

    <div
      ref="outputEl"
      class="terminal-output"
      @scroll="handleScroll"
      @click="onOutputClick"
    >
      <div v-if="activeSession.output.length === 0" class="terminal-placeholder">
        输入命令并按回车执行...<br />
        <span class="placeholder-hint">↑/↓ 浏览历史 | Ctrl+F 搜索输出</span>
      </div>
      <div
        v-for="(line, i) in (searchQuery.trim() ? filteredOutput : activeSession.output)"
        :key="i"
        class="terminal-line"
        :class="line.type"
      >
        <span v-if="line.type === 'cmd'" class="line-prompt">{{ getPromptPrefix() }}$&nbsp;</span>
        <span v-if="line.timestamp" class="line-time">{{ formatTime(line.timestamp) }}</span>
        <template v-if="parseAnsi(line.text).length > 1 || parseAnsi(line.text)[0]?.color">
          <span
            v-for="(seg, si) in parseAnsi(line.text)"
            :key="si"
            :style="{
              color: seg.color || undefined,
              fontWeight: seg.bold ? 'bold' : undefined,
            }"
            v-html="highlightText(seg.text)"
          />
        </template>
        <template v-else>
          <pre v-html="highlightText(line.text)" />
        </template>
        <span v-if="line.duration !== undefined" class="line-duration">{{ formatDuration(line.duration) }}</span>
      </div>
      <div v-if="activeSession.running" class="terminal-line running">
        <span class="running-indicator">●</span> 执行中...
      </div>
    </div>

    <div v-if="showHistoryPopup && activeSession.history.length > 0" class="history-popup">
      <div class="history-popup-title">命令历史</div>
      <div
        v-for="(cmd, i) in [...activeSession.history].reverse().slice(0, 20)"
        :key="i"
        class="history-item"
        @click="selectHistoryItem(cmd)"
      >
        {{ cmd }}
      </div>
    </div>

    <div class="terminal-input">
      <van-field
        v-model="activeSession.cwd"
        placeholder="工作目录(可选)"
        class="cwd-field"
      />
      <van-field
        v-model="activeSession.command"
        placeholder="输入命令..."
        class="cmd-field"
        @keydown="handleKeydown"
      />
      <van-button
        size="small"
        type="primary"
        :disabled="!activeSession.command.trim() || activeSession.running"
        @click="executeCommand"
      >
        执行
      </van-button>
    </div>
  </div>
</template>

<style scoped>
.terminal-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #0a0a0a;
  color: #e0e0e0;
  position: relative;
}

.terminal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.terminal-tabs {
  display: flex;
  align-items: stretch;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
}

.terminal-tabs::-webkit-scrollbar {
  height: 0;
}

.terminal-tab {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 14px;
  font-size: 12px;
  color: var(--text-muted);
  cursor: pointer;
  border-right: 1px solid var(--border);
  white-space: nowrap;
  transition: color 0.2s, background 0.2s;
  user-select: none;
}

.terminal-tab:hover {
  color: var(--text-primary);
  background: rgba(255, 255, 255, 0.03);
}

.terminal-tab.active {
  color: var(--accent);
  background: rgba(66, 165, 245, 0.08);
  border-bottom: 2px solid var(--accent);
}

.tab-name {
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-close {
  font-size: 14px;
  line-height: 1;
  color: var(--text-muted);
  padding: 0 2px;
  border-radius: 3px;
  transition: color 0.15s, background 0.15s;
}

.tab-close:hover {
  color: var(--error);
  background: rgba(244, 67, 54, 0.15);
}

.tab-add {
  font-size: 16px;
  font-weight: 600;
  padding: 8px 14px;
  color: var(--text-muted);
}

.tab-add:hover {
  color: var(--accent);
}

.terminal-actions {
  display: flex;
  gap: 4px;
  padding: 4px 8px;
  flex-shrink: 0;
}

.terminal-search {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.search-field {
  flex: 1;
  background: #0a0a0a;
  border-radius: 6px;
}

:deep(.search-field .van-field__control) {
  color: #e0e0e0;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
}

.search-count {
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
}

.terminal-output {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.6;
}

.terminal-placeholder {
  color: var(--text-muted);
  font-style: italic;
  line-height: 1.8;
}

.placeholder-hint {
  font-size: 11px;
  opacity: 0.6;
}

.terminal-line {
  margin-bottom: 2px;
  white-space: pre-wrap;
  word-break: break-all;
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 2px;
}

.terminal-line pre {
  margin: 0;
  font-family: inherit;
  font-size: inherit;
}

.line-prompt {
  color: #69f0ae;
  font-weight: 600;
  flex-shrink: 0;
}

.line-time {
  color: var(--text-muted);
  font-size: 11px;
  margin-right: 4px;
  flex-shrink: 0;
  opacity: 0.7;
}

.line-duration {
  color: var(--text-muted);
  font-size: 11px;
  margin-left: 8px;
  flex-shrink: 0;
  opacity: 0.6;
}

.terminal-line.cmd {
  color: var(--accent);
}

.terminal-line.stdout {
  color: #e0e0e0;
}

.terminal-line.stderr {
  color: #ffab40;
}

.terminal-line.error {
  color: var(--error);
}

.terminal-line.running {
  color: var(--accent);
  display: flex;
  align-items: center;
  gap: 6px;
}

.running-indicator {
  animation: pulse 1s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.history-popup {
  position: absolute;
  bottom: 52px;
  left: 16px;
  right: 16px;
  max-height: 240px;
  overflow-y: auto;
  background: #1a1a2e;
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.5);
  z-index: 100;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
}

.history-popup-title {
  padding: 8px 12px;
  font-size: 11px;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border);
  position: sticky;
  top: 0;
  background: #1a1a2e;
}

.history-item {
  padding: 6px 12px;
  cursor: pointer;
  color: #e0e0e0;
  transition: background 0.15s;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.history-item:hover {
  background: rgba(66, 165, 245, 0.12);
  color: var(--accent);
}

.terminal-input {
  display: flex;
  gap: 8px;
  padding: 8px 16px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}

.cwd-field {
  width: 200px;
  background: #0a0a0a;
  border-radius: 6px;
}

.cmd-field {
  flex: 1;
  background: #0a0a0a;
  border-radius: 6px;
}

:deep(.cwd-field .van-field__control),
:deep(.cmd-field .van-field__control) {
  color: #e0e0e0;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
}

:deep(.search-highlight) {
  background: rgba(255, 215, 64, 0.35);
  color: #fff;
  border-radius: 2px;
  padding: 0 1px;
}

.terminal-view.mobile .cwd-field {
  display: none;
}

.terminal-view.mobile .terminal-actions {
  gap: 2px;
  padding: 4px 4px;
}

.terminal-view.mobile .terminal-tab {
  padding: 8px 10px;
  font-size: 11px;
}

.terminal-view.mobile .tab-name {
  max-width: 60px;
}
</style>
