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
  ],
})

export default router
