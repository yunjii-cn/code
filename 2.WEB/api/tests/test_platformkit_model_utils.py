# 2026-06-08 TASK-1.3 Phase 1 收尾：platformkit.shared 新增模块测试

"""Tests for platformkit.shared.model_utils"""

import os
import sys

import pytest

# 添加入口路径
THIS_DIR = os.path.dirname(os.path.abspath(__file__))
APP_DIR = os.path.dirname(THIS_DIR)
sys.path.insert(0, APP_DIR)

from platformkit.shared import (
    guess_tool_support_by_name,
    normalize_model_entries,
    TOOL_CALLING_MODEL_PATTERNS,
)


class TestNormalizeModelEntries:
    def test_normalize_ollama_list(self):
        raw = [
            {"id": "llama3.1:8b", "name": "Llama 3.1 8B"},
            {"id": "qwen2.5:32b", "name": "Qwen 2.5 32B"},
        ]
        result = normalize_model_entries(raw, "ollama")
        assert len(result) == 2
        assert result[0]["id"] == "llama3.1:8b"
        assert result[0]["provider"] == "ollama"
        assert result[1]["name"] == "Qwen 2.5 32B"

    def test_empty_list(self):
        assert normalize_model_entries([], "ollama") == []

    def test_none_list(self):
        assert normalize_model_entries(None, "ollama") == []

    def test_skips_empty_ids(self):
        raw = [
            {"id": "valid", "name": "Valid"},
            {"id": "", "name": "Invalid"},
            {"id": None, "name": "Invalid 2"},
        ]
        result = normalize_model_entries(raw, "openrouter")
        assert len(result) == 1
        assert result[0]["id"] == "valid"

    def test_provider_passed_through(self):
        raw = [{"id": "m1", "name": "M1"}]
        assert normalize_model_entries(raw, "anthropic")[0]["provider"] == "anthropic"

    def test_truncates_to_120(self):
        raw = [{"id": f"m{i}", "name": f"M{i}"} for i in range(200)]
        result = normalize_model_entries(raw, "ollama")
        assert len(result) == 120

    def test_uses_id_as_name_fallback(self):
        raw = [{"id": "foo"}]
        result = normalize_model_entries(raw, "ollama")
        assert result[0]["name"] == "foo"

    def test_falls_back_to_display_name(self):
        raw = [{"id": "foo", "display_name": "Foo Display"}]
        result = normalize_model_entries(raw, "ollama")
        assert result[0]["name"] == "Foo Display"


class TestGuessToolSupport:
    def test_qwen_supports_tools(self):
        assert guess_tool_support_by_name("qwen2.5:32b") is True

    def test_llama3_1_supports_tools(self):
        assert guess_tool_support_by_name("llama3.1:8b") is True

    def test_cloud_models_dont_support(self):
        assert guess_tool_support_by_name("llama3.1:cloud") is False

    def test_unknown_model_doesnt_support(self):
        assert guess_tool_support_by_name("totally-unknown-model:7b") is False

    def test_empty_name(self):
        assert guess_tool_support_by_name("") is False

    def test_case_insensitive(self):
        assert guess_tool_support_by_name("QWEN3:32B") is True


class TestToolCallingPatternsExported:
    def test_patterns_is_list(self):
        assert isinstance(TOOL_CALLING_MODEL_PATTERNS, (list, tuple, set))

    def test_patterns_not_empty(self):
        assert len(TOOL_CALLING_MODEL_PATTERNS) > 0
