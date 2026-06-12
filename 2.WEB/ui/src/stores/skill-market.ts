/**
 * skill-market store — 技能市场 Pinia 状态
 *
 * 2026-06-09 TASK-4.5 引入
 *
 * 职责：
 *  - 缓存官方目录（catalog）
 *  - 缓存已安装列表（global + project）
 *  - 缓存分类列表
 *  - 缓存市场统计
 *  - 提供 install / uninstall / rate / upload / apply 等动作
 *
 * 设计原则：
 *  - 不直接 import api（避免循环依赖）；通过动态 import 按需加载
 *  - 状态用 ref 暴露；计算属性用 computed
 *  - 业务组件从 store 拿数据，不要每个组件都重新 fetch
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  Skill,
  InstalledSkill,
  SkillCategory,
  SkillMarketStats,
  SkillRatingInfo,
} from '@/api'

const STORAGE_LAST_FILTER = 'yunji:skill-market:last-filter'

interface LastFilter {
  category?: string
  search?: string
  tag?: string
}

function _loadLastFilter(): LastFilter {
  try {
    const raw = localStorage.getItem(STORAGE_LAST_FILTER)
    if (!raw) return {}
    return JSON.parse(raw) as LastFilter
  } catch {
    return {}
  }
}

function _saveLastFilter(f: LastFilter): void {
  try {
    localStorage.setItem(STORAGE_LAST_FILTER, JSON.stringify(f))
  } catch { /* ignore */ }
}

