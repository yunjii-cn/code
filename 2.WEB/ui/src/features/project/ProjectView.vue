<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import { useChatStore } from '@/stores/chat'
import { useDevice } from '@/composables/useDevice'
import { projectApi } from '@/api'
import * as platform from '@/platform'
import { showDialog, showConfirmDialog, showToast } from 'vant'

const { isDesktop } = useDevice()

const router = useRouter()
const projectStore = useProjectStore()
const chatStore = useChatStore()

const showCreateDialog = ref(false)
const newProjectName = ref('')
const newProjectPath = ref('')
const creating = ref(false)

const selectedProjectId = ref<string | null>(null)

const selectedProject = computed(() => {
  if (!selectedProjectId.value) return projectStore.activeProject
  return projectStore.projects.find((p) => p.id === selectedProjectId.value) || projectStore.activeProject
})

const conversations = computed(() => projectStore.conversations)

const conversationSearchKeyword = ref('')
const searchedConversations = ref<Array<{ id: string; title: string; projectId: string; createdAt: number; updatedAt: number }> | null>(null)
const isSearching = ref(false)

const displayedConversations = computed(() => {
  if (searchedConversations.value !== null) return searchedConversations.value
  return conversations.value
})

const showRenameDialog = ref(false)
const renameProjectId = ref('')
const renameProjectName = ref('')
const renaming = ref(false)

const showTemplatePicker = ref(false)
const templates = ref<Array<{ id: string; name: string; category: string; desc?: string }>>([])
const loadingTemplates = ref(false)
const applyingTemplate = ref(false)
const selectedTemplateId = ref<string | null>(null)

const showContextDialog = ref(false)
const projectContext = ref<Record<string, unknown> | null>(null)
const loadingContext = ref(false)

const showConvRenameDialog = ref(false)
const convRenameId = ref('')
const convRenameTitle = ref('')
const convRenaming = ref(false)

const showUpdatePathDialog = ref(false)
const updatePathProjectId = ref('')
const updatePathValue = ref('')
const updatingPath = ref(false)

function formatTime(ts: number) {
  const d = new Date(ts)
  const now = new Date()
  const isToday = d.toDateString() === now.toDateString()
  const yesterday = new Date(now)
  yesterday.setDate(yesterday.getDate() - 1)
  const isYesterday = d.toDateString() === yesterday.toDateString()
  const time = d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  if (isToday) return `今天 ${time}`
  if (isYesterday) return `昨天 ${time}`
  return d.toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' }) + ' ' + time
}

async function loadProjects() {
  await projectStore.loadProjects()
  if (projectStore.activeProject) {
    selectedProjectId.value = projectStore.activeProject.id
    await projectStore.loadConversations(projectStore.activeProject.id)
  }
}

// 2026-06-16 修复：自动选中第一个项目（避免空状态）
watch(
  () => projectStore.projects,
  (list) => {
    if (!selectedProjectId.value && list.length > 0) {
      const first = projectStore.activeProject || list[0]
      if (first) {
        selectedProjectId.value = first.id
        if (!projectStore.activeProject) {
          projectStore.switchProject(first.id)
        }
      }
    }
  },
  { immediate: true }
)

async function handleSelectProject(projectId: string) {
  selectedProjectId.value = projectId
  conversationSearchKeyword.value = ''
  searchedConversations.value = null
  await projectStore.switchProject(projectId)
}

async function handleCreateProject() {
  if (!newProjectName.value.trim() || !newProjectPath.value.trim()) return
  creating.value = true
  try {
    const project = await projectStore.createProject(newProjectName.value.trim(), newProjectPath.value.trim())
    showCreateDialog.value = false
    newProjectName.value = ''
    newProjectPath.value = ''
    await handleSelectProject(project.id)
  } catch (e: unknown) {
    showDialog({ message: `创建失败：${(e as Error).message}` })
  } finally {
    creating.value = false
  }
}

async function handleDeleteProject(projectId: string, projectName: string) {
  try {
    await showConfirmDialog({
      title: '删除项目',
      message: `确定要删除项目「${projectName}」吗？此操作不可恢复。`,
      confirmButtonText: '删除',
      confirmButtonColor: 'var(--error)',
      cancelButtonText: '取消',
    })
    await projectStore.deleteProject(projectId)
    if (selectedProjectId.value === projectId) {
      selectedProjectId.value = projectStore.activeProject?.id || null
      if (selectedProjectId.value) {
        await projectStore.loadConversations(selectedProjectId.value)
      }
    }
  } catch {
    // 用户取消
  }
}

