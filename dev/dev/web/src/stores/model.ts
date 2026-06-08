import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as platform from '@/platform'

export interface Provider {
  id: string
  name: string
  type: string
  enabled: boolean
}

export interface Model {
  id: string
  name: string
  provider: string
  size?: string
  quantization?: string
}

export const useModelStore = defineStore('model', () => {
  const providers = ref<Provider[]>([])
  const models = ref<Model[]>([])
  const currentProvider = ref('')
  const currentModel = ref('')

  async function fetchProviders() {
    providers.value = await platform.getProviders()
    if (providers.value.length > 0 && !currentProvider.value) {
      currentProvider.value = providers.value[0].id
    }
  }

  async function fetchModels() {
    if (!currentProvider.value) return
    models.value = await platform.getModels(currentProvider.value)
    if (models.value.length > 0 && !currentModel.value) {
      currentModel.value = models.value[0].id
    }
  }

  async function switchProvider(providerId: string) {
    currentProvider.value = providerId
    currentModel.value = ''
    models.value = []
    await fetchModels()
  }

  function switchModel(modelId: string) {
    currentModel.value = modelId
  }

  return {
    providers,
    models,
    currentProvider,
    currentModel,
    fetchProviders,
    fetchModels,
    switchProvider,
    switchModel,
  }
})
