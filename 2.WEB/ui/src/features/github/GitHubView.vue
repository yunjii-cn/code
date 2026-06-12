<script setup lang="ts">
/**
 * GitHub 集成视图 — TASK-2.1
 * 三个标签页：Issues / PRs / 状态
 * 双栏布局（list + detail）
 * "Ask PR" 注入 Chat 上下文
 */
import { ref, computed, onMounted, watch } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { useRouter } from 'vue-router'
import { useDevice } from '@/composables/useDevice'
import { useProjectStore } from '@/stores/project'
import { githubApi } from '@/api'

const { isMobile } = useDevice()
const router = useRouter()
const projectStore = useProjectStore()

// ============ 通用状态 ============
const activeTab = ref<'issues' | 'prs' | 'status'>('issues')
const loading = ref(false)
const projectPath = computed(() => projectStore.activeProject?.path || '')

// ============ gh 环境状态 ============
interface GhStatus {
  installed: boolean
  authenticated: boolean
  version?: string
  user?: string
}
const ghStatus = ref<GhStatus | null>(null)
const repoInfo = ref<{ name: string; owner: string; defaultBranch: string; description: string; url: string } | null>(null)

// ============ Issues 状态 ============
const issueState = ref<'open' | 'closed' | 'all'>('open')
const issueAssignee = ref('')
const issueAuthor = ref('')
const issues = ref<Array<{
  number: number
  title: string
  state: string
  author: string
  assignees: string[]
  labels: string[]
  createdAt: string
  updatedAt: string
  url: string
  body: string
}>>([])
const selectedIssueNumber = ref<number | null>(null)
const issueDetail = ref<any>(null)
const issueDetailLoading = ref(false)

// ============ PRs 状态 ============
const prState = ref<'open' | 'closed' | 'merged' | 'draft' | 'all'>('open')
const prSearch = ref('')
const prs = ref<Array<{
  number: number
  title: string
  state: string
  author: string
  headRef: string
  baseRef: string
  isDraft: boolean
  createdAt: string
  updatedAt: string
  url: string
  additions: number
  deletions: number
  changedFiles: number
}>>([])
const selectedPrNumber = ref<number | null>(null)
const prDetail = ref<any>(null)
const prDetailLoading = ref(false)
const prDetailTab = ref<'overview' | 'files' | 'diff' | 'comments'>('overview')
const prFiles = ref<Array<{ path: string; additions: number; deletions: number; changeType: string }>>([])
const prFilesLoading = ref(false)
const prDiff = ref('')
const prDiffLoading = ref(false)
const prComments = ref<Array<any>>([])
const prCommentsLoading = ref(false)

const showAskPrDialog = ref(false)
const askPrPrompt = ref('')

const showAddCommentDialog = ref(false)
const newCommentBody = ref('')
const addCommentLoading = ref(false)

// ============ 工具函数 ============
function formatDate(iso: string): string {
  if (!iso) return ''
  try {
    const d = new Date(iso)
    return d.toLocaleString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
  } catch {
    return iso
  }
}

function stateColor(state: string): string {
  const s = (state || '').toLowerCase()
  if (s === 'open') return '#4CAF50'
  if (s === 'merged') return '#9C27B0'
  if (s === 'closed') return '#F44336'
  if (s === 'draft') return '#9E9E9E'
  return '#42A5F5'
}

function stateLabel(state: string): string {
  const s = (state || '').toLowerCase()
  if (s === 'open') return '打开'
  if (s === 'merged') return '已合并'
  if (s === 'closed') return '已关闭'
  if (s === 'draft') return '草稿'
  return state
}

// ============ 加载 gh 状态 ============
async function loadGhStatus() {
  try {
    const res: any = await githubApi.status()
    ghStatus.value = res?.data || null
  } catch (e: any) {
    ghStatus.value = { installed: false, authenticated: false }
    showToast(`gh 状态查询失败: ${e.message}`)
  }
}

async function loadRepoInfo() {
  if (!ghStatus.value?.authenticated) {
    repoInfo.value = null
    return
  }
  try {
    const res: any = await githubApi.repo(projectPath.value)
    if (res?.ok) {
      repoInfo.value = res.data
    } else {
      repoInfo.value = null
    }
  } catch {
    repoInfo.value = null
  }
}

// ============ Issues 操作 ============
async function loadIssues() {
  if (!ghStatus.value?.authenticated) {
    showToast('请先登录 gh CLI')
    return
  }
  loading.value = true
  try {
    const res: any = await githubApi.listIssues({
      project_path: projectPath.value,
      state: issueState.value,
      assignee: issueAssignee.value,
      author: issueAuthor.value,
      limit: 30,
    })
    if (res?.ok) {
      issues.value = res.data || []
      if (issues.value.length === 0) {
        showToast('该状态下没有 Issue')
      }
    } else {
      showToast(res?.error || '加载 Issues 失败')
    }
  } catch (e: any) {
    showToast(`加载失败: ${e.message}`)
  } finally {
    loading.value = false
  }
}