async function handleConversationClick(conversationId: string) {
  chatStore.clearChat()
  router.push({ path: '/', query: { session: conversationId } })
}

async function handleNewConversation() {
  chatStore.clearChat()
  router.push('/')
}

async function handleRenameProject(projectId: string, currentName: string) {
  renameProjectId.value = projectId
  renameProjectName.value = currentName
  showRenameDialog.value = true
}

async function confirmRenameProject() {
  if (!renameProjectName.value.trim()) return
  renaming.value = true
  try {
    await projectApi.rename(renameProjectId.value, renameProjectName.value.trim())
    await projectStore.loadProjects()
    if (selectedProjectId.value === renameProjectId.value) {
      const updated = projectStore.projects.find((p) => p.id === renameProjectId.value)
      if (updated) {
        selectedProjectId.value = updated.id
      }
    }
    showRenameDialog.value = false
    showToast('重命名成功')
  } catch (e: unknown) {
    showDialog({ message: `重命名失败：${(e as Error).message}` })
  } finally {
    renaming.value = false
  }
}

async function handleOpenTemplatePicker() {
  showTemplatePicker.value = true
  loadingTemplates.value = true
  selectedTemplateId.value = null
  try {
    const data = await projectApi.listTemplates() as unknown as Array<{ id: string; name: string; category: string; desc?: string }>
    templates.value = data
  } catch (e: unknown) {
    showDialog({ message: `加载模板失败：${(e as Error).message}` })
  } finally {
    loadingTemplates.value = false
  }
}

async function handleCreateFromTemplate() {
  if (!selectedTemplateId.value || !selectedProject.value) return
  applyingTemplate.value = true
  try {
    await projectApi.createFromTemplate({
      project_id: selectedProject.value.id,
      template_id: selectedTemplateId.value,
    })
    showTemplatePicker.value = false
    showToast('模板应用成功')
    await projectStore.loadConversations(selectedProject.value.id)
  } catch (e: unknown) {
    showDialog({ message: `应用模板失败：${(e as Error).message}` })
  } finally {
    applyingTemplate.value = false
  }
}

async function handleShowContext(projectId: string) {
  showContextDialog.value = true
  loadingContext.value = true
  projectContext.value = null
  try {
    const data = await projectApi.getContext(projectId) as unknown as Record<string, unknown>
    projectContext.value = data
  } catch (e: unknown) {
    showDialog({ message: `获取上下文失败：${(e as Error).message}` })
  } finally {
    loadingContext.value = false
  }
}

async function handleSearchConversations() {
  const keyword = conversationSearchKeyword.value.trim()
  if (!keyword) {
    searchedConversations.value = null
    return
  }
  if (!selectedProject.value) return
  isSearching.value = true
  try {
    const results = await projectStore.searchConversations(keyword)
    searchedConversations.value = results
  } catch (e: unknown) {
    showDialog({ message: `搜索失败：${(e as Error).message}` })
  } finally {
    isSearching.value = false
  }
}

function clearSearch() {
  conversationSearchKeyword.value = ''
  searchedConversations.value = null
}

async function handleDeleteConversation(sessionId: string, title: string) {
  try {
    await showConfirmDialog({
      title: '删除会话',
      message: `确定要删除会话「${title}」吗？此操作不可恢复。`,
      confirmButtonText: '删除',
      confirmButtonColor: 'var(--error)',
      cancelButtonText: '取消',
    })
    await projectStore.deleteConversation(sessionId)
    if (searchedConversations.value) {
      searchedConversations.value = searchedConversations.value.filter((c) => c.id !== sessionId)
    }
  } catch {
    // 用户取消
  }
}

function handleRenameConversation(sessionId: string, currentTitle: string) {
  convRenameId.value = sessionId
  convRenameTitle.value = currentTitle
  showConvRenameDialog.value = true
}

async function confirmRenameConversation() {
  if (!convRenameTitle.value.trim()) return
  convRenaming.value = true
  try {
    await projectStore.renameConversation(convRenameId.value, convRenameTitle.value.trim())
    showConvRenameDialog.value = false
    showToast('会话重命名成功')
  } catch (e: unknown) {
    showDialog({ message: `重命名失败：${(e as Error).message}` })
  } finally {
    convRenaming.value = false
  }
}

