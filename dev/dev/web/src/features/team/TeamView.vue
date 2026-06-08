<!--
  TeamView.vue
  2026-06-09 TASK-3.10 引入：团队模式 UI（4-5 Agent 并行）
  布局：
    ┌──────────┬────────────────────────┬───────────────────────┐
    │ 任务看板 │ 团队对话流              │ Agent 面板            │
    │ 📋 待办 │ 🎯 主管: ...           │ 🎯 主管 [协调中]    │
    │ 🔄 进行中│ 💻 开发: ...           │ 💻 开发 [工作中]    │
    │ ⏸ 待审批│ ⏸ 审批节点             │ 🧪 测试 [等待]      │
    │ ✅ 完成 │ 📌 共享上下文           │ 📝 文档 [等待]      │
    └──────────┴────────────────────────┴───────────────────────┘
  流程：主管拆解 → 派发 → 并行执行 → 交叉审查 → 仲裁
-->
<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import {
  teamApi,
  type TeamSession,
  type TeamStats,
  type TeamStatus,
} from '@/api'
import AgentCard from './AgentCard.vue'
import TaskBoard from './TaskBoard.vue'
import ApprovalNode from './ApprovalNode.vue'
import SharedContext from './SharedContext.vue'
import PlanPanel from './PlanPanel.vue'

const sessions = ref<TeamSession[]>([])
const selectedId = ref<string | null>(null)
const stats = ref<TeamStats | null>(null)
const teamStatus = ref<TeamStatus | null>(null)
const loading = ref(false)
const statusFilter = ref<string>('')

// 创建会话
const showCreate = ref(false)
const newRequirement = ref('')
const creating = ref(false)

// 添加任务
const showAddTask = ref(false)
const newTaskTitle = ref('')
const newTaskDescription = ref('')
const newTaskRole = ref('developer')
const newTaskDeps = ref<string[]>([])
const addingTask = ref(false)

// 派发任务
const showAssign = ref(false)
const assignTaskId = ref('')
const assignAgentId = ref('')
const assigning = ref(false)

// 完成任务
const showComplete = ref(false)
const completeTaskId = ref('')
const completeOutput = ref('')
const completeFiles = ref('')
const completing = ref(false)

// 失败任务（UI 在右键菜单中触发，目前先保留入口）
const showFail = ref(false)
const failTaskId = ref('')
const failError = ref('')
const failing = ref(false)

// 审批
const showApproval = ref(false)
const approvalTaskId = ref('')
const approvalRole = ref('coordinator')
const approvalReason = ref('')
const requestingApproval = ref(false)

// 团队消息输入
const chatRole = ref('coordinator')
const chatContent = ref('')
const sending = ref(false)

const selectedSession = computed<TeamSession | null>(
  () => sessions.value.find(s => s.id === selectedId.value) || null
)

// ──────────── 加载 ────────────

async function loadAll() {
  loading.value = true
  try {
    const [sessResp, statsResp, statusResp] = await Promise.all([
      teamApi.sessions({ status: statusFilter.value || undefined }),
      teamApi.stats(),
      teamApi.status(),
    ])
    sessions.value = (sessResp.data || []) as TeamSession[]
    stats.value = statsResp.data
    teamStatus.value = statusResp.data
    if (!selectedId.value && sessions.value.length > 0) {
      selectedId.value = sessions.value[0].id
    }
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  } finally {
    loading.value = false
  }
}

async function refreshSelected() {
  if (!selectedId.value) return
  try {
    const resp = await teamApi.session(selectedId.value)
    const updated = resp.data as TeamSession
    const idx = sessions.value.findIndex(s => s.id === updated.id)
    if (idx >= 0) {
      sessions.value[idx] = updated
    } else {
      sessions.value.unshift(updated)
    }
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  }
}

onMounted(loadAll)

// ──────────── 会话操作 ────────────

async function handleCreate() {
  if (!newRequirement.value.trim()) {
    showToast('请输入需求')
    return
  }
  creating.value = true
  try {
    const resp = await teamApi.create(newRequirement.value.trim())
    const session = resp.data as TeamSession
    sessions.value.unshift(session)
    selectedId.value = session.id
    showCreate.value = false
    newRequirement.value = ''
    showToast({ type: 'success', message: '团队会话已创建' })
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  } finally {
    creating.value = false
  }
}