async function loadIssueDetail(num: number) {
  selectedIssueNumber.value = num
  issueDetailLoading.value = true
  issueDetail.value = null
  try {
    const res: any = await githubApi.getIssue(num, projectPath.value)
    if (res?.ok) {
      issueDetail.value = res.data
    } else {
      showToast(res?.error || '加载 Issue 详情失败')
    }
  } catch (e: any) {
    showToast(`加载失败: ${e.message}`)
  } finally {
    issueDetailLoading.value = false
  }
}

async function openIssueInBrowser(num: number) {
  try {
    const res: any = await githubApi.open(String(num), projectPath.value)
    if (res?.ok) showToast('已在浏览器中打开')
    else showToast(res?.error || '打开失败')
  } catch (e: any) {
    showToast(`打开失败: ${e.message}`)
  }
}

// ============ PRs 操作 ============
async function loadPrs() {
  if (!ghStatus.value?.authenticated) {
    showToast('请先登录 gh CLI')
    return
  }
  loading.value = true
  try {
    const res: any = await githubApi.listPrs({
      project_path: projectPath.value,
      state: prState.value,
      search: prSearch.value,
      limit: 30,
    })
    if (res?.ok) {
      prs.value = res.data || []
      if (prs.value.length === 0) {
        showToast('该状态下没有 PR')
      }
    } else {
      showToast(res?.error || '加载 PRs 失败')
    }
  } catch (e: any) {
    showToast(`加载失败: ${e.message}`)
  } finally {
    loading.value = false
  }
}

async function loadPrDetail(num: number) {
  selectedPrNumber.value = num
  prDetailLoading.value = true
  prDetail.value = null
  prFiles.value = []
  prDiff.value = ''
  prComments.value = []
  prDetailTab.value = 'overview'
  try {
    const res: any = await githubApi.getPr(num, projectPath.value)
    if (res?.ok) {
      prDetail.value = res.data
    } else {
      showToast(res?.error || '加载 PR 详情失败')
    }
  } catch (e: any) {
    showToast(`加载失败: ${e.message}`)
  } finally {
    prDetailLoading.value = false
  }
}

async function loadPrFiles(num: number) {
  prFilesLoading.value = true
  try {
    const res: any = await githubApi.getPrFiles(num, projectPath.value)
    if (res?.ok) {
      prFiles.value = res.data || []
    } else {
      showToast(res?.error || '加载文件清单失败')
    }
  } catch (e: any) {
    showToast(`加载失败: ${e.message}`)
  } finally {
    prFilesLoading.value = false
  }
}

async function loadPrDiff(num: number) {
  prDiffLoading.value = true
  try {
    const res: any = await githubApi.getPrDiff(num, projectPath.value)
    if (res?.ok) {
      prDiff.value = res.data || ''
    } else {
      showToast(res?.error || '加载 diff 失败')
    }
  } catch (e: any) {
    showToast(`加载失败: ${e.message}`)
  } finally {
    prDiffLoading.value = false
  }
}

async function loadPrComments(num: number) {
  prCommentsLoading.value = true
  try {
    const res: any = await githubApi.listPrComments(num, projectPath.value)
    if (res?.ok) {
      prComments.value = res.data || []
    } else {
      showToast(res?.error || '加载评论失败')
    }
  } catch (e: any) {
    showToast(`加载失败: ${e.message}`)
  } finally {
    prCommentsLoading.value = false
  }
}

async function openPrInBrowser(num: number) {
  try {
    const res: any = await githubApi.open(String(num), projectPath.value)
    if (res?.ok) showToast('已在浏览器中打开')
    else showToast(res?.error || '打开失败')
  } catch (e: any) {
    showToast(`打开失败: ${e.message}`)
  }
}

// ============ Ask PR 注入 Chat ============
function buildAskPrPrompt(pr: any): string {
  if (!pr) return ''
  const lines: string[] = []
  lines.push(`请阅读下面这个 PR 并帮我分析：`)
  lines.push('')
  lines.push(`# PR #${pr.number}: ${pr.title}`)
  lines.push('')
  lines.push(`**仓库**: ${repoInfo.value?.name || ''}`)
  lines.push(`**作者**: @${pr.author || ''}`)
  lines.push(`**状态**: ${pr.state}${pr.isDraft ? ' (Draft)' : ''}`)
  lines.push(`**分支**: ${pr.headRef} → ${pr.baseRef}`)
  lines.push(`**变更**: +${pr.additions || 0} / -${pr.deletions || 0} (${pr.changedFiles || 0} 文件)`)
  if (pr.url) lines.push(`**链接**: ${pr.url}`)
  lines.push('')
  if (pr.body) {
    lines.push(`## 描述`)
    lines.push('')
    lines.push(pr.body)
    lines.push('')
  }
  if (pr.labels && pr.labels.length > 0) {
    lines.push(`**标签**: ${pr.labels.map((l: string) => '`' + l + '`').join(', ')}`)
    lines.push('')
  }
  lines.push('请帮我：')
  lines.push('1. 总结这个 PR 改动的核心目的')
  lines.push('2. 指出潜在的问题（bug / 安全 / 性能）')
  lines.push('3. 给出改进建议')
  return lines.join('\n')
}

