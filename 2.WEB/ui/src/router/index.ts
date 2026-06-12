import { createRouter, createWebHistory } from 'vue-router'
import { defineAsyncComponent } from 'vue'
// 2026-06-09 TASK-4.7：骨架屏加载组件
import AppSkeleton from '@shared/components/AppSkeleton.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'chat',
      component: defineAsyncComponent({ loader: () => import('@features/chat'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/models',
      name: 'models',
      component: defineAsyncComponent({ loader: () => import('@features/models'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/projects',
      name: 'projects',
      component: defineAsyncComponent({ loader: () => import('@features/project'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/env',
      name: 'env',
      component: defineAsyncComponent({ loader: () => import('@features/env'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/settings',
      name: 'settings',
      component: defineAsyncComponent({ loader: () => import('@features/settings'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/version',
      name: 'version',
      component: defineAsyncComponent({ loader: () => import('@features/version'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/github',
      name: 'github',
      component: defineAsyncComponent({ loader: () => import('@features/github'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/knowledge',
      name: 'knowledge',
      component: defineAsyncComponent({ loader: () => import('@features/knowledge'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/responsive',
      name: 'responsive',
      component: defineAsyncComponent({ loader: () => import('@features/responsive'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/agent',
      name: 'agent',
      component: defineAsyncComponent({ loader: () => import('@features/agent'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/team',
      name: 'team',
      component: defineAsyncComponent({ loader: () => import('@features/team'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/collaboration',
      name: 'collaboration',
      component: defineAsyncComponent({ loader: () => import('@features/collaboration'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/skill-market',
      name: 'skill-market',
      component: defineAsyncComponent({ loader: () => import('@features/skill-market'), loadingComponent: AppSkeleton, delay: 200 }),
    },
    {
      path: '/model-router',
      name: 'model-router',
      component: defineAsyncComponent({ loader: () => import('@features/model-router'), loadingComponent: AppSkeleton, delay: 200 }),
    },
  ],
})

export default router
