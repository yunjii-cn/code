<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { showToast } from 'vant'
import { useDevice } from '@/composables/useDevice'
import { useProjectStore } from '@/stores/project'
import { projectApi } from '@/api'

const { isMobile } = useDevice()
const projectStore = useProjectStore()

const projectClaudeMd = ref('')
const globalClaudeMd = ref('')
const activeTab = ref<'project' | 'global'>('project')
const saving = ref(false)

async function loadClaudeMd() {
  const project = projectStore.activeProject
  if (project) {
    try {
      const data: any = await projectApi.getClaudeMd(project.id)
      projectClaudeMd.value = data?.content || ''
    } catch {
      projectClaudeMd.value = ''
    }
  }
  try {
    const data: any = await projectApi.getGlobalClaudeMd()
    globalClaudeMd.value = data?.content || ''
  } catch {
    globalClaudeMd.value = ''
  }
}

async function saveClaudeMd() {
  saving.value = true
  try {
    if (activeTab.value === 'project' && projectStore.activeProject) {
      await projectApi.saveClaudeMd({
        project_id: projectStore.activeProject.id,
        content: projectClaudeMd.value,
      })
    } else {
      await projectApi.saveGlobalClaudeMd(globalClaudeMd.value)
    }
    showToast('保存成功')
  } catch {
    showToast('保存失败')
  } finally {
    saving.value = false
  }
}

onMounted(loadClaudeMd)
</script>

<template>
  <div class="claudemd-view" :class="{ mobile: isMobile }">
    <div class="page-header">
      <h2 class="page-title">CLAUDE.md 编辑器</h2>
      <van-button size="small" type="primary" :loading="saving" @click="saveClaudeMd">保存</van-button>
    </div>

    <div class="tab-bar">
      <div
        class="tab-item"
        :class="{ active: activeTab === 'project' }"
        @click="activeTab = 'project'"
      >
        项目级
      </div>
      <div
        class="tab-item"
        :class="{ active: activeTab === 'global' }"
        @click="activeTab = 'global'"
      >
        全局
      </div>
    </div>

    <div class="editor-area">
      <div v-if="activeTab === 'project'" class="editor-hint">
        <span v-if="projectStore.activeProject">
          当前项目: {{ projectStore.activeProject.name }}
        </span>
        <span v-else class="hint-warn">请先选择一个项目</span>
      </div>
      <van-field
        v-model="projectClaudeMd"
        v-show="activeTab === 'project'"
        type="textarea"
        :rows="20"
        placeholder="在此编写项目级 CLAUDE.md..."
        class="md-editor"
        autosize
      />
      <van-field
        v-model="globalClaudeMd"
        v-show="activeTab === 'global'"
        type="textarea"
        :rows="20"
        placeholder="在此编写全局 CLAUDE.md..."
        class="md-editor"
        autosize
      />
    </div>
  </div>
</template>

<style scoped>
.claudemd-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  color: var(--text-primary);
  overflow: hidden;
}

.page-header {
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

.tab-bar {
  display: flex;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.tab-item {
  padding: 10px 20px;
  font-size: 14px;
  color: var(--text-muted);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
}

.tab-item.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.tab-item:hover {
  color: var(--text-primary);
}

.editor-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.editor-hint {
  padding: 8px 16px;
  font-size: 12px;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border);
}

.hint-warn {
  color: var(--warning);
}

.md-editor {
  flex: 1;
  background: var(--bg-primary);
}

:deep(.md-editor .van-field__control) {
  color: var(--text-primary);
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  line-height: 1.7;
  min-height: 400px;
}

:deep(.van-button--primary) {
  background: var(--accent);
  border-color: var(--accent);
}
</style>