async function handleDelete(id: string) {
  try {
    await showConfirmDialog({
      title: '删除团队会话？',
      message: '该操作不可撤销',
    })
    await teamApi.remove(id)
    sessions.value = sessions.value.filter(s => s.id !== id)
    if (selectedId.value === id) {
      selectedId.value = sessions.value[0]?.id || null
    }
    showToast('已删除')
  } catch (e) {
    if ((e as Error).message !== 'cancel') {
      showToast({ type: 'fail', message: (e as Error).message })
    }
  }
}

async function handleClose(id: string) {
  try {
    await teamApi.close(id)
    await refreshSelected()
    showToast('已归档')
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  }
}

// ──────────── 任务操作 ────────────

function openAddTask() {
  if (!selectedId.value) {
    showToast('请先选择会话')
    return
  }
  showAddTask.value = true
  newTaskTitle.value = ''
  newTaskDescription.value = ''
  newTaskRole.value = 'developer'
  newTaskDeps.value = []
}

async function handleAddTask() {
  if (!newTaskTitle.value.trim() || !selectedId.value) return
  addingTask.value = true
  try {
    await teamApi.addTask(selectedId.value, {
      title: newTaskTitle.value.trim(),
      description: newTaskDescription.value.trim(),
      role: newTaskRole.value,
      dependencies: newTaskDeps.value,
    })
    showAddTask.value = false
    await refreshSelected()
    showToast({ type: 'success', message: '任务已添加' })
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  } finally {
    addingTask.value = false
  }
}

function openAssign(taskId: string) {
  assignTaskId.value = taskId
  assignAgentId.value = ''
  showAssign.value = true
}

async function handleAssign() {
  if (!assignAgentId.value.trim() || !selectedId.value) {
    showToast('请输入 Agent ID')
    return
  }
  assigning.value = true
  try {
    await teamApi.assignTask(selectedId.value, assignTaskId.value, assignAgentId.value.trim())
    showAssign.value = false
    await refreshSelected()
    showToast({ type: 'success', message: '任务已派发' })
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  } finally {
    assigning.value = false
  }
}

function openComplete(taskId: string) {
  completeTaskId.value = taskId
  completeOutput.value = ''
  completeFiles.value = ''
  showComplete.value = true
}

async function handleComplete() {
  if (!selectedId.value) return
  completing.value = true
  try {
    const filePaths = completeFiles.value
      .split(/[\n,]/)
      .map(s => s.trim())
      .filter(Boolean)
    await teamApi.completeTask(selectedId.value, completeTaskId.value, {
      output: completeOutput.value.trim() || undefined,
      file_paths: filePaths.length ? filePaths : undefined,
    })
    showComplete.value = false
    await refreshSelected()
    showToast({ type: 'success', message: '任务已完成' })
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  } finally {
    completing.value = false
  }
}

function openFail(taskId: string) {
  failTaskId.value = taskId
  failError.value = ''
  showFail.value = true
}

async function handleFail() {
  if (!failError.value.trim() || !selectedId.value) return
  failing.value = true
  try {
    await teamApi.failTask(selectedId.value, failTaskId.value, failError.value.trim())
    showFail.value = false
    await refreshSelected()
    showToast('已上报失败')
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  } finally {
    failing.value = false
  }
}

// ──────────── 审批操作 ────────────

function openApproval(taskId: string) {
  approvalTaskId.value = taskId
  approvalRole.value = 'coordinator'
  approvalReason.value = ''
  showApproval.value = true
}

async function handleRequestApproval() {
  if (!approvalReason.value.trim() || !selectedId.value) return
  requestingApproval.value = true
  try {
    await teamApi.requestApproval(selectedId.value, {
      task_id: approvalTaskId.value,
      approver_role: approvalRole.value,
      reason: approvalReason.value.trim(),
    })
    showApproval.value = false
    await refreshSelected()
    showToast({ type: 'success', message: '审批节点已创建' })
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  } finally {
    requestingApproval.value = false
  }
}

async function handleDecide(approvalId: string, approve: boolean, comment: string) {
  if (!selectedId.value) return
  try {
    await teamApi.decideApproval(selectedId.value, approvalId, { approve, comment })
    await refreshSelected()
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  }
}

// ──────────── 共享上下文 ────────────

async function handleContextAdd(key: string, value: string, source: string) {
  if (!selectedId.value) return
  try {
    await teamApi.updateContext(selectedId.value, { key, value, source })
    await refreshSelected()
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  }
}

