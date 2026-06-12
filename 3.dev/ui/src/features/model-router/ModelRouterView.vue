<script setup lang="ts">
/**
 * ModelRouterView — 智能模型路由主视图
 * 2026-06-09 TASK-4.6
 *
 * 布局：
 *   ┌────────────────────────────────┬──────────────────┐
 *   │ 实时评估 + 规则编辑器            │ 路由历史          │
 *   │                                │                  │
 *   └────────────────────────────────┴──────────────────┘
 */
import { ref, computed, onMounted } from 'vue'
import { showToast } from 'vant'
import { useModelRouterStore } from '@/stores/model-router'
import type { RoutingRules } from '@/api'
import ComplexityMeter from './ComplexityMeter.vue'
import RoutingRuleEditor from './RoutingRuleEditor.vue'
import RouteHistoryList from './RouteHistoryList.vue'

const store = useModelRouterStore()

const promptInput = ref('')
const historyCount = ref(0)
const hasImages = ref(false)
const evaluating = ref(false)
const deciding = ref(false)
const recordEnabled = ref(true)

const tierColors: Record<string, string> = {
  simple: '#10b981',
  medium: '#f59e0b',
  complex: '#ef4444',
}

const stats = computed(() => store.tierStats)
const providerStats = computed(() => store.providerStats)
const error = computed(() => store.error)

async function onEvaluate() {
  if (!promptInput.value.trim()) {
    showToast('请输入 prompt')
    return
  }
  evaluating.value = true
  try {
    const fakeHistory = Array(historyCount.value).fill({ role: 'user' })
    await store.evaluate(promptInput.value, fakeHistory, hasImages.value)
  } finally {
    evaluating.value = false
  }
}

async function onDecide() {
  if (!promptInput.value.trim()) {
    showToast('请输入 prompt')
    return
  }
  deciding.value = true
  try {
    const fakeHistory = Array(historyCount.value).fill({ role: 'user' })
    const decision = await store.decide(promptInput.value, fakeHistory, hasImages.value, recordEnabled.value)
    if (decision) {
      const color = tierColors[decision.tier] || '#666'
      showToast({
        message: `→ ${decision.tier} (${(decision.complexity.total * 100).toFixed(0)}%) · ${decision.primary.provider}:${decision.primary.model}`,
        duration: 3000,
      })
    }
  } finally {
    deciding.value = false
  }
}

