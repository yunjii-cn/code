<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { showToast, showConfirmDialog } from 'vant'
import { useDevice } from '@/composables/useDevice'
import { systemApi } from '@/api'

const { isMobile } = useDevice()

interface Plugin {
  id: string
  name: string
  version: string
  description: string
  enabled: boolean
}

const plugins = ref<Plugin[]>([])
const loading = ref(false)
const installJson = ref('')
const showInstall = ref(false)

async function loadPlugins() {
  loading.value = true
  try {
    const data: any = await systemApi.listPlugins()
    plugins.value = (data?.plugins || data || []).map((p: any) => ({
      id: p.id || p.name,
      name: p.name || p.id,
      version: p.version || '1.0.0',
      description: p.description || '',
      enabled: p.enabled !== false,
    }))
  } catch {
    plugins.value = []
  } finally {
    loading.value = false
  }
}

async function installPlugin() {
  if (!installJson.value.trim()) {
    showToast('请输入插件JSON')
    return
  }
  try {
    await systemApi.installPlugin(installJson.value.trim())
    showToast('安装成功')
    showInstall.value = false
    installJson.value = ''
    await loadPlugins()
  } catch (e: any) {
    showToast(`安装失败: ${e.message}`)
  }
}

async function uninstallPlugin(id: string, name: string) {
  try {
    await showConfirmDialog({
      title: '卸载插件',
      message: `确定要卸载「${name}」吗？`,
      confirmButtonColor: 'var(--error)',
    })
    await systemApi.uninstallPlugin(id)
    showToast('已卸载')
    await loadPlugins()
  } catch {
    // 用户取消
  }
}

async function executePlugin(id: string) {
  try {
    await systemApi.executePlugin(id)
    showToast('执行成功')
  } catch (e: any) {
    showToast(`执行失败: ${e.message}`)
  }
}

onMounted(loadPlugins)
</script>

<template>
  <div class="plugin-view" :class="{ mobile: isMobile }">
    <div class="page-header">
      <h2 class="page-title">插件管理</h2>
      <van-button size="small" type="primary" @click="showInstall = true">安装插件</van-button>
    </div>

    <van-loading v-if="loading" class="page-loading" color="var(--accent)" vertical>加载中...</van-loading>

    <div v-else class="plugin-list">
      <div v-for="p in plugins" :key="p.id" class="plugin-card">
        <div class="plugin-info">
          <div class="plugin-name-row">
            <span class="plugin-name">{{ p.name }}</span>
            <span class="plugin-version">v{{ p.version }}</span>
          </div>
          <p class="plugin-desc">{{ p.description || '无描述' }}</p>
        </div>
        <div class="plugin-actions">
          <van-button size="mini" plain @click="executePlugin(p.id)">执行</van-button>
          <van-button size="mini" plain color="var(--error)" @click="uninstallPlugin(p.id, p.name)">
            卸载
          </van-button>
        </div>
      </div>
      <div v-if="plugins.length === 0" class="empty-hint">暂无已安装插件</div>
    </div>

    <van-popup v-model:show="showInstall" position="bottom" round :style="{ maxHeight: '60%' }">
      <div class="install-sheet">
        <div class="sheet-title">安装插件</div>
        <van-field
          v-model="installJson"
          type="textarea"
          :rows="6"
          placeholder="粘贴插件JSON元数据..."
          class="dark-field"
        />
        <van-button type="primary" block @click="installPlugin" style="margin-top: 12px">
          安装
        </van-button>
      </div>
    </van-popup>
  </div>
</template>

<style scoped>
.plugin-view {
  height: 100%;
  overflow-y: auto;
  padding: 24px;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.plugin-view.mobile {
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

.plugin-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.plugin-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
}

.plugin-info {
  flex: 1;
  min-width: 0;
}

.plugin-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.plugin-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.plugin-version {
  font-size: 11px;
  color: var(--accent);
  background: rgba(66, 165, 245, 0.1);
  padding: 1px 8px;
  border-radius: 8px;
}

.plugin-desc {
  font-size: 12px;
  color: var(--text-muted);
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.empty-hint {
  text-align: center;
  padding: 30px 0;
  color: var(--text-muted);
}

.install-sheet {
  padding: 20px 16px;
  background: var(--bg-card);
}

.sheet-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 12px;
}

.dark-field {
  background: var(--bg-primary);
  border-radius: 8px;
}

:deep(.dark-field .van-field__control) {
  color: var(--text-primary);
  font-family: 'Consolas', monospace;
  font-size: 13px;
}

:deep(.van-button--primary) {
  background: var(--accent);
  border-color: var(--accent);
}
</style>
