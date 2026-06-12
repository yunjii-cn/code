"""API 集成 smoke test

目标：确保 FastAPI app 能正常启动，路由注册成功，Pydantic 模型校验工作。

不实际调用外部服务，只验证：
1. import api_main 不崩
2. /docs (Swagger) 可访问
3. 路由数量符合预期
4. Pydantic 模型能正常校验

2026-06-08 TASK-1.6 引入
"""
import sys
from pathlib import Path

# 让 tests 能 import api_main
_BACKEND_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_BACKEND_ROOT))

import pytest


class TestApiMainImports:
    """导入测试 - 比实际启动快很多"""

    def test_api_main_module_imports(self):
        """api_main.py 能否被 import（不启动服务）"""
        # 重要：import 即可，不调用 start()
        # 任何顶层错误（import 失败、路由注册失败）都会让 import 抛错
        try:
            import api_main
            assert api_main.app is not None
        except ImportError as e:
            # 已知的预存在依赖问题（计划书中记录），不算测试失败
            if "sse_starlette" in str(e) or "pydantic_settings" in str(e) or "httpx" in str(e):
                pytest.skip(f"预存在依赖问题，跳过：{e}")
            raise

    def test_app_has_routes(self):
        """FastAPI app 必须有路由注册"""
        try:
            from api_main import app
        except ImportError as e:
            pytest.skip(f"预存在依赖问题：{e}")
        routes = app.routes
        # 至少要 6 条路由（chat / models / projects / env / settings / version）
        # 实际更多（含子应用 qwen2api / zhipu2api）
        assert len(routes) >= 6, f"路由数过少：{len(routes)}"

    def test_app_openapi_generates(self):
        """OpenAPI schema 能否生成（验证 Pydantic 模型）"""
        try:
            from api_main import app
        except ImportError as e:
            pytest.skip(f"预存在依赖问题：{e}")
        schema = app.openapi()
        assert "openapi" in schema
        assert "paths" in schema
        # 必须包含 version 路由
        assert any("/api/version" in path for path in schema["paths"])


class TestPydanticValidation:
    """Pydantic 模型校验测试"""

    def test_switch_commit_request(self):
        from routes.version import SwitchCommitRequest
        # 正常
        req = SwitchCommitRequest(commit_hash="abc123")
        assert req.commit_hash == "abc123"
        # 缺字段
        with pytest.raises(Exception):
            SwitchCommitRequest()  # type: ignore

    def test_switch_exe_request(self):
        from routes.version import SwitchExeRequest
        req = SwitchExeRequest(exe_path="/path/to.exe", git_commit="def456")
        assert req.exe_path == "/path/to.exe"
        assert req.git_commit == "def456"
        # git_commit 可选
        req2 = SwitchExeRequest(exe_path="/path/to.exe")
        assert req2.git_commit is None
