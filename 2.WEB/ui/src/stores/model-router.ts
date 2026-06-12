/**
 * model-router store — 智能模型路由 Pinia 状态
 *
 * 2026-06-09 TASK-4.6 引入
 *
 * 职责：
 *  - 缓存路由规则（current + defaults）
 *  - 缓存历史
 *  - 提供 evaluate / decide / saveRules / reset / clearHistory 等动作
 *  - 维护最后一次决策（用于 UI 即时反馈）
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  RoutingRules,
  RoutingDecision,
  RoutingHistoryItem,
  ComplexityScore,
} from '@/api'

export const useModelRouterStore = defineStore('model-router', () => {
  // ─── 状态 ───
  const rules = ref<RoutingRules | null>(null)
  const defaults = ref<RoutingRules | null>(null)
  const history = ref<RoutingHistoryItem[]>([])
  const lastDecision = ref<RoutingDecision | null>(null)
  const lastScore = ref<ComplexityScore | null>(null)
  const loading = ref(false)
  const saving = ref(false)
  const error = ref<string | null>(null)

  // ─── 计算 ───
  const tierStats = computed(() => {
    const stats: Record<string, number> = {}
    for (const h of history.value) {
      stats[h.tier] = (stats[h.tier] || 0) + 1
    }
    return stats
  })

  const providerStats = computed(() => {
    const stats: Record<string, number> = {}
    for (const h of history.value) {
      const p = h.primary?.provider || 'unknown'
      stats[p] = (stats[p] || 0) + 1
    }
    return stats
  })

  const fallbackRate = computed(() => {
    if (!history.value.length) return 0
    // 简化：rationale 包含 "complex" 视为高复杂度，可能触发过降级
    return 0  // 后端没记录是否降级，先返 0
  })

  // ─── 内部 ───
  function _setError(e: string | null) {
    error.value = e
  }

  // ─── 动作 ───

  async function fetchRules() {
    loading.value = true
    _setError(null)
    try {
      const { modelRouterApi } = await import('@/api')
      const r = await modelRouterApi.getRules()
      rules.value = r.data
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`获取规则失败: ${reason}`)
    } finally {
      loading.value = false
    }
  }

  async function fetchDefaults() {
    try {
      const { modelRouterApi } = await import('@/api')
      const r = await modelRouterApi.getDefaults()
      defaults.value = r.data
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`获取默认规则失败: ${reason}`)
    }
  }

  async function saveRules(newRules: RoutingRules) {
    saving.value = true
    _setError(null)
    try {
      const { modelRouterApi } = await import('@/api')
      const r = await modelRouterApi.saveRules(newRules)
      rules.value = r.data
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`保存规则失败: ${reason}`)
      throw e
    } finally {
      saving.value = false
    }
  }

  async function resetToDefaults() {
    saving.value = true
    _setError(null)
    try {
      const { modelRouterApi } = await import('@/api')
      const r = await modelRouterApi.resetRules()
      rules.value = r.data
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`重置失败: ${reason}`)
      throw e
    } finally {
      saving.value = false
    }
  }

  async function evaluate(prompt: string, historyIn?: Array<Record<string, unknown>>, hasImages: boolean = false): Promise<ComplexityScore | null> {
    _setError(null)
    try {
      const { modelRouterApi } = await import('@/api')
      const r = await modelRouterApi.evaluate({ prompt, history: historyIn ?? null, has_images: hasImages })
      lastScore.value = r.data
      return r.data
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`评估失败: ${reason}`)
      return null
    }
  }

  async function decide(prompt: string, historyIn?: Array<Record<string, unknown>>, hasImages: boolean = false, record: boolean = true): Promise<RoutingDecision | null> {
    _setError(null)
    try {
      const { modelRouterApi } = await import('@/api')
      const r = await modelRouterApi.decide({ prompt, history: historyIn ?? null, has_images: hasImages, record })
      lastDecision.value = r.data
      lastScore.value = r.data.complexity
      if (record) {
        // 把最新决策加到 history 顶部
        history.value = [{
          ...r.data,
          prompt_preview: prompt.slice(0, 200),
        }, ...history.value].slice(0, 200)
      }
      return r.data
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`路由决策失败: ${reason}`)
      return null
    }
  }

  async function fetchHistory(limit: number = 50) {
    loading.value = true
    _setError(null)
    try {
      const { modelRouterApi } = await import('@/api')
      const r = await modelRouterApi.getHistory(limit)
      history.value = r.data || []
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`获取历史失败: ${reason}`)
    } finally {
      loading.value = false
    }
  }

  async function clearHistory() {
    _setError(null)
    try {
      const { modelRouterApi } = await import('@/api')
      await modelRouterApi.clearHistory()
      history.value = []
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`清空历史失败: ${reason}`)
    }
  }

  async function refreshAll() {
    await Promise.all([
      fetchRules(),
      fetchDefaults(),
      fetchHistory(50),
    ])
  }

  return {
    // 状态
    rules,
    defaults,
    history,
    lastDecision,
    lastScore,
    loading,
    saving,
    error,
    // 计算
    tierStats,
    providerStats,
    fallbackRate,
    // 动作
    fetchRules,
    fetchDefaults,
    saveRules,
    resetToDefaults,
    evaluate,
    decide,
    fetchHistory,
    clearHistory,
    refreshAll,
  }
})

export type ModelRouterStore = ReturnType<typeof useModelRouterStore>
