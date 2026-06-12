<template>
  <div class="markdown-renderer" v-html="renderedHtml"></div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { marked } from 'marked'
import hljs from 'highlight.js'
import 'highlight.js/styles/github-dark.css'

const props = defineProps<{
  content: string
}>()

const renderer = new marked.Renderer()

renderer.code = function ({ text, lang }: { text: string; lang?: string }) {
  const language = lang && hljs.getLanguage(lang) ? lang : ''
  let highlighted: string
  try {
    highlighted = language
      ? hljs.highlight(text, { language }).value
      : hljs.highlightAuto(text).value
  } catch {
    highlighted = text.replace(/</g, '&lt;').replace(/>/g, '&gt;')
  }
  const langLabel = language || 'text'
  return `<div class="md-code-block"><div class="md-code-header"><span class="md-code-lang">${langLabel}</span><button class="md-copy-btn" onclick="window.__mdCopyCode(this)">复制</button></div><pre class="md-code-pre"><code class="hljs language-${langLabel}">${highlighted}</code></pre></div>`
}

renderer.heading = function ({ text, depth }: { text: string; depth: number }) {
  return `<h${depth} class="md-heading md-h${depth}">${text}</h${depth}>`
}

renderer.link = function ({ href, text }: { href: string; text: string }) {
  return `<a href="${href}" target="_blank" rel="noopener noreferrer" class="md-link">${text}</a>`
}

renderer.blockquote = function ({ text }: { text: string }) {
  return `<blockquote class="md-blockquote">${text}</blockquote>`
}

renderer.table = function (token: any) {
  const header = token.header || ''
  const body = token.body || ''
  return `<div class="md-table-wrap"><table class="md-table"><thead>${header}</thead><tbody>${body}</tbody></table></div>`
}

marked.setOptions({
  renderer,
  gfm: true,
  breaks: true,
})

;(window as any).__mdCopyCode = (btn: HTMLButtonElement) => {
  const codeBlock = btn.closest('.md-code-block')
  const codeEl = codeBlock?.querySelector('code')
  if (codeEl) {
    navigator.clipboard.writeText(codeEl.textContent || '').then(() => {
      btn.textContent = '已复制'
      setTimeout(() => { btn.textContent = '复制' }, 2000)
    }).catch(() => {
      const textarea = document.createElement('textarea')
      textarea.value = codeEl.textContent || ''
      document.body.appendChild(textarea)
      textarea.select()
      document.execCommand('copy')
      document.body.removeChild(textarea)
      btn.textContent = '已复制'
      setTimeout(() => { btn.textContent = '复制' }, 2000)
    })
  }
}

const renderedHtml = computed(() => {
  if (!props.content) return ''
  try {
    return marked.parse(props.content) as string
  } catch {
    return props.content.replace(/</g, '&lt;').replace(/\n/g, '<br>')
  }
})
</script>

<style scoped>
.markdown-renderer {
  font-size: 14px;
  line-height: 1.7;
  color: var(--text-primary);
  word-break: break-word;
}

.markdown-renderer :deep(.md-heading) {
  margin: 16px 0 8px;
  font-weight: 600;
  color: var(--text-primary);
}

.markdown-renderer :deep(.md-h1) { font-size: 20px; }
.markdown-renderer :deep(.md-h2) { font-size: 18px; }
.markdown-renderer :deep(.md-h3) { font-size: 16px; }

.markdown-renderer :deep(.md-link) {
  color: var(--accent);
  text-decoration: none;
}

.markdown-renderer :deep(.md-link:hover) {
  text-decoration: underline;
}

.markdown-renderer :deep(ul),
.markdown-renderer :deep(ol) {
  padding-left: 20px;
  margin: 8px 0;
}

.markdown-renderer :deep(li) {
  margin: 4px 0;
}

.markdown-renderer :deep(.md-blockquote) {
  margin: 8px 0;
  padding: 8px 16px;
  border-left: 3px solid var(--accent);
  background-color: rgba(66, 165, 245, 0.05);
  border-radius: 0 8px 8px 0;
  color: var(--text-secondary);
}

.markdown-renderer :deep(.md-table-wrap) {
  overflow-x: auto;
  margin: 8px 0;
}

.markdown-renderer :deep(.md-table) {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.markdown-renderer :deep(.md-table th),
.markdown-renderer :deep(.md-table td) {
  padding: 8px 12px;
  border: 1px solid var(--border);
  text-align: left;
}

.markdown-renderer :deep(.md-table th) {
  background-color: rgba(255, 255, 255, 0.05);
  font-weight: 600;
}

.markdown-renderer :deep(.md-code-block) {
  margin: 8px 0;
  border-radius: 8px;
  overflow: hidden;
  background: #0d0d0d;
  border: 1px solid var(--border);
}

.markdown-renderer :deep(.md-code-header) {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  background: #1a1a2e;
  border-bottom: 1px solid var(--border);
}

.markdown-renderer :deep(.md-code-lang) {
  font-size: 12px;
  color: var(--text-muted);
  text-transform: uppercase;
}

.markdown-renderer :deep(.md-copy-btn) {
  background: transparent;
  border: 1px solid #444;
  color: var(--text-secondary);
  padding: 2px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
}

.markdown-renderer :deep(.md-copy-btn:hover) {
  border-color: var(--accent);
  color: var(--accent);
}

.markdown-renderer :deep(.md-code-pre) {
  margin: 0;
  padding: 12px;
  overflow-x: auto;
}

.markdown-renderer :deep(.md-code-pre code) {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.5;
}

.markdown-renderer :deep(p) {
  margin: 6px 0;
}

.markdown-renderer :deep(p:last-child) {
  margin-bottom: 0;
}

.markdown-renderer :deep(code) {
  padding: 2px 6px;
  border-radius: 4px;
  background-color: rgba(255, 255, 255, 0.08);
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
}

.markdown-renderer :deep(.md-code-block code) {
  padding: 0;
  background: none;
}

.markdown-renderer :deep(hr) {
  border: none;
  border-top: 1px solid var(--border);
  margin: 16px 0;
}

.markdown-renderer :deep(strong) {
  font-weight: 600;
  color: var(--accent-light);
}

.markdown-renderer :deep(em) {
  font-style: italic;
  color: var(--text-secondary);
}

.markdown-renderer :deep(img) {
  max-width: 100%;
  border-radius: 6px;
}
</style>
