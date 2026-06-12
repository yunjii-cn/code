<script setup lang="ts">
/**
 * SkillCard — 技能市场卡片
 * 2026-06-09 TASK-4.5
 *
 * Props: skill, isInstalled, isInstalling
 * Emits: install, uninstall, view
 */
import { computed } from 'vue'
import { showToast } from 'vant'
import type { Skill } from '@/api'

const props = defineProps<{
  skill: Skill
  isInstalled?: boolean
  isInstalling?: boolean
}>()

const emit = defineEmits<{
  install: [skill: Skill]
  uninstall: [skill: Skill]
  view: [skill: Skill]
}>()

const ratingText = computed(() => {
  const r = props.skill.rating ?? 0
  return r.toFixed(1)
})

const ratingCountText = computed(() => {
  const n = props.skill.rating_count ?? 0
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return String(n)
})

const downloadsText = computed(() => {
  const n = props.skill.downloads ?? 0
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return String(n)
})

function onInstall(e: Event) {
  e.stopPropagation()
  if (props.isInstalling) return
  emit('install', props.skill)
}

function onUninstall(e: Event) {
  e.stopPropagation()
  if (props.isInstalling) return
  emit('uninstall', props.skill)
}

function onCardClick() {
  emit('view', props.skill)
}

function copyId(e: Event) {
  e.stopPropagation()
  try {
    navigator.clipboard?.writeText(props.skill.id)
    showToast('已复制 ID')
  } catch {
    showToast('复制失败')
  }
}
</script>

<template>
  <div class="skill-card" :class="{ 'is-installed': isInstalled }" @click="onCardClick">
    <div class="card-header">
      <div class="card-icon" :title="skill.category">
        <span class="icon-emoji">{{ (skill.icon ? '' : (skill.tags?.[0]?.[0] || '⚡').toUpperCase()) || '⚡' }}</span>
      </div>
      <div class="card-title-area">
        <div class="card-title">{{ skill.name }}</div>
        <div class="card-meta">
          <span class="meta-version">v{{ skill.version }}</span>
          <span class="meta-author">@{{ skill.author || 'YunJi' }}</span>
        </div>
      </div>
      <div class="card-rating">
        <div class="rating-score">⭐ {{ ratingText }}</div>
        <div class="rating-count">({{ ratingCountText }})</div>
      </div>
    </div>

    <div class="card-description">{{ skill.description }}</div>

    <div class="card-tags">
      <span v-for="tag in (skill.tags || []).slice(0, 4)" :key="tag" class="tag-chip" @click.stop>
        #{{ tag }}
      </span>
    </div>

    <div class="card-footer">
      <div class="card-stats">
        <span class="stat">📦 {{ downloadsText }}</span>
        <span class="stat-id" :title="`点击复制 ID: ${skill.id}`" @click="copyId">🆔 {{ skill.id }}</span>
      </div>
      <div class="card-actions">
        <button v-if="!isInstalled" class="btn btn-install" :disabled="isInstalling" @click="onInstall">
          {{ isInstalling ? '安装中…' : '安装' }}
        </button>
        <button v-else class="btn btn-uninstall" :disabled="isInstalling" @click="onUninstall">
          {{ isInstalling ? '处理中…' : '已安装' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.skill-card {
  display: flex;
  flex-direction: column;
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 10px;
  padding: 14px;
  cursor: pointer;
  transition: all 0.2s ease;
  gap: 10px;
  position: relative;
}

.skill-card:hover {
  border-color: var(--yj-accent, #7c3aed);
  transform: translateY(-2px);
  box-shadow: 0 4px 16px rgba(124, 58, 237, 0.15);
}

.skill-card.is-installed {
  border-color: var(--yj-success, #10b981);
  background: linear-gradient(135deg, var(--yj-bg-elevated, #1a1a1a), rgba(16, 185, 129, 0.04));
}

.card-header {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.card-icon {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  background: linear-gradient(135deg, var(--yj-accent, #7c3aed), #a855f7);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 20px;
}

.icon-emoji {
  color: #fff;
  font-weight: bold;
}

.card-title-area {
  flex: 1;
  min-width: 0;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--yj-text-primary, #f5f5f5);
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-meta {
  display: flex;
  gap: 6px;
  font-size: 12px;
  color: var(--yj-text-secondary, #888);
  margin-top: 2px;
}

.card-rating {
  text-align: right;
  flex-shrink: 0;
}

.rating-score {
  font-size: 13px;
  font-weight: 600;
  color: #fbbf24;
}

.rating-count {
  font-size: 11px;
  color: var(--yj-text-secondary, #888);
}

.card-description {
  font-size: 13px;
  color: var(--yj-text-secondary, #aaa);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  min-height: 38px;
}

.card-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.tag-chip {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  background: rgba(124, 58, 237, 0.12);
  color: var(--yj-accent, #a855f7);
}

.card-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-top: 8px;
  border-top: 1px solid var(--yj-border, #2a2a2a);
  margin-top: auto;
}

.card-stats {
  display: flex;
  gap: 10px;
  font-size: 12px;
  color: var(--yj-text-secondary, #888);
  min-width: 0;
  overflow: hidden;
}

.stat-id {
  font-family: 'Cascadia Code', 'Consolas', monospace;
  cursor: pointer;
  user-select: none;
}

.stat-id:hover {
  color: var(--yj-accent, #a855f7);
}

.card-actions {
  flex-shrink: 0;
}

.btn {
  font-size: 12px;
  padding: 5px 12px;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s ease;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-install {
  background: var(--yj-accent, #7c3aed);
  color: #fff;
}

.btn-install:hover:not(:disabled) {
  background: #6d28d9;
}

.btn-uninstall {
  background: rgba(16, 185, 129, 0.15);
  color: var(--yj-success, #10b981);
  border: 1px solid var(--yj-success, #10b981);
}

.btn-uninstall:hover:not(:disabled) {
  background: rgba(16, 185, 129, 0.25);
}
</style>
