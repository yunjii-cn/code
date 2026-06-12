<template>
  <div class="code-block">
    <div class="code-block-header">
      <span class="code-block-lang">{{ language || 'code' }}</span>
      <button class="code-block-copy" @click="handleCopy">
        {{ copied ? '✓ 已复制' : '复制' }}
      </button>
    </div>
    <pre class="code-block-pre"><code :class="`language-${language}`">{{ code }}</code></pre>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  code: string
  language?: string
}>()

const copied = ref(false)

async function handleCopy() {
  try {
    await navigator.clipboard.writeText(props.code)
    copied.value = true
    setTimeout(() => { copied.value = false }, 2000)
  } catch {
    const textarea = document.createElement('textarea')
    textarea.value = props.code
    document.body.appendChild(textarea)
    textarea.select()
    document.execCommand('copy')
    document.body.removeChild(textarea)
    copied.value = true
    setTimeout(() => { copied.value = false }, 2000)
  }
}
</script>

<style scoped>
.code-block {
  border-radius: 8px;
  overflow: hidden;
  margin: 8px 0;
  background-color: #1a1a2e;
  border: 1px solid var(--border);
}

.code-block-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background-color: rgba(255, 255, 255, 0.05);
  border-bottom: 1px solid var(--border);
}

.code-block-lang {
  font-size: 12px;
  color: var(--text-muted);
  text-transform: uppercase;
}

.code-block-copy {
  font-size: 12px;
  color: var(--accent);
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px 8px;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.code-block-copy:hover {
  background-color: rgba(66, 165, 245, 0.1);
}

.code-block-pre {
  margin: 0;
  padding: 12px 16px;
  overflow-x: auto;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.6;
  color: #e0e0e0;
  tab-size: 2;
}

.code-block-pre code {
  font-family: inherit;
  white-space: pre;
}
</style>