// ──────────── 团队消息 ────────────

async function handleSendMessage() {
  if (!chatContent.value.trim() || !selectedId.value) return
  sending.value = true
  try {
    await teamApi.addMessage(selectedId.value, {
      role: chatRole.value,
      content: chatContent.value.trim(),
      type: 'chat',
    })
    chatContent.value = ''
    await refreshSelected()
  } catch (e) {
    showToast({ type: 'fail', message: (e as Error).message })
  } finally {
    sending.value = false
  }
}

// ──────────── Agent 状态计算 ────────────

interface AgentState {
  role: 'coordinator' | 'architect' | 'developer' | 'tester' | 'documenter'
  label: string
  readOnly: boolean
  status: 'idle' | 'working' | 'waiting' | 'reviewing' | 'arbitrating'
  currentTask: string | null
  tasksCompleted: number
}

const ROLES_DEF: Array<{ role: 'coordinator' | 'architect' | 'developer' | 'tester' | 'documenter'; label: string; readOnly: boolean }> = [
  { role: 'coordinator', label: '🎯 主管', readOnly: false },
  { role: 'architect', label: '🏗️ 架构师', readOnly: true },
  { role: 'developer', label: '💻 开发', readOnly: false },
  { role: 'tester', label: '🧪 测试', readOnly: false },
  { role: 'documenter', label: '📝 文档', readOnly: false },
]

const agentStates = computed<AgentState[]>(() => {
  if (!selectedSession.value) {
    return ROLES_DEF.map(r => ({
      ...r,
      status: 'idle' as const,
      currentTask: null,
      tasksCompleted: 0,
    }))
  }
  const s = selectedSession.value
  return ROLES_DEF.map(r => {
    const myTasks = s.tasks.filter(t => t.role === r.role)
    const inProgress = myTasks.find(t => t.status === 'in_progress')
    const awaiting = myTasks.find(t => t.status === 'awaiting_approval')
    const done = myTasks.filter(t => t.status === 'done' || t.status === 'approved').length

    let status: AgentState['status'] = 'idle'
    let currentTask: string | null = null
    if (inProgress) {
      status = 'working'
      currentTask = inProgress.title
    } else if (awaiting) {
      status = 'waiting'
      currentTask = awaiting.title
    } else if (s.status === 'arbitrating' && r.role === 'coordinator') {
      status = 'arbitrating'
    } else if (s.status === 'reviewing' && (r.role === 'tester' || r.role === 'architect')) {
      status = 'reviewing'
    }
    return { ...r, status, currentTask, tasksCompleted: done }
  })
})

// 待审批列表
const pendingApprovals = computed(() => {
  if (!selectedSession.value) return []
  return selectedSession.value.approvals.filter(a => a.status === 'pending')
})

function timeAgo(ts: number): string {
  const diff = Date.now() / 1000 - ts
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  return `${Math.floor(diff / 86400)} 天前`
}

function taskTitleById(id: string): string {
  if (!selectedSession.value) return ''
  const task = selectedSession.value.tasks.find(t => t.id === id)
  return task?.title || `#${id}`
}

function toggleDep(taskId: string) {
  if (newTaskDeps.value.includes(taskId)) {
    newTaskDeps.value = newTaskDeps.value.filter(d => d !== taskId)
  } else {
    newTaskDeps.value.push(taskId)
  }
}
</script>

