<script setup lang="ts">
/**
 * MessageStream — 对话消息流（带 awareness 实时高亮）
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 */
import { ref, computed, nextTick, watch, onMounted, onUnmounted } from 'vue'
import type { Message, WorkspaceDoc, Conversation } from '@/composables/WorkspaceDoc'
import { useCollaborationStore } from '@/stores/collaboration'

const props = defineProps<{
  workspaceDoc: WorkspaceDoc | null
  conversation: Conversation | null
  messages: Message[]
  currentUserId: string | null
}>()

const emit = defineEmits<{
  (e: 'send', content: string): void
  (e: 'askAI', content: string): void
}>()

const store = useCollaborationStore()
const inputText = ref('')
const listEl = ref<HTMLElement | null>(null)

const scrollToBottom = () => {
  nextTick(() => {
    if (listEl.value) {
      listEl.value.scrollTop = listEl.value.scrollHeight
    }
  })
}

watch(
  () => props.messages.length,
  () => scrollToBottom(),
)
watch(
  () => props.conversation?.id,
  () => scrollToBottom(),
)
onMounted(scrollToBottom)

function onSend() {
  const text = inputText.value.trim()
  if (!text || !props.workspaceDoc || !props.conversation) return
  const authorId = props.currentUserId || 'anonymous'
  const authorName = store.currentUser?.display_name || store.currentUser?.username || '我'
  const msgId = 'm-' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6)
  props.workspaceDoc.appendMessage({
    id: msgId,
    conversationId: props.conversation.id,
    role: 'user',
    content: text,
    authorId,
    authorName,
  })
  inputText.value = ''
  // 自动滚动
  scrollToBottom()
}

function onAskAI() {
  const text = inputText.value.trim()
  if (!text) return
  emit('askAI', text)
  inputText.value = ''
}

function _authorColor(id: string): string {
  if (id === props.currentUserId) return '#22c55e'
  if (id === 'ai-agent' || id.startsWith('ai-')) return '#7c3aed'
  // 简单 hash 颜色
  let h = 0
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) & 0xffffff
  return `hsl(${h % 360}, 60%, 55%)`
}

