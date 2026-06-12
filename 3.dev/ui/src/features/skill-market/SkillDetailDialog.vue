<script setup lang="ts">
/**
 * SkillDetailDialog — 技能详情弹窗
 * 2026-06-09 TASK-4.5
 *
 * 展示完整信息：长描述、标签、prompt 模板预览、文件列表、依赖
 * 支持：评分（1-5 星）、安装/卸载、应用
 */
import { ref, computed, watch } from 'vue'
import { showToast, showDialog } from 'vant'
import type { Skill, SkillRatingInfo } from '@/api'

const props = defineProps<{
  show: boolean
  skill: Skill | null
  isInstalled: boolean
  isBusy?: boolean
}>()

const emit = defineEmits<{
  'update:show': [v: boolean]
  install: [skill: Skill]
  uninstall: [skill: Skill]
  rate: [skill: Skill, score: number]
  apply: [skill: Skill]
}>()

const showRate = ref(false)
const currentScore = ref(5)
const showApply = ref(false)
const variableInputs = ref<{ key: string; value: string }[]>([])
const renderedPrompt = ref('')

const fileEntries = computed(() => {
  if (!props.skill?.files) return []
  return Object.entries(props.skill.files).map(([path, content]) => ({ path, content }))
})

const templateVars = computed(() => {
  if (!props.skill?.prompt_template) return []
  const matches = props.skill.prompt_template.matchAll(/\{\{(\w+)\}\}/g)
  const seen = new Set<string>()
  for (const m of matches) seen.add(m[1])
  return Array.from(seen)
})

watch(() => props.skill, (s) => {
  if (s) {
    variableInputs.value = templateVars.value.map(v => ({ key: v, value: '' }))
    renderedPrompt.value = ''
  }
}, { immediate: true })

function close() {
  emit('update:show', false)
  showRate.value = false
  showApply.value = false
}

async function onInstall() {
  if (!props.skill) return
  emit('install', props.skill)
}

async function onUninstall() {
  if (!props.skill) return
  try {
    await showDialog({
      title: '确认卸载',
      message: `确定要从全局目录卸载 "${props.skill.name}" 吗？`,
      showCancelButton: true,
    })
  } catch {
    return
  }
  emit('uninstall', props.skill)
}

function onRate() {
  showRate.value = true
}

async function submitRate() {
  if (!props.skill) return
  emit('rate', props.skill, currentScore.value)
  showRate.value = false
  showToast(`已评分 ${currentScore.value} 星`)
}

function onApply() {
  showApply.value = true
}

function doApply() {
  if (!props.skill) return
  const variables: Record<string, string> = {}
  for (const v of variableInputs.value) {
    if (v.key) variables[v.key] = v.value
  }
  // 构造合成消息：把渲染后的 prompt 传给后端
  renderedPrompt.value = ''
  // 实际渲染在后端做
  emit('apply', props.skill)
  showApply.value = false
  showToast('已发送 prompt 模板到主聊天')
}

async function copyText(text: string) {
  try {
    await navigator.clipboard?.writeText(text)
    showToast('已复制')
  } catch {
    showToast('复制失败')
  }
}
</script>