function handleUpdatePath(projectId: string, currentPath: string) {
  updatePathProjectId.value = projectId
  updatePathValue.value = currentPath
  showUpdatePathDialog.value = true
}

async function confirmUpdatePath() {
  if (!updatePathValue.value.trim()) return
  updatingPath.value = true
  try {
    await projectApi.update(updatePathProjectId.value, { workspace_path: updatePathValue.value.trim() })
    await projectStore.loadProjects()
    showUpdatePathDialog.value = false
    showToast('路径更新成功')
  } catch (e: unknown) {
    showDialog({ message: `更新路径失败：${(e as Error).message}` })
  } finally {
    updatingPath.value = false
  }
}

function renderContextValue(value: unknown): string {
  if (value === null || value === undefined) return '-'
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  if (Array.isArray(value)) {
    if (value.length === 0) return '无'
    return value.map((v) => typeof v === 'string' ? v : JSON.stringify(v)).join('、')
  }
  if (typeof value === 'object') return JSON.stringify(value, null, 2)
  return String(value)
}

function contextLabel(key: string): string {
  const labels: Record<string, string> = {
    tech_stack: '技术栈',
    recent_files: '最近文件',
    project_type: '项目类型',
    language: '编程语言',
    framework: '框架',
    description: '描述',
    git_branch: 'Git 分支',
    file_count: '文件数量',
    last_modified: '最后修改',
    dependencies: '依赖',
    workspace_path: '工作区路径',
  }
  return labels[key] || key
}

onMounted(loadProjects)
</script>

