<script setup lang="ts">
/**
 * InstalledSkillsPanel — 已安装技能列表
 * 2026-06-09 TASK-4.5
 *
 * 展示全局 + 项目级已安装技能，支持卸载、应用、查看
 */
import { computed } from 'vue'
import { showDialog, showToast } from 'vant'
import type { InstalledSkill } from '@/api'

const props = defineProps<{
  globalList: InstalledSkill[]
  projectList: InstalledSkill[]
  busyIds?: Set<string>
}>()

const emit = defineEmits<{
  uninstall: [skill: InstalledSkill, scope: 'global' | 'project']
  apply: [skill: InstalledSkill, scope: 'global' | 'project']
  refresh: []
  exportSkill: [skill: InstalledSkill, scope: 'global' | 'project']
}>()

const hasAny = computed(() => props.globalList.length > 0 || props.projectList.length > 0)

function isBusy(id: string): boolean {
  return props.busyIds?.has(id) ?? false
}

async function onUninstall(skill: InstalledSkill, scope: 'global' | 'project') {
  if (isBusy(skill.id)) return
  try {
    await showDialog({
      title: '确认卸载',
      message: `确定要从${scope === 'global' ? '全局' : '项目'}目录卸载 "${skill.id}" 吗？`,
      showCancelButton: true,
    })
  } catch {
    return
  }
  emit('uninstall', skill, scope)
}

function onApply(skill: InstalledSkill, scope: 'global' | 'project') {
  emit('apply', skill, scope)
  showToast(`已应用 ${skill.name || skill.id} 的模板`)
}

function onExport(skill: InstalledSkill, scope: 'global' | 'project') {
  emit('exportSkill', skill, scope)
}

function formatTime(ts?: number) {
  if (!ts) return ''
  const d = new Date(ts * 1000)
  return d.toLocaleString('zh-CN', { hour12: false })
}
</script>

<template>
  <div class="installed-panel">
    <div class="panel-header">
      <div class="panel-title">
        <span class="title-icon">📦</span>
        <span>已安装技能</span>
        <span class="title-count">{{ globalList.length + projectList.length }}</span>
      </div>
      <button class="btn-refresh" @click="emit('refresh')">🔄 刷新</button>
    </div>

    <div v-if="!hasAny" class="panel-empty">
      <div class="empty-icon">📭</div>
      <div class="empty-text">还没有安装任何技能</div>
      <div class="empty-hint">去市场挑一个喜欢的吧 →</div>
    </div>

    <div v-else class="panel-body">
      <!-- 全局 -->
      <div v-if="globalList.length" class="scope-section">
        <div class="scope-title">
          <span>🌐 全局</span>
          <span class="scope-count">{{ globalList.length }}</span>
        </div>
        <div class="scope-list">
          <div
            v-for="s in globalList"
            :key="`g-${s.id}`"
            class="installed-item"
          >
            <div class="item-main">
              <div class="item-name">{{ s.name || s.id }}</div>
              <div class="item-meta">
                <span class="meta-id">{{ s.id }}</span>
                <span v-if="s.version" class="meta-version">v{{ s.version }}</span>
                <span v-if="s.installed_at" class="meta-time">{{ formatTime(s.installed_at) }}</span>
              </div>
              <div v-if="s.description" class="item-desc">{{ s.description }}</div>
            </div>
            <div class="item-actions">
              <button class="mini-btn primary" :disabled="isBusy(s.id)" @click="onApply(s, 'global')">应用</button>
              <button class="mini-btn" :disabled="isBusy(s.id)" @click="onExport(s, 'global')">导出</button>
              <button class="mini-btn danger" :disabled="isBusy(s.id)" @click="onUninstall(s, 'global')">卸载</button>
            </div>
          </div>
        </div>
      </div>

      <!-- 项目级 -->
      <div v-if="projectList.length" class="scope-section">
        <div class="scope-title">
          <span>📁 项目</span>
          <span class="scope-count">{{ projectList.length }}</span>
        </div>
        <div class="scope-list">
          <div
            v-for="s in projectList"
            :key="`p-${s.id}`"
            class="installed-item"
          >
            <div class="item-main">
              <div class="item-name">{{ s.name || s.id }}</div>
              <div class="item-meta">
                <span class="meta-id">{{ s.id }}</span>
                <span v-if="s.version" class="meta-version">v{{ s.version }}</span>
                <span v-if="s.installed_at" class="meta-time">{{ formatTime(s.installed_at) }}</span>
              </div>
              <div v-if="s.description" class="item-desc">{{ s.description }}</div>
            </div>
            <div class="item-actions">
              <button class="mini-btn primary" :disabled="isBusy(s.id)" @click="onApply(s, 'project')">应用</button>
              <button class="mini-btn" :disabled="isBusy(s.id)" @click="onExport(s, 'project')">导出</button>
              <button class="mini-btn danger" :disabled="isBusy(s.id)" @click="onUninstall(s, 'project')">卸载</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.installed-panel {
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  max-height: 600px;
  overflow: hidden;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 14px;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
  flex-shrink: 0;
}

