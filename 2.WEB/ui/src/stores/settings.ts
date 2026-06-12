import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as platform from '@/platform'

export interface UserSettings {
  theme: string
  language: string
  fontSize: number
  autoSave: boolean
  [key: string]: unknown
}

export interface SystemInfo {
  os: string
  cpu: string
  memory: number
  gpu?: string
  pythonVersion?: string
  nodeVersion?: string
}

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<UserSettings>({
    theme: 'dark',
    language: 'zh-CN',
    fontSize: 14,
    autoSave: true,
  })
  const systemInfo = ref<SystemInfo | null>(null)

  async function fetchSettings() {
    const data = await platform.getSettings()
    settings.value = data as UserSettings
  }

  async function saveSettings(newSettings: Partial<UserSettings>) {
    await platform.saveSettings(newSettings)
    Object.assign(settings.value, newSettings)
  }

  async function fetchSystemInfo() {
    systemInfo.value = await platform.getSystemInfo()
  }

  return {
    settings,
    systemInfo,
    fetchSettings,
    saveSettings,
    fetchSystemInfo,
  }
})
