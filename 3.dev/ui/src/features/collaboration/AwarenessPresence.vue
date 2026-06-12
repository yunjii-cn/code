<script setup lang="ts">
/**
 * AwarenessPresence — 在线协作者列表（人 + AI）
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 */
import { computed } from 'vue'
import { useCollaborationStore } from '@/stores/collaboration'

const store = useCollaborationStore()

const humans = computed(() => store.humanUsers)
const ais = computed(() => store.aiActivities)

function _activityLabel(a: { type: string; description: string }): string {
  if (a.description) return a.description
  switch (a.type) {
    case 'idle': return '空闲'
    case 'thinking': return '思考中…'
    case 'streaming': return '正在回复…'
    case 'reviewing': return '正在评审…'
    case 'patching': return '正在写补丁…'
    case 'branching': return '正在创建分支…'
    case 'merging': return '正在合并…'
    default: return a.type
  }
}
</script>

<template>
  <div class="presence-panel">
    <div class="presence-header">
      <span class="dot" :class="store.connectionStatus"></span>
      <span class="status-text">
        {{
          store.connectionStatus === 'connected' ? '已连接' :
          store.connectionStatus === 'connecting' ? '连接中…' :
          store.connectionStatus === 'offline' ? '离线（本地可用）' :
          '未连接'
        }}
      </span>
    </div>

    <div class="presence-section">
      <div class="section-label">AI 协作者 ({{ ais.length }})</div>
      <div v-if="ais.length === 0" class="empty">暂无 AI 在线</div>
      <div v-else class="avatar-list">
        <div
          v-for="ai in ais"
          :key="ai.clientId"
          class="avatar-item ai"
          :title="`${ai.user.name} · ${_activityLabel(ai.activity)}`"
        >
          <div class="avatar-circle" :style="{ background: ai.user.color }">
            {{ ai.user.avatar || ai.user.name.slice(0, 1) }}
          </div>
          <div class="avatar-meta">
            <div class="name">{{ ai.user.name }}</div>
            <div class="activity">{{ _activityLabel(ai.activity) }}</div>
          </div>
        </div>
      </div>
    </div>

    <div class="presence-section">
      <div class="section-label">人类协作者 ({{ humans.length }})</div>
      <div v-if="humans.length === 0" class="empty">等待其他人加入…</div>
      <div v-else class="avatar-list">
        <div
          v-for="h in humans"
          :key="h.clientId"
          class="avatar-item"
          :title="h.user.name"
        >
          <div class="avatar-circle" :style="{ background: h.user.color }">
            {{ h.user.name.slice(0, 1) }}
          </div>
          <div class="avatar-meta">
            <div class="name">{{ h.user.name }}</div>
            <div v-if="h.activity.type !== 'idle'" class="activity">
              {{ _activityLabel(h.activity) }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.presence-panel {
  padding: 12px;
  border-bottom: 1px solid var(--yj-border, #2a2a2a);
}
.presence-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  font-size: 13px;
  color: var(--yj-text-secondary, #aaa);
}
.dot {
  width: 8px; height: 8px; border-radius: 50%;
  background: #555;
}
.dot.connected { background: #22c55e; box-shadow: 0 0 8px #22c55e66; }
.dot.connecting { background: #eab308; animation: pulse 1s infinite; }
.dot.offline { background: #6b7280; }
.dot.disconnected { background: #ef4444; }
@keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.4; } }
.section-label {
  font-size: 11px;
  text-transform: uppercase;
  color: var(--yj-text-muted, #666);
  margin: 8px 0;
  letter-spacing: 0.05em;
}
.empty { font-size: 12px; color: var(--yj-text-muted, #666); padding: 4px 0; }
.avatar-list { display: flex; flex-direction: column; gap: 8px; }
.avatar-item {
  display: flex; align-items: center; gap: 10px;
  padding: 4px 6px; border-radius: 6px;
  transition: background 0.15s;
}
.avatar-item:hover { background: var(--yj-hover, rgba(255,255,255,0.05)); }
.avatar-item.ai {
  background: linear-gradient(90deg, rgba(124,58,237,0.08), transparent);
  border-left: 2px solid #7c3aed;
  padding-left: 8px;
}
.avatar-circle {
  width: 28px; height: 28px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  color: white; font-weight: 600; font-size: 12px;
  flex-shrink: 0;
}
.avatar-meta { flex: 1; min-width: 0; }
.avatar-meta .name { font-size: 13px; color: var(--yj-text, #eee); }
.avatar-meta .activity { font-size: 11px; color: var(--yj-text-secondary, #aaa); }
</style>
