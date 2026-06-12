# 2026-06-08 TASK-1.3 Phase 1 收尾：platformkit.shared 新增模块测试

"""Tests for platformkit.shared.http_client"""

import os
import sys

import pytest

# 添加入口路径
THIS_DIR = os.path.dirname(os.path.abspath(__file__))
APP_DIR = os.path.dirname(THIS_DIR)
sys.path.insert(0, APP_DIR)

from platformkit.shared import fetch_json_with_timeout


class TestFetchJsonWithTimeout:
    def test_returns_dict_shape(self):
        result = fetch_json_with_timeout("http://127.0.0.1:1/nonexistent", timeout_ms=500)
        assert isinstance(result, dict)
        assert "ok" in result
        assert "status" in result
        assert "data" in result
        assert "text" in result
        assert result["ok"] is False
        assert result["status"] == 0  # 网络错误

    def test_timeout_returns_error(self):
        # 不可达地址应该超时或连接失败
        result = fetch_json_with_timeout("http://10.255.255.1:1/test", timeout_ms=200)
        assert result["ok"] is False

    def test_default_method_get(self):
        # 方法默认 GET
        result = fetch_json_with_timeout("http://127.0.0.1:1/", timeout_ms=200)
        assert result["ok"] is False  # 网络错误但方法参数不影响 shape

    def test_custom_method(self):
        result = fetch_json_with_timeout("http://127.0.0.1:1/", method="POST", timeout_ms=200)
        assert result["ok"] is False
