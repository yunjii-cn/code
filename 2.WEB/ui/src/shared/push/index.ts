// src/shared/push/index.ts
// 2026-06-09 TASK-4.3 引入：Web Push 模块入口

export { usePushStore } from './push-store'
export {
  detectPushCapability,
  requestNotificationPermission,
  subscribePush,
  unsubscribePush,
  getCurrentSubscription,
  testPush,
} from './push-subscribe'
export type { PushPermission, PushSubscribeResult } from './push-subscribe'
export { default as PushNotificationSettings } from './PushNotificationSettings.vue'
