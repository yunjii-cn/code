<!--
  ⚠️ 兼容期 re-export shim
  原文件: src/views/VersionView.vue
  新位置: src/features/version/VersionView.vue
  迁移日期: 2026-06-08（TASK-1.4）

  这个 shim 保留 1-2 周，确保老引用不破。
  新代码请直接 import VersionView from '@features/version'
-->
<script setup lang="ts">
import VersionView from '@features/version/VersionView.vue'
</script>

<template>
  <VersionView />
</template>
