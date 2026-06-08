<script setup lang="ts">
import { ref, computed, nextTick, watch, onMounted } from 'vue'
import { showConfirmDialog, showToast } from 'vant'
import { useChatStore } from '@/stores/chat'
import { aiApi } from '@/api'
import { useModelStore } from '@/stores/model'
import { useProjectStore } from '@/stores/project'
import { useDevice } from '@/composables/useDevice'
import type { Conversation } from '@/stores/project'
import MarkdownRenderer from '@/components/MarkdownRenderer.vue'
import RightSidebar from '@/components/RightSidebar.vue'

const { isMobile, isDesktop } = useDevice()

const chatStore = useChatStore()
const modelStore = useModelStore()
const projectStore = useProjectStore()

const inputText = ref('')
const messagesContainer = ref<HTMLElement>()

// 2026-06-08 TASK-2.3 引入：图片附件（多模态）
interface AttachedImage {
  id: string
  dataUrl: string
  name: string
  size: number
}
const attachedImages = ref<AttachedImage[]>([])
const imageInputRef = ref<HTMLInputElement | null>(null)
const MAX_IMAGES = 4
const MAX_IMAGE_SIZE = 5 * 1024 * 1024 // 5MB

function pickImages() {
  imageInputRef.value?.click()
}

async function handleImageFiles(files: FileList | File[]) {
  const list = Array.from(files)
  for (const file of list) {
    if (attachedImages.value.length >= MAX_IMAGES) {
      showToast(`最多 ${MAX_IMAGES} 张图片`)
      break
    }
    if (!file.type.startsWith('image/')) {
      showToast(`跳过非图片文件: ${file.name}`)
      continue
    }
    if (file.size > MAX_IMAGE_SIZE) {
      showToast(`图片过大 (${(file.size / 1024 / 1024).toFixed(1)}MB > 5MB): ${file.name}`)
      continue
    }
    try {
      const dataUrl = await readFileAsDataUrl(file)
      attachedImages.value.push({
        id: `img-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        dataUrl,
        name: file.name,
        size: file.size,
      })
    } catch (e: any) {
      showToast(`读取失败: ${e.message}`)
    }
  }
}

function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(reader.result as string)
    reader.onerror = () => reject(reader.error || new Error('FileReader error'))
    reader.readAsDataURL(file)
  })
}

function onImageInputChange(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    handleImageFiles(target.files)
    target.value = ''
  }
}

function removeImage(id: string) {
  const idx = attachedImages.value.findIndex(i => i.id === id)
  if (idx >= 0) attachedImages.value.splice(idx, 1)
}

function clearImages() {
  attachedImages.value = []
}

function handleImagePaste(e: ClipboardEvent) {
  if (!e.clipboardData) return
  const items = e.clipboardData.items
  const files: File[] = []
  for (let i = 0; i < items.length; i++) {
    const it = items[i]
    if (it.kind === 'file' && it.type.startsWith('image/')) {
      const f = it.getAsFile()
      if (f) files.push(f)
    }
  }
  if (files.length > 0) {
    e.preventDefault()
    handleImageFiles(files)
  }
}

function handleImageDrop(e: DragEvent) {
  if (!e.dataTransfer) return
  const files = e.dataTransfer.files
  if (files && files.length > 0) {
    const images = Array.from(files).filter(f => f.type.startsWith('image/'))
    if (images.length > 0) {
      e.preventDefault()
      handleImageFiles(images)
    }
  }
}
const sidebarCollapsed = ref(false)
const rightSidebarCollapsed = ref(false)
const showSessionSheet = ref(false)
const showProviderPicker = ref(false)
const showModelPicker = ref(false)

const searchKeyword = ref('')
const searchedConversations = ref<Conversation[] | null>(null)

const renameDialogVisible = ref(false)
const renameSessionId = ref('')
const renameInput = ref('')

const providerActions = computed(() =>
  modelStore.providers
    .filter((p) => p.enabled)
    .map((p) => ({ name: p.name, value: p.id }))
)

const modelActions = computed(() =>
  modelStore.models.map((m) => ({ name: m.name, value: m.id }))
)

const currentProviderName = computed(() => {
  const p = modelStore.providers.find((p) => p.id === modelStore.currentProvider)
  return p?.name || '选择提供商'
})

const currentModelName = computed(() => {
  const m = modelStore.models.find((m) => m.id === modelStore.currentModel)
  return m?.name || '选择模型'
})

const conversations = computed(() =>
  searchedConversations.value ?? projectStore.conversations
)

const todayConversations = computed(() => {
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime()
  return conversations.value.filter((c) => c.updatedAt >= today)
})

const earlierConversations = computed(() => {
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime()
  return conversations.value.filter((c) => c.updatedAt < today)
})

function scrollToBottom() {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
    }
  })
}

watch(
  () => chatStore.messages.length,
  () => scrollToBottom()
)

watch(
  () => chatStore.messages[chatStore.messages.length - 1]?.content,
  () => scrollToBottom()
)

watch(
  () => chatStore.isStreaming,
  async (newVal, oldVal) => {
    if (oldVal === true && newVal === false) {
      await autoSaveConversation()
    }
  }
)

async function autoSaveConversation() {
  if (!chatStore.currentSessionId || chatStore.messages.length === 0) return
  if (!projectStore.activeProject) return

  const conv = projectStore.conversations.find(
    (c) => c.id === chatStore.currentSessionId
  )
  let title = conv?.title
  if (!title) {
    const firstUserMsg = chatStore.messages.find((m) => m.role === 'user')
    if (firstUserMsg) {
      title = firstUserMsg.content.slice(0, 30).replace(/\n/g, ' ')
      if (firstUserMsg.content.length > 30) title += '...'
    }
  }

  await projectStore.saveConversation(
    chatStore.currentSessionId,
    chatStore.messages.map((m) => ({ role: m.role, content: m.content })),
    title
  )
}

async function handleSearch() {
  const keyword = searchKeyword.value.trim()
  if (!keyword) {
    searchedConversations.value = null
    return
  }
  searchedConversations.value = await projectStore.searchConversations(keyword)
}

function clearSearch() {
  searchKeyword.value = ''
  searchedConversations.value = null
}

watch(searchKeyword, (val) => {
  if (!val.trim()) {
    searchedConversations.value = null
  }
})

async function handleDeleteConversation(sessionId: string) {
  try {
    await showConfirmDialog({
      title: '删除对话',
      message: '确定要删除这个对话吗？此操作不可撤销。',
      confirmButtonText: '删除',
      confirmButtonColor: '#e53935',
      cancelButtonText: '取消',
    })
    await projectStore.deleteConversation(sessionId)
    if (chatStore.currentSessionId === sessionId) {
      chatStore.clearChat()
    }
  } catch {
    // cancelled
  }
}

function handleRenameConversation(sessionId: string, currentTitle: string) {
  renameSessionId.value = sessionId
  renameInput.value = currentTitle
  renameDialogVisible.value = true
}

async function confirmRename() {
  const newTitle = renameInput.value.trim()
  if (!newTitle || !renameSessionId.value) return
  await projectStore.renameConversation(renameSessionId.value, newTitle)
  renameSessionId.value = ''
  renameInput.value = ''
}

function handleRenameBeforeClose(action: string, done: () => void) {
  if (action === 'confirm') {
    const newTitle = renameInput.value.trim()
    if (!newTitle) {
      return
    }
  }
  done()
}

async function handleSend() {
  const text = inputText.value.trim()
  if (!text || chatStore.isStreaming) return
  const images = attachedImages.value.length > 0
    ? attachedImages.value.map(i => i.dataUrl)
    : undefined
  inputText.value = ''
  clearImages()
  scrollToBottom()
  await chatStore.sendMessage(text, images)
}

function handleKeydown(e: KeyboardEvent) {
  // 2026-06-08 TASK-2.4: Autocomplete 打开时键盘交给它处理
  if (autocompleteOpen.value && onAutocompleteKeydown(e)) return
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleSend()
  }
}

// 2026-06-08 TASK-2.4 引入：Composer Autocomplete
type AutocompleteType = 'file' | 'command' | null
interface AutocompleteItem {
  id: string
  label: string
  description: string
  value: string
  type: 'file' | 'command'
}
const autocompleteOpen = ref(false)
const autocompleteType = ref<AutocompleteType>(null)
const autocompleteQuery = ref('')
const autocompleteStart = ref(0)
const autocompleteIndex = ref(0)
const autocompleteItems = ref<AutocompleteItem[]>([])
const autocompleteLoading = ref(false)
let autocompleteTimer: number | null = null

interface SlashCommand {
  id: string
  name: string
  description: string
  prompt: string
}
let cachedSlashCommands: SlashCommand[] | null = null

async function loadSlashCommands(): Promise<SlashCommand[]> {
  if (cachedSlashCommands) return cachedSlashCommands
  try {
    const res: any = await aiApi.listSlashCommands()
    if (res?.ok && res.data) {
      cachedSlashCommands = res.data as SlashCommand[]
      return cachedSlashCommands
    }
  } catch {}
  return []
}

function detectTrigger(text: string, cursorPos: number): { type: 'file' | 'command'; start: number; query: string } | null {
  const before = text.slice(0, cursorPos)
  // 2026-06-08 TASK-2.4: 用 new RegExp 绕开 TS 6.0 + vue-tsc 3.2 的 regex literal 解析 bug
  const atRe = new RegExp('(^|[^A-Za-z0-9])@([\\w./-]*)$')
  const atMatch = before.match(atRe)
  if (atMatch) {
    return { type: 'file', start: atMatch.index! + atMatch[1].length, query: atMatch[2] }
  }
  const slashRe = new RegExp('(^|\\s)/([\\w-]*)$')
  const slashMatch = before.match(slashRe)
  if (slashMatch && slashMatch[1]) {
    return { type: 'command', start: slashMatch.index! + slashMatch[1].length, query: slashMatch[2] }
  }
  return null
}

async function performAutocompleteSearch(type: 'file' | 'command', query: string) {
  autocompleteLoading.value = true
  try {
    if (type === 'file') {
      const project = projectStore.activeProject
      const res: any = await aiApi.searchFiles({
        workspace_path: project?.path || '',
        query,
        limit: 20,
      })
      if (res?.ok && Array.isArray(res.data)) {
        autocompleteItems.value = res.data.map((it: any) => ({
          id: it.path,
          label: it.name,
          description: it.path,
          value: '@' + it.path + (it.type === 'dir' ? '/' : ''),
          type: 'file' as const,
        }))
      } else {
        autocompleteItems.value = []
      }
    } else {
      const all = await loadSlashCommands()
      const q = query.toLowerCase()
      const filtered = all.filter(c => !q || c.name.toLowerCase().includes(q) || c.description.toLowerCase().includes(q))
      autocompleteItems.value = filtered.map(c => ({
        id: c.id,
        label: c.name,
        description: c.description,
        value: c.name + ' ',
        type: 'command' as const,
      }))
    }
    autocompleteIndex.value = 0
  } catch (e: any) {
    autocompleteItems.value = []
  } finally {
    autocompleteLoading.value = false
  }
}

function scheduleAutocomplete() {
  if (autocompleteTimer) {
    window.clearTimeout(autocompleteTimer)
  }
  autocompleteTimer = window.setTimeout(() => {
    const text = inputText.value
    const cursorPos = (document.activeElement as HTMLTextAreaElement)?.selectionStart ?? text.length
    const trigger = detectTrigger(text, cursorPos)
    if (trigger) {
      autocompleteType.value = trigger.type
      autocompleteStart.value = trigger.start
      autocompleteQuery.value = trigger.query
      autocompleteOpen.value = true
      performAutocompleteSearch(trigger.type, trigger.query)
    } else {
      autocompleteOpen.value = false
    }
  }, 150)
}

function closeAutocomplete() {
  autocompleteOpen.value = false
  autocompleteType.value = null
  autocompleteItems.value = []
}

function applyAutocompleteItem(item: AutocompleteItem) {
  if (item.id === 'clear' && item.type === 'command') {
    if (chatStore.messages.length > 0) {
      showConfirmDialog({
        title: '清空对话',
        message: '确定要清空当前对话吗？',
      }).then(() => {
        chatStore.clearChat()
        showToast('对话已清空')
      }).catch(() => {})
    } else {
      showToast('对话已为空')
    }
    inputText.value = ''
    closeAutocomplete()
    return
  }
  const triggerLen = 1
  const queryLen = autocompleteQuery.value.length
  const before = inputText.value.slice(0, autocompleteStart.value)
  const afterCursor = inputText.value.slice(autocompleteStart.value + triggerLen + queryLen)
  inputText.value = before + item.value + afterCursor
  closeAutocomplete()
  nextTick(() => {
    const ta = document.querySelector('.input-field textarea') as HTMLTextAreaElement | null
    if (ta) {
      ta.focus()
      const newPos = before.length + item.value.length
      ta.setSelectionRange(newPos, newPos)
    }
  })
}

function onAutocompleteKeydown(e: KeyboardEvent): boolean {
  if (!autocompleteOpen.value || autocompleteItems.value.length === 0) return false
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    autocompleteIndex.value = (autocompleteIndex.value + 1) % autocompleteItems.value.length
    return true
  }
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    autocompleteIndex.value = (autocompleteIndex.value - 1 + autocompleteItems.value.length) % autocompleteItems.value.length
    return true
  }
  if (e.key === 'Enter' || e.key === 'Tab') {
    e.preventDefault()
    const item = autocompleteItems.value[autocompleteIndex.value]
    if (item) applyAutocompleteItem(item)
    return true
  }
  if (e.key === 'Escape') {
    e.preventDefault()
    closeAutocomplete()
    return true
  }
  return false
}

function handleStop() {
  chatStore.stopStream()
}

async function selectProvider(id: string) {
  showProviderPicker.value = false
  await modelStore.switchProvider(id)
  chatStore.switchModel(modelStore.currentProvider, modelStore.currentModel)
}

function selectModel(id: string) {
  showModelPicker.value = false
  modelStore.switchModel(id)
  chatStore.switchModel(modelStore.currentProvider, modelStore.currentModel)
}

async function loadConversation(id: string) {
  showSessionSheet.value = false
  if (!projectStore.activeProject) return
  await chatStore.loadSession(projectStore.activeProject.id, id)
  scrollToBottom()
}

function newConversation() {
  chatStore.clearChat()
  showSessionSheet.value = false
}

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value
}

function toggleRightSidebar() {
  rightSidebarCollapsed.value = !rightSidebarCollapsed.value
}

onMounted(async () => {
  await modelStore.fetchProviders()
  if (modelStore.currentProvider) {
    await modelStore.fetchModels()
  }
  chatStore.switchModel(modelStore.currentProvider, modelStore.currentModel)
  if (projectStore.activeProject) {
    await projectStore.loadConversations(projectStore.activeProject.id)
  }
})
</script>

<template>
  <div class="chat-view" :class="{ desktop: isDesktop, mobile: isMobile }">
    <div v-if="isDesktop" class="top-bar">
      <div class="top-bar-left">
        <van-icon name="bars" class="sidebar-toggle" @click="toggleSidebar" />
        <div class="model-selectors">
          <div class="selector" @click="showProviderPicker = true">
            <span>{{ currentProviderName }}</span>
            <van-icon name="arrow-down" size="12" />
          </div>
          <div class="selector" @click="showModelPicker = true">
            <span>{{ currentModelName }}</span>
            <van-icon name="arrow-down" size="12" />
          </div>
        </div>
      </div>
      <div class="top-bar-right">
        <van-icon
          name="wap-nav"
          class="sidebar-toggle right-toggle"
          :class="{ active: !rightSidebarCollapsed }"
          size="18"
          @click="toggleRightSidebar"
        />
      </div>
    </div>

    <div v-else class="top-bar mobile-top">
      <div class="selector" @click="showModelPicker = true">
        <span>{{ currentModelName }}</span>
        <van-icon name="arrow-down" size="12" />
      </div>
    </div>

    <div class="main-area">
      <div
        v-if="isDesktop"
        class="sidebar"
        :class="{ collapsed: sidebarCollapsed }"
      >
        <div class="sidebar-header">
          <van-button size="small" type="primary" block @click="newConversation">
            + 新建对话
          </van-button>
          <div class="search-box">
            <van-field
              v-model="searchKeyword"
              placeholder="搜索对话..."
              clearable
              left-icon="search"
              class="search-field"
              @update:model-value="handleSearch"
              @clear="clearSearch"
            />
          </div>
        </div>
        <div class="session-list">
          <template v-if="todayConversations.length">
            <div class="session-group-label">今天</div>
            <div
              v-for="conv in todayConversations"
              :key="conv.id"
              class="session-item"
              :class="{ active: conv.id === chatStore.currentSessionId }"
              @click="loadConversation(conv.id)"
            >
              <van-icon name="chat-o" size="16" />
              <span class="session-title">{{ conv.title }}</span>
              <div class="session-actions">
                <van-icon
                  name="edit"
                  size="14"
                  class="action-icon"
                  @click.stop="handleRenameConversation(conv.id, conv.title)"
                />
                <van-icon
                  name="delete-o"
                  size="14"
                  class="action-icon delete-icon"
                  @click.stop="handleDeleteConversation(conv.id)"
                />
              </div>
            </div>
          </template>
          <template v-if="earlierConversations.length">
            <div class="session-group-label">更早</div>
            <div
              v-for="conv in earlierConversations"
              :key="conv.id"
              class="session-item"
              :class="{ active: conv.id === chatStore.currentSessionId }"
              @click="loadConversation(conv.id)"
            >
              <van-icon name="chat-o" size="16" />
              <span class="session-title">{{ conv.title }}</span>
              <div class="session-actions">
                <van-icon
                  name="edit"
                  size="14"
                  class="action-icon"
                  @click.stop="handleRenameConversation(conv.id, conv.title)"
                />
                <van-icon
                  name="delete-o"
                  size="14"
                  class="action-icon delete-icon"
                  @click.stop="handleDeleteConversation(conv.id)"
                />
              </div>
            </div>
          </template>
          <van-empty
            v-if="!conversations.length"
            description="暂无会话"
            image="search"
          />
        </div>
      </div>

      <div class="chat-main">
        <div ref="messagesContainer" class="messages">
          <van-empty
            v-if="!chatStore.messages.length"
            description="开始一段对话吧"
            image="search"
          />
          <div
            v-for="(msg, idx) in chatStore.messages"
            :key="msg.id"
            class="message-row"
            :class="msg.role"
          >
            <div v-if="msg.role === 'assistant'" class="avatar ai-avatar">
              <van-icon name="robot-o" size="20" />
            </div>
            <div class="bubble" :class="msg.role">
              <MarkdownRenderer
                v-if="msg.role === 'assistant'"
                :content="msg.content"
              />
              <template v-else>{{ msg.content }}</template>
              <span
                v-if="
                  msg.role === 'assistant' &&
                  chatStore.isStreaming &&
                  idx === chatStore.messages.length - 1
                "
                class="streaming-cursor"
              >▊</span>
            </div>
            <div v-if="msg.role === 'user'" class="avatar user-avatar">
              <van-icon name="user-o" size="20" />
            </div>
          </div>
        </div>

        <div v-if="autocompleteOpen" class="autocomplete-popup" :class="{ mobile: isMobile }">
          <div class="ac-header">
            <span class="ac-title">
              {{ autocompleteType === 'file' ? '📁 文件' : '⚡ 命令' }}
            </span>
            <span v-if="autocompleteQuery" class="ac-query">@{{ autocompleteQuery }}</span>
            <span v-else class="ac-query">输入关键字过滤...</span>
          </div>
          <van-loading v-if="autocompleteLoading" color="var(--accent)" size="14" />
          <div v-else-if="autocompleteItems.length === 0" class="ac-empty">无匹配项</div>
          <div v-else class="ac-list">
            <div
              v-for="(item, idx) in autocompleteItems"
              :key="item.id"
              class="ac-item"
              :class="{ active: idx === autocompleteIndex }"
              @mouseenter="autocompleteIndex = idx"
              @click="applyAutocompleteItem(item)"
            >
              <div class="ac-item-label">
                <van-icon v-if="item.type === 'file'" name="description" size="14" class="ac-icon" />
                <van-icon v-else name="flash" size="14" class="ac-icon" />
                {{ item.label }}
              </div>
              <div class="ac-item-desc">{{ item.description }}</div>
            </div>
          </div>
          <div class="ac-hint">
            <span>↑↓ 选择</span>
            <span>Enter/Tab 确认</span>
            <span>Esc 关闭</span>
          </div>
        </div>

        <div v-if="attachedImages.length > 0" class="image-attachments">
          <div v-for="img in attachedImages" :key="img.id" class="image-thumb">
            <img :src="img.dataUrl" :alt="img.name" class="thumb-img" />
            <van-icon name="cross" class="thumb-remove" @click="removeImage(img.id)" />
          </div>
        </div>

        <div class="input-area">
          <van-icon
            v-if="isMobile"
            name="chat-o"
            class="session-btn"
            size="22"
            @click="showSessionSheet = true"
          />
          <van-icon
            name="photograph"
            class="attach-btn"
            size="22"
            :class="{ disabled: attachedImages.length >= MAX_IMAGES }"
            @click="pickImages"
          />
          <input
            ref="imageInputRef"
            type="file"
            accept="image/*"
            multiple
            style="display: none"
            @change="onImageInputChange"
          />
          <van-field
            v-model="inputText"
            type="textarea"
            :rows="1"
            :autosize="{ maxHeight: 96, minHeight: 40 }"
            placeholder="输入消息...（支持 @ 文件 / / 命令 / Ctrl+V 图片）"
            class="input-field"
            @keydown="handleKeydown"
            @paste="handleImagePaste"
            @drop="handleImageDrop"
            @dragover.prevent
            @input="scheduleAutocomplete"
          />
          <van-button
            v-if="chatStore.isStreaming"
            class="stop-btn"
            size="small"
            color="#e53935"
            round
            @click="handleStop"
          >
            停止
          </van-button>
          <van-button
            v-else
            class="send-btn"
            size="small"
            color="#42A5F5"
            round
            :disabled="!inputText.trim()"
            @click="handleSend"
          >
            <van-icon name="play" size="16" color="#fff" />
          </van-button>
        </div>
      </div>

      <RightSidebar
        v-if="isDesktop"
        :collapsed="rightSidebarCollapsed"
        @toggle="toggleRightSidebar"
      />
    </div>

    <van-popup
      v-model:show="showProviderPicker"
      position="bottom"
      round
      :style="{ maxHeight: '50%' }"
    >
      <div class="picker-sheet">
        <div class="picker-title">选择提供商</div>
        <van-cell-group>
          <van-cell
            v-for="p in modelStore.providers.filter((p) => p.enabled)"
            :key="p.id"
            :title="p.name"
            clickable
            :class="{ 'cell-active': p.id === modelStore.currentProvider }"
            @click="selectProvider(p.id)"
          >
            <template #right-icon>
              <van-icon
                v-if="p.id === modelStore.currentProvider"
                name="success"
                color="#42A5F5"
              />
            </template>
          </van-cell>
        </van-cell-group>
      </div>
    </van-popup>

    <van-popup
      v-model:show="showModelPicker"
      position="bottom"
      round
      :style="{ maxHeight: '50%' }"
    >
      <div class="picker-sheet">
        <div class="picker-title">选择模型</div>
        <van-cell-group>
          <van-cell
            v-for="m in modelStore.models"
            :key="m.id"
            :title="m.name"
            clickable
            :class="{ 'cell-active': m.id === modelStore.currentModel }"
            @click="selectModel(m.id)"
          >
            <template #right-icon>
              <van-icon
                v-if="m.id === modelStore.currentModel"
                name="success"
                color="#42A5F5"
              />
            </template>
          </van-cell>
        </van-cell-group>
      </div>
    </van-popup>

    <van-action-sheet v-model:show="showSessionSheet" title="会话列表">
      <div class="session-sheet-content">
        <van-button
          size="small"
          type="primary"
          block
          class="new-chat-btn"
          @click="newConversation"
        >
          + 新建对话
        </van-button>
        <div class="search-box mobile-search">
          <van-field
            v-model="searchKeyword"
            placeholder="搜索对话..."
            clearable
            left-icon="search"
            class="search-field"
            @update:model-value="handleSearch"
            @clear="clearSearch"
          />
        </div>
        <template v-if="todayConversations.length">
          <div class="session-group-label">今天</div>
          <van-swipe-cell v-for="conv in todayConversations" :key="conv.id">
            <van-cell
              :title="conv.title"
              clickable
              :class="{ 'cell-active': conv.id === chatStore.currentSessionId }"
              @click="loadConversation(conv.id)"
            >
              <template #right-icon>
                <van-icon
                  name="edit"
                  size="16"
                  class="cell-action-icon"
                  @click.stop="handleRenameConversation(conv.id, conv.title)"
                />
              </template>
            </van-cell>
            <template #right>
              <van-button
                square
                type="danger"
                text="删除"
                class="swipe-delete-btn"
                @click="handleDeleteConversation(conv.id)"
              />
            </template>
          </van-swipe-cell>
        </template>
        <template v-if="earlierConversations.length">
          <div class="session-group-label">更早</div>
          <van-swipe-cell v-for="conv in earlierConversations" :key="conv.id">
            <van-cell
              :title="conv.title"
              clickable
              :class="{ 'cell-active': conv.id === chatStore.currentSessionId }"
              @click="loadConversation(conv.id)"
            >
              <template #right-icon>
                <van-icon
                  name="edit"
                  size="16"
                  class="cell-action-icon"
                  @click.stop="handleRenameConversation(conv.id, conv.title)"
                />
              </template>
            </van-cell>
            <template #right>
              <van-button
                square
                type="danger"
                text="删除"
                class="swipe-delete-btn"
                @click="handleDeleteConversation(conv.id)"
              />
            </template>
          </van-swipe-cell>
        </template>
        <van-empty
          v-if="!conversations.length"
          description="暂无会话"
          image="search"
        />
      </div>
    </van-action-sheet>

    <van-dialog
      v-model:show="renameDialogVisible"
      title="重命名对话"
      show-cancel-button
      confirm-button-text="确定"
      cancel-button-text="取消"
      :before-close="handleRenameBeforeClose"
      @confirm="confirmRename"
    >
      <div class="rename-dialog-body">
        <van-field
          v-model="renameInput"
          placeholder="输入新名称"
          class="rename-field"
        />
      </div>
    </van-dialog>
  </div>
</template>

<style scoped>
.chat-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--bg-primary);
  color: var(--text-primary);
  overflow: hidden;
}

.chat-view.desktop {
}

.top-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  min-height: 48px;
}

.top-bar-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.top-bar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.sidebar-toggle {
  cursor: pointer;
  color: var(--text-secondary);
  font-size: 20px;
}

.sidebar-toggle:hover {
  color: var(--accent);
}

.right-toggle.active {
  color: var(--accent);
  background: rgba(66, 165, 245, 0.1);
  border-radius: 4px;
}

.model-selectors {
  display: flex;
  gap: 12px;
}

.selector {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 12px;
  background: var(--bg-primary);
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-secondary);
  transition: background 0.2s;
}

.selector:hover {
  background: var(--bg-card-hover);
  color: var(--accent);
}

.mobile-top {
  justify-content: center;
}

.main-area {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.sidebar {
  width: 240px;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  transition: width 0.3s, opacity 0.3s;
  overflow: hidden;
}

.sidebar.collapsed {
  width: 0;
  opacity: 0;
  border-right: none;
}

.sidebar-header {
  padding: 12px;
  border-bottom: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.search-box {
  width: 100%;
}

.search-field {
  background: var(--bg-primary);
  border-radius: 6px;
  overflow: hidden;
  padding: 0;
}

:deep(.search-field .van-field__control) {
  color: var(--text-primary);
  font-size: 13px;
}

:deep(.search-field .van-field__control::placeholder) {
  color: var(--text-muted);
}

:deep(.search-field .van-field__left-icon) {
  color: var(--text-muted);
}

:deep(.search-field .van-field__clear) {
  color: var(--text-muted);
}

.session-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.session-group-label {
  padding: 8px 16px 4px;
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 600;
}

.session-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  cursor: pointer;
  color: var(--text-secondary);
  transition: background 0.2s;
  white-space: nowrap;
  overflow: hidden;
  position: relative;
}

.session-item:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.session-item.active {
  background: var(--bg-card-hover);
  color: var(--accent);
}

.session-title {
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
  min-width: 0;
}

.session-actions {
  display: none;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.session-item:hover .session-actions {
  display: flex;
}

.action-icon {
  color: var(--text-muted);
  cursor: pointer;
  padding: 2px;
  border-radius: 4px;
  transition: color 0.2s, background 0.2s;
}

.action-icon:hover {
  color: var(--accent);
  background: var(--bg-card-hover);
}

.delete-icon:hover {
  color: #e53935;
}

.chat-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.messages {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.message-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  max-width: 85%;
}

.message-row.user {
  align-self: flex-end;
  flex-direction: row-reverse;
}

.message-row.assistant {
  align-self: flex-start;
}

.avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.ai-avatar {
  background: var(--bg-card);
  color: var(--accent);
}

.user-avatar {
  background: var(--accent-dark);
  color: #fff;
}

.bubble {
  padding: 10px 14px;
  border-radius: 12px;
  line-height: 1.6;
  font-size: 14px;
  position: relative;
  word-break: break-word;
}

.bubble.user {
  background: var(--accent-dark);
  color: #fff;
  border-bottom-right-radius: 4px;
}

.bubble.assistant {
  background: var(--bg-card);
  color: var(--text-primary);
  border-bottom-left-radius: 4px;
}

.streaming-cursor {
  display: inline;
  animation: blink 1s step-end infinite;
  color: var(--accent);
  font-weight: bold;
  margin-left: 2px;
}

@keyframes blink {
  50% {
    opacity: 0;
  }
}

.input-area {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  padding: 12px 16px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}

.session-btn {
  color: var(--text-muted);
  cursor: pointer;
  flex-shrink: 0;
  margin-bottom: 6px;
}

.session-btn:hover {
  color: var(--accent);
}

/* 2026-06-08 TASK-2.3 引入：图片附件按钮 + 缩略图 */
.attach-btn {
  color: var(--text-muted);
  cursor: pointer;
  flex-shrink: 0;
  margin-bottom: 6px;
  transition: color 0.15s;
}
.attach-btn:hover {
  color: var(--accent);
}
.attach-btn.disabled {
  color: var(--text-muted);
  opacity: 0.4;
  cursor: not-allowed;
}

.image-attachments {
  display: flex;
  gap: 8px;
  padding: 8px 16px 0 16px;
  background: var(--bg-secondary);
  flex-wrap: wrap;
  border-top: 1px solid var(--border);
}
.image-thumb {
  position: relative;
  width: 60px;
  height: 60px;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid var(--border);
  background: var(--bg-primary);
}
.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.thumb-remove {
  position: absolute;
  top: 2px;
  right: 2px;
  background: rgba(0, 0, 0, 0.6);
  color: #fff;
  font-size: 14px;
  padding: 2px;
  border-radius: 50%;
  cursor: pointer;
  line-height: 1;
}
.thumb-remove:hover {
  background: rgba(244, 67, 54, 0.8);
}

/* 2026-06-08 TASK-2.4 引入：Composer Autocomplete 弹窗 */
.autocomplete-popup {
  position: absolute;
  bottom: 100%;
  left: 16px;
  right: 16px;
  max-height: 240px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 -4px 12px rgba(0, 0, 0, 0.3);
  z-index: 10;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  margin-bottom: 4px;
}
.autocomplete-popup.mobile {
  left: 8px;
  right: 8px;
  max-height: 50vh;
}
.ac-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  font-size: 12px;
}
.ac-title {
  color: var(--accent);
  font-weight: 600;
}
.ac-query {
  color: var(--text-muted);
  font-family: 'Consolas', monospace;
  font-size: 11px;
  flex: 1;
  text-align: right;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ac-empty {
  text-align: center;
  padding: 20px;
  color: var(--text-muted);
  font-size: 12px;
}
.ac-list {
  flex: 1;
  overflow-y: auto;
}
.ac-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 12px;
  cursor: pointer;
  border-left: 3px solid transparent;
}
.ac-item:hover,
.ac-item.active {
  background: var(--bg-secondary);
  border-left-color: var(--accent);
}
.ac-item-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-primary);
  font-weight: 500;
}
.ac-icon {
  color: var(--text-muted);
  flex-shrink: 0;
}
.ac-item-desc {
  font-size: 11px;
  color: var(--text-muted);
  font-family: 'Consolas', monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ac-hint {
  display: flex;
  gap: 12px;
  padding: 4px 12px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
  font-size: 10px;
  color: var(--text-muted);
}

.input-field {
  flex: 1;
  background: var(--bg-primary);
  border-radius: 8px;
  overflow: hidden;
}

:deep(.input-field .van-field__control) {
  color: var(--text-primary);
  font-size: 14px;
  line-height: 1.5;
}

:deep(.input-field .van-field__control::placeholder) {
  color: var(--text-muted);
}

.send-btn,
.stop-btn {
  flex-shrink: 0;
  margin-bottom: 4px;
  min-width: 48px;
}

.picker-sheet {
  padding: 16px 0;
  background: var(--bg-card);
}

.picker-title {
  text-align: center;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  padding: 8px 0 16px;
}

:deep(.picker-sheet .van-cell) {
  background: var(--bg-card);
  color: var(--text-secondary);
}

:deep(.picker-sheet .van-cell:active) {
  background: var(--bg-card-hover);
}

:deep(.cell-active .van-cell__title) {
  color: var(--accent);
}

.session-sheet-content {
  padding: 16px;
  max-height: 60vh;
  overflow-y: auto;
  background: var(--bg-card);
}

.new-chat-btn {
  margin-bottom: 12px;
}

.mobile-search {
  margin-bottom: 12px;
}

:deep(.session-sheet-content .van-cell) {
  background: var(--bg-card);
  color: var(--text-secondary);
}

:deep(.session-sheet-content .van-swipe-cell) {
  background: var(--bg-card);
}

:deep(.session-sheet-content .van-swipe-cell__content) {
  background: var(--bg-card);
}

.cell-action-icon {
  color: var(--text-muted);
  cursor: pointer;
  transition: color 0.2s;
}

.cell-action-icon:hover {
  color: var(--accent);
}

.swipe-delete-btn {
  height: 100%;
}

:deep(.van-action-sheet__header) {
  background: var(--bg-card);
  color: var(--text-primary);
}

:deep(.van-popup) {
  background: var(--bg-card);
}

:deep(.van-empty__description) {
  color: var(--text-muted);
}

:deep(.van-empty__image img) {
  filter: brightness(0.6);
}

.rename-dialog-body {
  padding: 16px;
}

.rename-field {
  background: var(--bg-primary);
  border-radius: 6px;
  overflow: hidden;
}

:deep(.rename-field .van-field__control) {
  color: var(--text-primary);
  font-size: 14px;
}

:deep(.rename-field .van-field__control::placeholder) {
  color: var(--text-muted);
}

:deep(.van-dialog) {
  background: var(--bg-card);
}

:deep(.van-dialog__header) {
  color: var(--text-primary);
}

:deep(.van-dialog__confirm) {
  color: var(--accent);
}

:deep(.van-dialog__cancel) {
  color: var(--text-muted);
}

.chat-view.mobile .message-row {
  max-width: 95%;
}

.chat-view.mobile .messages {
  padding: 12px 8px;
  gap: 12px;
}

.chat-view.mobile .input-area {
  padding: 8px 12px;
}

.chat-view.mobile .bubble {
  padding: 8px 12px;
  font-size: 13px;
}
</style>
