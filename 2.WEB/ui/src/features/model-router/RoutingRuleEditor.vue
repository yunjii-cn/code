<script setup lang="ts">
/**
 * RoutingRuleEditor — 路由规则编辑器
 * 2026-06-09 TASK-4.6
 *
 * 编辑 tiers（name/max_complexity/primary/fallback）+ 全局选项（auto_fallback/history_limit）
 */
import { ref, watch, computed } from 'vue'
import { showDialog, showToast } from 'vant'
import type { RoutingRules, RoutingTier, ModelRef } from '@/api'

const props = defineProps<{
  rules: RoutingRules | null
  saving?: boolean
}>()

const emit = defineEmits<{
  save: [rules: RoutingRules]
  reset: []
  cancel: []
}>()

const localRules = ref<RoutingRules | null>(null)
const dirty = ref(false)

watch(() => props.rules, (r) => {
  if (r) {
    localRules.value = JSON.parse(JSON.stringify(r)) as RoutingRules
    dirty.value = false
  }
}, { immediate: true })

const PROVIDERS = [
  'ollama',
  'anthropic',
  'openai',
  'openrouter',
  'zhipu',
  'qwen',
  'custom',
]

function isDirty(): boolean {
  return JSON.stringify(localRules.value) !== JSON.stringify(props.rules)
}

watch(localRules, () => {
  dirty.value = isDirty()
}, { deep: true })

function addTier() {
  if (!localRules.value) return
  if (localRules.value.tiers.length >= 6) {
    showToast('最多 6 个 tier')
    return
  }
  localRules.value.tiers.push({
    name: `tier-${localRules.value.tiers.length + 1}`,
    max_complexity: 1.0,
    primary: { provider: 'ollama', model: 'qwen2.5:7b' },
    fallback: null,
  })
}

function removeTier(index: number) {
  if (!localRules.value) return
  if (localRules.value.tiers.length <= 1) {
    showToast('至少保留 1 个 tier')
    return
  }
  localRules.value.tiers.splice(index, 1)
}

function moveTier(index: number, direction: -1 | 1) {
  if (!localRules.value) return
  const arr = localRules.value.tiers
  const newIdx = index + direction
  if (newIdx < 0 || newIdx >= arr.length) return
  ;[arr[index], arr[newIdx]] = [arr[newIdx], arr[index]]
}

function setFallback(tier: RoutingTier, hasFallback: boolean) {
  if (hasFallback && !tier.fallback) {
    tier.fallback = { provider: 'ollama', model: 'qwen2.5:1.5b' }
  } else if (!hasFallback) {
    tier.fallback = null
  }
}

function onSave() {
  if (!localRules.value) return
  // 校验
  for (let i = 0; i < localRules.value.tiers.length; i++) {
    const t = localRules.value.tiers[i]
    if (!t.name.trim()) {
      showToast(`第 ${i + 1} 个 tier 缺少名称`)
      return
    }
    if (t.max_complexity < 0 || t.max_complexity > 1.5) {
      showToast(`tier "${t.name}" 阈值必须在 0-1.5 之间`)
      return
    }
    if (!t.primary.provider.trim() || !t.primary.model.trim()) {
      showToast(`tier "${t.name}" 缺少 primary 模型`)
      return
    }
  }
  emit('save', JSON.parse(JSON.stringify(localRules.value)))
}

function onReset() {
  showDialog({
    title: '恢复默认规则',
    message: '将丢失当前所有自定义规则，确定恢复为默认？',
    showCancelButton: true,
  }).then(() => {
    emit('reset')
  }).catch(() => {})
}

function onCancel() {
  if (dirty.value) {
    showDialog({
      title: '放弃修改？',
      message: '当前有未保存的修改',
      showCancelButton: true,
    }).then(() => {
      emit('cancel')
    }).catch(() => {})
  } else {
    emit('cancel')
  }
}

const tierCount = computed(() => localRules.value?.tiers.length || 0)
</script>