function openAskPrDialog() {
  if (!prDetail.value) {
    showToast('请先选择 PR')
    return
  }
  askPrPrompt.value = buildAskPrPrompt(prDetail.value)
  showAskPrDialog.value = true
}

async function copyAskPrPrompt() {
  if (!askPrPrompt.value) return
  try {
    await navigator.clipboard.writeText(askPrPrompt.value)
    showToast('已复制到剪贴板')
  } catch {
    showToast('复制失败，请手动复制')
  }
}

async function injectAskPrToChat() {
  if (!askPrPrompt.value) return
  // v1.0：跳转到 / 主页，sessionStorage 存 prompt。
  // v1.1 增强：ChatView 自动检测并填入 inputText。
  sessionStorage.setItem('yj_ask_pr_prompt', askPrPrompt.value)
  showToast('已跳转到 Chat，请在 Composer 粘贴 (Ctrl+V)')
  showAskPrDialog.value = false
  router.push('/')
}

// ============ 评论 ============
async function submitNewComment() {
  if (!selectedPrNumber.value) return
  if (!newCommentBody.value.trim()) {
    showToast('评论内容不能为空')
    return
  }
  addCommentLoading.value = true
  try {
    const res: any = await githubApi.createPrComment(
      selectedPrNumber.value,
      newCommentBody.value,
      projectPath.value,
    )
    if (res?.ok) {
      showToast('评论已发布')
      newCommentBody.value = ''
      showAddCommentDialog.value = false
      await loadPrComments(selectedPrNumber.value)
    } else {
      showToast(res?.error || '发布失败')
    }
  } catch (e: any) {
    showToast(`发布失败: ${e.message}`)
  } finally {
    addCommentLoading.value = false
  }
}

// ============ Diff 解析 ============
function parseDiffLines(diff: string): Array<{ type: 'add' | 'remove' | 'header' | 'context'; text: string }> {
  if (!diff) return []
  return diff.split('\n').map(line => {
    if (line.startsWith('+++') || line.startsWith('---') || line.startsWith('@@') || line.startsWith('diff --git') || line.startsWith('index ')) {
      return { type: 'header', text: line }
    }
    if (line.startsWith('+')) {
      return { type: 'add', text: line }
    }
    if (line.startsWith('-')) {
      return { type: 'remove', text: line }
    }
    return { type: 'context', text: line }
  })
}

// ============ 监听器 ============
watch(prDetailTab, (newVal) => {
  if (!selectedPrNumber.value) return
  if (newVal === 'files' && prFiles.value.length === 0) {
    loadPrFiles(selectedPrNumber.value)
  } else if (newVal === 'diff' && !prDiff.value) {
    loadPrDiff(selectedPrNumber.value)
  } else if (newVal === 'comments' && prComments.value.length === 0) {
    loadPrComments(selectedPrNumber.value)
  }
})

watch(activeTab, (newVal) => {
  if (newVal === 'issues' && issues.value.length === 0 && ghStatus.value?.authenticated) {
    loadIssues()
  } else if (newVal === 'prs' && prs.value.length === 0 && ghStatus.value?.authenticated) {
    loadPrs()
  }
})

// ============ 初始化 ============
onMounted(async () => {
  await loadGhStatus()
  await loadRepoInfo()
  if (ghStatus.value?.authenticated) {
    await loadIssues()
  }
})
</script>

