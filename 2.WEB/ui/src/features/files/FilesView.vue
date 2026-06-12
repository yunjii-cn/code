<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { showToast } from 'vant'
import hljs from 'highlight.js'
import 'highlight.js/styles/github-dark.css'
import { useDevice } from '@/composables/useDevice'
import { useProjectStore } from '@/stores/project'
import { systemApi } from '@/api'

const { isMobile } = useDevice()
const projectStore = useProjectStore()

interface FileNode {
  name: string
  path: string
  type: 'file' | 'dir'
  children?: FileNode[]
  expanded?: boolean
  size?: number
  modified?: string
}

interface FlatNode {
  node: FileNode
  depth: number
  relativePath?: string
}

const CODE_EXTENSIONS = new Set([
  'py', 'js', 'ts', 'vue', 'css', 'html', 'json', 'md',
  'yaml', 'yml', 'toml', 'cfg', 'ini', 'sh', 'bat',
  'jsx', 'tsx', 'scss', 'less', 'xml', 'sql',
  'rs', 'go', 'java', 'c', 'cpp', 'h', 'hpp',
  'rb', 'php', 'swift', 'kt', 'dart', 'lua', 'r',
])

const IMAGE_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'gif', 'svg', 'ico', 'webp', 'bmp'])

const EXT_TO_LANG: Record<string, string> = {
  py: 'python', js: 'javascript', ts: 'typescript',
  jsx: 'javascript', tsx: 'typescript', vue: 'html',
  css: 'css', scss: 'scss', less: 'less',
  html: 'html', xml: 'xml', json: 'json',
  md: 'markdown', yaml: 'yaml', yml: 'yaml',
  toml: 'ini', cfg: 'ini', ini: 'ini',
  sh: 'bash', bat: 'dos', sql: 'sql',
  rs: 'rust', go: 'go', java: 'java',
  c: 'c', cpp: 'cpp', h: 'c', hpp: 'cpp',
  rb: 'ruby', php: 'php', swift: 'swift',
  kt: 'kotlin', dart: 'dart', lua: 'lua', r: 'r',
}

const IMAGE_MIME_TYPES: Record<string, string> = {
  png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg',
  gif: 'image/gif', svg: 'image/svg+xml', ico: 'image/x-icon',
  webp: 'image/webp', bmp: 'image/bmp',
}

const tree = ref<FileNode[]>([])
const loading = ref(false)
const searchQuery = ref('')
const fileContent = ref('')
const viewingFile = ref('')
const viewingFilePath = ref('')
const viewing = ref(false)
const viewingFileType = ref<'code' | 'image' | 'binary'>('code')
const viewingNode = ref<FileNode | null>(null)
const highlightedCode = ref('')
const fileLanguage = ref('')
const imageSrc = ref('')
const copyBtnText = ref('复制')

function getFileExtension(filename: string): string {
  const parts = filename.split('.')
  return parts.length > 1 ? parts.pop()!.toLowerCase() : ''
}

function getFileType(filename: string): 'code' | 'image' | 'binary' {
  const ext = getFileExtension(filename)
  if (CODE_EXTENSIONS.has(ext)) return 'code'
  if (IMAGE_EXTENSIONS.has(ext)) return 'image'
  return 'binary'
}

