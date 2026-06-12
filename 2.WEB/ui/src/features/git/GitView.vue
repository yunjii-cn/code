<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { showToast } from 'vant'
import { useDevice } from '@/composables/useDevice'
import { useProjectStore } from '@/stores/project'
import { systemApi, gitApi } from '@/api'
import DiffView from './components/DiffView.vue'

const { isMobile } = useDevice()
const projectStore = useProjectStore()

const gitStatus = ref<Array<{ file: string; status: string; staged: boolean }>>([])
const gitLog = ref<Array<{ hash: string; message: string; author: string; date: string }>>([])
const commitMessage = ref('')
const loading = ref(false)

const currentBranch = ref('')
const branches = ref<Array<{ name: string; isRemote: boolean; isCurrent: boolean }>>([])
const showBranchPicker = ref(false)

const showDiff = ref(false)
const diffOutput = ref('')
const diffLoading = ref(false)
const diffMode = ref<'unstaged' | 'staged'>('unstaged')

// 2026-06-10 TASK-2.2 增强：从 raw diff 解析出文件列表 + 当前激活的 DiffView 文件
const diffFiles = computed<string[]>(() => {
  if (!diffOutput.value) return []
  // raw diff 形如: "diff --git a/foo.txt b/foo.txt\n..."
  // 按 "diff --git " 拆分
  const blocks = diffOutput.value.split(/^diff --git /m).filter(Boolean)
  return blocks.map((b) => {
    const firstLine = b.split('\n')[0] || ''
    // 形如: a/foo.txt b/foo.txt → 提取 foo.txt
    const m = firstLine.match(/^a\/(.+?)\s+b\//)
    return m ? m[1] : firstLine
  }).filter(Boolean)
})
const currentDiffFile = ref<string>('')
watch(diffFiles, (files) => {
  if (files.length > 0 && !files.includes(currentDiffFile.value)) {
    currentDiffFile.value = files[0]
  } else if (files.length === 0) {
    currentDiffFile.value = ''
  }
})

function onDiffRestored() {
  // 单文件回退成功后重新加载整个 diff
  if (diffMode.value === 'staged') {
    loadStagedDiff()
  } else {
    loadDiff()
  }
  loadGitStatus()
}

// 2026-06-08 TASK-2.2 引入：DiffView 增强
const diffSearchKeyword = ref('')
const showRestoreConfirm = ref(false)
const restoreTargetFile = ref('')

const showCommitArea = ref(false)

const selectedCommit = ref<typeof gitLog.value[0] | null>(null)
const commitDetail = ref<{
  hash: string
  message: string
  author: string
  date: string
  files: string[]
  diffStat: string
} | null>(null)
const commitDetailLoading = ref(false)
const showCommitDetail = ref(false)

const actionLoading = ref('')

const stagedFiles = computed(() => gitStatus.value.filter(f => f.staged))
const unstagedFiles = computed(() => gitStatus.value.filter(f => !f.staged))

async function runGit(cmd: string): Promise<{ stdout: string; stderr: string; returncode: number }> {
  const project = projectStore.activeProject
  if (!project) throw new Error('未选择项目')
  const data: any = await systemApi.runCommand({ cmd, cwd: project.path })
  return {
    stdout: data?.stdout || '',
    stderr: data?.stderr || '',
    returncode: data?.returncode ?? 0,
  }
}

async function loadGitStatus() {
  const project = projectStore.activeProject
  if (!project) return
  loading.value = true
  try {
    const data: any = await systemApi.getGitStatus(project.path)
    const rawFiles = data?.files || []
    gitStatus.value = rawFiles.map((f: any) => {
      const status = f.status || f.state || ''
      const file = f.file || f.path || ''
      const staged = /^[MADRC]/.test(status)
      return { file, status, staged }
    })
  } catch {
    gitStatus.value = []
  } finally {
    loading.value = false
  }
}

async function loadGitLog() {
  const project = projectStore.activeProject
  if (!project) return
  try {
    const data: any = await systemApi.getGitLog(project.path)
    gitLog.value = (data?.commits || data || []).map((c: any) => ({
      hash: c.hash || c.commit?.substring(0, 8),
      message: c.message || c.subject,
      author: c.author || '',
      date: c.date || c.committed_date,
    }))
  } catch {
    gitLog.value = []
  }
}

async function loadCurrentBranch() {
  try {
    const result = await runGit('git rev-parse --abbrev-ref HEAD')
    currentBranch.value = result.stdout.trim()
  } catch {
    currentBranch.value = ''
  }
}

async function loadBranches() {
  try {
    const result = await runGit('git branch -a')
    const lines = result.stdout.split('\n').filter(Boolean)
    branches.value = lines.map(line => {
      const trimmed = line.trim()
      const isCurrent = trimmed.startsWith('*')
      const name = trimmed.replace(/^\*\s*/, '')
      const isRemote = name.startsWith('remotes/')
      return { name, isRemote, isCurrent }
    })
  } catch {
    branches.value = []
  }
}

async function switchBranch(branchName: string) {
  const localName = branchName.replace(/^remotes\/origin\//, '')
  if (localName === currentBranch.value) {
    showBranchPicker.value = false
    return
  }
  try {
    actionLoading.value = 'checkout'
    await runGit(`git checkout ${localName}`)
    showToast(`已切换到 ${localName}`)
    showBranchPicker.value = false
    await refreshAll()
  } catch (e: any) {
    showToast(`切换失败: ${e.message}`)
  } finally {
    actionLoading.value = ''
  }
}

async function loadDiff() {
  diffMode.value = 'unstaged'
  diffLoading.value = true
  try {
    const result = await runGit('git diff')
    diffOutput.value = result.stdout || '无未暂存的更改'
  } catch (e: any) {
    diffOutput.value = `获取差异失败: ${e.message}`
  } finally {
    diffLoading.value = false
  }
}

async function loadStagedDiff() {
  diffMode.value = 'staged'
  diffLoading.value = true
  try {
    const result = await runGit('git diff --cached')
    diffOutput.value = result.stdout || '无已暂存的更改'
  } catch (e: any) {
    diffOutput.value = `获取差异失败: ${e.message}`
  } finally {
    diffLoading.value = false
  }
}

// 2026-06-08 TASK-2.2 引入：DiffView 增强 - 单文件回退
async function restoreSingleFile(file: string, stagedOnly: boolean = false) {
  const project = projectStore.activeProject
  if (!project) return
  try {
    actionLoading.value = `restore-${file}`
    const res: any = await systemApi.restoreFile(project.path, file, stagedOnly)
    if (res?.ok !== false && !res?.error) {
      showToast(stagedOnly ? '已取消暂存' : '已回退文件')
      showRestoreConfirm.value = false
      restoreTargetFile.value = ''
      await loadGitStatus()
    } else {
      showToast(`回退失败: ${res?.error || '未知错误'}`)
    }
  } catch (e: any) {
    showToast(`回退失败: ${e.message}`)
  } finally {
    actionLoading.value = ''
  }
}

function confirmRestore(file: string) {
  restoreTargetFile.value = file
  showRestoreConfirm.value = true
}

// 2026-06-08 TASK-2.2 引入：搜索高亮（把 keyword 包成高亮 HTML）
function highlightText(text: string, keyword: string): string {
  if (!keyword || !keyword.trim()) return text
  try {
    const safe = text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
    const pattern = new RegExp(keyword.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi')
    return safe.replace(pattern, (m) => `<mark class="diff-search-mark">${m}</mark>`)
  } catch {
    return text
  }
}

function openDiffView() {
  showDiff.value = true
  loadDiff()
}

function openStagedDiffView() {
  showDiff.value = true
  loadStagedDiff()
}

async function stageFile(file: string) {
  try {
    actionLoading.value = `add-${file}`
    await runGit(`git add "${file}"`)
    showToast('已暂存')
    await loadGitStatus()
  } catch (e: any) {
    showToast(`暂存失败: ${e.message}`)
  } finally {
    actionLoading.value = ''
  }
}

async function unstageFile(file: string) {
  try {
    actionLoading.value = `reset-${file}`
    await runGit(`git reset HEAD "${file}"`)
    showToast('已取消暂存')
    await loadGitStatus()
  } catch (e: any) {
    showToast(`取消暂存失败: ${e.message}`)
  } finally {
    actionLoading.value = ''
  }
}

async function stageAll() {
  try {
    actionLoading.value = 'add-all'
    await runGit('git add .')
    showToast('已暂存所有文件')
    await loadGitStatus()
  } catch (e: any) {
    showToast(`暂存失败: ${e.message}`)
  } finally {
    actionLoading.value = ''
  }
}

async function gitCommit() {
  if (!commitMessage.value.trim()) {
    showToast('请输入提交信息')
    return
  }
  const project = projectStore.activeProject
  if (!project) return
  try {
    actionLoading.value = 'commit'
    await systemApi.gitCommit(project.path, commitMessage.value.trim())
    showToast('提交成功')
    commitMessage.value = ''
    showCommitArea.value = false
    await loadGitStatus()
    await loadGitLog()
  } catch (e: any) {
    showToast(`提交失败: ${e.message}`)
  } finally {
    actionLoading.value = ''
  }
}

async function stageAllAndCommit() {
  if (!commitMessage.value.trim()) {
    showToast('请输入提交信息')
    return
  }
  try {
    actionLoading.value = 'commit'
    await runGit('git add .')
    const project = projectStore.activeProject
    if (!project) return
    await systemApi.gitCommit(project.path, commitMessage.value.trim())
    showToast('暂存并提交成功')
    commitMessage.value = ''
    showCommitArea.value = false
    await loadGitStatus()
    await loadGitLog()
  } catch (e: any) {
    showToast(`操作失败: ${e.message}`)
  } finally {
    actionLoading.value = ''
  }
}

async function gitPull() {
  try {
    actionLoading.value = 'pull'
    const result = await runGit('git pull')
    if (result.returncode !== 0) {
      showToast(`拉取失败: ${result.stderr || '未知错误'}`)
    } else {
      showToast('拉取成功')
      await refreshAll()
    }
  } catch (e: any) {
    showToast(`拉取失败: ${e.message}`)
  } finally {
    actionLoading.value = ''
  }
}

async function gitPush() {
  try {
    actionLoading.value = 'push'
    const result = await runGit('git push')
    if (result.returncode !== 0) {
      showToast(`推送失败: ${result.stderr || '未知错误'}`)
    } else {
      showToast('推送成功')
    }
  } catch (e: any) {
    showToast(`推送失败: ${e.message}`)
  } finally {
    actionLoading.value = ''
  }
}

async function loadCommitDetail(commit: typeof gitLog.value[0]) {
  selectedCommit.value = commit
  showCommitDetail.value = true
  commitDetailLoading.value = true
  commitDetail.value = null
  try {
    const [statResult, showResult] = await Promise.all([
      runGit(`git show --stat --format="" ${commit.hash}`),
      runGit(`git log -1 --format="%H%n%B%n%an%n%ai" ${commit.hash}`),
    ])
    const lines = showResult.stdout.split('\n')
    const fullHash = lines[0] || commit.hash
    const fullMessage = lines.slice(1, -2).join('\n').trim() || commit.message
    const author = lines[lines.length - 2] || commit.author
    const date = lines[lines.length - 1] || commit.date
    const files = statResult.stdout
      .split('\n')
      .filter(l => l.includes('|'))
      .map(l => l.split('|')[0].trim())
    commitDetail.value = {
      hash: fullHash,
      message: fullMessage,
      author,
      date,
      files,
      diffStat: statResult.stdout,
    }
  } catch (e: any) {
    commitDetail.value = {
      hash: commit.hash,
      message: commit.message,
      author: commit.author,
      date: commit.date,
      files: [],
      diffStat: `加载失败: ${e.message}`,
    }
  } finally {
    commitDetailLoading.value = false
  }
}

function statusColor(status: string): string {
  if (status.includes('M')) return '#FF9800'
  if (status.includes('A') || status.includes('new')) return '#4CAF50'
  if (status.includes('D') || status.includes('deleted')) return '#F44336'
  if (status.includes('?') || status.includes('untracked')) return '#9E9E9E'
  return '#42A5F5'
}

function statusLabel(status: string): string {
  const map: Record<string, string> = {
    M: '已修改', A: '已新增', D: '已删除', R: '已重命名', C: '已复制',
    '??': '未跟踪',
  }
  const key = status.replace(/\s/g, '')
  return map[key] || map[key.charAt(0)] || status
}

function parseDiffLines(diff: string): Array<{ type: 'add' | 'remove' | 'header' | 'context'; text: string }> {
  return diff.split('\n').map(line => {
    if (line.startsWith('+++') || line.startsWith('---') || line.startsWith('@@')) {
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

async function refreshAll() {
  await Promise.all([loadGitStatus(), loadGitLog(), loadCurrentBranch(), loadBranches()])
}

onMounted(async () => {
  if (projectStore.activeProject) {
    await refreshAll()
  }
})
</script>

<template>
  <div class="git-view" :class="{ mobile: isMobile }">
    <div class="page-header">
      <h2 class="page-title">Git 管理</h2>
      <div class="header-actions">
        <van-button size="small" plain :loading="actionLoading === 'pull'" @click="gitPull">拉取</van-button>
        <van-button size="small" plain :loading="actionLoading === 'push'" @click="gitPush">推送</van-button>
        <van-button size="small" type="primary" @click="refreshAll">刷新</van-button>
      </div>
    </div>

    <div v-if="!projectStore.activeProject" class="empty-state">
      <p>请先选择一个项目</p>
    </div>

    <template v-else>
      <div class="branch-section">
        <div class="branch-row">
          <div class="current-branch" @click="showBranchPicker = true">
            <span class="branch-icon">⎇</span>
            <span class="branch-name">{{ currentBranch || '...' }}</span>
            <span class="branch-arrow">▼</span>
          </div>
          <van-button size="mini" plain @click="openDiffView">查看差异</van-button>
        </div>
      </div>

      <div class="commit-section">
        <div class="section-title" @click="showCommitArea = !showCommitArea">
          <span>提交更改</span>
          <span v-if="stagedFiles.length > 0" class="staged-count">{{ stagedFiles.length }} 已暂存</span>
          <span class="expand-icon" :class="{ expanded: showCommitArea }">▶</span>
        </div>
        <div v-if="showCommitArea" class="commit-body">
          <van-field
            v-model="commitMessage"
            type="textarea"
            rows="3"
            autosize
            placeholder="输入提交信息..."
            class="dark-field commit-textarea"
          />
          <div class="commit-actions">
            <van-button
              size="small"
              type="primary"
              :disabled="!commitMessage.trim()"
              :loading="actionLoading === 'commit'"
              @click="gitCommit"
            >
              提交 ({{ stagedFiles.length }} 文件)
            </van-button>
            <van-button
              size="small"
              plain
              :disabled="!commitMessage.trim() || unstagedFiles.length === 0"
              :loading="actionLoading === 'commit'"
              @click="stageAllAndCommit"
            >
              全部暂存并提交
            </van-button>
          </div>
        </div>
        <div v-else class="commit-collapsed" @click="showCommitArea = true">
          <van-field
            v-model="commitMessage"
            placeholder="输入提交信息..."
            class="dark-field"
            @keydown.enter="showCommitArea = true"
          />
        </div>
      </div>

      <div class="section">
        <div class="section-title-row">
          <span class="section-title">文件状态 ({{ gitStatus.length }})</span>
          <div class="section-actions">
            <van-button
              v-if="unstagedFiles.length > 0"
              size="mini"
              plain
              :loading="actionLoading === 'add-all'"
              @click="stageAll"
            >
              全部暂存
            </van-button>
          </div>
        </div>
        <van-loading v-if="loading" class="page-loading" color="var(--accent)" vertical>加载中...</van-loading>
        <div v-else-if="gitStatus.length === 0" class="empty-hint">工作区干净，无待提交更改</div>
        <div v-else>
          <div v-if="stagedFiles.length > 0" class="status-group">
            <div class="status-group-title">已暂存 ({{ stagedFiles.length }})</div>
            <div class="status-list">
              <div v-for="item in stagedFiles" :key="'staged-' + item.file" class="status-item staged">
                <span class="status-badge" :style="{ color: statusColor(item.status) }">{{ item.status }}</span>
                <span class="status-file" :title="item.file">{{ item.file }}</span>
                <span class="status-label">{{ statusLabel(item.status) }}</span>
                <div class="status-actions">
                  <van-button size="mini" plain @click="openStagedDiffView">差异</van-button>
                  <van-button size="mini" plain type="danger" :loading="actionLoading === 'reset-' + item.file" @click="unstageFile(item.file)">取消暂存</van-button>
                </div>
              </div>
            </div>
          </div>
          <div v-if="unstagedFiles.length > 0" class="status-group">
            <div class="status-group-title">未暂存 ({{ unstagedFiles.length }})</div>
            <div class="status-list">
              <div v-for="item in unstagedFiles" :key="'unstaged-' + item.file" class="status-item unstaged">
                <span class="status-badge" :style="{ color: statusColor(item.status) }">{{ item.status }}</span>
                <span class="status-file" :title="item.file">{{ item.file }}</span>
                <span class="status-label">{{ statusLabel(item.status) }}</span>
                <div class="status-actions">
                  <van-button size="mini" plain :loading="actionLoading === 'add-' + item.file" @click="stageFile(item.file)">暂存</van-button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="section">
        <div class="section-title">提交历史</div>
        <div v-if="gitLog.length === 0" class="empty-hint">暂无提交记录</div>
        <div v-else class="log-list">
          <div
            v-for="item in gitLog"
            :key="item.hash"
            class="log-item"
            @click="loadCommitDetail(item)"
          >
            <div class="log-hash">{{ item.hash }}</div>
            <div class="log-message">{{ item.message }}</div>
            <div class="log-meta">
              <span v-if="item.author">{{ item.author }}</span>
              <span v-if="item.date">{{ item.date }}</span>
            </div>
          </div>
        </div>
      </div>
    </template>

    <van-popup v-model:show="showBranchPicker" round position="bottom" class="branch-popup">
      <div class="popup-header">
        <span class="popup-title">切换分支</span>
        <van-button size="mini" plain @click="showBranchPicker = false">关闭</van-button>
      </div>
      <div class="branch-list">
        <div class="branch-group" v-if="branches.filter(b => !b.isRemote).length > 0">
          <div class="branch-group-title">本地分支</div>
          <div
            v-for="b in branches.filter(b => !b.isRemote)"
            :key="b.name"
            class="branch-item"
            :class="{ active: b.isCurrent }"
            @click="switchBranch(b.name)"
          >
            <span v-if="b.isCurrent" class="branch-current-dot">●</span>
            <span class="branch-item-name">{{ b.name }}</span>
            <span v-if="b.isCurrent" class="branch-current-label">当前</span>
          </div>
        </div>
        <div class="branch-group" v-if="branches.filter(b => b.isRemote).length > 0">
          <div class="branch-group-title">远程分支</div>
          <div
            v-for="b in branches.filter(b => b.isRemote)"
            :key="b.name"
            class="branch-item"
            @click="switchBranch(b.name)"
          >
            <span class="branch-item-name">{{ b.name }}</span>
          </div>
        </div>
      </div>
    </van-popup>

    <van-popup v-model:show="showDiff" round position="bottom" class="diff-popup" :style="{ height: '80%' }">
      <div class="popup-header">
        <span class="popup-title">差异查看</span>
        <div class="diff-tabs">
          <van-button size="mini" :type="diffMode === 'unstaged' ? 'primary' : 'default'" @click="loadDiff">未暂存</van-button>
          <van-button size="mini" :type="diffMode === 'staged' ? 'primary' : 'default'" @click="loadStagedDiff">已暂存</van-button>
        </div>
        <van-button size="mini" plain @click="showDiff = false">关闭</van-button>
      </div>
      <div class="diff-content">
        <van-loading v-if="diffLoading" color="var(--accent)" vertical>加载中...</van-loading>
        <template v-else-if="diffFiles.length === 0">
          <div class="empty-hint">{{ diffOutput || '无改动' }}</div>
        </template>
        <template v-else>
          <!-- 2026-06-10 TASK-2.2 增强：文件切换 tab + DiffView 彩色渲染 -->
          <div class="diff-file-tabs">
            <button
              v-for="file in diffFiles"
              :key="file"
              class="diff-file-tab"
              :class="{ 'is-active': currentDiffFile === file }"
              @click="currentDiffFile = file"
            >
              {{ file }}
            </button>
          </div>
          <div v-if="currentDiffFile && projectStore.activeProject" class="diff-view-wrapper">
            <DiffView
              :project-path="projectStore.activeProject.path"
              :file-path="currentDiffFile"
              :staged="diffMode === 'staged'"
              @restored="onDiffRestored"
            />
          </div>
        </template>
      </div>
    </van-popup>

    <!-- 2026-06-08 TASK-2.2 引入：单文件回退确认对话框 -->
    <van-dialog
      v-model:show="showRestoreConfirm"
      title="确认回退文件"
      show-cancel-button
      :style="{ width: '85vw', maxWidth: '420px' }"
    >
      <div class="restore-confirm">
        <p>即将把 <code>{{ restoreTargetFile }}</code> 回退到 HEAD 版本。</p>
        <p class="warn">未提交的本地修改将丢失，且无法恢复！</p>
      </div>
      <template #footer>
        <van-button size="small" type="danger" :loading="actionLoading === 'restore-' + restoreTargetFile" @click="restoreSingleFile(restoreTargetFile, false)">确认回退</van-button>
      </template>
    </van-dialog>

    <van-popup v-model:show="showCommitDetail" round position="bottom" class="detail-popup" :style="{ height: '70%' }">
      <div class="popup-header">
        <span class="popup-title">提交详情</span>
        <van-button size="mini" plain @click="showCommitDetail = false">关闭</van-button>
      </div>
      <div class="detail-content">
        <van-loading v-if="commitDetailLoading" color="var(--accent)" vertical>加载中...</van-loading>
        <template v-else-if="commitDetail">
          <div class="detail-hash">{{ commitDetail.hash }}</div>
          <div class="detail-message">{{ commitDetail.message }}</div>
          <div class="detail-meta">
            <span v-if="commitDetail.author">{{ commitDetail.author }}</span>
            <span v-if="commitDetail.date">{{ commitDetail.date }}</span>
          </div>
          <div v-if="commitDetail.files.length > 0" class="detail-files">
            <div class="detail-section-title">变更文件 ({{ commitDetail.files.length }})</div>
            <div v-for="f in commitDetail.files" :key="f" class="detail-file">{{ f }}</div>
          </div>
          <div v-if="commitDetail.diffStat" class="detail-stat">
            <div class="detail-section-title">差异统计</div>
            <pre class="diff-stat-output">{{ commitDetail.diffStat }}</pre>
          </div>
        </template>
      </div>
    </van-popup>
  </div>
</template>

<style scoped>
.git-view {
  height: 100%;
  overflow-y: auto;
  padding: 24px;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.git-view.mobile {
  padding: 16px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
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

.empty-state,
.empty-hint {
  text-align: center;
  padding: 30px 0;
  color: var(--text-muted);
  font-size: 14px;
}

.branch-section {
  margin-bottom: 20px;
}

.branch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.current-branch {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  background: var(--bg-card);
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.2s;
  flex: 1;
  min-width: 0;
}

.current-branch:hover {
  background: var(--bg-card-hover);
}

.branch-icon {
  color: var(--accent);
  font-size: 16px;
}

.branch-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--accent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.branch-arrow {
  color: var(--text-muted);
  font-size: 10px;
  margin-left: auto;
}

.commit-section {
  margin-bottom: 24px;
}

.section {
  margin-bottom: 24px;
}

.section-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
}

.section-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.section-title-row .section-title {
  margin-bottom: 0;
}

.section-actions {
  display: flex;
  gap: 6px;
}

.staged-count {
  font-size: 12px;
  color: var(--accent);
  font-weight: 400;
  background: rgba(66, 165, 245, 0.15);
  padding: 2px 8px;
  border-radius: 10px;
}

.expand-icon {
  font-size: 10px;
  color: var(--text-muted);
  transition: transform 0.2s;
  margin-left: auto;
}

.expand-icon.expanded {
  transform: rotate(90deg);
}

.commit-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.commit-textarea {
  width: 100%;
}

.commit-collapsed {
  cursor: pointer;
}

.commit-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.commit-row {
  display: flex;
  gap: 8px;
}

.commit-row .dark-field {
  flex: 1;
}

.dark-field {
  background: var(--bg-card);
  border-radius: 8px;
}

:deep(.dark-field .van-field__control) {
  color: var(--text-primary);
}

:deep(.commit-textarea .van-field__control) {
  color: var(--text-primary);
  min-height: 60px;
}

.status-group {
  margin-bottom: 16px;
}

.status-group-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 8px;
  padding-left: 4px;
}

.status-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: var(--bg-card);
  border-radius: 8px;
  font-size: 13px;
  flex-wrap: wrap;
}

.status-item.staged {
  border-left: 3px solid #4CAF50;
}

.status-item.unstaged {
  border-left: 3px solid #FF9800;
}

.status-badge {
  font-family: 'Consolas', monospace;
  font-weight: 600;
  font-size: 12px;
  min-width: 24px;
}

.status-file {
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}

.status-label {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
}

.status-actions {
  display: flex;
  gap: 4px;
  margin-left: auto;
  flex-shrink: 0;
}

.log-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.log-item {
  padding: 10px 14px;
  background: var(--bg-card);
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.2s;
}

.log-item:hover {
  background: var(--bg-card-hover);
}

.log-hash {
  font-family: 'Consolas', monospace;
  font-size: 12px;
  color: var(--accent);
  margin-bottom: 2px;
}

.log-message {
  font-size: 14px;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.log-meta {
  display: flex;
  gap: 12px;
  font-size: 11px;
  color: var(--text-muted);
}

.popup-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
}

.popup-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.branch-popup {
  max-height: 60vh;
}

.branch-list {
  padding: 8px 0;
  max-height: calc(60vh - 50px);
  overflow-y: auto;
}

.branch-group {
  margin-bottom: 8px;
}

.branch-group-title {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 8px 16px 4px;
}

.branch-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  cursor: pointer;
  transition: background 0.2s;
}

.branch-item:hover {
  background: var(--bg-card-hover);
}

.branch-item.active {
  background: rgba(66, 165, 245, 0.1);
}

.branch-current-dot {
  color: #4CAF50;
  font-size: 12px;
}

.branch-item-name {
  font-size: 14px;
  color: var(--text-primary);
  font-family: 'Consolas', monospace;
}

.branch-current-label {
  font-size: 11px;
  color: var(--accent);
  margin-left: auto;
}

.diff-popup {
  display: flex;
  flex-direction: column;
}

.diff-tabs {
  display: flex;
  gap: 6px;
}

.diff-content {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
}

.diff-lines {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 12px;
  line-height: 1.5;
}

/* 2026-06-08 TASK-2.2 引入：DiffView 增强 - 工具栏 + 文件块 + 搜索高亮 */
.diff-toolbar {
  padding: 6px 16px;
  border-bottom: 1px solid var(--border);
}
.diff-search-field {
  background: var(--bg-card);
  border-radius: 6px;
  padding: 2px 10px;
}
.diff-view-wrapper {
  flex: 1;
  min-height: 0;
  padding: 8px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.diff-file-tabs {
  display: flex;
  gap: 4px;
  padding: 6px 8px;
  overflow-x: auto;
  border-bottom: 1px solid var(--border, #333);
  background: var(--bg-secondary, #252525);
}

.diff-file-tab {
  padding: 4px 10px;
  background: transparent;
  border: 1px solid var(--border, #444);
  border-radius: 4px;
  color: var(--text-secondary, #999);
  cursor: pointer;
  font-size: 12px;
  font-family: ui-monospace, monospace;
  white-space: nowrap;
  transition: all 0.15s;
}

.diff-file-tab:hover {
  background: var(--bg-hover, #333);
}

.diff-file-tab.is-active {
  background: var(--accent, #42a5f5);
  color: white;
  border-color: var(--accent, #42a5f5);
}

.diff-files {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.diff-file-block {
  background: var(--bg-card);
  border-radius: 6px;
  overflow: hidden;
}
.diff-file-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
}
.diff-file-name {
  font-family: 'Consolas', monospace;
  font-size: 12px;
  color: var(--accent);
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}
:deep(.diff-search-mark) {
  background: #FFEB3B;
  color: #000;
  padding: 0 2px;
  border-radius: 2px;
}
.restore-confirm {
  padding: 12px 16px;
}
.restore-confirm p {
  margin: 8px 0;
  font-size: 13px;
}
.restore-confirm code {
  font-family: 'Consolas', monospace;
  background: rgba(255,255,255,0.08);
  padding: 2px 6px;
  border-radius: 4px;
  word-break: break-all;
}
.restore-confirm .warn {
  color: #F44336;
  font-size: 12px;
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
}

.diff-line.add {
  background: rgba(76, 175, 80, 0.15);
  color: #81C784;
}

.diff-line.remove {
  background: rgba(244, 67, 54, 0.15);
  color: #E57373;
}

.diff-line.header {
  background: rgba(66, 165, 245, 0.1);
  color: var(--accent);
  font-weight: 600;
}

.diff-line.context {
  color: var(--text-muted);
}

.detail-popup {
  display: flex;
  flex-direction: column;
}

.detail-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.detail-hash {
  font-family: 'Consolas', monospace;
  font-size: 12px;
  color: var(--accent);
  margin-bottom: 8px;
}

.detail-message {
  font-size: 14px;
  color: var(--text-primary);
  margin-bottom: 8px;
  white-space: pre-wrap;
}

.detail-meta {
  display: flex;
  gap: 12px;
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 16px;
}

.detail-section-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.detail-files {
  margin-bottom: 16px;
}

.detail-file {
  font-family: 'Consolas', monospace;
  font-size: 12px;
  color: var(--text-secondary);
  padding: 4px 8px;
  background: var(--bg-card);
  border-radius: 4px;
  margin-bottom: 4px;
}

.detail-stat {
  margin-top: 8px;
}

.diff-stat-output {
  font-family: 'Consolas', monospace;
  font-size: 12px;
  color: var(--text-secondary);
  background: var(--bg-card);
  border-radius: 8px;
  padding: 12px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}

:deep(.van-button--primary) {
  background: var(--accent);
  border-color: var(--accent);
}

:deep(.van-popup) {
  background: var(--bg-secondary);
}

:deep(.van-loading) {
  display: flex;
  justify-content: center;
  padding: 20px 0;
}
</style>