<template>
  <div class="github-view" :class="{ mobile: isMobile }">
    <div class="page-header">
      <h2 class="page-title">GitHub 集成</h2>
      <div class="header-actions">
        <van-button size="small" plain @click="loadGhStatus">检查 gh</van-button>
        <van-button size="small" plain @click="loadRepoInfo">仓库信息</van-button>
      </div>
    </div>

    <div class="gh-status-bar">
      <span v-if="!ghStatus" class="status-loading">检查 gh 中...</span>
      <template v-else>
        <span class="status-item" :class="ghStatus.installed ? 'ok' : 'err'">
          gh {{ ghStatus.installed ? '✓ ' + ghStatus.version : '✗ 未安装' }}
        </span>
        <span class="status-item" :class="ghStatus.authenticated ? 'ok' : 'err'">
          {{ ghStatus.authenticated ? '✓ 已登录' : '✗ 未登录' }}
        </span>
        <span v-if="ghStatus.user" class="status-item user">@{{ ghStatus.user }}</span>
        <span v-if="repoInfo" class="status-item repo">{{ repoInfo.name }}</span>
        <span v-else-if="ghStatus.authenticated" class="status-item muted">非 GitHub 仓库或无远程</span>
      </template>
    </div>

    <div v-if="ghStatus && !ghStatus.installed" class="install-guide">
      <p>未检测到 GitHub CLI（<code>gh</code>），无法使用此功能。</p>
      <p class="install-cmd">
        <strong>Windows (winget):</strong> <code>winget install --id GitHub.cli</code><br>
        <strong>macOS:</strong> <code>brew install gh</code><br>
        <strong>登录:</strong> <code>gh auth login</code>
      </p>
    </div>

    <div v-else-if="ghStatus && !ghStatus.authenticated" class="install-guide">
      <p>gh CLI 已安装但未登录。</p>
      <p class="install-cmd">在终端执行 <code>gh auth login</code> 完成 OAuth 登录后刷新此页。</p>
    </div>

    <div v-else-if="ghStatus && ghStatus.authenticated && !repoInfo" class="install-guide">
      <p>当前目录不是 GitHub 仓库，或没有 GitHub 远程。</p>
      <p v-if="projectPath" class="install-cmd">项目路径: <code>{{ projectPath }}</code></p>
      <p v-else class="install-cmd">请先在「项目管理」选择一个 GitHub 仓库作为当前项目。</p>
    </div>

    <template v-else>
      <van-tabs v-model:active="activeTab" sticky>
        <!-- ============ Issues 标签 ============ -->
        <van-tab title="Issues" name="issues">
          <div class="tab-content">
            <div class="filters">
              <van-dropdown-menu>
                <van-dropdown-item v-model="issueState" :options="[
                  { text: '打开', value: 'open' },
                  { text: '已关闭', value: 'closed' },
                  { text: '全部', value: 'all' },
                ]" @change="loadIssues" />
              </van-dropdown-menu>
              <van-button size="small" plain @click="loadIssues" :loading="loading">刷新</van-button>
            </div>

            <div class="split-view">
              <div class="list-pane">
                <van-loading v-if="loading" color="var(--accent)" vertical>加载中...</van-loading>
                <div v-else-if="issues.length === 0" class="empty-hint">暂无 Issue</div>
                <div v-else class="item-list">
                  <div
                    v-for="it in issues"
                    :key="it.number"
                    class="item-card"
                    :class="{ active: selectedIssueNumber === it.number }"
                    @click="loadIssueDetail(it.number)"
                  >
                    <div class="item-head">
                      <span class="item-number">#{{ it.number }}</span>
                      <span class="item-state" :style="{ color: stateColor(it.state) }">●</span>
                      <span class="item-title">{{ it.title }}</span>
                    </div>
                    <div class="item-meta">
                      <span v-if="it.author">@{{ it.author }}</span>
                      <span v-if="it.labels.length > 0" class="item-labels">
                        <span v-for="lbl in it.labels.slice(0, 3)" :key="lbl" class="label-chip">{{ lbl }}</span>
                      </span>
                      <span class="item-date">{{ formatDate(it.updatedAt) }}</span>
                    </div>
                  </div>
                </div>
              </div>

              <div class="detail-pane">
                <div v-if="!selectedIssueNumber" class="empty-hint">← 选择一个 Issue 查看详情</div>
                <van-loading v-else-if="issueDetailLoading" color="var(--accent)" vertical>加载详情...</van-loading>
                <div v-else-if="issueDetail" class="detail-content">
                  <div class="detail-header">
                    <span class="detail-number">#{{ issueDetail.number }}</span>
                    <span class="detail-state" :style="{ color: stateColor(issueDetail.state) }">●</span>
                    <h3 class="detail-title">{{ issueDetail.title }}</h3>
                  </div>
                  <div class="detail-meta">
                    <span>作者: @{{ issueDetail.author }}</span>
                    <span>状态: {{ stateLabel(issueDetail.state) }}</span>
                    <span>创建: {{ formatDate(issueDetail.createdAt) }}</span>
                  </div>
                  <div v-if="issueDetail.labels.length > 0" class="detail-labels">
                    <span v-for="lbl in issueDetail.labels" :key="lbl" class="label-chip">{{ lbl }}</span>
                  </div>
                  <div v-if="issueDetail.assignees.length > 0" class="detail-meta">
                    <span>指派给: {{ issueDetail.assignees.map((a: string) => '@' + a).join(', ') }}</span>
                  </div>
                  <div class="detail-body">{{ issueDetail.body || '(无描述)' }}</div>
                  <div v-if="issueDetail.comments.length > 0" class="comments-section">
                    <div class="section-title">评论 ({{ issueDetail.comments.length }})</div>
                    <div v-for="c in issueDetail.comments" :key="c.id" class="comment-card">
                      <div class="comment-head">
                        <span class="comment-author">@{{ c.author }}</span>
                        <span class="comment-date">{{ formatDate(c.createdAt) }}</span>
                      </div>
                      <div class="comment-body">{{ c.body }}</div>
                    </div>
                  </div>
                  <div class="detail-actions">
                    <van-button size="small" plain @click="openIssueInBrowser(issueDetail.number)">在浏览器中打开</van-button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </van-tab>

        <!-- ============ PRs 标签 ============ -->
        <van-tab title="Pull Requests" name="prs">
          <div class="tab-content">
            <div class="filters">
              <van-dropdown-menu>
                <van-dropdown-item v-model="prState" :options="[
                  { text: '打开', value: 'open' },
                  { text: '已合并', value: 'merged' },
                  { text: '已关闭', value: 'closed' },
                  { text: '草稿', value: 'draft' },
                  { text: '全部', value: 'all' },
                ]" @change="loadPrs" />
              </van-dropdown-menu>
              <van-field
                v-model="prSearch"
                placeholder="搜索 PR..."
                class="dark-field search-field"
                @keydown.enter="loadPrs"
              >
                <template #button>
                  <van-button size="mini" plain @click="loadPrs">搜索</van-button>
                </template>
              </van-field>
              <van-button size="small" plain @click="loadPrs" :loading="loading">刷新</van-button>
            </div>

            <div class="split-view">
              <div class="list-pane">
                <van-loading v-if="loading" color="var(--accent)" vertical>加载中...</van-loading>
                <div v-else-if="prs.length === 0" class="empty-hint">暂无 PR</div>
                <div v-else class="item-list">
                  <div
                    v-for="it in prs"
                    :key="it.number"
                    class="item-card"
                    :class="{ active: selectedPrNumber === it.number }"
                    @click="loadPrDetail(it.number)"
                  >
                    <div class="item-head">
                      <span class="item-number">#{{ it.number }}</span>
                      <span class="item-state" :style="{ color: stateColor(it.state) }">●</span>
                      <span class="item-title">{{ it.title }}</span>
                    </div>
                    <div class="item-meta">
                      <span>@{{ it.author }}</span>
                      <span class="pr-branches">{{ it.headRef }} → {{ it.baseRef }}</span>
                      <span class="pr-stats">
                        <span class="add">+{{ it.additions }}</span>
                        <span class="del">-{{ it.deletions }}</span>
                        <span class="files">{{ it.changedFiles }} files</span>
                      </span>
                    </div>
                  </div>
                </div>
              </div>

              <div class="detail-pane">
                <div v-if="!selectedPrNumber" class="empty-hint">← 选择一个 PR 查看详情</div>
                <van-loading v-else-if="prDetailLoading" color="var(--accent)" vertical>加载详情...</van-loading>
                <div v-else-if="prDetail" class="detail-content">
                  <div class="detail-header">
                    <span class="detail-number">#{{ prDetail.number }}</span>
                    <span class="item-state" :style="{ color: stateColor(prDetail.state) }">●</span>
                    <h3 class="detail-title">{{ prDetail.title }}</h3>
                  </div>
                  <div class="detail-meta">
                    <span>作者: @{{ prDetail.author }}</span>
                    <span>状态: {{ stateLabel(prDetail.state) }}{{ prDetail.isDraft ? ' (Draft)' : '' }}</span>
                    <span>{{ prDetail.headRef }} → {{ prDetail.baseRef }}</span>
                    <span>+{{ prDetail.additions }} / -{{ prDetail.deletions }} ({{ prDetail.changedFiles }} files)</span>
                  </div>
                  <div v-if="prDetail.labels.length > 0" class="detail-labels">
                    <span v-for="lbl in prDetail.labels" :key="lbl" class="label-chip">{{ lbl }}</span>
                  </div>
                  <div class="detail-tabs">
                    <van-button size="mini" :type="prDetailTab === 'overview' ? 'primary' : 'default'" @click="prDetailTab = 'overview'">概览</van-button>
                    <van-button size="mini" :type="prDetailTab === 'files' ? 'primary' : 'default'" @click="prDetailTab = 'files'">文件</van-button>
                    <van-button size="mini" :type="prDetailTab === 'diff' ? 'primary' : 'default'" @click="prDetailTab = 'diff'">Diff</van-button>
                    <van-button size="mini" :type="prDetailTab === 'comments' ? 'primary' : 'default'" @click="prDetailTab = 'comments'">评论</van-button>
                  </div>

                  <div v-if="prDetailTab === 'overview'" class="tab-pane">
                    <pre class="body-pre">{{ prDetail.body || '(无描述)' }}</pre>
                  </div>
                  <div v-else-if="prDetailTab === 'files'" class="tab-pane">
                    <van-loading v-if="prFilesLoading" color="var(--accent)" vertical>加载中...</van-loading>
                    <div v-else-if="prFiles.length === 0" class="empty-hint">无变更文件</div>
                    <div v-else class="file-list">
                      <div v-for="f in prFiles" :key="f.path" class="file-item">
                        <span class="file-path" :title="f.path">{{ f.path }}</span>
                        <span class="file-stats">
                          <span class="add">+{{ f.additions }}</span>
                          <span class="del">-{{ f.deletions }}</span>
                        </span>
                      </div>
                    </div>
                  </div>
                  <div v-else-if="prDetailTab === 'diff'" class="tab-pane">
                    <van-loading v-if="prDiffLoading" color="var(--accent)" vertical>加载中...</van-loading>
                    <div v-else-if="!prDiff" class="empty-hint">无 diff</div>
                    <div v-else class="diff-lines">
                      <div
                        v-for="(line, i) in parseDiffLines(prDiff)"
                        :key="i"
                        class="diff-line"
                        :class="line.type"
                      >
                        <pre>{{ line.text }}</pre>
                      </div>
                    </div>
                  </div>
                  <div v-else-if="prDetailTab === 'comments'" class="tab-pane">
                    <van-loading v-if="prCommentsLoading" color="var(--accent)" vertical>加载中...</van-loading>
                    <template v-else>
                      <van-button size="small" type="primary" plain @click="showAddCommentDialog = true" class="add-comment-btn">添加评论</van-button>
                      <div v-if="prComments.length === 0" class="empty-hint">暂无评论</div>
                      <div v-else class="comments-list">
                        <div v-for="c in prComments" :key="c.id + '-' + c.kind" class="comment-card">
                          <div class="comment-head">
                            <span class="comment-author">@{{ c.author }}</span>
                            <span class="comment-kind">{{ c.kind === 'review' ? 'Review' : 'Comment' }}<span v-if="c.state"> ({{ c.state }})</span></span>
                            <span class="comment-date">{{ formatDate(c.createdAt) }}</span>
                          </div>
                          <div v-if="c.body" class="comment-body">{{ c.body }}</div>
                        </div>
                      </div>
                    </template>
                  </div>

                  <div class="detail-actions">
                    <van-button size="small" type="primary" @click="openAskPrDialog">Ask PR</van-button>
                    <van-button size="small" plain @click="openPrInBrowser(prDetail.number)">在浏览器中打开</van-button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </van-tab>

        <!-- ============ 状态标签 ============ -->
        <van-tab title="状态" name="status">
          <div class="tab-content status-tab">
            <div class="status-card">
              <div class="status-card-title">gh CLI 状态</div>
              <div v-if="ghStatus" class="status-grid">
                <div class="status-row">
                  <span class="status-key">安装:</span>
                  <span class="status-val" :class="ghStatus.installed ? 'ok' : 'err'">
                    {{ ghStatus.installed ? '是' : '否' }}
                  </span>
                </div>
                <div class="status-row">
                  <span class="status-key">版本:</span>
                  <span class="status-val">{{ ghStatus.version || '-' }}</span>
                </div>
                <div class="status-row">
                  <span class="status-key">认证:</span>
                  <span class="status-val" :class="ghStatus.authenticated ? 'ok' : 'err'">
                    {{ ghStatus.authenticated ? '已登录' : '未登录' }}
                  </span>
                </div>
                <div class="status-row">
                  <span class="status-key">用户:</span>
                  <span class="status-val">{{ ghStatus.user ? '@' + ghStatus.user : '-' }}</span>
                </div>
              </div>
            </div>

            <div v-if="repoInfo" class="status-card">
              <div class="status-card-title">当前仓库</div>
              <div class="status-grid">
                <div class="status-row">
                  <span class="status-key">名称:</span>
                  <span class="status-val">{{ repoInfo.name }}</span>
                </div>
                <div class="status-row">
                  <span class="status-key">默认分支:</span>
                  <span class="status-val">{{ repoInfo.defaultBranch }}</span>
                </div>
                <div class="status-row">
                  <span class="status-key">描述:</span>
                  <span class="status-val">{{ repoInfo.description || '(无)' }}</span>
                </div>
                <div class="status-row">
                  <span class="status-key">链接:</span>
                  <a v-if="repoInfo.url" :href="repoInfo.url" target="_blank" class="status-val link">{{ repoInfo.url }}</a>
                  <span v-else class="status-val">-</span>
                </div>
              </div>
            </div>

            <div v-if="projectPath" class="status-card">
              <div class="status-card-title">项目上下文</div>
              <div class="status-row">
                <span class="status-key">当前项目:</span>
                <span class="status-val path">{{ projectPath }}</span>
              </div>
            </div>
          </div>
        </van-tab>
      </van-tabs>
    </template>

    <!-- ============ Ask PR 对话框 ============ -->
    <van-dialog
      v-model:show="showAskPrDialog"
      title="Ask PR — 注入 Chat 上下文"
      :style="{ width: '90vw', maxWidth: '720px' }"
      close-on-click-overlay
      show-cancel-button
      :before-close="(action: string) => { if (action === 'confirm') { copyAskPrPrompt() } return true }"
    >
      <div class="ask-pr-dialog">
        <p class="dialog-hint">下方为注入 Chat 的 PR 上下文。点击「确认」复制到剪贴板，然后到 Chat 粘贴 (Ctrl+V)。</p>
        <textarea v-model="askPrPrompt" class="ask-pr-textarea" readonly rows="18" />
      </div>
      <template #footer>
        <div class="dialog-footer">
          <van-button size="small" plain @click="copyAskPrPrompt">复制</van-button>
          <van-button size="small" type="primary" @click="injectAskPrToChat">跳转到 Chat</van-button>
        </div>
      </template>
    </van-dialog>

    <!-- ============ 添加评论对话框 ============ -->
    <van-dialog
      v-model:show="showAddCommentDialog"
      title="添加评论"
      :style="{ width: '90vw', maxWidth: '500px' }"
      show-cancel-button
    >
      <div class="ask-pr-dialog">
        <van-field
          v-model="newCommentBody"
          type="textarea"
          rows="6"
          autosize
          placeholder="输入评论内容..."
          class="dark-field"
        />
      </div>
      <template #footer>
        <van-button size="small" type="primary" :loading="addCommentLoading" @click="submitNewComment">发布</van-button>
      </template>
    </van-dialog>
  </div>
