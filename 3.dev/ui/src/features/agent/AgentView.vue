<!--
  AgentView.vue
  2026-06-09 TASK-3.8 引入：独行模式 Code Agent UI
  功能：
    - 会话列表（按状态过滤）
    - 创建会话（弹窗）
    - 4 阶段 UI：Plan（粘贴文本）→ Approve（每步按钮）→ Execute（start/complete/fail）→ Learn（添加 L1-L4）
    - diffs/learnings 列表
    - 关闭会话
-->
<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import {
  agentApi,
  type AgentSession,
  type AgentPlanStep,
  type AgentStats,
} from '@/api'

const sessions = ref<AgentSession[]>([])
const selectedId = ref<string | null>(null)
const stats = ref<AgentStats | null>(null)
const loading = ref(false)
const statusFilter = ref<string>('')  // '' / 'draft' / 'planned' / 'executing' / 'closed' ...

// 弹窗
const showCreate = ref(false)
const showPlanEditor = ref(false)
const newRequirement = ref('')
const planText = ref('')
const creating = ref(false)
const savingPlan = ref(false)

// 添加 learn 表单
const showLearnDialog = ref(false)
const learnLayer = ref<'L1' | 'L2' | 'L3' | 'L4'>('L1')
const learnContent = ref('')
const learnStepId = ref<string | null>(null)
const addingLearn = ref(false)

const STATUS_META: Record<string, { label: string; icon: string; color: string }> = {
  draft:     { label: '草稿',   icon: '📝', color: '#94a3b8' },
  planned:   { label: '已规划', icon: '🗺️', color: '#3b82f6' },
  approved:  { label: '已批准', icon: '👍', color: '#10b981' },
  executing: { label: '执行中', icon: '⚙️', color: '#f59e0b' },
  done:      { label: '完成',   icon: '✅', color: '#10b981' },
  failed:    { label: '失败',   icon: '❌', color: '#ef4444' },
  learning:  { label: '学习中', icon: '🧠', color: '#8b5cf6' },
  closed:    { label: '已归档', icon: '📦', color: '#6b7280' },
}

const STEP_META: Record<string, { label: string; icon: string; color: string }> = {
  pending:   { label: '待审',   icon: '⏳', color: '#94a3b8' },
  approved:  { label: '已批',   icon: '👍', color: '#3b82f6' },
  rejected:  { label: '已拒',   icon: '🚫', color: '#ef4444' },
  executing: { label: '执行',   icon: '⚙️', color: '#f59e0b' },
  done:      { label: '完成',   icon: '✅', color: '#10b981' },
  failed:    { label: '失败',   icon: '❌', color: '#ef4444' },
  skipped:   { label: '跳过',   icon: '⏭️', color: '#6b7280' },
}

const LAYER_META: Record<string, { label: string; icon: string; color: string }> = {
  L1: { label: '纠正', icon: '✏️', color: 'var(--danger)' },
  L2: { label: '模式', icon: '🔁', color: 'var(--warning)' },
  L3: { label: '事实', icon: '📌', color: 'var(--accent)' },
  L4: { label: '偏好', icon: '⭐', color: 'var(--success)' },
}

const selectedSession = computed(() => sessions.value.find(s => s.id === selectedId.value) || null)

const progressText = computed(() => {
  if (!selectedSession.value) return ''
  const total = selectedSession.value.plan.length
  if (total === 0) return '暂无计划'
  const done = selectedSession.value.plan.filter(p => p.status === 'done').length
  const rejected = selectedSession.value.plan.filter(p => p.status === 'rejected').length
  const failed = selectedSession.value.plan.filter(p => p.status === 'failed').length
  return `${done}/${total} 完成 · ${rejected} 拒绝 · ${failed} 失败`
})