<template>
  <div class="team-view">
    <!-- 顶栏：会话选择 + 操作 -->
    <div class="topbar">
      <div class="topbar-left">
        <h2 class="page-title">👥 团队模式</h2>
        <select v-model="statusFilter" class="filter-select" @change="loadAll">
          <option value="">全部状态</option>
          <option value="planning">规划中</option>
          <option value="in_progress">执行中</option>
          <option value="reviewing">审查中</option>
          <option value="arbitrating">仲裁中</option>
          <option value="done">已完成</option>
          <option value="closed">已归档</option>
        </select>
      </div>
      <div class="topbar-right">
        <div v-if="stats" class="stats">
          <span class="stat-item">会话 {{ stats.total_sessions }}</span>
          <span class="stat-item">任务 {{ stats.total_tasks }}</span>
          <span class="stat-item">审批 {{ stats.total_approvals }}</span>
        </div>
        <button class="btn btn-primary" @click="showCreate = true">+ 新建团队</button>
      </div>
    </div>

    <!-- 会话列表（横向滚动） -->
    <div class="session-list-bar">
      <div
        v-for="s in sessions"
        :key="s.id"
        class="session-chip"
        :class="{ active: selectedId === s.id }"
        @click="selectedId = s.id"
      >
        <span class="chip-id">#{{ s.id.slice(0, 6) }}</span>
        <span class="chip-req">{{ s.requirement.length > 20 ? s.requirement.slice(0, 20) + '…' : s.requirement }}</span>
        <span class="chip-status" :class="'status-' + s.status">{{ s.status }}</span>
      </div>
      <div v-if="sessions.length === 0" class="empty-sessions">暂无团队会话</div>
    </div>

    <!-- 三栏布局 -->
    <div class="main-grid">
      <!-- 左：任务看板 -->
      <div class="col col-board">
        <div class="col-header">
          <span class="col-title">📋 任务看板</span>
          <button v-if="selectedSession" class="btn-mini" @click="openAddTask">+ 任务</button>
        </div>
        <div v-if="selectedSession" class="col-body">
          <TaskBoard
            :tasks="selectedSession.tasks"
            @select="(id) => { /* 选中任务 */ }"
            @assign="openAssign"
            @start="(id) => { openComplete(id) }"
            @complete="openComplete"
            @fail="openFail"
          />
        </div>
        <div v-else class="col-empty">请先选择会话</div>
      </div>

      <!-- 中：团队对话流 -->
      <div class="col col-chat">
        <div class="col-header">
          <span class="col-title">💬 团队对话流</span>
          <span v-if="selectedSession" class="col-subtitle">
            {{ selectedSession.messages.length }} 条消息
          </span>
        </div>
        <div v-if="selectedSession" class="col-body chat-body">
          <div class="messages">
            <div
              v-for="msg in selectedSession.messages"
              :key="msg.id"
              class="msg"
              :class="['role-' + msg.role, 'type-' + msg.type]"
            >
              <div class="msg-header">
                <span class="msg-role">{{ msg.role }}</span>
                <span class="msg-time">{{ timeAgo(msg.ts) }}</span>
                <span v-if="msg.task_id" class="msg-task">📋 #{{ msg.task_id }}</span>
              </div>
              <div class="msg-content">{{ msg.content }}</div>
            </div>
          </div>

          <div v-if="pendingApprovals.length" class="approvals-section">
            <div class="approvals-title">⏸ 待审批 ({{ pendingApprovals.length }})</div>
            <ApprovalNode
              v-for="a in pendingApprovals"
              :key="a.id"
              :approval="a"
              :task-title="taskTitleById(a.task_id)"
              :can-decide="true"
              @decide="handleDecide"
            />
          </div>

          <SharedContext
            :items="selectedSession.shared_context"
            :can-edit="true"
            @add="handleContextAdd"
          />

          <div class="chat-input-bar">
            <select v-model="chatRole" class="role-select">
              <option value="coordinator">🎯 主管</option>
              <option value="architect">🏗️ 架构</option>
              <option value="developer">💻 开发</option>
              <option value="tester">🧪 测试</option>
              <option value="documenter">📝 文档</option>
            </select>
            <input
              v-model="chatContent"
              class="chat-input"
              placeholder="发条消息..."
              @keyup.enter="handleSendMessage"
            />
            <button class="btn btn-primary btn-small" :disabled="sending" @click="handleSendMessage">发送</button>
          </div>
        </div>
        <div v-else class="col-empty">请先选择会话</div>
      </div>

      <!-- 右：Agent 面板 + 计划面板 -->
      <div class="col col-right">
        <div class="agent-panel">
          <div class="col-header">
            <span class="col-title">🤖 Agent 面板</span>
          </div>
          <div class="agent-grid">
            <AgentCard
              v-for="a in agentStates"
              :key="a.role"
              :role="a.role"
              :label="a.label"
              :status="a.status"
              :current-task="a.currentTask"
              :tasks-completed="a.tasksCompleted"
              :read-only="a.readOnly"
            />
          </div>
        </div>

        <div class="plan-section">
          <div class="col-header">
            <span class="col-title">📊 计划面板</span>
            <div v-if="selectedSession" class="col-actions">
              <button class="btn-mini" @click="openAddTask">+ 任务</button>
              <button v-if="pendingApprovals.length === 0" class="btn-mini primary" @click="openApproval('')">+ 审批</button>
              <button v-if="selectedSession.status !== 'closed'" class="btn-mini warn" @click="handleClose(selectedSession.id)">归档</button>
              <button class="btn-mini danger" @click="handleDelete(selectedSession.id)">删除</button>
            </div>
          </div>
          <div class="plan-body">
            <PlanPanel :session="selectedSession" />
          </div>
        </div>
      </div>
    </div>

    <!-- 弹窗：创建会话 -->
    <van-popup v-model:show="showCreate" position="center" round :style="{ width: '90%', maxWidth: '500px' }">
      <div class="popup">
        <h3>新建团队会话</h3>
        <p class="hint">主管将自动拆解为多 Agent 任务</p>
        <textarea
          v-model="newRequirement"
          class="popup-textarea"
          placeholder="需求描述（如：实现用户登录功能）"
          rows="4"
        />
        <div class="popup-actions">
          <button class="btn btn-cancel" @click="showCreate = false">取消</button>
          <button class="btn btn-primary" :disabled="creating" @click="handleCreate">创建</button>
        </div>
      </div>
    </van-popup>

    <!-- 弹窗：添加任务 -->
    <van-popup v-model:show="showAddTask" position="center" round :style="{ width: '90%', maxWidth: '500px' }">
      <div class="popup">
        <h3>添加任务</h3>
        <input v-model="newTaskTitle" class="popup-input" placeholder="任务标题" />
        <textarea
          v-model="newTaskDescription"
          class="popup-textarea"
          placeholder="任务描述"
          rows="2"
        />
        <select v-model="newTaskRole" class="popup-select">
          <option value="architect">🏗️ 架构师（只读）</option>
          <option value="developer">💻 开发工程师</option>
          <option value="tester">🧪 测试工程师</option>
          <option value="documenter">📝 文档工程师</option>
        </select>
        <div v-if="selectedSession && selectedSession.tasks.length" class="deps-selector">
          <div class="deps-label">依赖（可多选）:</div>
          <div class="deps-list">
            <label v-for="t in selectedSession.tasks" :key="t.id" class="dep-chip">
              <input
                type="checkbox"
                :checked="newTaskDeps.includes(t.id)"
                @change="toggleDep(t.id)"
              />
              <span>#{{ t.id }} {{ t.title.slice(0, 15) }}</span>
            </label>
          </div>
        </div>
        <div class="popup-actions">
          <button class="btn btn-cancel" @click="showAddTask = false">取消</button>
          <button class="btn btn-primary" :disabled="addingTask" @click="handleAddTask">添加</button>
        </div>
      </div>
    </van-popup>

    <!-- 弹窗：派发任务 -->
    <van-popup v-model:show="showAssign" position="center" round :style="{ width: '90%', maxWidth: '400px' }">
      <div class="popup">
        <h3>派发任务 #{{ assignTaskId }}</h3>
        <input v-model="assignAgentId" class="popup-input" placeholder="Agent ID（如 dev-1）" />
        <div class="popup-actions">
          <button class="btn btn-cancel" @click="showAssign = false">取消</button>
          <button class="btn btn-primary" :disabled="assigning" @click="handleAssign">派发</button>
        </div>
      </div>
    </van-popup>

    <!-- 弹窗：完成任务 -->
    <van-popup v-model:show="showComplete" position="center" round :style="{ width: '90%', maxWidth: '500px' }">
      <div class="popup">
        <h3>完成任务 #{{ completeTaskId }}</h3>
        <textarea
          v-model="completeOutput"
          class="popup-textarea"
          placeholder="输出摘要（可选）"
          rows="2"
        />
        <textarea
          v-model="completeFiles"
          class="popup-textarea"
          placeholder="涉及的文件路径（每行一个或逗号分隔，触发角色边界检查）"
          rows="3"
        />
        <div class="popup-actions">
          <button class="btn btn-cancel" @click="showComplete = false">取消</button>
          <button class="btn btn-primary" :disabled="completing" @click="handleComplete">完成</button>
        </div>
      </div>
    </van-popup>

    <!-- 弹窗：失败上报 -->
    <van-popup v-model:show="showFail" position="center" round :style="{ width: '90%', maxWidth: '400px' }">
      <div class="popup">
        <h3>上报失败 #{{ failTaskId }}</h3>
        <textarea v-model="failError" class="popup-textarea" placeholder="错误描述" rows="3" />
        <div class="popup-actions">
          <button class="btn btn-cancel" @click="showFail = false">取消</button>
          <button class="btn btn-primary" :disabled="failing" @click="handleFail">上报</button>
        </div>
      </div>
    </van-popup>

    <!-- 弹窗：触发审批 -->
    <van-popup v-model:show="showApproval" position="center" round :style="{ width: '90%', maxWidth: '500px' }">
      <div class="popup">
        <h3>触发审批</h3>
        <select v-if="!approvalTaskId && selectedSession" v-model="approvalTaskId" class="popup-select">
          <option value="">-- 选择任务 --</option>
          <option v-for="t in selectedSession.tasks" :key="t.id" :value="t.id">
            #{{ t.id }} {{ t.title }} ({{ t.status }})
          </option>
        </select>
        <div v-else class="approval-task">
          任务: #{{ approvalTaskId }}
        </div>
        <select v-model="approvalRole" class="popup-select">
          <option value="coordinator">🎯 主管（仲裁权）</option>
          <option value="architect">🏗️ 架构师</option>
          <option value="tester">🧪 测试工程师</option>
        </select>
        <textarea
          v-model="approvalReason"
          class="popup-textarea"
          placeholder="审批原因（如：需要主管确认合并）"
          rows="3"
        />
        <div class="popup-actions">
          <button class="btn btn-cancel" @click="showApproval = false">取消</button>
          <button class="btn btn-primary" :disabled="requestingApproval" @click="handleRequestApproval">提交</button>
        </div>
      </div>
    </van-popup>
  </div>
