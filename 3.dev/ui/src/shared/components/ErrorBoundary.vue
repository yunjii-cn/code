<script setup lang="ts">
/**
 * ErrorBoundary — 错误边界组件
 * 2026-06-09 TASK-4.7
 *
 * 用 Vue 3 的 onErrorCaptured 捕获子组件错误，
 * 显示友好的错误提示 + 重试按钮，而不是白屏。
 *
 * 用法：
 *  <ErrorBoundary>
 *    <SomeRiskyComponent />
 *  </ErrorBoundary>
 */
import { ref, onErrorCaptured } from 'vue'

const hasError = ref(false)
const errorMessage = ref('')
const errorStack = ref('')

onErrorCaptured((err, _instance, info) => {
  hasError.value = true
  errorMessage.value = err instanceof Error ? err.message : String(err)
  errorStack.value = err instanceof Error ? (err.stack || '') : ''
  // 不继续向上传播
  console.error(`[ErrorBoundary] ${info}:`, err)
  return false
})

function retry() {
  hasError.value = false
  errorMessage.value = ''
  errorStack.value = ''
}

function reload() {
  window.location.reload()
}
</script>

<template>
  <div v-if="hasError" class="error-boundary">
    <div class="eb-icon">⚠️</div>
    <div class="eb-title">出了点问题</div>
    <div class="eb-message">{{ errorMessage }}</div>
    <details v-if="errorStack" class="eb-details">
      <summary>详细错误信息</summary>
      <pre class="eb-stack">{{ errorStack }}</pre>
    </details>
    <div class="eb-actions">
      <button class="btn btn-primary" @click="retry">重试</button>
      <button class="btn btn-secondary" @click="reload">刷新页面</button>
    </div>
  </div>
  <slot v-else />
</template>

<style scoped>
.error-boundary {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  text-align: center;
  gap: 12px;
  min-height: 200px;
}

.eb-icon {
  font-size: 48px;
  opacity: 0.6;
}

.eb-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.eb-message {
  font-size: 13px;
  color: var(--text-secondary);
  max-width: 400px;
  word-break: break-word;
}

.eb-details {
  max-width: 600px;
  width: 100%;
  text-align: left;
}

.eb-details summary {
  font-size: 12px;
  color: var(--text-muted);
  cursor: pointer;
  margin-bottom: 6px;
}

.eb-stack {
  font-size: 11px;
  color: var(--text-muted);
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 10px;
  overflow-x: auto;
  max-height: 200px;
  white-space: pre-wrap;
  word-break: break-all;
}

.eb-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.btn {
  font-size: 13px;
  padding: 8px 16px;
  border-radius: var(--radius-md);
  border: none;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s ease;
}

.btn-primary {
  background: var(--accent);
  color: #fff;
}

.btn-primary:hover {
  background: var(--accent-dark);
}

.btn-secondary {
  background: var(--bg-card);
  color: var(--text-primary);
  border: 1px solid var(--border);
}

.btn-secondary:hover {
  border-color: var(--accent);
}
</style>
