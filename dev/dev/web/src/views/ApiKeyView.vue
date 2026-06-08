<!--
  ⚠️ 兼容期 re-export shim
  原文件: src/views/ApiKeyView.vue
  新位置: src/features/apikey/ApiKeyView.vue
  迁移日期: 2026-06-08（TASK-1.4 Phase 1 收尾）
-->
<script setup lang="ts">
import ApiKeyView from '@features/apikey/ApiKeyView.vue'
</script>

<template>
  <ApiKeyView />
</template>