function formatSize(bytes?: number): string {
  if (bytes === undefined || bytes === null) return ''
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`
}

function formatDate(dateStr?: string): string {
  if (!dateStr) return ''
  try {
    const date = new Date(dateStr)
    if (isNaN(date.getTime())) return dateStr
    const y = date.getFullYear()
    const m = String(date.getMonth() + 1).padStart(2, '0')
    const d = String(date.getDate()).padStart(2, '0')
    const h = String(date.getHours()).padStart(2, '0')
    const min = String(date.getMinutes()).padStart(2, '0')
    return `${y}-${m}-${d} ${h}:${min}`
  } catch {
    return dateStr
  }
}

function highlightCode(code: string, language: string): string {
  if (!code) return ''
  try {
    if (language && hljs.getLanguage(language)) {
      return hljs.highlight(code, { language }).value
    }
    return hljs.highlightAuto(code).value
  } catch {
    return code.replace(/</g, '&lt;').replace(/>/g, '&gt;')
  }
}

function buildTree(data: any[]): FileNode[] {
  if (!Array.isArray(data)) return []
  return data.map((item) => ({
    name: item.name || item.path?.split('/').pop() || item.path?.split('\\').pop() || '',
    path: item.path || '',
    type: item.type === 'directory' || item.type === 'dir' ? 'dir' : 'file',
    children: item.children ? buildTree(item.children) : undefined,
    expanded: false,
    size: item.size,
    modified: item.modified,
  }))
}

function flattenExpandedTree(nodes: FileNode[], depth: number = 0): FlatNode[] {
  const result: FlatNode[] = []
  for (const node of nodes) {
    result.push({ node, depth })
    if (node.type === 'dir' && node.expanded && node.children) {
      result.push(...flattenExpandedTree(node.children, depth + 1))
    }
  }
  return result
}

function searchAllNodes(nodes: FileNode[], query: string, parentPath: string = ''): FlatNode[] {
  const result: FlatNode[] = []
  for (const node of nodes) {
    const relPath = parentPath ? `${parentPath}/${node.name}` : node.name
    if (node.name.toLowerCase().includes(query)) {
      result.push({ node, depth: 0, relativePath: relPath })
    }
    if (node.children) {
      result.push(...searchAllNodes(node.children, query, relPath))
    }
  }
  return result
}

const isSearchMode = computed(() => searchQuery.value.trim().length > 0)

const displayNodes = computed(() => {
  if (isSearchMode.value) {
    return searchAllNodes(tree.value, searchQuery.value.toLowerCase().trim())
  }
  return flattenExpandedTree(tree.value)
})

const codeLines = computed(() => fileContent.value.split('\n'))

const breadcrumbs = computed(() => {
  const project = projectStore.activeProject
  if (!project || !viewingFilePath.value) return []
  const projectPath = project.path.replace(/\\/g, '/')
  const filePath = viewingFilePath.value.replace(/\\/g, '/')
  let relativePath = ''
  if (filePath.startsWith(projectPath + '/')) {
    relativePath = filePath.slice(projectPath.length + 1)
  } else if (filePath.startsWith(projectPath)) {
    relativePath = filePath.slice(projectPath.length).replace(/^\//, '')
  } else {
    const parts = filePath.split('/')
    const projParts = projectPath.split('/')
    relativePath = parts.slice(projParts.length).join('/')
  }
  if (!relativePath) return []
  const segments = relativePath.split('/')
  const result: Array<{ name: string; path: string }> = []
  let currentPath = project.path
  for (const segment of segments) {
    currentPath = currentPath + '\\' + segment
    result.push({ name: segment, path: currentPath })
  }
  return result
})

async function loadTree() {
  const project = projectStore.activeProject
  if (!project) {
    showToast('请先选择一个项目')
    return
  }
  loading.value = true
  try {
    const data: any = await systemApi.getFileTree(project.path)
    tree.value = buildTree(data)
  } catch {
    showToast('加载文件树失败')
  } finally {
    loading.value = false
  }
}

function toggleNode(node: FileNode) {
  if (node.type === 'dir') {
    node.expanded = !node.expanded
  } else {
    viewFile(node)
  }
}

async function viewFile(node: FileNode) {
  viewing.value = true
  viewingFile.value = node.name
  viewingFilePath.value = node.path
  viewingNode.value = node
  fileContent.value = ''
  highlightedCode.value = ''
  imageSrc.value = ''
  copyBtnText.value = '复制'

  const fileType = getFileType(node.name)
  viewingFileType.value = fileType

  const project = projectStore.activeProject
  if (!project) return

  if (fileType === 'binary') return

  if (fileType === 'image') {
    await loadImage(node)
    return
  }

  fileContent.value = '加载中...'
  try {
    const data: any = await systemApi.runCommand({
      cmd: `type "${node.path}"`,
      cwd: project.path,
    })
    fileContent.value = data?.stdout || '(空文件)'
    const ext = getFileExtension(node.name)
    fileLanguage.value = EXT_TO_LANG[ext] || ''
    highlightedCode.value = highlightCode(fileContent.value, fileLanguage.value)
  } catch {
    fileContent.value = '无法读取文件'
    highlightedCode.value = ''
  }
}

async function loadImage(node: FileNode) {
  if ((node.size || 0) > 2 * 1024 * 1024) {
    imageSrc.value = ''
    return
  }
  const project = projectStore.activeProject
  if (!project) return
  try {
    const ext = getFileExtension(node.name)
    const mimeType = IMAGE_MIME_TYPES[ext] || 'image/png'
    const safePath = node.path.replace(/'/g, "''")
    const data: any = await systemApi.runCommand({
      cmd: `powershell -Command "[Convert]::ToBase64String([IO.File]::ReadAllBytes('${safePath}'))"`,
      cwd: project.path,
    })
    const base64 = (data?.stdout || '').replace(/\s/g, '')
    imageSrc.value = base64 ? `data:${mimeType};base64,${base64}` : ''
  } catch {
    imageSrc.value = ''
  }
}

function closeViewer() {
  viewing.value = false
  viewingFile.value = ''
  viewingFilePath.value = ''
  viewingNode.value = null
  fileContent.value = ''
  highlightedCode.value = ''
  imageSrc.value = ''
}

function expandToPath(nodes: FileNode[], targetPath: string): boolean {
  for (const node of nodes) {
    if (node.path === targetPath) {
      if (node.type === 'dir') node.expanded = true
      return true
    }
    if (node.type === 'dir' && node.children) {
      if (expandToPath(node.children, targetPath)) {
        node.expanded = true
        return true
      }
    }
  }
  return false
}

function navigateToBreadcrumb(path: string) {
  closeViewer()
  expandToPath(tree.value, path)
}

async function copyContent() {
  try {
    await navigator.clipboard.writeText(fileContent.value)
  } catch {
    const textarea = document.createElement('textarea')
    textarea.value = fileContent.value
    document.body.appendChild(textarea)
    textarea.select()
    document.execCommand('copy')
    document.body.removeChild(textarea)
  }
  copyBtnText.value = '已复制'
  showToast('已复制到剪贴板')
  setTimeout(() => {
    copyBtnText.value = '复制'
  }, 2000)
}

function getNodeIcon(node: FileNode): string {
  if (node.type === 'dir') {
    return node.expanded ? 'arrow-down' : 'arrow'
  }
  const ext = getFileExtension(node.name)
  if (IMAGE_EXTENSIONS.has(ext)) return 'photo-o'
  return 'description'
}

function getNodeMeta(node: FileNode): string {
  const parts: string[] = []
  if (node.type === 'dir') {
    if (node.children) parts.push(`${node.children.length} 项`)
  } else {
    if (node.size !== undefined && node.size !== null) parts.push(formatSize(node.size))
  }
  if (node.modified) parts.push(formatDate(node.modified))
  return parts.join(' · ')
}

onMounted(loadTree)
</script>

<template>
  <div class="files-view" :class="{ mobile: isMobile }">
    <div class="files-header">
      <h2 class="page-title">文件浏览器</h2>
      <van-button size="small" type="primary" @click="loadTree">刷新</van-button>
    </div>

    <div v-if="!projectStore.activeProject" class="empty-state">
      <p>请先在项目页面选择一个项目</p>
    </div>

    <template v-else>
      <div class="files-layout">
        <div class="tree-panel" :class="{ 'tree-hidden': viewing && isMobile }">
          <div class="search-bar">
            <van-field
              v-model="searchQuery"
              placeholder="搜索文件..."
              clearable
              left-icon="search"
              class="dark-field"
            />
          </div>

          <div v-if="breadcrumbs.length > 0" class="breadcrumbs">
            <span class="breadcrumb-root" @click="navigateToBreadcrumb(projectStore.activeProject!.path)">
              <van-icon name="home-o" size="12" />
            </span>
            <template v-for="(crumb, i) in breadcrumbs" :key="i">
              <van-icon name="arrow" size="10" class="breadcrumb-sep" />
              <span
                class="breadcrumb-item"
                :class="{ active: i === breadcrumbs.length - 1 }"
                @click="i < breadcrumbs.length - 1 && navigateToBreadcrumb(crumb.path)"
              >
                {{ crumb.name }}
              </span>
            </template>
          </div>

          <van-loading v-if="loading" class="page-loading" color="var(--accent)" vertical>
            加载中...
          </van-loading>
          <div v-else-if="tree.length === 0" class="empty-state">
            <p>暂无文件</p>
          </div>
          <div v-else-if="displayNodes.length === 0 && isSearchMode" class="empty-state">
            <p>未找到匹配的文件</p>
          </div>
          <div v-else class="tree-content">
            <div
              v-for="item in displayNodes"
              :key="item.node.path + '-' + item.depth"
              class="tree-node"
              :class="{
                dir: item.node.type === 'dir',
                file: item.node.type === 'file',
                expanded: item.node.type === 'dir' && item.node.expanded,
                active: viewingFilePath === item.node.path,
              }"
              :style="{ paddingLeft: `${item.depth * 16 + 8}px` }"
              @click="toggleNode(item.node)"
            >
              <van-icon
                :name="getNodeIcon(item.node)"
                size="14"
                class="node-icon"
              />
              <div class="node-info">
                <span class="node-name">
                  {{ isSearchMode && item.relativePath ? item.relativePath : item.node.name }}
                </span>
                <span v-if="getNodeMeta(item.node)" class="node-meta">
                  {{ getNodeMeta(item.node) }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <div v-if="viewing" class="viewer-panel">
          <div class="viewer-header">
            <div class="viewer-title-area">
              <span class="viewer-title">{{ viewingFile }}</span>
              <span v-if="fileLanguage && viewingFileType === 'code'" class="viewer-lang-badge">
                {{ fileLanguage }}
              </span>
              <span v-if="viewingNode?.size !== undefined" class="viewer-size">
                {{ formatSize(viewingNode.size) }}
              </span>
            </div>
            <div class="viewer-actions">
              <van-button
                v-if="viewingFileType === 'code'"
                size="mini"
                plain
                @click="copyContent"
              >
                {{ copyBtnText }}
              </van-button>
              <van-button size="mini" plain @click="closeViewer">关闭</van-button>
            </div>
          </div>

          <div v-if="breadcrumbs.length > 1" class="viewer-breadcrumbs">
            <span class="vbc-item" @click="navigateToBreadcrumb(projectStore.activeProject!.path)">
              根目录
            </span>
            <template v-for="(crumb, i) in breadcrumbs.slice(0, -1)" :key="i">
              <van-icon name="arrow" size="10" class="vbc-sep" />
              <span class="vbc-item" @click="navigateToBreadcrumb(crumb.path)">
                {{ crumb.name }}
              </span>
            </template>
          </div>

          <div v-if="viewingFileType === 'code'" class="code-viewer">
            <div v-if="highlightedCode" class="code-body">
              <div class="line-numbers">
                <div v-for="(_, i) in codeLines" :key="i" class="line-number">{{ i + 1 }}</div>
              </div>
              <pre class="code-content"><code v-html="highlightedCode"></code></pre>
            </div>
            <div v-else class="code-loading">
              <van-loading color="var(--accent)" vertical>加载中...</van-loading>
            </div>
          </div>

          <div v-else-if="viewingFileType === 'image'" class="image-viewer">
            <img v-if="imageSrc" :src="imageSrc" :alt="viewingFile" class="preview-image" />
            <div v-else class="binary-notice">
              <van-icon name="photo-o" size="48" color="var(--text-muted)" />
              <p>图片加载失败或文件过大（&gt;2MB）</p>
            </div>
          </div>

          <div v-else class="binary-viewer">
            <div class="binary-notice">
              <van-icon name="warning-o" size="48" color="var(--text-muted)" />
              <p>二进制文件 - 无法预览</p>
              <span v-if="viewingNode?.size !== undefined" class="binary-size">
                文件大小: {{ formatSize(viewingNode.size) }}
              </span>
            </div>
          </div>
        </div>

        <div v-else-if="!isMobile" class="viewer-panel viewer-empty">
          <div class="empty-preview">
            <van-icon name="description" size="48" color="var(--text-muted)" />
            <p>选择文件以预览</p>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.files-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  color: var(--text-primary);
  overflow: hidden;
}

.files-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.page-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
}

.empty-state {
  text-align: center;
  padding: 40px 0;
  color: var(--text-muted);
}

.files-layout {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.tree-panel {
  width: 300px;
  border-right: 1px solid var(--border);
  overflow-y: auto;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
}

.tree-hidden {
  display: none;
}

.search-bar {
  padding: 8px;
  flex-shrink: 0;
}

.dark-field {
  background: var(--bg-card);
  border-radius: 8px;
  padding: 6px 10px;
}

:deep(.dark-field .van-field__control) {
  color: var(--text-primary);
}

:deep(.dark-field .van-field__left-icon) {
  color: var(--text-muted);
}

.breadcrumbs {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 12px 8px;
  font-size: 12px;
  color: var(--text-muted);
  overflow-x: auto;
  flex-shrink: 0;
  flex-wrap: nowrap;
  white-space: nowrap;
}

.breadcrumb-root {
  cursor: pointer;
  display: flex;
  align-items: center;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.breadcrumb-root:hover {
  color: var(--accent);
}

.breadcrumb-sep {
  flex-shrink: 0;
  color: var(--text-muted);
}

.breadcrumb-item {
  cursor: pointer;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.breadcrumb-item:hover {
  color: var(--accent);
}

.breadcrumb-item.active {
  color: var(--accent);
  cursor: default;
}

.tree-content {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow-y: auto;
}

.tree-node {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  padding-right: 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-secondary);
  transition: background 0.15s;
}

.tree-node:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.tree-node.dir {
  color: var(--accent);
  font-weight: 500;
}

.tree-node.file {
  color: var(--text-secondary);
}

.tree-node.active {
  background: var(--bg-card-hover);
  color: var(--accent);
}

.node-icon {
  flex-shrink: 0;
}

.node-info {
  display: flex;
  align-items: baseline;
  gap: 8px;
  overflow: hidden;
  flex: 1;
  min-width: 0;
}

.node-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 1;
  min-width: 0;
}

.node-meta {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
  flex-shrink: 0;
}

.viewer-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.viewer-empty {
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-preview {
  text-align: center;
  color: var(--text-muted);
}

.empty-preview p {
  margin-top: 12px;
  font-size: 14px;
}

.viewer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  gap: 8px;
}

.viewer-title-area {
  display: flex;
  align-items: center;
  gap: 8px;
  overflow: hidden;
  min-width: 0;
}

.viewer-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--accent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.viewer-lang-badge {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 4px;
  background: rgba(66, 165, 245, 0.15);
  color: var(--accent);
  flex-shrink: 0;
}

.viewer-size {
  font-size: 11px;
  color: var(--text-muted);
  flex-shrink: 0;
}

.viewer-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.viewer-breadcrumbs {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 16px;
  font-size: 12px;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  overflow-x: auto;
  white-space: nowrap;
}

.vbc-item {
  cursor: pointer;
  color: var(--text-secondary);
}

.vbc-item:hover {
  color: var(--accent);
}

.vbc-sep {
  flex-shrink: 0;
  color: var(--text-muted);
}

.code-viewer {
  flex: 1;
  overflow: auto;
}

.code-body {
  display: flex;
  min-height: 100%;
}

.line-numbers {
  padding: 16px 0;
  text-align: right;
  user-select: none;
  flex-shrink: 0;
  border-right: 1px solid var(--border);
  background: var(--bg-secondary);
}

.line-number {
  padding: 0 12px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-muted);
}

.code-content {
  flex: 1;
  padding: 16px;
  margin: 0;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  line-height: 1.6;
  overflow-x: auto;
  white-space: pre;
}

.code-content :deep(code) {
  font-family: inherit;
  font-size: inherit;
  line-height: inherit;
  white-space: inherit;
  background: none;
  padding: 0;
}

.code-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
}

.image-viewer {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: auto;
  padding: 16px;
}

.preview-image {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 6px;
}

.binary-viewer {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
}

.binary-notice {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: var(--text-muted);
  text-align: center;
}

.binary-notice p {
  font-size: 14px;
  margin: 0;
}

.binary-size {
  font-size: 12px;
  color: var(--text-muted);
}

.files-view.mobile .tree-panel {
  width: 100%;
  border-right: none;
}

.files-view.mobile .viewer-panel {
  width: 100%;
}

.files-view.mobile .node-meta {
  display: none;
}

:deep(.van-button--primary) {
  background: var(--accent);
  border-color: var(--accent);
}
</style>