</template>

<style scoped>
.team-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--app-bg, #020617);
  color: var(--text-primary, #f1f5f9);
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  background: var(--topbar-bg, #0f172a);
  border-bottom: 1px solid var(--border-color, #1e293b);
  flex-shrink: 0;
}

.topbar-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.page-title {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.filter-select {
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 4px;
  padding: 4px 8px;
  color: #f1f5f9;
  font-size: 12px;
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.stats {
  display: flex;
  gap: 8px;
  font-size: 11px;
  color: var(--text-tertiary, #94a3b8);
}

.stat-item {
  font-family: monospace;
}

.btn {
  font-size: 12px;
  padding: 6px 14px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: #3b82f6;
  color: #fff;
}

.btn-primary:hover:not(:disabled) {
  background: #2563eb;
}

.btn-cancel {
  background: #475569;
  color: #f1f5f9;
}

.btn-cancel:hover {
  background: #64748b;
}

.btn-small {
  padding: 4px 10px;
  font-size: 11px;
}

.session-list-bar {
  display: flex;
  gap: 6px;
  padding: 8px 16px;
  background: var(--card-bg, #0f172a);
  border-bottom: 1px solid var(--border-color, #1e293b);
  overflow-x: auto;
  flex-shrink: 0;
  min-height: 50px;
}

.session-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 16px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s;
}

.session-chip:hover {
  border-color: #3b82f6;
}

.session-chip.active {
  background: #3b82f6;
  border-color: #3b82f6;
  color: #fff;
}

.chip-id {
  font-family: monospace;
  font-size: 10px;
  opacity: 0.7;
}

.chip-status {
  font-size: 9px;
  padding: 1px 4px;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 3px;
  text-transform: uppercase;
}

.empty-sessions {
  font-size: 12px;
  color: var(--text-tertiary, #64748b);
  font-style: italic;
  padding: 8px;
}

.main-grid {
  display: grid;
  grid-template-columns: 1.2fr 1.5fr 1fr;
  gap: 1px;
  flex: 1;
  background: var(--border-color, #1e293b);
  min-height: 0;
}

.col {
  display: flex;
  flex-direction: column;
  background: var(--app-bg, #020617);
  min-height: 0;
  overflow: hidden;
}

.col-board {
  /* 任务看板 */
}

.col-chat {
  /* 团队对话流 */
}

.col-right {
  display: flex;
  flex-direction: column;
}

.col-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--card-bg, #0f172a);
  border-bottom: 1px solid var(--border-color, #1e293b);
  flex-shrink: 0;
}

.col-title {
  font-size: 13px;
  font-weight: 600;
}

.col-subtitle {
  font-size: 11px;
  color: var(--text-tertiary, #64748b);
}

.col-actions {
  display: flex;
  gap: 4px;
}

.btn-mini {
  font-size: 10px;
  padding: 2px 8px;
  background: #334155;
  color: #f1f5f9;
  border: none;
  border-radius: 3px;
  cursor: pointer;
}

.btn-mini:hover {
  background: #475569;
}

.btn-mini.primary {
  background: #3b82f6;
}

.btn-mini.warn {
  background: #f59e0b;
  color: #000;
}

.btn-mini.danger {
  background: #ef4444;
}

.col-body {
  flex: 1;
  padding: 8px;
  overflow: hidden;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.col-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-tertiary, #64748b);
  font-style: italic;
}

/* 团队对话流 */
.chat-body {
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
}

.messages {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-height: 0;
  max-height: 200px;
}

.msg {
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 12px;
}

.msg.role-coordinator {
  border-left: 3px solid #3b82f6;
}

.msg.role-architect {
  border-left: 3px solid #06b6d4;
}

.msg.role-developer {
  border-left: 3px solid #10b981;
}

.msg.role-tester {
  border-left: 3px solid #f59e0b;
}

.msg.role-documenter {
  border-left: 3px solid #8b5cf6;
}

.msg.type-system {
  background: #0f172a;
  font-style: italic;
}

.msg.type-approval {
  background: rgba(245, 158, 11, 0.1);
  border-color: rgba(245, 158, 11, 0.3);
}

.msg-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 2px;
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
}

.msg-role {
  font-weight: 600;
  text-transform: uppercase;
}

.msg-task {
  font-family: monospace;
  color: var(--accent-color, #60a5fa);
}

.msg-content {
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary, #f1f5f9);
}

.approvals-section {
  border-top: 1px solid var(--border-color, #1e293b);
  padding-top: 8px;
}

.approvals-title {
  font-size: 12px;
  font-weight: 600;
  color: #f59e0b;
  margin-bottom: 6px;
}

.chat-input-bar {
  display: flex;
  gap: 4px;
  align-items: center;
  padding-top: 6px;
  border-top: 1px solid var(--border-color, #1e293b);
}

.role-select {
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 4px;
  padding: 4px 6px;
  color: #f1f5f9;
  font-size: 11px;
}

.chat-input {
  flex: 1;
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 4px;
  padding: 5px 8px;
  color: #f1f5f9;
  font-size: 12px;
}

.chat-input:focus {
  outline: none;
  border-color: var(--accent-color, #3b82f6);
}

/* 右栏：Agent 面板 + 计划面板 */
.agent-panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex-shrink: 0;
}

.agent-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  padding: 8px;
  flex-shrink: 0;
  max-height: 240px;
  overflow-y: auto;
}

.plan-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--border-color, #1e293b);
  min-height: 0;
}

.plan-body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

/* 弹窗 */
.popup {
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.popup h3 {
  margin: 0;
  font-size: 16px;
  color: var(--text-primary, #f1f5f9);
}

.hint {
  font-size: 12px;
  color: var(--text-tertiary, #64748b);
  margin: -8px 0 0 0;
}

.popup-input,
.popup-textarea,
.popup-select {
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 6px;
  padding: 8px 10px;
  color: #f1f5f9;
  font-size: 13px;
  font-family: inherit;
  width: 100%;
  box-sizing: border-box;
}

.popup-textarea {
  resize: vertical;
}

.popup-input:focus,
.popup-textarea:focus,
.popup-select:focus {
  outline: none;
  border-color: var(--accent-color, #3b82f6);
}

.popup-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.deps-selector {
  background: #0f172a;
  border-radius: 6px;
  padding: 8px;
}

.deps-label {
  font-size: 12px;
  color: var(--text-tertiary, #94a3b8);
  margin-bottom: 4px;
}

.deps-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.dep-chip {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11px;
  background: #1e293b;
  padding: 2px 6px;
  border-radius: 3px;
  cursor: pointer;
}

.approval-task {
  background: #0f172a;
  padding: 6px 10px;
  border-radius: 4px;
  font-size: 12px;
  color: var(--text-secondary, #cbd5e1);
}

/* 移动端 */
@media (max-width: 768px) {
  .main-grid {
    grid-template-columns: 1fr;
    grid-template-rows: auto auto auto;
  }
  .agent-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}
</style>
