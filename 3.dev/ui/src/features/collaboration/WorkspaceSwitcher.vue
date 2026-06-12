<script setup lang="ts">
/**
 * WorkspaceSwitcher — 顶部工作区选择器
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 */
import { ref, onMounted } from 'vue'
import { showToast, showDialog } from 'vant'
import { useCollaborationStore } from '@/stores/collaboration'
import { workspaceApi } from '@/api'
import type { Workspace as ApiWorkspace } from '@/api'

const store = useCollaborationStore()
const showPicker = ref(false)
const workspaces = ref<ApiWorkspace[]>([])
const loading = ref(false)
const newName = ref('')

async function loadList() {
  if (!store.tokens?.access_token) return
  loading.value = true
  try {
    const r = await workspaceApi.list(store.tokens.access_token)
    workspaces.value = r.data
  } catch (e) {
    showToast({ type: 'fail', message: '加载工作区失败' })
  } finally {
    loading.value = false
  }
}

async function onSelect(ws: ApiWorkspace) {
  showPicker.value = false
  if (ws.id === store.activeWorkspaceId) return
  await store.switchWorkspace(ws.id)
  showToast({ type: 'success', message: `已切换到 ${ws.name}` })
  await store.connect()
}

async function onCreate() {
  if (!newName.value.trim()) {
    showToast('请输入工作区名')
    return
  }
  try {
    const ws = await store.createWorkspace(newName.value.trim(), '')
    showToast({ type: 'success', message: '工作区已创建' })
    newName.value = ''
    await loadList()
    await onSelect(ws)
  } catch (e) {
    const err = e instanceof Error ? e.message : String(e)
    showToast({ type: 'fail', message: err })
  }
}

async function onDelete(ws: ApiWorkspace) {
  try {
    const action = await showDialog({
      title: '删除工作区',
      message: `确认删除工作区"${ws.name}"吗？此操作不可恢复。`,
      showCancelButton: true,
    })
    if (action !== 'confirm') return
    // 注：后端尚未实现 DELETE /workspaces/:id，先给提示
    showToast('删除功能开发中')
  } catch { /* 取消 */ }
}

onMounted(() => {
  if (store.isAuthenticated) loadList()
})

defineExpose({ reload: loadList })
</script>

<template>
  <div class="ws-switcher">
    <van-button
      size="small"
      plain
      hairline
      icon="cluster-o"
      @click="showPicker = true"
    >
      {{ workspaces.find((w: ApiWorkspace) => w.id === store.activeWorkspaceId)?.name || '选择工作区' }}
    </van-button>

    <van-popup v-model:show="showPicker" position="bottom" round :style="{ maxHeight: '70vh' }">
      <div class="picker-content">
        <div class="picker-header">
          <span>切换工作区</span>
          <van-icon name="cross" @click="showPicker = false" />
        </div>
        <van-loading v-if="loading" size="20" style="margin: 20px auto;" />
        <van-cell-group v-else>
          <van-cell
            v-for="ws in workspaces"
            :key="ws.id"
            :title="ws.name"
            :label="`${ws.role} · ${new Date(ws.created_at).toLocaleDateString()}`"
            :icon="ws.id === store.activeWorkspaceId ? 'success' : ''"
            is-link
            @click="onSelect(ws)"
          >
            <template #right-icon>
              <van-icon
                v-if="ws.role === 'owner'"
                name="delete-o"
                @click.stop="onDelete(ws)"
                style="margin-left: 8px; color: #ef4444;"
              />
            </template>
          </van-cell>
        </van-cell-group>

        <div class="new-ws">
          <van-field
            v-model="newName"
            placeholder="新工作区名称"
            clearable
          />
          <van-button type="primary" size="small" @click="onCreate">创建</van-button>
        </div>
      </div>
    </van-popup>
  </div>
</template>

<style scoped>
.ws-switcher { display: inline-block; }
.picker-content { padding: 16px; }
.picker-header {
  display: flex; justify-content: space-between; align-items: center;
  font-size: 16px; font-weight: 600;
  padding: 8px 0 16px;
  border-bottom: 1px solid var(--yj-border, #eee);
  margin-bottom: 12px;
}
.new-ws {
  display: flex; gap: 8px; align-items: center;
  margin-top: 16px; padding-top: 16px;
  border-top: 1px solid var(--yj-border, #eee);
}
</style>