<template>
  <van-dialog
    :show="show"
    @update:show="(v: boolean) => emit('update:show', v)"
    :title="skill?.name || '技能详情'"
    close-on-click-overlay
    :style="{ width: '90vw', maxWidth: '720px' }"
  >
    <div v-if="skill" class="detail">
      <!-- 头部 -->
      <div class="detail-header">
        <div class="header-left">
          <div class="header-icon">⚡</div>
          <div>
            <div class="header-name">{{ skill.name }}</div>
            <div class="header-meta">
              v{{ skill.version }} · {{ skill.author || 'YunJi' }} · {{ skill.category }}
            </div>
          </div>
        </div>
        <div class="header-rating">
          <div class="rating-big">⭐ {{ (skill.rating || 0).toFixed(1) }}</div>
          <div class="rating-sub">
            {{ skill.rating_count || 0 }} 评分 ·
            {{ skill.downloads || 0 }} 下载
          </div>
        </div>
      </div>

      <!-- 描述 -->
      <div class="section">
        <div class="section-title">简介</div>
        <div class="section-content">{{ skill.description }}</div>
      </div>

      <!-- 长描述 -->
      <div v-if="skill.long_description" class="section">
        <div class="section-title">详细介绍</div>
        <pre class="long-desc">{{ skill.long_description }}</pre>
      </div>

      <!-- 标签 -->
      <div v-if="skill.tags?.length" class="section">
        <div class="section-title">标签</div>
        <div class="tags">
          <span v-for="t in skill.tags" :key="t" class="tag">#{{ t }}</span>
        </div>
      </div>

      <!-- Prompt 模板 -->
      <div v-if="skill.prompt_template" class="section">
        <div class="section-title-row">
          <div class="section-title">Prompt 模板</div>
          <button class="btn-mini" @click="copyText(skill.prompt_template)">📋 复制</button>
        </div>
        <pre class="code-block">{{ skill.prompt_template }}</pre>
      </div>

      <!-- 附带文件 -->
      <div v-if="fileEntries.length" class="section">
        <div class="section-title">附带文件 ({{ fileEntries.length }})</div>
        <div class="files">
          <div v-for="f in fileEntries" :key="f.path" class="file-item">
            <div class="file-path">📄 {{ f.path }}</div>
            <pre class="file-content">{{ f.content.length > 200 ? f.content.slice(0, 200) + '…' : f.content }}</pre>
          </div>
        </div>
      </div>

      <!-- 依赖 -->
      <div v-if="skill.dependencies?.length" class="section">
        <div class="section-title">依赖</div>
        <div class="deps">
          <span v-for="d in skill.dependencies" :key="d" class="dep">📦 {{ d }}</span>
        </div>
      </div>

      <!-- 元信息 -->
      <div class="section meta-grid">
        <div v-if="skill.license" class="meta-item">
          <span class="meta-label">许可证</span>
          <span class="meta-value">{{ skill.license }}</span>
        </div>
        <div v-if="skill.min_yunji_version" class="meta-item">
          <span class="meta-label">最低版本</span>
          <span class="meta-value">{{ skill.min_yunji_version }}</span>
        </div>
        <div v-if="skill.repository" class="meta-item">
          <span class="meta-label">仓库</span>
          <a class="meta-value" :href="skill.repository" target="_blank">{{ skill.repository }}</a>
        </div>
      </div>

      <!-- 操作 -->
      <div class="actions">
        <button v-if="!isInstalled" class="btn btn-primary" :disabled="isBusy" @click="onInstall">
          {{ isBusy ? '安装中…' : '📥 安装到全局' }}
        </button>
        <button v-else class="btn btn-secondary" :disabled="isBusy" @click="onUninstall">
          {{ isBusy ? '处理中…' : '🗑️ 卸载' }}
        </button>
        <button class="btn btn-secondary" @click="onRate">⭐ 评分</button>
        <button class="btn btn-secondary" @click="onApply" :disabled="!isInstalled">💬 应用到聊天</button>
      </div>
    </div>

    <!-- 评分弹窗 -->
    <van-dialog
      v-model:show="showRate"
      title="给技能打分"
      show-cancel-button
      @confirm="submitRate"
    >
      <div class="rate-pad">
        <div class="rate-stars">
          <span
            v-for="n in 5"
            :key="n"
            class="rate-star"
            :class="{ active: n <= currentScore }"
            @click="currentScore = n"
          >★</span>
        </div>
        <div class="rate-text">{{ currentScore }} / 5</div>
        <div class="rate-hint">点击星星选择分数</div>
      </div>
    </van-dialog>

    <!-- 应用弹窗 -->
    <van-dialog
      v-model:show="showApply"
      title="应用技能到聊天"
      show-cancel-button
      @confirm="doApply"
    >
      <div class="apply-pad">
        <div class="apply-hint">填写模板变量（缺失则保留占位符）：</div>
        <div v-if="!variableInputs.length" class="apply-empty">此模板无变量</div>
        <div v-for="(v, idx) in variableInputs" :key="idx" class="apply-input-row">
          <label class="apply-label">{{ v.key }}</label>
          <input
            class="apply-input"
            v-model="v.value"
            :placeholder="`输入 {{${v.key}}}`"
          />
        </div>
      </div>
    </van-dialog>
  </van-dialog>
</template>

<style scoped>
.detail {
  padding: 16px;
  max-height: 70vh;
  overflow-y: auto;
}

.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
  margin-bottom: 16px;
  gap: 12px;
}

.header-left {
  display: flex;
  gap: 12px;
  align-items: center;
  min-width: 0;
  flex: 1;
}

