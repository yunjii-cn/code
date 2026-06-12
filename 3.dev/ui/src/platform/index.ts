import { Capacitor } from '@capacitor/core'
import * as desktop from './desktop'
import * as mobile from './mobile'

export const isMobile = Capacitor.isNativePlatform()
export const isDesktop = !isMobile
export const platform: 'desktop' | 'mobile' = isMobile ? 'mobile' : 'desktop'

const impl = isMobile ? mobile : desktop

export const chat = impl.chat
export const stopChat = impl.stopChat
export const loadSession = impl.loadSession
export const getProviders = impl.getProviders
export const getModels = impl.getModels
export const listProjects = impl.listProjects
export const createProject = impl.createProject
export const switchProject = impl.switchProject
export const deleteProject = impl.deleteProject
export const getActiveProject = impl.getActiveProject
export const getConversations = impl.getConversations
export const getSettings = impl.getSettings
export const saveSettings = impl.saveSettings
export const getSystemInfo = impl.getSystemInfo
