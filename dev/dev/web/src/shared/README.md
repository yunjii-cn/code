# shared/

> 2026-06-08 TASK-1.4 引入。跨 feature 复用的纯组件 / 工具 / 类型 / 设计系统。

## 当前结构

```
shared/
├── components/   ← Design System 原子组件（YJ* 前缀）
├── styles/       ← 全局 CSS 变量与 reset
│   ├── tokens.css    ← 设计令牌（颜色 / 间距 / 字号 / 圆角 / 阴影 / 动效 / 层级）
│   └── reset.css     ← 现代化 CSS reset
├── composables/  ← 计划中（W4）
├── types/        ← 计划中
├── utils/        ← 计划中
└── tokens/       ← TS 侧的设计令牌常量（计划中）
```

## Design System v1（已完成）

5 个原子组件，统一 `YJ` 前缀：

| 组件        | 用途                                                 |
| ----------- | ---------------------------------------------------- |
| `YJButton`  | 按钮（type / size / loading / block / round）        |
| `YJModal`   | 模态框（Teleport / 拖拽 / Esc / 点击外部关闭）       |
| `YJPopover` | 气泡（click / hover / manual 触发）                  |
| `YJToast`   | 轻提示（success / error / warning / info / loading） |
| `YJPanel`   | 面板（可折叠 / 可调整大小）                          |

### 引入方式

```ts
// ✅ 推荐：通过共享入口
import { YJButton, YJModal, useToast } from '@shared/components'

// 或全局注册（main.ts 一次性引入）
import YJButton from '@shared/components/YJButton.vue'
app.component('YJButton', YJButton)
```

### 完整使用示例

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { YJButton, YJModal, YJPopover, YJPanel, useToast } from '@shared/components'

const showModal = ref(false)
const showPopover = ref(false)
const panelCollapsed = ref(false)
const toast = useToast()

function handleSave() {
  toast.success('保存成功')
}
</script>

<template>
  <!-- 按钮 -->
  <YJButton type="primary" @click="handleSave">保存</YJButton>
  <YJButton type="danger" :loading="saving">删除</YJButton>
  <YJButton type="ghost" size="small" round>取消</YJButton>

  <!-- 模态框 -->
  <YJModal v-model:show="showModal" title="编辑项目" draggable>
    <p>内容</p>
    <template #footer>
      <YJButton @click="showModal = false">取消</YJButton>
      <YJButton type="primary" @click="handleSave">确定</YJButton>
    </template>
  </YJModal>

  <!-- 气泡 -->
  <YJPopover v-model:show="showPopover" trigger="hover" placement="top">
    <template #content>提示内容</template>
    <YJButton>悬停查看</YJButton>
  </YJPopover>

  <!-- 面板 -->
  <YJPanel title="项目信息" v-model:collapsed="panelCollapsed" resizable>
    <p>面板内容</p>
  </YJPanel>
</template>
```

### Toast 全局 API

```ts
import { useToast } from '@shared/components'

const toast = useToast()

toast.success('操作成功')
toast.error('操作失败')
toast.warning('请注意')
toast.info('提示信息')
const id = toast.loading('加载中...')
// ...完成后
toast.remove(id)
toast.clear() // 清空所有
```

> ⚠️ `useToast()` 依赖 `window.__yjToast` 全局实例，必须在 [App.vue](../App.vue) 中至少渲染一次 `<YJToast />`。

### 设计令牌（CSS 变量）

所有组件**不硬编码**颜色 / 间距 / 字号，全部从 `tokens.css` 取值：

| 类别 | 变量示例                                                        | 取值                                          |
| ---- | --------------------------------------------------------------- | --------------------------------------------- |
| 背景 | `--bg-primary` / `--bg-card` / `--bg-card-hover`                | 5 层级                                        |
| 边框 | `--border` / `--border-light` / `--border-strong`               | 3 层级                                        |
| 文本 | `--text-primary` / `--text-secondary` / `--text-muted`          | 4 层级                                        |
| 主色 | `--accent` / `--accent-2` / `--accent-3`                        | 蓝 5 层级                                     |
| 成功 | `--success` / `--success-2` / `--success-3`                     | 绿 5 层级                                     |
| 警告 | `--warning` / `--warning-2` / `--warning-3`                     | 橙 5 层级                                     |
| 危险 | `--danger` / `--danger-2` / `--danger-3`                        | 红 5 层级                                     |
| 信息 | `--info` / `--info-2` / `--info-3`                              | 青 5 层级                                     |
| 间距 | `--space-1` ~ `--space-10`                                      | 4 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48 / 64 |
| 字号 | `--font-xs` / `--font-sm` / `--font-base` / `--font-md` ...     | 12 / 13 / 14 / 16 / 18 / 20 / 24 / 32         |
| 圆角 | `--radius-sm` / `--radius-md` / `--radius-lg` / `--radius-xl`   | 4 / 6 / 10 / 16                               |
| 阴影 | `--shadow-sm` / `--shadow-md` / `--shadow-lg` / `--shadow-xl`   | 4 层级                                        |
| 动效 | `--transition-fast` / `--transition-base` / `--transition-slow` | 150 / 200 / 300ms                             |
| 层级 | `--z-base` / `--z-modal` / `--z-popover` / `--z-toast`          | z-index 体系                                  |

> 修改任何视觉风格（主题色 / 间距 / 字号），只需改 `tokens.css`，所有组件自动生效。

## 准入标准

满足以下**全部**才能进入 `@shared/`：

- [ ] 至少 2 个 feature 需要
- [ ] 无副作用（不直接调 API / store）
- [ ] 无特定 feature 业务语义
- [ ] 优先使用 `tokens.css` 的 CSS 变量

## 禁止事项

- ❌ 把某一个 feature 的私有组件放这里
- ❌ 包含 API 调用（API 统一在 `@/api`）
- ❌ 包含路由配置（路由在 `@/router`）
- ❌ 包含 Pinia store（store 暂时在 `@/stores`，未来按归属下沉到 feature）
- ❌ 在组件中硬编码颜色 / 间距 / 字号

## 迁移计划

Phase 2（W5-W8）逐步把现有 14 个 view 中的：

- 重复的 modal / drawer → 替换为 `<YJModal>`
- 重复的 toast / notification → 替换为 `useToast()`
- 重复的折叠面板 → 替换为 `<YJPanel>`
- 重复的按钮 → 替换为 `<YJButton>`

完成后即可在 Phase 3 / 4 引入更多设计系统组件（Tag / Badge / Tabs / Steps / Empty 等）。

## 已知问题

无。