.header-icon {
  width: 48px;
  height: 48px;
  border-radius: 10px;
  background: linear-gradient(135deg, var(--yj-accent, #7c3aed), #a855f7);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  flex-shrink: 0;
}

.header-name {
  font-size: 18px;
  font-weight: 600;
  color: var(--yj-text-primary, #f5f5f5);
}

.header-meta {
  font-size: 12px;
  color: var(--yj-text-secondary, #888);
  margin-top: 4px;
}

.header-rating {
  text-align: right;
  flex-shrink: 0;
}

.rating-big {
  font-size: 22px;
  font-weight: 700;
  color: #fbbf24;
}

.rating-sub {
  font-size: 11px;
  color: var(--yj-text-secondary, #888);
  margin-top: 2px;
}

.section {
  margin-bottom: 16px;
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--yj-text-secondary, #888);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 6px;
}

.section-title-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.btn-mini {
  background: transparent;
  border: 1px solid var(--yj-border, #2a2a2a);
  color: var(--yj-text-secondary, #aaa);
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
}

.btn-mini:hover {
  border-color: var(--yj-accent, #7c3aed);
}

.section-content {
  font-size: 14px;
  line-height: 1.6;
  color: var(--yj-text-primary, #ddd);
}

.long-desc {
  font-family: inherit;
  font-size: 13px;
  line-height: 1.6;
  color: var(--yj-text-secondary, #aaa);
  white-space: pre-wrap;
  background: var(--yj-bg-base, #0d0d0d);
  padding: 10px;
  border-radius: 6px;
  margin: 0;
}

.code-block {
  font-family: 'Cascadia Code', 'Consolas', monospace;
  font-size: 12px;
  line-height: 1.5;
  background: var(--yj-bg-base, #0d0d0d);
  padding: 12px;
  border-radius: 6px;
  color: #c4b5fd;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 200px;
  overflow-y: auto;
  margin: 0;
}

.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.tag {
  font-size: 11px;
  padding: 3px 8px;
  border-radius: 10px;
  background: rgba(124, 58, 237, 0.12);
  color: var(--yj-accent, #a855f7);
}

.files {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.file-item {
  background: var(--yj-bg-base, #0d0d0d);
  border-radius: 6px;
  padding: 8px;
}

.file-path {
  font-family: 'Cascadia Code', 'Consolas', monospace;
  font-size: 12px;
  color: var(--yj-text-secondary, #888);
  margin-bottom: 4px;
}

.file-content {
  font-family: 'Cascadia Code', 'Consolas', monospace;
  font-size: 11px;
  color: #aaa;
  margin: 0;
  white-space: pre-wrap;
  max-height: 80px;
  overflow: hidden;
}

.deps {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.dep {
  font-size: 12px;
  padding: 3px 10px;
  border-radius: 10px;
  background: rgba(59, 130, 246, 0.12);
  color: #60a5fa;
}

.meta-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 8px;
  background: var(--yj-bg-base, #0d0d0d);
  padding: 10px;
  border-radius: 6px;
}

.meta-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.meta-label {
  font-size: 10px;
  color: var(--yj-text-secondary, #888);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.meta-value {
  font-size: 13px;
  color: var(--yj-text-primary, #ddd);
  word-break: break-all;
}

a.meta-value {
  color: var(--yj-accent, #a855f7);
  text-decoration: none;
}

a.meta-value:hover {
  text-decoration: underline;
}

.actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  padding-top: 12px;
  border-top: 1px solid var(--yj-border, #2a2a2a);
}

.btn {
  font-size: 13px;
  padding: 8px 14px;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s ease;
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
  background: var(--yj-bg-base, #0d0d0d);
  color: var(--yj-text-primary, #ddd);
  border: 1px solid var(--yj-border, #2a2a2a);
}

.btn-secondary:hover:not(:disabled) {
  border-color: var(--yj-accent, #7c3aed);
}

.rate-pad,
.apply-pad {
  padding: 20px;
  min-width: 280px;
}

.rate-stars {
  display: flex;
  justify-content: center;
  gap: 8px;
  margin-bottom: 8px;
}

.rate-star {
  font-size: 36px;
  color: #444;
  cursor: pointer;
  transition: color 0.15s;
}

.rate-star.active {
  color: #fbbf24;
}

.rate-text {
  text-align: center;
  font-size: 18px;
  font-weight: 600;
  color: var(--yj-text-primary, #f5f5f5);
}

.rate-hint {
  text-align: center;
  font-size: 12px;
  color: var(--yj-text-secondary, #888);
  margin-top: 4px;
}

.apply-hint {
  font-size: 13px;
  color: var(--yj-text-secondary, #aaa);
  margin-bottom: 12px;
}

.apply-empty {
  text-align: center;
  font-size: 12px;
  color: var(--yj-text-secondary, #888);
  padding: 20px;
}

.apply-input-row {
  margin-bottom: 10px;
}

.apply-label {
  display: block;
  font-size: 12px;
  color: var(--yj-text-secondary, #888);
  margin-bottom: 4px;
  font-family: 'Cascadia Code', 'Consolas', monospace;
}

.apply-input {
  width: 100%;
  background: var(--yj-bg-base, #0d0d0d);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 4px;
  padding: 8px 10px;
  color: var(--yj-text-primary, #ddd);
  font-size: 13px;
  box-sizing: border-box;
}

.apply-input:focus {
  outline: none;
  border-color: var(--yj-accent, #7c3aed);
}
</style>
