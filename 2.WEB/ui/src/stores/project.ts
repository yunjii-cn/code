import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as platform from '@/platform'
import { projectApi } from '@/api'

export interface Project {
  id: string
  name: string
  path: string
  description?: string
  createdAt: number
}

export interface Conversation {
  id: string
  title: string
  projectId: string
  createdAt: number
  updatedAt: number
}

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const activeProject = ref<Project | null>(null)
  const conversations = ref<Conversation[]>([])
  const loading = ref(false)

  async function loadProjects() {
    loading.value = true
    try {
      projects.value = await platform.listProjects()
      activeProject.value = await platform.getActiveProject()
      if (activeProject.value) {
        await loadConversations(activeProject.value.id)
      }
    } finally {
      loading.value = false
    }
  }

  async function createProject(name: string, workspacePath?: string) {
    const project = await platform.createProject({ name, workspace_path: workspacePath })
    projects.value.push(project)
    return project
  }

  async function switchProject(projectId: string) {
    await platform.switchProject(projectId)
    activeProject.value = projects.value.find((p) => p.id === projectId) || null
    if (activeProject.value) {
      await loadConversations(activeProject.value.id)
    }
  }

  async function deleteProject(projectId: string) {
    await platform.deleteProject(projectId)
    projects.value = projects.value.filter((p) => p.id !== projectId)
    if (activeProject.value?.id === projectId) {
      activeProject.value = projects.value[0] || null
    }
  }

  async function loadConversations(projectId: string) {
    conversations.value = await platform.getConversations(projectId)
  }

  async function saveConversation(sessionId: string, messages: Array<{ role: string; content: string }>, title?: string) {
    if (!activeProject.value) return
    await projectApi.saveConversation({
      project_id: activeProject.value.id,
      session_id: sessionId,
      messages,
      title,
    })
    await loadConversations(activeProject.value.id)
  }

  async function deleteConversation(sessionId: string) {
    if (!activeProject.value) return
    await projectApi.deleteConversation(activeProject.value.id, sessionId)
    conversations.value = conversations.value.filter((c) => c.id !== sessionId)
  }

  async function renameConversation(sessionId: string, newTitle: string) {
    if (!activeProject.value) return
    await projectApi.renameConversation({
      project_id: activeProject.value.id,
      session_id: sessionId,
      new_title: newTitle,
    })
    await loadConversations(activeProject.value.id)
  }

  async function searchConversations(keyword: string) {
    if (!activeProject.value) return []
    return await projectApi.searchConversations(activeProject.value.id, keyword) as unknown as Conversation[]
  }

  return {
    projects,
    activeProject,
    conversations,
    loading,
    loadProjects,
    createProject,
    switchProject,
    deleteProject,
    loadConversations,
    saveConversation,
    deleteConversation,
    renameConversation,
    searchConversations,
  }
})