export const useSkillMarketStore = defineStore('skill-market', () => {
  // ─── 状态 ───
  const catalog = ref<Skill[]>([])
  const categories = ref<SkillCategory[]>([])
  const stats = ref<SkillMarketStats | null>(null)
  const installedGlobal = ref<InstalledSkill[]>([])
  const installedProject = ref<InstalledSkill[]>([])

  // 当前过滤器
  const filter = ref<LastFilter>(_loadLastFilter())
  const loading = ref(false)
  const error = ref<string | null>(null)

  // ─── 计算 ───
  const filteredCatalog = computed<Skill[]>(() => {
    const f = filter.value
    let arr = catalog.value
    if (f.category && f.category !== 'all') {
      arr = arr.filter(s => s.category === f.category)
    }
    if (f.tag) {
      const t = f.tag.toLowerCase()
      arr = arr.filter(s => s.tags?.some(x => x.toLowerCase() === t))
    }
    if (f.search) {
      const q = f.search.toLowerCase()
      arr = arr.filter(s =>
        s.id.toLowerCase().includes(q) ||
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        (s.tags || []).some(x => x.toLowerCase().includes(q))
      )
    }
    return arr
  })

  const installedIdSet = computed(() => {
    const s = new Set<string>()
    for (const i of installedGlobal.value) s.add(i.id)
    for (const i of installedProject.value) s.add(i.id)
    return s
  })

  const featuredSkills = computed(() =>
    [...catalog.value].sort((a, b) => (b.rating || 0) - (a.rating || 0)).slice(0, 6),
  )

  // ─── 内部 ───
  function _setFilter(f: LastFilter) {
    filter.value = f
    _saveLastFilter(f)
  }

  function _setError(e: string | null) {
    error.value = e
  }

  // ─── 动作 ───

  async function fetchStats() {
    try {
      const { skillMarketApi } = await import('@/api')
      const r = await skillMarketApi.stats()
      stats.value = r.data
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`获取统计失败: ${reason}`)
    }
  }

  async function fetchCategories() {
    try {
      const { skillMarketApi } = await import('@/api')
      const r = await skillMarketApi.categories()
      categories.value = r.data?.categories || []
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`获取分类失败: ${reason}`)
    }
  }

  async function fetchCatalog(params: { category?: string; search?: string; tag?: string } = {}) {
    loading.value = true
    _setError(null)
    try {
      const { skillMarketApi } = await import('@/api')
      const r = await skillMarketApi.list({ ...params, limit: 200 })
      catalog.value = r.data || []
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`获取技能目录失败: ${reason}`)
    } finally {
      loading.value = false
    }
  }

  async function fetchInstalled(scope: 'global' | 'project' = 'global', workspacePath: string = '') {
    try {
      const { skillMarketApi } = await import('@/api')
      const r = await skillMarketApi.installed({ scope, workspace_path: workspacePath })
      if (scope === 'global') installedGlobal.value = r.data || []
      else installedProject.value = r.data || []
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`获取已安装列表失败: ${reason}`)
    }
  }

  async function install(skillId: string, scope: 'global' | 'project' = 'global', workspacePath: string = '') {
    _setError(null)
    try {
      const { skillMarketApi } = await import('@/api')
      await skillMarketApi.install({ skill_id: skillId, scope, workspace_path: workspacePath })
      await fetchInstalled(scope, workspacePath)
      await fetchStats()
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`安装失败: ${reason}`)
      throw e
    }
  }

  async function uninstall(skillId: string, scope: 'global' | 'project' = 'global', workspacePath: string = '') {
    _setError(null)
    try {
      const { skillMarketApi } = await import('@/api')
      await skillMarketApi.uninstall({ skill_id: skillId, scope, workspace_path: workspacePath })
      await fetchInstalled(scope, workspacePath)
      await fetchStats()
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`卸载失败: ${reason}`)
      throw e
    }
  }

  async function rate(skillId: string, score: number): Promise<SkillRatingInfo> {
    _setError(null)
    try {
      const { skillMarketApi } = await import('@/api')
      const r = await skillMarketApi.rate(skillId, score)
      return r.data
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`评分失败: ${reason}`)
      throw e
    }
  }

  async function upload(meta: Partial<Skill> & { id: string; name: string; version: string; category: string; prompt_template: string }, scope: 'global' | 'project' = 'global', workspacePath: string = '') {
    _setError(null)
    try {
      const { skillMarketApi } = await import('@/api')
      await skillMarketApi.upload({ meta, scope, workspace_path: workspacePath })
      await fetchInstalled(scope, workspacePath)
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`上传失败: ${reason}`)
      throw e
    }
  }

  async function apply(skillId: string, variables: Record<string, string> = {}, scope: 'global' | 'project' = 'global', workspacePath: string = ''): Promise<string> {
    _setError(null)
    try {
      const { skillMarketApi } = await import('@/api')
      const r = await skillMarketApi.apply({ skill_id: skillId, variables, scope, workspace_path: workspacePath })
      return r.data.rendered_prompt
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`应用失败: ${reason}`)
      throw e
    }
  }

  async function exportSkill(skillId: string, scope: 'global' | 'project' = 'global', workspacePath: string = ''): Promise<{ filename: string; size: number; base64: string }> {
    _setError(null)
    try {
      const { skillMarketApi } = await import('@/api')
      const r = await skillMarketApi.exportSkill({ skill_id: skillId, scope, workspace_path: workspacePath })
      return r.data
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e)
      _setError(`导出失败: ${reason}`)
      throw e
    }
  }

  async function refreshAll() {
    await Promise.all([
      fetchStats(),
      fetchCategories(),
      fetchCatalog(),
      fetchInstalled('global'),
    ])
  }

  return {
    // 状态
    catalog,
    categories,
    stats,
    installedGlobal,
    installedProject,
    filter,
    loading,
    error,
    // 计算
    filteredCatalog,
    installedIdSet,
    featuredSkills,
    // 动作
    fetchStats,
    fetchCategories,
    fetchCatalog,
    fetchInstalled,
    install,
    uninstall,
    rate,
    upload,
    apply,
    exportSkill,
    refreshAll,
    // 内部
    _setFilter,
    _setError,
  }
})

export type SkillMarketStore = ReturnType<typeof useSkillMarketStore>