<template>
  <div class="project-view" :class="{ 'project-view--desktop': isDesktop, 'project-view--mobile': !isDesktop }">
    <!-- 桌面端：左右分栏 -->
    <template v-if="isDesktop">
      <div class="project-sidebar">
        <div class="sidebar-header">
          <span class="sidebar-title">项目列表</span>
          <van-button size="small" type="primary" icon="plus" @click="showCreateDialog = true">新建项目</van-button>
        </div>
        <div class="project-list">
          <div
            v-for="project in projectStore.projects"
            :key="project.id"
            class="project-item"
            :class="{
              'project-item--active': project.id === selectedProjectId || (project.id === projectStore.activeProject?.id && !selectedProjectId),
            }"
            @click="handleSelectProject(project.id)"
          >
            <div class="project-item-dot" :class="{ 'project-item-dot--active': project.id === projectStore.activeProject?.id }"></div>
            <div class="project-item-info">
              <span class="project-item-name">{{ project.name }}</span>
              <span class="project-item-path">{{ project.path }}</span>
            </div>
            <div class="project-item-actions">
              <van-icon
                name="edit"
                class="project-item-action"
                title="重命名"
                @click.stop="handleRenameProject(project.id, project.name)"
              />
              <van-icon
                name="description"
                class="project-item-action"
                title="上下文"
                @click.stop="handleShowContext(project.id)"
              />
              <van-icon
                name="wap-nav"
                class="project-item-action"
                title="更新路径"
                @click.stop="handleUpdatePath(project.id, project.path)"
              />
              <van-icon
                v-if="project.id === projectStore.activeProject?.id"
                name="success"
                class="project-item-check"
              />
              <van-icon
                name="delete-o"
                class="project-item-action project-item-action--danger"
                title="删除"
                @click.stop="handleDeleteProject(project.id, project.name)"
              />
            </div>
          </div>
          <van-empty v-if="projectStore.projects.length === 0" description="暂无项目" />
        </div>
      </div>

      <div class="conversation-panel">
        <div class="panel-header">
          <span class="panel-title">{{ selectedProject?.name || '选择一个项目' }}</span>
          <div class="panel-header-actions">
            <van-button
              v-if="selectedProject"
              size="small"
              plain
              icon="description"
              @click="handleShowContext(selectedProject.id)"
            >上下文</van-button>
            <van-button
              v-if="selectedProject"
              size="small"
              plain
              icon="bars"
              @click="handleOpenTemplatePicker"
            >从模板创建</van-button>
            <van-button
              v-if="selectedProject"
              size="small"
              type="primary"
              icon="chat-o"
              @click="handleNewConversation"
            >新建会话</van-button>
          </div>
        </div>
        <div v-if="selectedProject" class="conversation-search">
          <van-search
            v-model="conversationSearchKeyword"
            placeholder="搜索会话"
            shape="round"
            :clearable="true"
            @search="handleSearchConversations"
            @clear="clearSearch"
          />
        </div>
        <div v-if="selectedProject" class="conversation-list">
          <div
            v-for="conv in displayedConversations"
            :key="conv.id"
            class="conversation-card"
            @click="handleConversationClick(conv.id)"
          >
            <div class="conversation-card-header">
              <van-icon name="chat-o" class="conversation-card-icon" />
              <span class="conversation-card-title">{{ conv.title || '未命名会话' }}</span>
            </div>
            <div class="conversation-card-right">
              <span class="conversation-card-time">{{ formatTime(conv.updatedAt) }}</span>
              <van-icon
                name="edit"
                class="conversation-card-action"
                title="重命名"
                @click.stop="handleRenameConversation(conv.id, conv.title || '未命名会话')"
              />
              <van-icon
                name="delete-o"
                class="conversation-card-action conversation-card-action--danger"
                title="删除"
                @click.stop="handleDeleteConversation(conv.id, conv.title || '未命名会话')"
              />
            </div>
          </div>
          <van-empty v-if="displayedConversations.length === 0" :description="isSearching ? '搜索中...' : (searchedConversations !== null ? '未找到匹配的会话' : '暂无会话')" />
        </div>
        <div v-else class="panel-empty">
          <van-icon name="folder-o" size="48" color="var(--text-muted)" />
          <p>请从左侧选择一个项目</p>
        </div>
      </div>
    </template>

    <!-- 手机端：卡片列表 -->
    <template v-else>
      <div class="mobile-header">
        <van-button type="primary" icon="plus" block @click="showCreateDialog = true">新建项目</van-button>
      </div>
      <div class="mobile-project-list">
        <div
          v-for="project in projectStore.projects"
          :key="project.id"
          class="mobile-project-card"
          :class="{ 'mobile-project-card--active': project.id === projectStore.activeProject?.id }"
          @click="handleSelectProject(project.id)"
        >
          <div class="mobile-project-card-top">
            <div class="mobile-project-card-left">
              <van-icon name="folder" class="mobile-project-icon" />
              <span class="mobile-project-name">{{ project.name }}</span>
              <van-tag v-if="project.id === projectStore.activeProject?.id" type="primary" size="medium">活跃</van-tag>
            </div>
            <div class="mobile-project-card-actions">
              <van-icon
                name="edit"
                class="mobile-project-action"
                @click.stop="handleRenameProject(project.id, project.name)"
              />
              <van-icon
                name="description"
                class="mobile-project-action"
                @click.stop="handleShowContext(project.id)"
              />
              <van-icon
                name="delete-o"
                class="mobile-project-action mobile-project-action--danger"
                @click.stop="handleDeleteProject(project.id, project.name)"
              />
            </div>
          </div>
          <div class="mobile-project-card-bottom">
            <span class="mobile-project-path">{{ project.path }}</span>
            <van-icon
              name="wap-nav"
              class="mobile-project-path-edit"
              title="更新路径"
              @click.stop="handleUpdatePath(project.id, project.path)"
            />
          </div>
        </div>
        <van-empty v-if="projectStore.projects.length === 0" description="暂无项目，点击上方按钮创建" />
      </div>

      <!-- 手机端会话列表 -->
      <div v-if="selectedProject" class="mobile-conversation-section">
        <div class="mobile-section-header">
          <span>会话列表</span>
          <div class="mobile-section-actions">
            <van-button size="small" plain icon="bars" @click="handleOpenTemplatePicker">模板</van-button>
            <van-button size="small" type="primary" icon="chat-o" @click="handleNewConversation">新建会话</van-button>
          </div>
        </div>
        <div class="mobile-conversation-search">
          <van-search
            v-model="conversationSearchKeyword"
            placeholder="搜索会话"
            shape="round"
            :clearable="true"
            @search="handleSearchConversations"
            @clear="clearSearch"
          />
        </div>
        <div class="mobile-conversation-list">
          <div
            v-for="conv in displayedConversations"
            :key="conv.id"
            class="mobile-conversation-card"
            @click="handleConversationClick(conv.id)"
          >
            <div class="mobile-conversation-top">
              <van-icon name="chat-o" class="mobile-conversation-icon" />
              <span class="mobile-conversation-title">{{ conv.title || '未命名会话' }}</span>
              <div class="mobile-conversation-actions">
                <van-icon
                  name="edit"
                  class="mobile-conversation-action"
                  @click.stop="handleRenameConversation(conv.id, conv.title || '未命名会话')"
                />
                <van-icon
                  name="delete-o"
                  class="mobile-conversation-action mobile-conversation-action--danger"
                  @click.stop="handleDeleteConversation(conv.id, conv.title || '未命名会话')"
                />
              </div>
            </div>
            <span class="mobile-conversation-time">{{ formatTime(conv.updatedAt) }}</span>
          </div>
          <van-empty v-if="displayedConversations.length === 0" :description="searchedConversations !== null ? '未找到匹配的会话' : '暂无会话'" />
        </div>
      </div>
    </template>

    <!-- 创建项目对话框 -->
    <van-dialog
      v-model:show="showCreateDialog"
      title="新建项目"
      show-cancel-button
      :before-close="(_action: string, done: () => void) => { done() }"
      @confirm="handleCreateProject"
      @cancel="newProjectName = ''; newProjectPath = ''"
    >
      <div class="create-form">
        <van-field
          v-model="newProjectName"
          label="项目名称"
          placeholder="请输入项目名称"
          :rules="[{ required: true, message: '请输入项目名称' }]"
        />
        <van-field
          v-model="newProjectPath"
          label="项目路径"
          placeholder="请输入项目路径"
          :rules="[{ required: true, message: '请输入项目路径' }]"
        />
      </div>
    </van-dialog>

    <!-- 重命名项目对话框 -->
    <van-dialog
      v-model:show="showRenameDialog"
      title="重命名项目"
      show-cancel-button
      :confirm-button-loading="renaming"
      :before-close="(_action: string, done: () => void) => { if (!renaming) done() }"
      @confirm="confirmRenameProject"
    >
      <div class="create-form">
        <van-field
          v-model="renameProjectName"
          label="项目名称"
          placeholder="请输入新名称"
        />
      </div>
    </van-dialog>

    <!-- 模板选择器对话框 -->
    <van-dialog
      v-model:show="showTemplatePicker"
      title="从模板创建"
      show-cancel-button
      :confirm-button-loading="applyingTemplate"
      :confirm-button-text="'应用模板'"
      :before-close="(_action: string, done: () => void) => { if (!applyingTemplate) done() }"
      @confirm="handleCreateFromTemplate"
    >
      <div class="template-picker">
        <div v-if="loadingTemplates" class="template-loading">
          <van-loading size="24px">加载模板中...</van-loading>
        </div>
        <div v-else-if="templates.length === 0" class="template-empty">
          <span>暂无可用模板</span>
        </div>
        <div v-else class="template-list">
          <div
            v-for="tpl in templates"
            :key="tpl.id"
            class="template-item"
            :class="{ 'template-item--selected': selectedTemplateId === tpl.id }"
            @click="selectedTemplateId = tpl.id"
          >
            <div class="template-item-radio">
              <van-icon v-if="selectedTemplateId === tpl.id" name="success" />
              <span v-else class="template-item-radio-dot"></span>
            </div>
            <div class="template-item-info">
              <span class="template-item-name">{{ tpl.name }}</span>
              <span v-if="tpl.desc" class="template-item-desc">{{ tpl.desc }}</span>
              <van-tag plain>{{ tpl.category }}</van-tag>
            </div>
          </div>
        </div>
      </div>
    </van-dialog>

    <!-- 项目上下文对话框 -->
    <van-dialog
      v-model:show="showContextDialog"
      title="项目上下文"
      :show-confirm-button="true"
      :show-cancel-button="false"
      confirm-button-text="关闭"
    >
      <div class="context-dialog">
        <div v-if="loadingContext" class="context-loading">
          <van-loading size="24px">加载中...</van-loading>
        </div>
        <div v-else-if="projectContext" class="context-content">
          <div
            v-for="(value, key) in projectContext"
            :key="key"
            class="context-item"
          >
            <span class="context-item-label">{{ contextLabel(key as string) }}</span>
            <span class="context-item-value">{{ renderContextValue(value) }}</span>
          </div>
          <div v-if="Object.keys(projectContext).length === 0" class="context-empty">
            <span>暂无上下文信息</span>
          </div>
        </div>
        <div v-else class="context-empty">
          <span>无法获取上下文</span>
        </div>
      </div>
    </van-dialog>

    <!-- 会话重命名对话框 -->
    <van-dialog
      v-model:show="showConvRenameDialog"
      title="重命名会话"
      show-cancel-button
      :confirm-button-loading="convRenaming"
      :before-close="(_action: string, done: () => void) => { if (!convRenaming) done() }"
      @confirm="confirmRenameConversation"
    >
      <div class="create-form">
        <van-field
          v-model="convRenameTitle"
          label="会话标题"
          placeholder="请输入新标题"
        />
      </div>
    </van-dialog>

    <!-- 更新项目路径对话框 -->
    <van-dialog
      v-model:show="showUpdatePathDialog"
      title="更新工作区路径"
      show-cancel-button
      :confirm-button-loading="updatingPath"
      :before-close="(_action: string, done: () => void) => { if (!updatingPath) done() }"
      @confirm="confirmUpdatePath"
    >
      <div class="create-form">
        <van-field
          v-model="updatePathValue"
          label="工作区路径"
          placeholder="请输入新的工作区路径"
        />
      </div>
    </van-dialog>
  </div>
