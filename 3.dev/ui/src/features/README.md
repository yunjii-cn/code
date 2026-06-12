# features/

> 2026-06-08 TASK-1.4 引入。Feature-sliced 架构的页面层。

## 目录结构

```
features/
├── version/          ← 已迁移（叶子节点 POC）
│   ├── VersionView.vue
│   └── index.ts
├── models/           ← 待迁移
├── env/              ← 待迁移
├── settings/         ← 待迁移
└── plugin/           ← 待迁移
```

## 迁移原则

1. **每个 feature 独立目录** — `features/<name>/`，包含一个主 view + 子组件 + 类型
2. **index.ts 统一导出** — 默认导出主 view，命名导出子组件/类型
3. **跨 feature 复用** — 放 `@shared/`，**禁止**在 feature 之间互相 import
4. **router 引用** — `import VersionView from '@features/version'`
5. **兼容期** — `views/<Name>View.vue` 留一个 re-export shim（过渡期 1-2 周），新代码禁止再用

## 迁移顺序

按依赖最少的叶子节点优先（避免大爆炸）：

1. ✅ `version` (607 行，无内部依赖)
2. `plugin` (245 行，独立)
3. `models` (664 行，独立)
4. `env` (469 行，依赖 system 路由)
5. `settings` (644 行，依赖多个 store)

> 复杂页面（chat / project）留 Phase 2 处理，需配合状态层重构。

## 禁止事项

- ❌ `import X from '@/views/X.vue'`（新代码）→ 用 `@features/x`
- ❌ 跨 feature import：`@features/chat` 不能 import `@features/models`
- ❌ 在 feature 内部直接 import `@/views/`
- ❌ 重复定义 API 调用（统一走 `@/api`）