</template>

<style scoped>
.github-view {
  height: 100%;
  overflow-y: auto;
  padding: 24px;
  background: var(--bg-primary);
  color: var(--text-primary);
}
.github-view.mobile {
  padding: 12px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}
.header-actions {
  display: flex;
  gap: 8px;
}

.gh-status-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  padding: 10px 14px;
  background: var(--bg-card);
  border-radius: 8px;
  font-size: 13px;
  margin-bottom: 16px;
}
.status-item.ok { color: #4CAF50; }
.status-item.err { color: #F44336; }
.status-item.user { color: var(--accent); }
.status-item.repo { color: var(--text-primary); font-weight: 600; }
.status-item.muted { color: var(--text-muted); }
.status-loading { color: var(--text-muted); }

.install-guide {
  padding: 24px;
  background: var(--bg-card);
  border-radius: 8px;
  text-align: left;
}
.install-guide p { margin: 8px 0; }
.install-guide code {
  background: rgba(255,255,255,0.08);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: 'Consolas', monospace;
  font-size: 12px;
}
.install-cmd { line-height: 1.8; }

.tab-content {
  padding-top: 12px;
}

.filters {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
  align-items: center;
}
.search-field {
  flex: 1;
  background: var(--bg-card);
  border-radius: 8px;
  padding: 4px 10px;
}
:deep(.dark-field .van-field__control) {
  color: var(--text-primary);
}
:deep(.van-dropdown-menu__bar) {
  background: var(--bg-card);
}

.split-view {
  display: grid;
  grid-template-columns: minmax(280px, 1fr) minmax(320px, 1.4fr);
  gap: 12px;
  height: calc(100vh - 280px);
  min-height: 400px;
}
.mobile .split-view {
  grid-template-columns: 1fr;
  height: auto;
}

.list-pane,
.detail-pane {
  background: var(--bg-card);
  border-radius: 8px;
  padding: 8px;
  overflow-y: auto;
}

.item-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.item-card {
  padding: 10px 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s;
  border-left: 3px solid transparent;
}
.item-card:hover {
  background: var(--bg-card-hover);
}
.item-card.active {
  border-left-color: var(--accent);
  background: rgba(66, 165, 245, 0.08);
}
.item-head {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}
.item-number {
  font-family: 'Consolas', monospace;
  font-size: 11px;
  color: var(--accent);
  font-weight: 600;
}
.item-state {
  font-size: 12px;
}
.item-title {
  font-size: 13px;
  color: var(--text-primary);
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}
.item-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  font-size: 11px;
  color: var(--text-muted);
  align-items: center;
}
.item-labels { display: flex; gap: 4px; flex-wrap: wrap; }
.label-chip {
  background: rgba(66, 165, 245, 0.15);
  color: var(--accent);
  padding: 1px 6px;
  border-radius: 8px;
  font-size: 10px;
}
.item-date { margin-left: auto; }
.pr-branches {
  font-family: 'Consolas', monospace;
  font-size: 10px;
}
.pr-stats { display: flex; gap: 4px; font-family: 'Consolas', monospace; }
.pr-stats .add { color: #4CAF50; }
.pr-stats .del { color: #F44336; }
.pr-stats .files { color: var(--text-muted); }

.detail-pane {
  display: flex;
  flex-direction: column;
}
.empty-hint {
  text-align: center;
  padding: 30px 16px;
  color: var(--text-muted);
  font-size: 13px;
}
.detail-content {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: 1;
  overflow-y: auto;
  padding: 4px;
}
.detail-header {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.detail-number {
  font-family: 'Consolas', monospace;
  font-size: 12px;
  color: var(--accent);
  font-weight: 600;
}
.detail-state { font-size: 12px; }
.detail-title {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
  flex: 1;
  min-width: 0;
}
.detail-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 12px;
  color: var(--text-secondary);
}
.detail-labels {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}
.detail-body,
.body-pre {
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--bg-secondary);
  padding: 10px 12px;
  border-radius: 6px;
  margin: 0;
  font-family: inherit;
}
.body-pre {
  font-family: 'Consolas', monospace;
  font-size: 12px;
  max-height: 400px;
  overflow-y: auto;
}

.detail-tabs {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  padding: 6px 0;
}
.tab-pane {
  flex: 1;
  overflow-y: auto;
  min-height: 100px;
}
.file-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.file-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 10px;
  background: var(--bg-secondary);
  border-radius: 4px;
  font-size: 12px;
}
.file-path {
  font-family: 'Consolas', monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}
.file-stats { font-family: 'Consolas', monospace; display: flex; gap: 6px; }
.file-stats .add { color: #4CAF50; }
.file-stats .del { color: #F44336; }

.diff-lines {
  font-family: 'Consolas', monospace;
  font-size: 12px;
  line-height: 1.5;
}
.diff-line {
  margin-bottom: 1px;
  white-space: pre-wrap;
  word-break: break-all;
}
.diff-line pre {
  margin: 0;
  font-family: inherit;
  font-size: inherit;
  padding: 0 8px;
}
.diff-line.add { background: rgba(76, 175, 80, 0.15); color: #81C784; }
.diff-line.remove { background: rgba(244, 67, 54, 0.15); color: #E57373; }
.diff-line.header { background: rgba(66, 165, 245, 0.1); color: var(--accent); font-weight: 600; }
.diff-line.context { color: var(--text-muted); }

.section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 8px;
}
.comments-section { margin-top: 12px; }
.comments-list { display: flex; flex-direction: column; gap: 8px; }
.comment-card {
  padding: 10px 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
}
.comment-head {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 6px;
  font-size: 11px;
  color: var(--text-muted);
}
.comment-author {
  color: var(--accent);
  font-weight: 600;
}
.comment-kind {
  background: rgba(66, 165, 245, 0.12);
  padding: 1px 6px;
  border-radius: 6px;
  font-size: 10px;
}
.comment-date { margin-left: auto; }
.comment-body {
  font-size: 13px;
  white-space: pre-wrap;
  word-break: break-word;
}

.add-comment-btn { margin-bottom: 10px; }

.detail-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  padding-top: 10px;
  border-top: 1px solid var(--border);
}

.status-tab {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.status-card {
  padding: 14px 16px;
  background: var(--bg-card);
  border-radius: 8px;
}
.status-card-title {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 10px;
  color: var(--text-primary);
}
.status-grid {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.status-row {
  display: flex;
  gap: 10px;
  font-size: 13px;
}
.status-key { color: var(--text-muted); min-width: 80px; }
.status-val { color: var(--text-primary); }
.status-val.ok { color: #4CAF50; }
.status-val.err { color: #F44336; }
.status-val.path { font-family: 'Consolas', monospace; font-size: 12px; word-break: break-all; }
.status-val.link { color: var(--accent); text-decoration: underline; }

.ask-pr-dialog {
  padding: 12px 16px;
}
.dialog-hint {
  font-size: 12px;
  color: var(--text-muted);
  margin: 0 0 8px 0;
}
.ask-pr-textarea {
  width: 100%;
  min-height: 240px;
  font-family: 'Consolas', monospace;
  font-size: 12px;
  padding: 10px 12px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  border: 1px solid var(--border);
  border-radius: 6px;
  resize: vertical;
}
.dialog-footer {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

:deep(.van-button--primary) {
  background: var(--accent);
  border-color: var(--accent);
}
:deep(.van-tabs__nav--card) {
  background: var(--bg-card);
}
</style>