.panel-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  font-weight: 600;
  color: var(--yj-text-primary, #f5f5f5);
}

.title-icon {
  font-size: 16px;
}

.title-count {
  font-size: 11px;
  padding: 1px 8px;
  background: var(--yj-accent, #7c3aed);
  color: #fff;
  border-radius: 8px;
  font-weight: 500;
}

.btn-refresh {
  background: transparent;
  border: 1px solid var(--yj-border, #2a2a2a);
  color: var(--yj-text-secondary, #aaa);
  font-size: 12px;
  padding: 4px 10px;
  border-radius: 4px;
  cursor: pointer;
}

.btn-refresh:hover {
  border-color: var(--yj-accent, #7c3aed);
  color: var(--yj-text-primary, #ddd);
}

.panel-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  color: var(--yj-text-secondary, #888);
}

.empty-icon {
  font-size: 40px;
  margin-bottom: 8px;
  opacity: 0.4;
}

.empty-text {
  font-size: 14px;
  margin-bottom: 4px;
}

.empty-hint {
  font-size: 12px;
  color: var(--yj-accent, #a855f7);
}

.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.scope-section {
  margin-bottom: 8px;
}

.scope-title {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  font-size: 12px;
  font-weight: 600;
  color: var(--yj-text-secondary, #888);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  background: var(--yj-bg-base, #0d0d0d);
}

.scope-count {
  font-size: 10px;
  padding: 1px 6px;
  background: var(--yj-border, #2a2a2a);
  color: var(--yj-text-secondary, #aaa);
  border-radius: 8px;
  font-weight: 500;
}

.scope-list {
  padding: 0 8px;
}

.installed-item {
  display: flex;
  gap: 10px;
  padding: 10px;
  border-radius: 6px;
  margin: 4px 0;
  background: var(--yj-bg-base, #0d0d0d);
  transition: background 0.15s;
}

.installed-item:hover {
  background: var(--yj-bg-hover, #222);
}

.item-main {
  flex: 1;
  min-width: 0;
}

.item-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--yj-text-primary, #f5f5f5);
}

.item-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  font-size: 11px;
  color: var(--yj-text-secondary, #888);
  margin-top: 2px;
}

.meta-id {
  font-family: 'Cascadia Code', 'Consolas', monospace;
}

.meta-time {
  color: var(--yj-text-secondary, #666);
}

.item-desc {
  font-size: 12px;
  color: var(--yj-text-secondary, #aaa);
  margin-top: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
}

.item-actions {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex-shrink: 0;
}

.mini-btn {
  font-size: 11px;
  padding: 3px 10px;
  border-radius: 4px;
  border: 1px solid var(--yj-border, #2a2a2a);
  background: var(--yj-bg-elevated, #1a1a1a);
  color: var(--yj-text-primary, #ddd);
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s;
}

.mini-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.mini-btn:hover:not(:disabled) {
  border-color: var(--yj-accent, #7c3aed);
}

.mini-btn.primary {
  background: var(--yj-accent, #7c3aed);
  color: #fff;
  border-color: var(--yj-accent, #7c3aed);
}

.mini-btn.primary:hover:not(:disabled) {
  background: #6d28d9;
}

.mini-btn.danger {
  color: #ef4444;
  border-color: rgba(239, 68, 68, 0.3);
}

.mini-btn.danger:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.1);
  border-color: #ef4444;
}
</style>
