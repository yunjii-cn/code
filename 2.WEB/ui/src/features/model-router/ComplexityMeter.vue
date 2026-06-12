<script setup lang="ts">
/**
 * ComplexityMeter — 复杂度可视化进度条
 * 2026-06-09 TASK-4.6
 *
 * Props: score (ComplexityScore | null)
 * 显示：总分 + 4 个子分（length/keywords/structure/history）+ 触发关键词
 */
import { computed } from 'vue'
import type { ComplexityScore } from '@/api'

const props = defineProps<{
  score: ComplexityScore | null
}>()

const percent = computed(() => {
  if (!props.score) return 0
  return Math.round(props.score.total * 100)
})

const tier = computed(() => {
  if (!props.score) return { name: '-', color: '#666' }
  const t = props.score.total
  if (t < 0.3) return { name: 'simple', color: '#10b981', label: '简单' }
  if (t < 0.7) return { name: 'medium', color: '#f59e0b', label: '中等' }
  return { name: 'complex', color: '#ef4444', label: '复杂' }
})

interface BarInfo {
  label: string
  value: number
  color: string
  max: number
}

const bars = computed<BarInfo[]>(() => {
  if (!props.score) {
    return [
      { label: '长度', value: 0, color: '#3b82f6', max: 0.4 },
      { label: '关键词', value: 0, color: '#8b5cf6', max: 0.4 },
      { label: '结构', value: 0, color: '#06b6d4', max: 0.2 },
      { label: '会话', value: 0, color: '#f59e0b', max: 0.1 },
    ]
  }
  const s = props.score
  return [
    { label: '长度', value: Math.round(s.length * 100), color: '#3b82f6', max: 0.4 },
    { label: '关键词', value: Math.round(s.keywords * 100), color: '#8b5cf6', max: 0.4 },
    { label: '结构', value: Math.round(s.structure * 100), color: '#06b6d4', max: 0.2 },
    { label: '会话', value: Math.round(s.history * 100), color: '#f59e0b', max: 0.1 },
  ]
})
</script>

<template>
  <div class="complexity-meter" v-if="score">
    <!-- 总分 -->
    <div class="meter-header">
      <div class="meter-tier-badge" :style="{ background: tier.color }">
        {{ tier.label }}
      </div>
      <div class="meter-info">
        <div class="meter-total">
          <span class="total-num">{{ percent }}</span>
          <span class="total-unit">%</span>
        </div>
        <div class="meter-tier">tier: {{ tier.name }}</div>
      </div>
      <div class="meter-rationale">
        <div class="rationale-label">评估</div>
        <div class="rationale-bars">
          <div
            v-for="b in bars"
            :key="b.label"
            class="bar-row"
          >
            <div class="bar-label">{{ b.label }}</div>
            <div class="bar-track">
              <div
                class="bar-fill"
                :style="{
                  width: b.value + '%',
                  background: b.color,
                }"
              />
              <div
                class="bar-max"
                :style="{ left: ((b.max / 1) * 100) + '%' }"
              />
            </div>
            <div class="bar-value">{{ b.value }}%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 关键词 / 信号 -->
    <div v-if="score.matched_keywords.length || score.signals.length" class="meter-tags">
      <div v-if="score.matched_keywords.length" class="tag-group">
        <span class="tag-label">关键词：</span>
        <span v-for="kw in score.matched_keywords" :key="kw" class="keyword-chip">#{{ kw }}</span>
      </div>
      <div v-if="score.signals.length" class="tag-group">
        <span class="tag-label">信号：</span>
        <span v-for="s in score.signals" :key="s" class="signal-chip">{{ s }}</span>
      </div>
    </div>
  </div>
  <div v-else class="meter-empty">
    <div class="empty-text">输入 prompt 后点击"评估"</div>
  </div>
</template>

<style scoped>
.complexity-meter {
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 10px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.meter-empty {
  background: var(--yj-bg-elevated, #1a1a1a);
  border: 1px dashed var(--yj-border, #2a2a2a);
  border-radius: 10px;
  padding: 30px;
  text-align: center;
  color: var(--yj-text-secondary, #888);
}

.meter-header {
  display: grid;
  grid-template-columns: auto auto 1fr;
  gap: 16px;
  align-items: center;
}

.meter-tier-badge {
  padding: 8px 14px;
  border-radius: 8px;
  color: #fff;
  font-size: 14px;
  font-weight: 600;
  min-width: 60px;
  text-align: center;
}

.meter-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.meter-total {
  display: flex;
  align-items: baseline;
  gap: 2px;
  font-family: 'Cascadia Code', 'Consolas', monospace;
  color: var(--yj-text-primary, #f5f5f5);
}

.total-num {
  font-size: 28px;
  font-weight: 700;
}

.total-unit {
  font-size: 14px;
  color: var(--yj-text-secondary, #888);
}

.meter-tier {
  font-size: 11px;
  color: var(--yj-text-secondary, #888);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.meter-rationale {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.rationale-label {
  font-size: 11px;
  color: var(--yj-text-secondary, #888);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.rationale-bars {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.bar-row {
  display: grid;
  grid-template-columns: 50px 1fr 50px;
  align-items: center;
  gap: 8px;
}

.bar-label {
  font-size: 11px;
  color: var(--yj-text-secondary, #aaa);
}

.bar-track {
  position: relative;
  height: 6px;
  background: var(--yj-bg-base, #0d0d0d);
  border-radius: 3px;
  overflow: hidden;
}

.bar-fill {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s ease;
}

.bar-max {
  position: absolute;
  top: -1px;
  width: 1px;
  height: 8px;
  background: var(--yj-text-secondary, #666);
  opacity: 0.4;
}

.bar-value {
  font-size: 11px;
  color: var(--yj-text-secondary, #aaa);
  text-align: right;
  font-family: 'Cascadia Code', 'Consolas', monospace;
}

.meter-tags {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-top: 10px;
  border-top: 1px solid var(--yj-border, #2a2a2a);
}

.tag-group {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
}

.tag-label {
  font-size: 11px;
  color: var(--yj-text-secondary, #888);
  margin-right: 4px;
}

.keyword-chip,
.signal-chip {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  background: rgba(139, 92, 246, 0.12);
  color: #a78bfa;
}

.signal-chip {
  background: rgba(6, 182, 212, 0.12);
  color: #67e8f9;
}
</style>