async function onSaveRules(rules: RoutingRules) {
  try {
    await store.saveRules(rules)
    showToast('已保存')
  } catch (e) {
    showToast(`保存失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function onResetRules() {
  try {
    await store.resetToDefaults()
    showToast('已恢复默认规则')
  } catch (e) {
    showToast(`重置失败: ${e instanceof Error ? e.message : e}`)
  }
}

function useSample(type: 'simple' | 'medium' | 'complex') {
  const samples = {
    simple: '你好',
    medium: '请实现一个 React 函数组件，要求：\n- TypeScript\n- props 类型定义\n- 至少 2 个 Vitest 测试\n- Storybook 故事',
    complex: '请完整设计一个分布式微服务架构，包含 transformer 模型推理、kubernetes 部署、加密传输、性能优化算法实现。\n\n```python\nclass Transformer:\n    def __init__(self, dim): self.dim = dim\n```',
  }
  promptInput.value = samples[type]
}

onMounted(async () => {
  await store.refreshAll()
})
</script>

<template>
  <div class="model-router-view">
    <!-- 顶部统计 -->
    <div class="stats-bar">
      <div class="stat-card">
        <div class="stat-value">{{ store.history.length }}</div>
        <div class="stat-label">路由决策总数</div>
      </div>
      <div class="stat-card">
        <div class="stat-value" :style="{ color: tierColors.complex }">
          {{ stats.complex || 0 }}
        </div>
        <div class="stat-label">复杂任务</div>
      </div>
      <div class="stat-card">
        <div class="stat-value" :style="{ color: tierColors.medium }">
          {{ stats.medium || 0 }}
        </div>
        <div class="stat-label">中等任务</div>
      </div>
      <div class="stat-card">
        <div class="stat-value" :style="{ color: tierColors.simple }">
          {{ stats.simple || 0 }}
        </div>
        <div class="stat-label">简单任务</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ providerStats.ollama || 0 }}</div>
        <div class="stat-label">走本地 (Ollama)</div>
      </div>
    </div>

    <!-- 错误提示 -->
    <div v-if="error" class="error-banner">⚠️ {{ error }}</div>

    <!-- 主体两栏 -->
    <div class="main">
      <!-- 左：评估 + 规则 -->
      <div class="left-pane">
        <!-- 评估器 -->
        <div class="section">
          <div class="section-title">
            <span>🧪 实时评估器</span>
            <div class="sample-buttons">
              <button class="sample-btn" @click="useSample('simple')">简单</button>
              <button class="sample-btn" @click="useSample('medium')">中等</button>
              <button class="sample-btn" @click="useSample('complex')">复杂</button>
            </div>
          </div>

          <textarea
            v-model="promptInput"
            class="prompt-input"
            rows="5"
            placeholder="输入 prompt，测试路由决策…"
          />

          <div class="eval-options">
            <label class="opt-row">
              <span class="opt-name">历史消息</span>
              <input
                v-model.number="historyCount"
                type="number"
                min="0"
                max="200"
                class="opt-num"
              />
              <span class="opt-unit">条</span>
            </label>
            <label class="opt-row">
              <input v-model="hasImages" type="checkbox" />
              <span>包含图片（多模态）</span>
            </label>
            <label class="opt-row">
              <input v-model="recordEnabled" type="checkbox" />
              <span>记录到历史</span>
            </label>
          </div>

          <div class="eval-actions">
            <button
              class="btn btn-secondary"
              :disabled="evaluating || !promptInput.trim()"
              @click="onEvaluate"
            >
              {{ evaluating ? '评估中…' : '🔍 仅评估' }}
            </button>
            <button
              class="btn btn-primary"
              :disabled="deciding || !promptInput.trim()"
              @click="onDecide"
            >
              {{ deciding ? '决策中…' : '🚀 路由 + 记录' }}
            </button>
          </div>

          <ComplexityMeter :score="store.lastScore" />
        </div>

        <!-- 规则编辑器 -->
        <div class="section">
          <RoutingRuleEditor
            :rules="store.rules"
            :saving="store.saving"
            @save="onSaveRules"
            @reset="onResetRules"
          />
        </div>
      </div>

      <!-- 右：历史 -->
      <div class="right-pane">
        <RouteHistoryList
          :history="store.history"
          @clear="store.clearHistory()"
          @refresh="store.fetchHistory(50)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.model-router-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 16px;
  gap: 12px;
  background: var(--yj-bg-base, #0d0d0d);
  overflow: hidden;
}

.stats-bar {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 8px;
}

.stat-card {
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 8px;
  padding: 10px;
  text-align: center;
}

.stat-value {
  font-size: 18px;
  font-weight: 700;
  color: var(--yj-accent, #a855f7);
  font-family: 'Cascadia Code', 'Consolas', monospace;
}

.stat-label {
  font-size: 10px;
  color: var(--yj-text-secondary, #888);
  margin-top: 2px;
}

.error-banner {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid #ef4444;
  color: #fca5a5;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 12px;
}

.main {
  flex: 1;
  display: grid;
  grid-template-columns: 1fr 360px;
  gap: 12px;
  min-height: 0;
}

.left-pane {
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
}

.right-pane {
  min-height: 0;
}

.section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.section-title {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 14px;
  font-weight: 600;
  color: var(--yj-text-primary, #f5f5f5);
}

.sample-buttons {
  display: flex;
  gap: 4px;
}

.sample-btn {
  font-size: 11px;
  padding: 3px 10px;
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  color: var(--yj-text-secondary, #aaa);
  border-radius: 4px;
  cursor: pointer;
}

.sample-btn:hover {
  border-color: var(--yj-accent, #7c3aed);
  color: var(--yj-text-primary, #ddd);
}

.prompt-input {
  width: 100%;
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 6px;
  padding: 10px;
  color: var(--yj-text-primary, #ddd);
  font-size: 13px;
  font-family: 'Cascadia Code', 'Consolas', monospace;
  resize: vertical;
  box-sizing: border-box;
}

.prompt-input:focus {
  outline: none;
  border-color: var(--yj-accent, #7c3aed);
}

.eval-options {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  padding: 8px 12px;
  background: var(--yj-bg-elevated, #1a1a1a);
  border-radius: 6px;
}

.opt-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--yj-text-primary, #ddd);
}

.opt-name {
  color: var(--yj-text-secondary, #888);
}

.opt-num {
  width: 60px;
  background: var(--yj-bg-base, #0d0d0d);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 4px;
  padding: 3px 6px;
  color: var(--yj-text-primary, #ddd);
  font-size: 12px;
  text-align: center;
}

.opt-unit {
  color: var(--yj-text-secondary, #888);
}

.eval-actions {
  display: flex;
  gap: 8px;
}

.btn {
  font-size: 13px;
  padding: 8px 16px;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-weight: 500;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: var(--yj-accent, #7c3aed);
  color: #fff;
}

.btn-primary:hover:not(:disabled) {
  background: #6d28d9;
}

.btn-secondary {
  background: var(--yj-bg-elevated, #1a1a1a);
  color: var(--yj-text-primary, #ddd);
  border: 1px solid var(--yj-border, #2a2a2a);
}

.btn-secondary:hover:not(:disabled) {
  border-color: var(--yj-accent, #7c3aed);
}
</style>
