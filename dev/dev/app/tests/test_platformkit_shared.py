"""PlatformKit Shared - 常量与配置核心单测

2026-06-08 TASK-1.6 引入
目标覆盖率：100%（这些是核心常量，必须测）
"""
import sys
from pathlib import Path

import pytest

# 让 platformkit 可被 import
_BACKEND_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(_BACKEND_ROOT))

from platformkit.shared import (
    MODEL_KEYS,
    SETTINGS_KEYS,
    OLLAMA_AGENT_MAX_STEPS,
    TOOL_TEXT_LIMIT,
    COMMAND_OUTPUT_LIMIT,
    TOOL_CALLING_MODEL_PATTERNS,
    EnvFileManager,
)


class TestConstants:
    """常量完整性测试"""

    def test_model_keys_not_empty(self):
        assert isinstance(MODEL_KEYS, (list, tuple))
        assert len(MODEL_KEYS) > 0, "MODEL_KEYS 至少应包含 1 个模型字段"

    def test_model_keys_required_models(self):
        """必须覆盖的核心模型字段"""
        for model_field in ("ANTHROPIC_MODEL", "OLLAMA_MODEL"):
            assert model_field in MODEL_KEYS, f"缺少模型 {model_field}"

    def test_settings_keys_not_empty(self):
        assert isinstance(SETTINGS_KEYS, (list, tuple, set))
        assert len(SETTINGS_KEYS) > 0

    def test_settings_keys_are_strings(self):
        for key in SETTINGS_KEYS:
            assert isinstance(key, str), f"SETTINGS_KEYS 元素必须是字符串: {key!r}"

    def test_settings_keys_required(self):
        """核心配置项必须存在"""
        for required in ("MODEL_PROVIDER", "API_KEY", "API_BASE_URL"):
            assert required in SETTINGS_KEYS, f"缺少配置项 {required}"

    def test_numeric_limits_positive(self):
        assert OLLAMA_AGENT_MAX_STEPS > 0
        assert TOOL_TEXT_LIMIT > 0
        assert COMMAND_OUTPUT_LIMIT > 0

    def test_tool_calling_patterns_format(self):
        """TOOL_CALLING_MODEL_PATTERNS 应该是 list[str]（模型名前缀）"""
        assert isinstance(TOOL_CALLING_MODEL_PATTERNS, (list, tuple))
        assert len(TOOL_CALLING_MODEL_PATTERNS) > 0
        for pattern in TOOL_CALLING_MODEL_PATTERNS:
            assert isinstance(pattern, str)
            assert len(pattern) > 0


class TestEnvFileManagerParse:
    """parse_env_lines 是纯函数，覆盖率最高优先测"""

    def setup_method(self):
        # 用临时路径，不写盘
        self.mgr = EnvFileManager(env_path="/tmp/_test_env")

    def test_empty_string(self):
        lines, kv = self.mgr.parse_env_lines("")
        assert lines == []
        assert kv == {}

    def test_simple_kv(self):
        lines, kv = self.mgr.parse_env_lines("FOO=bar")
        assert lines == ["FOO=bar"]
        assert kv == {"FOO": "bar"}

    def test_comments_ignored(self):
        raw = "# comment\nFOO=bar\n# another\nBAZ=qux"
        lines, kv = self.mgr.parse_env_lines(raw)
        assert kv == {"FOO": "bar", "BAZ": "qux"}
        # 注释行原样保留
        assert "# comment" in lines

    def test_blank_lines_ignored(self):
        raw = "\n\nFOO=bar\n\n"
        _, kv = self.mgr.parse_env_lines(raw)
        assert kv == {"FOO": "bar"}

    def test_value_can_have_equals(self):
        """BASE64 / URL 等可能含 ="""
        raw = "TOKEN=abc=def=="
        _, kv = self.mgr.parse_env_lines(raw)
        assert kv == {"TOKEN": "abc=def=="}

    def test_value_with_quotes(self):
        raw = 'NAME="hello world"'
        _, kv = self.mgr.parse_env_lines(raw)
        # 当前实现不去引号，原样保留
        assert kv == {"NAME": '"hello world"'}

    def test_line_without_equals_ignored(self):
        raw = "INVALID_LINE\nFOO=bar"
        _, kv = self.mgr.parse_env_lines(raw)
        assert kv == {"FOO": "bar"}

    def test_empty_value(self):
        _, kv = self.mgr.parse_env_lines("FOO=")
        assert kv == {"FOO": ""}


class TestEnvFileManagerIO:
    """read_settings / write_settings 真实写盘测试"""

    def test_read_nonexistent_returns_empty(self, tmp_path):
        mgr = EnvFileManager(env_path=str(tmp_path / "nope.env"))
        assert mgr.read_settings() == {}

    def test_write_then_read_roundtrip(self, tmp_path):
        env_file = tmp_path / "test.env"
        mgr = EnvFileManager(env_path=str(env_file))
        # 取 SETTINGS_KEYS 的前 3 个写
        sample = {k: f"value_{k}" for k in list(SETTINGS_KEYS)[:3]}
        mgr.write_settings(sample)
        result = mgr.read_settings()
        for k, v in sample.items():
            assert result.get(k) == v, f"key {k} roundtrip 失败"

    def test_write_preserves_non_settings_keys(self, tmp_path):
        env_file = tmp_path / "test.env"
        env_file.write_text("# header\nCUSTOM=value\nAI_LANGUAGE=old\n", encoding="utf-8")
        mgr = EnvFileManager(env_path=str(env_file))
        mgr.write_settings({"AI_LANGUAGE": "new"})
        text = env_file.read_text(encoding="utf-8")
        assert "CUSTOM=value" in text, "非 SETTINGS_KEYS 的 key 必须保留"
        assert "AI_LANGUAGE=new" in text, "SETTINGS_KEYS 的 key 必须被更新"
        assert "# header" in text, "注释行必须保留"

    def test_write_empty_partial_does_not_crash(self, tmp_path):
        env_file = tmp_path / "test.env"
        mgr = EnvFileManager(env_path=str(env_file))
        mgr.write_settings({})  # 不应崩
        # 实现行为：写入所有 SETTINGS_KEYS 为空键（保持 schema 完整）
        result = mgr.read_settings()
        assert isinstance(result, dict)
        for k in SETTINGS_KEYS:
            assert k in result


class TestPlatformKitIdentity:
    """保证 re-export 模式下 platformkit.X === backend.X"""

    def test_envfilemanager_is_same_class(self):
        """platformkit.EnvFileManager 必须 === backend.EnvFileManager（避免双源漂移）"""
        import yunji_main_backend  # noqa: F401  触发 backend 加载
        from platformkit.shared.config_core import EnvFileManager as PkgMgr
        assert PkgMgr is sys.modules["yunji_main_backend"].EnvFileManager
