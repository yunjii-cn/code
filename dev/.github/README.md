# GitHub Actions

2026-06-08 TASK-1.8 引入。

## 工作流

- [ci.yml](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/.github/workflows/ci.yml) — push / PR 触发

## Jobs

| Job | 内容 | 触发 | 阻断 |
|-----|------|------|------|
| frontend-lint | `npm run lint` | push + PR | ✅ |
| frontend-typecheck | `vue-tsc -b --force` | push + PR | ✅ |
| frontend-build | `npm run build` | push + PR | ✅ |
| backend-test | `pytest --no-cov` | push + PR | ✅ |
| backend-import-test | `import main` / `api_main` / `platformkit` | push + PR | ✅ |
| markdown-lint | `markdownlint doc/` | push + PR | ⚠️ 仅警告（待文档规范完成后改阻断） |

## 分支保护

建议在 GitHub 设置：
- `main` 分支开启"Require status checks to pass before merging"
- 必须通过的 checks：上述 5 个阻断项

## 本地复现

```bash
# 前端
cd dev/web
npm ci
npm run lint
npx vue-tsc -b --force
npm run build

# 后端
cd dev/app
pip install -i https://pypi.tuna.tsinghua.edu.cn/simple pytest pytest-asyncio pytest-cov httpx fastapi sse_starlette pydantic_settings
python -m pytest
python -c "import sys; sys.path.insert(0, '.'); import main, api_main, platformkit.shared; print('OK')"
```

## 待办

- [ ] 文档规范（markdownlint 配置）— W3 末
- [ ] 远程构建（Windows runner，PyInstaller 打包）— 计划在 Phase 2 末
- [ ] Codecov / Coverage 报告上传 — W4 末
