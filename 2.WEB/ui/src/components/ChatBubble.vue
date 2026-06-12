<template>
  <div class="chat-bubble" :class="[`chat-bubble--${role}`, { 'chat-bubble--streaming': streaming }]">
    <div class="chat-bubble-avatar">
      <span v-if="role === 'user'">👤</span>
      <span v-else>🤖</span>
    </div>
    <div class="chat-bubble-body">
      <div class="chat-bubble-role">{{ role === 'user' ? '你' : 'AI' }}</div>
      <div class="chat-bubble-content">
        <MarkdownRenderer :content="content" />
        <span v-if="streaming" class="chat-bubble-cursor">▊</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import MarkdownRenderer from './MarkdownRenderer.vue'

defineProps<{
  role: 'user' | 'assistant' | 'system'
  content: string
  streaming?: boolean
}>()
</script>

<style scoped>
.chat-bubble {
  display: flex;
  gap: 12px;
  padding: 12px 16px;
  border-radius: 12px;
  max-width: 100%;
}

.chat-bubble--user {
  flex-direction: row-reverse;
}

.chat-bubble--assistant {
  flex-direction: row;
}

.chat-bubble--system {
  flex-direction: row;
  opacity: 0.7;
}

.chat-bubble-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
}

.chat-bubble--user .chat-bubble-avatar {
  background-color: var(--accent-dark);
  border-color: var(--accent);
}

.chat-bubble-body {
  flex: 1;
  min-width: 0;
}

.chat-bubble-role {
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.chat-bubble--user .chat-bubble-role {
  text-align: right;
}

.chat-bubble-content {
  font-size: 14px;
  line-height: 1.7;
  color: var(--text-primary);
  word-break: break-word;
}

.chat-bubble-cursor {
  display: inline-block;
  animation: blink 1s step-end infinite;
  color: var(--accent);
  margin-left: 2px;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}
</style>