<template>
  <div v-if="localRules" class="rule-editor">
    <div class="editor-header">
      <div class="editor-title">路由规则（{{ tierCount }} tier）</div>
      <div class="editor-actions">
        <button class="btn btn-ghost" @click="addTier">+ 添加 tier</button>
        <button class="btn btn-ghost btn-danger-text" @click="onReset">↺ 恢复默认</button>
        <button class="btn btn-ghost" @click="onCancel" :disabled="!dirty">取消</button>
        <button
          class="btn btn-primary"
          :disabled="!dirty || saving"
          @click="onSave"
        >
          {{ saving ? '保存中…' : '保存' }}
        </button>
      </div>
    </div>

    <!-- 全局选项 -->
    <div class="global-options">
      <label class="opt-row">
        <input
          v-model="localRules.auto_fallback"
          type="checkbox"
        />
        <span>启用自动降级（primary 失败时自动用 fallback）</span>
      </label>
      <label class="opt-row">
        <span>历史记录上限：</span>
        <input
          v-model.number="localRules.history_limit"
          type="number"
          min="10"
          max="2000"
          class="opt-num"
        />
        <span>条</span>
      </label>
    </div>

    <!-- Tier 列表 -->
    <div class="tier-list">
      <div
        v-for="(tier, idx) in localRules.tiers"
        :key="idx"
        class="tier-card"
      >
        <div class="tier-header">
          <div class="tier-name-area">
            <input
              v-model="tier.name"
              class="tier-name-input"
              placeholder="tier 名"
            />
            <span class="tier-order">#{{ idx + 1 }}</span>
          </div>
          <div class="tier-controls">
            <div class="threshold-control">
              <span class="ctrl-label">阈值</span>
              <input
                v-model.number="tier.max_complexity"
                type="number"
                min="0"
                max="1.5"
                step="0.05"
                class="threshold-input"
              />
            </div>
            <div class="tier-move">
              <button class="btn-icon" :disabled="idx === 0" @click="moveTier(idx, -1)">↑</button>
              <button class="btn-icon" :disabled="idx === localRules!.tiers.length - 1" @click="moveTier(idx, 1)">↓</button>
              <button class="btn-icon btn-danger" @click="removeTier(idx)">×</button>
            </div>
          </div>
        </div>

        <div class="model-row">
          <div class="model-col">
            <div class="model-label">🟢 Primary</div>
            <select v-model="tier.primary.provider" class="model-select">
              <option v-for="p in PROVIDERS" :key="p" :value="p">{{ p }}</option>
            </select>
            <input
              v-model="tier.primary.model"
              class="model-input"
              placeholder="模型名（如 qwen2.5:7b）"
            />
          </div>

          <div class="model-col">
            <div class="model-label-row">
              <span class="model-label">🟡 Fallback</span>
              <label class="fallback-toggle">
                <input
                  type="checkbox"
                  :checked="tier.fallback !== null"
                  @change="(e: Event) => setFallback(tier, (e.target as HTMLInputElement).checked)"
                />
                <span>启用</span>
              </label>
            </div>
            <template v-if="tier.fallback">
              <select v-model="tier.fallback.provider" class="model-select">
                <option v-for="p in PROVIDERS" :key="p" :value="p">{{ p }}</option>
              </select>
              <input
                v-model="tier.fallback.model"
                class="model-input"
                placeholder="模型名"
              />
            </template>
            <div v-else class="model-disabled">未启用降级</div>
          </div>
        </div>
      </div>
    </div>
  </div>
  <div v-else class="loading">加载中…</div>
</template>

<style scoped>
.rule-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.loading {
  padding: 30px;
  text-align: center;
  color: var(--yj-text-secondary, #888);
}

.editor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.editor-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--yj-text-primary, #f5f5f5);
}

.editor-actions {
  display: flex;
  gap: 6px;
}