function _formatTime(ts: number): string {
  const d = new Date(ts)
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function _isAiMessage(m: Message): boolean {
  return m.role === 'ai-agent' || m.role === 'assistant' || m.author_id.startsWith('ai-')
}

// 实时显示 awareness 上 AI 正在生成消息（即使还没写进 Y.Doc）
const aiTyping = computed(() => {
  const target = store.aiActivities.find(
    (a) =>
      a.activity.type === 'streaming' &&
      a.activity.target?.conversationId === props.conversation?.id,
  )
  return target
    ? {
        name: target.user.name,
        color: target.user.color,
        avatar: target.user.avatar || target.user.name.slice(0, 1),
      }
    : null
})
</script>

<template>
  <div class="message-stream">
    <div ref="listEl" class="messages">
      <div v-if="messages.length === 0 && !aiTyping" class="empty">
        开始对话吧
      </div>

      <div
        v-for="m in messages"
        :key="m.id"
        class="message"
        :class="{
          'is-ai': _isAiMessage(m),
          'is-self': m.author_id === currentUserId,
          'is-streaming': m.is_streaming,
        }"
      >
        <div class="avatar" :style="{ background: _authorColor(m.author_id) }">
          {{ m.author_name.slice(0, 1) }}
        </div>
        <div class="bubble-wrap">
          <div class="bubble-meta">
            <span class="author">{{ m.author_name }}</span>
            <span class="time">{{ _formatTime(m.created_at) }}</span>
            <span v-if="m.is_streaming" class="streaming-tag">生成中…</span>
          </div>
          <div class="bubble">
            <pre class="content">{{ m.content }}</pre>
            <span v-if="m.is_streaming" class="cursor-blink">▍</span>
          </div>
        </div>
      </div>

      <!-- AI 正在输入（但还没写进 Y.Doc 的占位） -->
      <div v-if="aiTyping" class="message is-ai is-streaming typing-placeholder">
        <div class="avatar" :style="{ background: aiTyping.color }">
          {{ aiTyping.avatar }}
        </div>
        <div class="bubble-wrap">
          <div class="bubble-meta">
            <span class="author">{{ aiTyping.name }}</span>
          </div>
          <div class="bubble">
            <span class="dots"><span></span><span></span><span></span></span>
          </div>
        </div>
      </div>
    </div>

    <div class="input-area">
      <van-field
        v-model="inputText"
        type="textarea"
        rows="1"
        autosize
        placeholder="输入消息…  @ai 让 AI 协作者参与"
        @keydown.enter.exact.prevent="onSend"
      />
      <div class="actions">
        <van-button size="small" plain @click="onAskAI" :disabled="!inputText.trim()">
          @AI
        </van-button>
        <van-button size="small" type="primary" @click="onSend" :disabled="!inputText.trim()">
          发送
        </van-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.message-stream {
  display: flex; flex-direction: column;
  height: 100%; background: var(--yj-content-bg, #0f0f0f);
}
.messages {
  flex: 1; overflow-y: auto;
  padding: 16px;
  display: flex; flex-direction: column; gap: 16px;
}
.empty { color: var(--yj-text-muted, #666); text-align: center; padding: 60px 0; }
.message {
  display: flex; gap: 10px;
  align-items: flex-start;
}
.message.is-self { flex-direction: row-reverse; }
.message.is-self .bubble-wrap { align-items: flex-end; }
.message.is-ai .bubble {
  background: rgba(124, 58, 237, 0.08);
  border-left: 2px solid #7c3aed;
}
.message.is-streaming .bubble {
  border-color: var(--yj-text, #aaa);
}
.avatar {
  width: 32px; height: 32px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  color: white; font-weight: 600; font-size: 13px;
  flex-shrink: 0;
}
.bubble-wrap { display: flex; flex-direction: column; max-width: 75%; }
.bubble-meta {
  display: flex; gap: 8px; align-items: center;
  font-size: 11px; color: var(--yj-text-muted, #888);
  margin-bottom: 4px;
}
.bubble-meta .author { font-weight: 500; color: var(--yj-text-secondary, #ccc); }
.streaming-tag {
  color: #7c3aed; font-weight: 500;
  animation: pulse 1.5s infinite;
}
@keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.4; } }
.bubble {
  background: var(--yj-bubble-bg, #1a1a1a);
  padding: 10px 12px;
  border-radius: 8px;
  font-size: 14px;
  line-height: 1.6;
  color: var(--yj-text, #e5e5e5);
  word-break: break-word;
  white-space: pre-wrap;
  position: relative;
}
.content { margin: 0; font-family: inherit; }
.cursor-blink {
  display: inline-block;
  animation: blink 1s steps(2, start) infinite;
  color: var(--yj-text, #aaa);
  margin-left: 2px;
}
@keyframes blink { to { visibility: hidden; } }
.typing-placeholder .bubble { padding: 12px 16px; }
.dots span {
  display: inline-block; width: 6px; height: 6px;
  border-radius: 50%; background: var(--yj-text-secondary, #aaa);
  margin: 0 2px;
  animation: bounce 1.4s infinite ease-in-out;
}
.dots span:nth-child(2) { animation-delay: 0.16s; }
.dots span:nth-child(3) { animation-delay: 0.32s; }
@keyframes bounce { 0%,80%,100% { transform: scale(0.7); opacity: 0.5; } 40% { transform: scale(1); opacity: 1; } }

.input-area {
  border-top: 1px solid var(--yj-border, #2a2a2a);
  padding: 12px 16px;
  background: var(--yj-content-bg, #0f0f0f);
}
.actions {
  display: flex; justify-content: flex-end; gap: 8px; margin-top: 8px;
}
</style>