async function loadAll() {
  loading.value = true
  try {
    const [sRes, stRes] = await Promise.all([
      agentApi.sessions(statusFilter.value ? { status: statusFilter.value } : {}),
      agentApi.stats(),
    ])
    sessions.value = ((sRes as { data: AgentSession[] }).data) || []
    stats.value = (stRes as { data: AgentStats }).data
    if (!selectedId.value && sessions.value.length > 0) {
      selectedId.value = sessions.value[0].id
    }
  } catch (e: unknown) {
    showToast(`加载失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    loading.value = false
  }
}

async function handleCreate() {
  if (!newRequirement.value.trim()) {
    showToast('请输入需求描述')
    return
  }
  creating.value = true
  try {
    const res = await agentApi.create(newRequirement.value.trim())
    const newSession = (res as { data: AgentSession }).data
    sessions.value = [newSession, ...sessions.value]
    selectedId.value = newSession.id
    showCreate.value = false
    newRequirement.value = ''
    showToast('会话已创建')
  } catch (e: unknown) {
    showToast(`创建失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    creating.value = false
  }
}

async function handleSetPlan() {
  if (!selectedId.value || !planText.value.trim()) {
    showToast('请输入计划文本')
    return
  }
  savingPlan.value = true
  try {
    const res = await agentApi.setPlan(selectedId.value, planText.value)
    updateLocal((res as { data: AgentSession }).data)
    showPlanEditor.value = false
    planText.value = ''
    showToast('计划已设置')
  } catch (e: unknown) {
    showToast(`设置计划失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    savingPlan.value = false
  }
}

async function handleApprove(step: AgentPlanStep) {
  if (!selectedId.value) return
  try {
    const res = await agentApi.approveStep(selectedId.value, step.id)
    updateLocal((res as { data: AgentSession }).data)
  } catch (e: unknown) {
    showToast(`批准失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handleReject(step: AgentPlanStep) {
  if (!selectedId.value) return
  try {
    const reason = prompt('拒绝原因（可选）：') || ''
    const res = await agentApi.rejectStep(selectedId.value, step.id, reason)
    updateLocal((res as { data: AgentSession }).data)
  } catch (e: unknown) {
    showToast(`拒绝失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handleStart(step: AgentPlanStep) {
  if (!selectedId.value) return
  try {
    const res = await agentApi.startStep(selectedId.value, step.id)
    updateLocal((res as { data: AgentSession }).data)
  } catch (e: unknown) {
    showToast(`开始执行失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handleComplete(step: AgentPlanStep) {
  if (!selectedId.value) return
  try {
    const summary = prompt('完成摘要（可选）：') || ''
    const res = await agentApi.completeStep(selectedId.value, step.id, summary)
    updateLocal((res as { data: AgentSession }).data)
    showToast('步骤完成')
  } catch (e: unknown) {
    showToast(`完成失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handleFail(step: AgentPlanStep) {
  if (!selectedId.value) return
  const error = prompt('失败原因：')
  if (!error) return
  try {
    const res = await agentApi.failStep(selectedId.value, step.id, error)
    updateLocal((res as { data: AgentSession }).data)
  } catch (e: unknown) {
    showToast(`标记失败失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handleAddLearn() {
  if (!selectedId.value) return
  if (!learnContent.value.trim()) {
    showToast('请输入学习内容')
    return
  }
  addingLearn.value = true
  try {
    const res = await agentApi.addLearning(selectedId.value, {
      layer: learnLayer.value,
      content: learnContent.value.trim(),
      step_id: learnStepId.value || undefined,
    })
    updateLocal((res as { data: AgentSession }).data)
    showLearnDialog.value = false
    learnContent.value = ''
    learnStepId.value = null
    showToast('知识已添加（同时写入知识库）')
  } catch (e: unknown) {
    showToast(`添加失败: ${e instanceof Error ? e.message : e}`)
  } finally {
    addingLearn.value = false
  }
}

async function handleClose() {
  if (!selectedId.value) return
  try {
    await showConfirmDialog({ title: '关闭会话', message: '关闭后会自动写入 memory.md 和 plan.md' })
  } catch {
    return
  }
  try {
    const res = await agentApi.close(selectedId.value)
    updateLocal((res as { data: AgentSession }).data)
    showToast('会话已关闭')
    await loadAll()
  } catch (e: unknown) {
    showToast(`关闭失败: ${e instanceof Error ? e.message : e}`)
  }
}

async function handleDelete(session: AgentSession) {
  try {
    await showConfirmDialog({ title: '删除会话', message: `确认删除会话 "${session.requirement.slice(0, 30)}..."？` })
  } catch {
    return
  }
  try {
    await agentApi.remove(session.id)
    if (selectedId.value === session.id) selectedId.value = null
    sessions.value = sessions.value.filter(s => s.id !== session.id)
    showToast('已删除')
  } catch (e: unknown) {
    showToast(`删除失败: ${e instanceof Error ? e.message : e}`)
  }
}

function updateLocal(updated: AgentSession) {
  const idx = sessions.value.findIndex(s => s.id === updated.id)
  if (idx >= 0) sessions.value[idx] = updated
}

function openLearnDialog(stepId: string | null = null) {
  learnStepId.value = stepId
  learnContent.value = ''
  learnLayer.value = 'L1'
  showLearnDialog.value = true
}

function openPlanEditor() {
  planText.value = selectedSession.value?.plan.map(p => `- ${p.description}`).join('\n') || ''
  showPlanEditor.value = true
}

function formatTime(ts: number | null): string {
  if (!ts) return '-'
  return new Date(ts * 1000).toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
}

onMounted(loadAll)
</script>

<template>
  <div class="ag-shell">
    <header class="ag-header">
      <div class="ag-title-row">
        <h2 class="ag-title">🤖 独行 Agent</h2>
        <div class="ag-header-stats">
          <span v-if="stats" class="ag-stat">
            {{ stats.total_sessions }} 会话 · {{ stats.total_learnings }} 学习 · {{ stats.total_diffs }} 变更
          </span>
          <van-button size="small" type="primary" @click="showCreate = true">+ 新建</van-button>
        </div>
      </div>
      <div class="ag-filter-row">
        <button
          v-for="s in ['', 'draft', 'planned', 'executing', 'done', 'closed']"
          :key="s || 'all'"
          class="ag-chip"
          :class="{ 'ag-chip--active': statusFilter === s }"
          @click="statusFilter = s; loadAll()"
        >
          {{ s === '' ? '全部' : STATUS_META[s]?.label || s }}
        </button>
      </div>
    </header>

    <div class="ag-main">
      <!-- 左：会话列表 -->
      <aside class="ag-list">
        <div v-if="loading" class="ag-loading">加载中...</div>
        <div v-else-if="sessions.length === 0" class="ag-empty">暂无会话</div>
        <div
          v-for="s in sessions"
          :key="s.id"
          class="ag-list-item"
          :class="{ 'ag-list-item--active': selectedId === s.id }"
          @click="selectedId = s.id"
        >
          <div class="ag-list-row1">
            <span class="ag-list-status" :style="{ color: STATUS_META[s.status]?.color }">
              {{ STATUS_META[s.status]?.icon }} {{ STATUS_META[s.status]?.label }}
            </span>
            <span class="ag-list-id">#{{ s.id.slice(0, 6) }}</span>
          </div>
          <div class="ag-list-requirement">{{ s.requirement }}</div>
          <div class="ag-list-meta">
            <span>{{ s.plan.length }} 步</span>
            <span>·</span>
            <span>{{ s.learnings.length }} 学习</span>
            <span>·</span>
            <span>{{ formatTime(s.updated_at) }}</span>
          </div>
        </div>
      </aside>

      <!-- 右：会话详情 -->
      <section class="ag-detail">
        <div v-if="!selectedSession" class="ag-detail-empty">
          <div class="ag-detail-empty-icon">🤖</div>
          <div>选择左侧会话或创建新会话</div>
        </div>
        <div v-else class="ag-detail-body">
          <div class="ag-detail-header">
            <div class="ag-detail-title">
              <span
                class="ag-detail-status"
                :style="{ color: STATUS_META[selectedSession.status]?.color }"
              >
                {{ STATUS_META[selectedSession.status]?.icon }} {{ STATUS_META[selectedSession.status]?.label }}
              </span>
              <h3 class="ag-detail-req">{{ selectedSession.requirement }}</h3>
            </div>
            <div class="ag-detail-actions">
              <van-button
                v-if="selectedSession.plan.length === 0"
                size="small" type="primary" @click="openPlanEditor"
              >📋 设置计划</van-button>
              <van-button
                v-else
                size="small" plain @click="openPlanEditor"
              >✏️ 编辑计划</van-button>
              <van-button size="small" plain @click="openLearnDialog()">🧠 + Learn</van-button>
              <van-button
                v-if="!['closed', 'done'].includes(selectedSession.status)"
                size="small" plain type="success" @click="handleClose"
              >📦 关闭</van-button>
              <van-button size="small" plain type="danger" @click="handleDelete(selectedSession)">🗑</van-button>
            </div>
          </div>

          <div v-if="selectedSession.plan.length > 0" class="ag-progress">{{ progressText }}</div>

          <!-- 步骤列表 -->
          <div v-if="selectedSession.plan.length > 0" class="ag-section">
            <h4 class="ag-section-title">📋 计划步骤</h4>
            <div
              v-for="(step, idx) in selectedSession.plan"
              :key="step.id"
              class="ag-step"
              :class="{ 'ag-step--done': step.status === 'done', 'ag-step--rejected': step.status === 'rejected', 'ag-step--executing': step.status === 'executing' }"
            >
              <div class="ag-step-row1">
                <span class="ag-step-num">{{ idx + 1 }}</span>
                <span class="ag-step-status" :style="{ color: STEP_META[step.status]?.color }">
                  {{ STEP_META[step.status]?.icon }} {{ STEP_META[step.status]?.label }}
                </span>
                <span v-if="step.reject_reason" class="ag-step-reject">原因：{{ step.reject_reason }}</span>
              </div>
              <div class="ag-step-desc">{{ step.description }}</div>
              <div v-if="step.result" class="ag-step-result">结果：{{ step.result }}</div>
              <div v-if="step.tool_calls.length > 0" class="ag-step-tools">
                <div v-for="(tc, ti) in step.tool_calls" :key="ti" class="ag-step-tool">
                  🔧 <b>{{ tc.name }}</b>({{ JSON.stringify(tc.args).slice(0, 60) }}) → {{ tc.result.slice(0, 80) }}
                </div>
              </div>
              <div class="ag-step-actions">
                <van-button
                  v-if="step.status === 'pending'"
                  size="mini" type="primary" plain @click="handleApprove(step)"
                >✓ 批准</van-button>
                <van-button
                  v-if="step.status === 'pending'"
                  size="mini" type="danger" plain @click="handleReject(step)"
                >✕ 拒绝</van-button>
                <van-button
                  v-if="step.status === 'approved'"
                  size="mini" type="warning" @click="handleStart(step)"
                >▶ 开始</van-button>
                <van-button
                  v-if="step.status === 'executing'"
                  size="mini" type="success" @click="handleComplete(step)"
                >✓ 完成</van-button>
                <van-button
                  v-if="step.status === 'executing'"
                  size="mini" type="danger" plain @click="handleFail(step)"
                >✕ 失败</van-button>
                <van-button
                  v-if="['done', 'rejected', 'failed'].includes(step.status)"
                  size="mini" plain @click="openLearnDialog(step.id)"
                >🧠 Learn</van-button>
              </div>
            </div>
          </div>

          <!-- diffs -->
          <div v-if="selectedSession.diffs.length > 0" class="ag-section">
            <h4 class="ag-section-title">📝 代码变更（{{ selectedSession.diffs.length }}）</h4>
            <div v-for="(d, di) in selectedSession.diffs" :key="di" class="ag-diff">
              <div class="ag-diff-path">📁 {{ d.file_path }}</div>
              <div class="ag-diff-stats">
                <span class="ag-diff-add">+{{ d.added_lines }}</span>
                <span class="ag-diff-rm">-{{ d.removed_lines }}</span>
                <span v-if="d.summary" class="ag-diff-sum">{{ d.summary }}</span>
              </div>
            </div>
          </div>

          <!-- learnings -->
          <div v-if="selectedSession.learnings.length > 0" class="ag-section">
            <h4 class="ag-section-title">🧠 学习成果（{{ selectedSession.learnings.length }}）</h4>
            <div v-for="(l, li) in selectedSession.learnings" :key="li" class="ag-learning">
              <span
                class="ag-learning-layer"
                :style="{ background: LAYER_META[l.layer]?.color }"
              >{{ LAYER_META[l.layer]?.icon }} {{ l.layer }} {{ LAYER_META[l.layer]?.label }}</span>
              <span class="ag-learning-content">{{ l.content }}</span>
            </div>
          </div>
        </div>
      </section>
    </div>

    <!-- 新建会话弹窗 -->
    <van-popup v-model:show="showCreate" position="bottom" :style="{ borderRadius: '12px 12px 0 0' }">
      <div class="ag-popup">
        <h3>新建会话</h3>
        <textarea
          v-model="newRequirement"
          class="ag-textarea"
          placeholder="例如：实现用户登录功能，包括 OAuth2 流程"
          rows="3"
        />
        <van-button
          type="primary" block :loading="creating" :disabled="!newRequirement.trim()"
          @click="handleCreate"
        >创建</van-button>
      </div>
    </van-popup>

    <!-- 计划编辑器弹窗 -->
    <van-popup v-model:show="showPlanEditor" position="bottom" :style="{ height: '70vh', borderRadius: '12px 12px 0 0' }">
      <div class="ag-popup ag-popup--tall">
        <h3>设置计划（每行一步）</h3>
        <p class="ag-hint">支持：- 步骤、1. 步骤、纯行步骤</p>
        <textarea
          v-model="planText"
          class="ag-textarea ag-textarea--tall"
          placeholder="- 写 User 模型&#10;- 加 /login 端点&#10;- 写测试"
          rows="10"
        />
        <van-button
          type="primary" block :loading="savingPlan" :disabled="!planText.trim()"
          @click="handleSetPlan"
        >保存计划</van-button>
      </div>
    </van-popup>

    <!-- Learn 弹窗 -->
    <van-popup v-model:show="showLearnDialog" position="bottom" :style="{ borderRadius: '12px 12px 0 0' }">
      <div class="ag-popup">
        <h3>添加学习</h3>
        <div class="ag-chip-row">
          <button
            v-for="l in ['L1', 'L2', 'L3', 'L4'] as const"
            :key="l"
            class="ag-chip"
            :class="{ 'ag-chip--active': learnLayer === l }"
            @click="learnLayer = l"
          >
            {{ LAYER_META[l].icon }} {{ l }} {{ LAYER_META[l].label }}
          </button>
        </div>
        <textarea
          v-model="learnContent"
          class="ag-textarea"
          :placeholder="learnLayer === 'L1' ? '例如：不要在 User 模型里写密码' : '学习内容...'"
          rows="3"
        />
        <van-button
          type="primary" block :loading="addingLearn" :disabled="!learnContent.trim()"
          @click="handleAddLearn"
        >添加到知识库</van-button>
      </div>
    </van-popup>
  </div>
</template>

<style scoped>
.ag-shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: inherit;
  overflow: hidden;
  padding-bottom: var(--tab-bar-total, 24px);
  box-sizing: border-box;
}

.ag-header {
  flex-shrink: 0;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
}

.ag-title-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.ag-title {
  margin: 0;
  font-size: var(--font-xl);
  font-weight: 600;
}

.ag-header-stats {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.ag-stat {
  font-size: var(--font-xs);
  color: var(--text-muted);
}

.ag-filter-row {
  display: flex;
  gap: var(--space-1);
  margin-top: 6px;
  flex-wrap: wrap;
}

.ag-chip {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 2px 10px;
  font-size: var(--font-xs);
  color: var(--text-secondary);
  cursor: pointer;
  font-family: inherit;
}

.ag-chip--active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.ag-main {
  flex: 1;
  display: flex;
  overflow: hidden;
  min-height: 0;
}

.ag-list {
  width: 280px;
  flex-shrink: 0;
  overflow-y: auto;
  border-right: 1px solid var(--border);
  background: var(--bg-secondary);
}

.ag-list-item {
  padding: var(--space-3);
  border-bottom: 1px solid var(--border);
  cursor: pointer;
}

.ag-list-item--active {
  background: var(--bg-card);
  border-left: 3px solid var(--accent);
}

.ag-list-row1 {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.ag-list-status {
  font-size: var(--font-xs);
  font-weight: 600;
}

.ag-list-id {
  font-size: var(--font-xs);
  color: var(--text-muted);
  font-family: Consolas, monospace;
}

.ag-list-requirement {
  font-size: var(--font-sm);
  color: var(--text-primary);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.ag-list-meta {
  display: flex;
  gap: 4px;
  font-size: var(--font-xs);
  color: var(--text-muted);
  margin-top: 4px;
}

.ag-loading,
.ag-empty {
  padding: var(--space-6);
  text-align: center;
  color: var(--text-muted);
}

.ag-detail {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
}

.ag-detail-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-muted);
}

.ag-detail-empty-icon {
  font-size: 56px;
  opacity: 0.5;
  margin-bottom: var(--space-2);
}

.ag-detail-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.ag-detail-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.ag-detail-title {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
  min-width: 200px;
}

.ag-detail-status {
  font-size: var(--font-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.ag-detail-req {
  margin: 0;
  font-size: var(--font-lg);
  font-weight: 600;
  line-height: 1.4;
}

.ag-detail-actions {
  display: flex;
  gap: var(--space-1);
  flex-wrap: wrap;
}

.ag-progress {
  padding: 6px 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: var(--font-sm);
  color: var(--text-secondary);
}

.ag-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.ag-section-title {
  margin: 0;
  font-size: var(--font-base);
  font-weight: 600;
  color: var(--text-primary);
}

.ag-step {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-left: 3px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.ag-step--done { border-left-color: #10b981; }
.ag-step--rejected { border-left-color: #ef4444; opacity: 0.7; }
.ag-step--executing { border-left-color: #f59e0b; }

.ag-step-row1 {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--font-xs);
}

.ag-step-num {
  background: var(--bg-secondary);
  border-radius: 999px;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 10px;
}

.ag-step-status {
  font-weight: 600;
}

.ag-step-reject {
  color: var(--danger);
  font-style: italic;
}

.ag-step-desc {
  font-size: var(--font-base);
  color: var(--text-primary);
  line-height: 1.4;
}

.ag-step-result {
  font-size: var(--font-sm);
  color: var(--success);
  font-family: Consolas, monospace;
  background: var(--bg-secondary);
  padding: 4px 6px;
  border-radius: 3px;
}

.ag-step-tools {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.ag-step-tool {
  font-size: var(--font-xs);
  color: var(--text-muted);
  font-family: Consolas, monospace;
  background: var(--bg-secondary);
  padding: 2px 6px;
  border-radius: 3px;
  word-break: break-all;
}

.ag-step-actions {
  display: flex;
  gap: var(--space-1);
  flex-wrap: wrap;
  padding-top: 4px;
  border-top: 1px solid var(--border);
  margin-top: 4px;
}

.ag-diff {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.ag-diff-path {
  font-family: Consolas, monospace;
  font-size: var(--font-sm);
  color: var(--text-primary);
  word-break: break-all;
}

.ag-diff-stats {
  display: flex;
  gap: 6px;
  font-family: Consolas, monospace;
  font-size: var(--font-xs);
  white-space: nowrap;
}

.ag-diff-add { color: #10b981; }
.ag-diff-rm { color: #ef4444; }
.ag-diff-sum { color: var(--text-muted); font-style: italic; }

.ag-learning {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  flex-wrap: wrap;
}

.ag-learning-layer {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: var(--font-xs);
  font-weight: 600;
  color: #fff;
}

.ag-learning-content {
  flex: 1;
  font-size: var(--font-sm);
  color: var(--text-primary);
  word-break: break-word;
}

.ag-popup {
  background: var(--bg-primary);
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.ag-popup--tall {
  height: 100%;
}

.ag-popup h3 {
  margin: 0;
  font-size: var(--font-lg);
  font-weight: 600;
}

.ag-hint {
  margin: 0;
  font-size: var(--font-xs);
  color: var(--text-muted);
}

.ag-textarea {
  width: 100%;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  color: var(--text-primary);
  font-size: var(--font-base);
  font-family: inherit;
  resize: vertical;
  box-sizing: border-box;
}

.ag-textarea--tall {
  min-height: 200px;
  font-family: Consolas, monospace;
}

.ag-textarea:focus {
  outline: none;
  border-color: var(--accent);
}

.ag-chip-row {
  display: flex;
  gap: var(--space-1);
  flex-wrap: wrap;
}

/* 移动端适配：左右改上下 */
@media (max-width: 768px) {
  .ag-main {
    flex-direction: column;
  }
  .ag-list {
    width: 100%;
    max-height: 40%;
    border-right: none;
    border-bottom: 1px solid var(--border);
  }
}
</style>