.btn {
  font-size: 12px;
  padding: 5px 12px;
  border-radius: 5px;
  border: 1px solid var(--yj-border, #2a2a2a);
  background: var(--yj-bg-elevated, #1a1a1a);
  color: var(--yj-text-primary, #ddd);
  cursor: pointer;
  transition: all 0.15s;
}

.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn-primary {
  background: var(--yj-accent, #7c3aed);
  color: #fff;
  border-color: var(--yj-accent, #7c3aed);
}

.btn-primary:hover:not(:disabled) {
  background: #6d28d9;
}

.btn-ghost:hover:not(:disabled) {
  border-color: var(--yj-accent, #7c3aed);
}

.btn-danger-text {
  color: #fca5a5;
}

.global-options {
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.opt-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--yj-text-primary, #ddd);
}

.opt-num {
  width: 80px;
  background: var(--yj-bg-base, #0d0d0d);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 4px;
  padding: 4px 8px;
  color: var(--yj-text-primary, #ddd);
  font-size: 12px;
}

.opt-num:focus {
  outline: none;
  border-color: var(--yj-accent, #7c3aed);
}

.tier-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.tier-card {
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.tier-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.tier-name-area {
  display: flex;
  align-items: center;
  gap: 6px;
}

.tier-name-input {
  background: var(--yj-bg-base, #0d0d0d);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 4px;
  padding: 4px 8px;
  color: var(--yj-text-primary, #ddd);
  font-size: 13px;
  font-weight: 500;
  min-width: 100px;
}

.tier-name-input:focus {
  outline: none;
  border-color: var(--yj-accent, #7c3aed);
}

.tier-order {
  font-size: 11px;
  color: var(--yj-text-secondary, #888);
  font-family: 'Cascadia Code', 'Consolas', monospace;
}

.tier-controls {
  display: flex;
  align-items: center;
  gap: 12px;
}

.threshold-control {
  display: flex;
  align-items: center;
  gap: 4px;
}

.ctrl-label {
  font-size: 11px;
  color: var(--yj-text-secondary, #888);
}

.threshold-input {
  width: 70px;
  background: var(--yj-bg-base, #0d0d0d);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 4px;
  padding: 4px 6px;
  color: var(--yj-text-primary, #ddd);
  font-size: 12px;
  font-family: 'Cascadia Code', 'Consolas', monospace;
}

.threshold-input:focus {
  outline: none;
  border-color: var(--yj-accent, #7c3aed);
}

.tier-move {
  display: flex;
  gap: 2px;
}

.btn-icon {
  width: 24px;
  height: 24px;
  background: var(--yj-bg-base, #0d0d0d);
  border: 1px solid var(--yj-border, #2a2a2a);
  color: var(--yj-text-secondary, #aaa);
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-icon:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.btn-icon:hover:not(:disabled) {
  border-color: var(--yj-accent, #7c3aed);
  color: var(--yj-text-primary, #ddd);
}

.btn-icon.btn-danger:hover:not(:disabled) {
  border-color: #ef4444;
  color: #ef4444;
}

.model-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.model-col {
  background: var(--yj-bg-base, #0d0d0d);
  border-radius: 6px;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.model-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--yj-text-secondary, #888);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.model-label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.fallback-toggle {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--yj-text-secondary, #aaa);
  cursor: pointer;
}

.model-select,
.model-input {
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 4px;
  padding: 5px 8px;
  color: var(--yj-text-primary, #ddd);
  font-size: 12px;
  font-family: 'Cascadia Code', 'Consolas', monospace;
  width: 100%;
  box-sizing: border-box;
}

.model-input {
  text-transform: lowercase;
}

.model-select:focus,
.model-input:focus {
  outline: none;
  border-color: var(--yj-accent, #7c3aed);
}

.model-disabled {
  font-size: 11px;
  color: var(--yj-text-secondary, #666);
  text-align: center;
  padding: 8px 0;
  font-style: italic;
}
</style>