</template>

<style scoped>
.project-view {
  width: 100%;
  height: 100%;
}

/* ===== 桌面端布局 ===== */
.project-view--desktop {
  display: flex;
  gap: 0;
}

.project-sidebar {
  width: 280px;
  min-width: 280px;
  height: 100%;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border);
  background-color: var(--bg-secondary);
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  border-bottom: 1px solid var(--border);
}

.sidebar-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.project-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.project-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: background-color 0.2s;
  position: relative;
}

.project-item:hover {
  background-color: var(--bg-card-hover);
}

.project-item--active {
  background-color: var(--bg-card);
}

.project-item-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--text-muted);
  flex-shrink: 0;
}

.project-item-dot--active {
  background-color: var(--success);
  box-shadow: 0 0 6px var(--success);
}

.project-item-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.project-item-name {
  font-size: 14px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.project-item-path {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.project-item-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.2s;
}

.project-item:hover .project-item-actions {
  opacity: 1;
}

.project-item-action {
  color: var(--text-muted);
  font-size: 15px;
  cursor: pointer;
  transition: color 0.2s;
}

.project-item-action:hover {
  color: var(--accent);
}

.project-item-action--danger:hover {
  color: var(--error);
}

.project-item-check {
  color: var(--success);
  font-size: 16px;
  flex-shrink: 0;
}

/* ===== 会话面板 ===== */
.conversation-panel {
  flex: 1;
  height: 100%;
  display: flex;
  flex-direction: column;
  background-color: var(--bg-primary);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
}

.panel-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.panel-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.conversation-search {
  padding: 8px 16px 0;
}

.conversation-search :deep(.van-search) {
  padding: 0;
}

.conversation-search :deep(.van-search__content) {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
}

.conversation-search :deep(.van-field__control) {
  color: var(--text-primary);
}

.conversation-list {
  flex: 1;
  overflow-y: auto;
  padding: 12px 20px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.conversation-card {
  padding: 14px 16px;
  border-radius: 10px;
  background-color: var(--bg-card);
  cursor: pointer;
  transition: background-color 0.2s;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.conversation-card:hover {
  background-color: var(--bg-card-hover);
}

.conversation-card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
}

.conversation-card-icon {
  color: var(--accent);
  font-size: 16px;
  flex-shrink: 0;
}

.conversation-card-title {
  font-size: 14px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.conversation-card-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.conversation-card-time {
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
}

.conversation-card-action {
  color: var(--text-muted);
  font-size: 15px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.2s, color 0.2s;
}

.conversation-card:hover .conversation-card-action {
  opacity: 1;
}

.conversation-card-action:hover {
  color: var(--accent);
}

.conversation-card-action--danger:hover {
  color: var(--error);
}

.panel-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--text-muted);
  font-size: 14px;
}

/* ===== 手机端布局 ===== */
.project-view--mobile {
  display: flex;
  flex-direction: column;
}

.mobile-header {
  padding: 12px 16px;
}

.mobile-project-list {
  padding: 0 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.mobile-project-card {
  padding: 14px 16px;
  border-radius: 10px;
  background-color: var(--bg-card);
  cursor: pointer;
  transition: background-color 0.2s;
  border: 1px solid transparent;
}

.mobile-project-card:active {
  background-color: var(--bg-card-hover);
}

.mobile-project-card--active {
  border-color: var(--accent);
}

.mobile-project-card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.mobile-project-card-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
}

.mobile-project-icon {
  color: var(--accent);
  font-size: 18px;
  flex-shrink: 0;
}

.mobile-project-name {
  font-size: 15px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mobile-project-card-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
  margin-left: 8px;
}

.mobile-project-action {
  color: var(--text-muted);
  font-size: 18px;
}

.mobile-project-action:active {
  color: var(--accent);
}

.mobile-project-action--danger:active {
  color: var(--error);
}

.mobile-project-card-bottom {
  margin-top: 6px;
  padding-left: 26px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.mobile-project-path {
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

.mobile-project-path-edit {
  color: var(--text-muted);
  font-size: 14px;
  flex-shrink: 0;
}

.mobile-project-path-edit:active {
  color: var(--accent);
}

/* ===== 手机端会话区域 ===== */
.mobile-conversation-section {
  margin-top: 20px;
  padding: 0 16px;
}

.mobile-section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.mobile-section-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mobile-conversation-search {
  margin-bottom: 8px;
}

.mobile-conversation-search :deep(.van-search) {
  padding: 0;
}

.mobile-conversation-search :deep(.van-search__content) {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
}

.mobile-conversation-search :deep(.van-field__control) {
  color: var(--text-primary);
}

.mobile-conversation-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.mobile-conversation-card {
  padding: 12px 14px;
  border-radius: 8px;
  background-color: var(--bg-card);
  cursor: pointer;
  transition: background-color 0.2s;
}

.mobile-conversation-card:active {
  background-color: var(--bg-card-hover);
}

.mobile-conversation-top {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mobile-conversation-icon {
  color: var(--accent);
  font-size: 15px;
  flex-shrink: 0;
}

.mobile-conversation-title {
  font-size: 14px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

.mobile-conversation-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.mobile-conversation-action {
  color: var(--text-muted);
  font-size: 16px;
}

.mobile-conversation-action:active {
  color: var(--accent);
}

.mobile-conversation-action--danger:active {
  color: var(--error);
}

.mobile-conversation-time {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 4px;
  padding-left: 23px;
}

/* ===== 创建表单 ===== */
.create-form {
  padding: 16px 16px 0;
}

.create-form :deep(.van-cell) {
  margin-bottom: 8px;
  border-radius: 8px;
}

.create-form :deep(.van-field__label) {
  color: var(--text-secondary);
}

.create-form :deep(.van-field__control) {
  color: var(--text-primary);
}

/* ===== 模板选择器 ===== */
.template-picker {
  padding: 16px;
  max-height: 400px;
  overflow-y: auto;
}

.template-loading,
.template-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px 0;
  color: var(--text-muted);
  font-size: 14px;
}

.template-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.template-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px;
  border-radius: 8px;
  background-color: var(--bg-card);
  cursor: pointer;
  transition: background-color 0.2s, border-color 0.2s;
  border: 1px solid transparent;
}

.template-item:hover {
  background-color: var(--bg-card-hover);
}

.template-item--selected {
  border-color: var(--accent);
  background-color: var(--bg-card-hover);
}

.template-item-radio {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-top: 2px;
  color: var(--accent);
  font-size: 18px;
}

.template-item-radio-dot {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 2px solid var(--border);
}

.template-item-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.template-item-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.template-item-desc {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.4;
}

/* ===== 上下文对话框 ===== */
.context-dialog {
  padding: 16px;
  max-height: 400px;
  overflow-y: auto;
}

.context-loading,
.context-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px 0;
  color: var(--text-muted);
  font-size: 14px;
}

.context-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.context-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  border-radius: 8px;
  background-color: var(--bg-card);
}

.context-item-label {
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 500;
}

.context-item-value {
  font-size: 14px;
  color: var(--text-primary);
  word-break: break-all;
  line-height: 1.5;
  white-space: pre-wrap;
}
</style>
