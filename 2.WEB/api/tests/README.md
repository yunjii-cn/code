# dev/app/tests/

> 2026-06-08 TASK-1.6 引入。Python 后端单测。

## 目录

```
tests/
├── __init__.py            （空，标记 Python 包）
├── conftest.py            （pytest 配置：自动加 sys.path）
├── test_platformkit_shared.py   20 测试 — 平台核心常量 + EnvFileManager
└── test_api_smoke.py            5 测试 — FastAPI 启动 + 路由 + Pydantic 校验
```

## 运行

```bash
cd dev/app
python -m pytest                    # 跑全部 + 覆盖率
python -m pytest -v                 # 详细
python -m pytest -k "EnvFile"       # 只跑 EnvFile 相关
python -m pytest --no-cov           # 关掉覆盖率（更快）
```

## 当前覆盖率

```
platformkit                       80%   (49 行，10 未覆盖)
- platformkit/__init__.py         100%
- platformkit/app/__init__.py     100%
- platformkit/shared/__init__.py  100%
- platformkit/shared/config_core.py  63%  (error-handling 路径)
- platformkit/shared/constants.py   89%  (importlib 失败分支)
- platformkit/shared/types.py     100%  (占位)
```

## 目标

- Phase 1 W2 末：platformkit 100%
- Phase 1 W4 末：services/ 60%+
- Phase 2 起：routes/ + services/ 80%+

## 写测试的约定

1. **避免外部依赖** — 不真发 HTTP 请求，不真读 Ollama，用 mock
2. **每个测试独立** — 用 `tmp_path` fixture 隔离文件操作
3. **断言要清晰** — `assert "FOO=new" in text, "FOO 必须被更新"`，失败信息可读
4. **跳过 vs 失败** — 预存在 bug 用 `pytest.skip()` 而不是 `xfail`，等修复后再启用
5. **不要碰** backend.py 的全局状态 — 那是单例，多测试会互相影响

## CI 集成

TASK-1.8 接入 GitHub Actions 后会自动跑 `pytest`，详见 `.github/workflows/ci.yml`。
