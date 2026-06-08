import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'chat',
      // 2026-06-08 TASK-1.4: 迁移到 feature-sliced
      component: () => import('@features/chat'),
    },
    {
      path: '/models',
      name: 'models',
      // 2026-06-08 TASK-1.4: 迁移到 feature-sliced
      component: () => import('@features/models'),
    },
    {
      path: '/projects',
      name: 'projects',
      // 2026-06-08 TASK-1.4: 迁移到 feature-sliced
      component: () => import('@features/project'),
    },
    {
      path: '/env',
      name: 'env',
      // 2026-06-08 TASK-1.4: 迁移到 feature-sliced
      component: () => import('@features/env'),
    },
    {
      path: '/settings',
      name: 'settings',
      // 2026-06-08 TASK-1.4: 迁移到 feature-sliced
      component: () => import('@features/settings'),
    },
    {
      path: '/version',
      name: 'version',
      // 2026-06-08 TASK-1.4: 迁移到 feature-sliced
      component: () => import('@features/version'),
    },
    {
      path: '/github',
      name: 'github',
      // 2026-06-08 TASK-2.1: GitHub 集成模块（Issues / PRs / Ask PR）
      component: () => import('@features/github'),
    },
    {
      path: '/knowledge',
      name: 'knowledge',
      // 2026-06-09 TASK-3.3: 自进化知识面板（四层知识 + 强度演化）
      component: () => import('@features/knowledge'),
    },
    {
      path: '/responsive',
      name: 'responsive',
      // 2026-06-09 TASK-3.6: 主动感知面板（4 watcher + WebSocket 实时）
      component: () => import('@features/responsive'),
    },
    {
      path: '/agent',
      name: 'agent',
      // 2026-06-09 TASK-3.8: 独行模式 Code Agent（4 阶段状态机 UI）
      component: () => import('@features/agent'),
    },
    {
      path: '/team',
      name: 'team',
      // 2026-06-09 TASK-3.10: 团队模式 4-5 Agent 并行（任务看板 / 团队对话流 / Agent 面板）
      component: () => import('@features/team'),
    },
  ],
})

export default router
