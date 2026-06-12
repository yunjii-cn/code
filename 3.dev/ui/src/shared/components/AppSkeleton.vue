<script setup lang="ts">
/**
 * AppSkeleton — 应用加载骨架屏
 * 2026-06-09 TASK-4.7
 *
 * 在路由懒加载期间显示，减少白屏感知时间
 */
</script>

<template>
  <div class="app-skeleton">
    <div class="sk-header">
      <div class="sk-tab" v-for="i in 5" :key="i" />
    </div>
    <div class="sk-body">
      <div class="sk-line sk-line-lg" />
      <div class="sk-line sk-line-md" />
      <div class="sk-line sk-line-sm" />
      <div class="sk-card">
        <div class="sk-line sk-line-md" />
        <div class="sk-line sk-line-sm" />
        <div class="sk-line sk-line-xs" />
      </div>
      <div class="sk-card">
        <div class="sk-line sk-line-md" />
        <div class="sk-line sk-line-sm" />
      </div>
    </div>
    <div class="sk-footer">
      <div class="sk-line sk-line-xs" />
    </div>
  </div>
</template>

<style scoped>
.app-skeleton {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  padding: 0;
}

.sk-header {
  display: flex;
  gap: 1px;
  height: 38px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  padding: 0 8px;
  align-items: center;
}

.sk-tab {
  width: 80px;
  height: 20px;
  border-radius: 3px;
  background: linear-gradient(90deg, var(--bg-card) 25%, var(--bg-card-hover) 50%, var(--bg-card) 75%);
  background-size: 200% 100%;
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}

.sk-body {
  flex: 1;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.sk-line {
  height: 14px;
  border-radius: 3px;
  background: linear-gradient(90deg, var(--bg-card) 25%, var(--bg-card-hover) 50%, var(--bg-card) 75%);
  background-size: 200% 100%;
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}

.sk-line-lg { width: 70%; height: 20px; }
.sk-line-md { width: 50%; }
.sk-line-sm { width: 35%; }
.sk-line-xs { width: 20%; }

.sk-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sk-footer {
  height: 26px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 12px;
}

@keyframes skeleton-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
</style>
