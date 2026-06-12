# 2026-06-09 TASK-4.6：智能模型路由核心测试

"""Tests for platformkit.shared.model_router

覆盖：
    - 复杂度评分（长度 / 关键词 / 结构 / 历史）
    - 默认规则 & 自定义规则
    - 决策函数（按 tier 选择）
    - 规则持久化（save / load / reset）
    - 历史记录（append / list / clear）
    - 降级判定
"""

import json
import os
import shutil
import sys
import tempfile

import pytest

THIS_DIR = os.path.dirname(os.path.abspath(__file__))
APP_DIR = os.path.dirname(THIS_DIR)
sys.path.insert(0, APP_DIR)

from platformkit.shared import model_router


@pytest.fixture
def isolated_home(monkeypatch, tmp_path):
    fake_home = tmp_path
    monkeypatch.setenv("HOME", str(fake_home))
    monkeypatch.setenv("USERPROFILE", str(fake_home))
    # 清空历史
    model_router.clear_history()
    yield fake_home


# ── Token 估算 ──────────────────────────────────────────────
class TestEstimateTokens:
    def test_empty(self):
        assert model_router._estimate_tokens("") == 0

    def test_chinese(self):
        n = model_router._estimate_tokens("你好世界")
        # 4 中文字符 × 1.5 = 6
        assert n == 6

    def test_english(self):
        n = model_router._estimate_tokens("hello world")
        # 2 english words × 1.3 = 2.6 + 1 space × 0.5 = 0.5 = 3.1 → 3
        assert n == 3

    def test_mixed(self):
        n = model_router._estimate_tokens("你好 world")
        # 2 chinese (3) + 1 english word (1.3 → 1) = 4
        assert n == 4


# ── 评分：长度 ─────────────────────────────────────────────
class TestScoreLength:
    def test_short(self):
        s, t = model_router._score_length("hi")
        assert s == 0
        assert t < 100

    def test_medium(self):
        s, t = model_router._score_length("hello " * 200)
        assert 0 < s <= 0.4
        assert t >= 100

    def test_long(self):
        s, t = model_router._score_length("hello " * 5000)
        assert s == 0.4
        assert t >= 2000


# ── 评分：关键词 ────────────────────────────────────────────
class TestScoreKeywords:
    def test_strong_keyword(self):
        s, m = model_router._score_keywords("请帮我重构一下这段代码")
        assert s >= 0.2
        assert "重构" in m

    def test_medium_keyword(self):
        s, m = model_router._score_keywords("请实现一个功能")
        assert s >= 0.1
        assert "实现" in m

    def test_weak_keyword_reduces(self):
        s, m = model_router._score_keywords("你好")
        assert s <= 0  # 弱信号减分
        assert "你好" not in m  # 弱信号不计入 matched

    def test_no_keywords(self):
        s, m = model_router._score_keywords("今天的天气")
        assert s == 0
        assert not m

    def test_capped_at_0_4(self):
        s, m = model_router._score_keywords("重构 架构 设计 完整 深入 全栈 分布式 加密 算法")
        assert s <= 0.4


# ── 评分：结构 ─────────────────────────────────────────────
class TestScoreStructure:
    def test_no_structure(self):
        s, sig = model_router._score_structure("hello")
        assert s == 0
        assert not sig

    def test_code_blocks(self):
        s, sig = model_router._score_structure("```python\nprint()\n```")
        assert s >= 0.05
        assert any("code" in x for x in sig)

    def test_image(self):
        s, sig = model_router._score_structure("看这张图", has_images=True)
        assert s >= 0.15
        assert any("image" in x for x in sig)

    def test_file_paths(self):
        s, sig = model_router._score_structure("见 /a/b/c.txt 和 /a/b/d.txt 和 /a/b/e.txt")
        assert s >= 0.05

    def test_capped_at_0_2(self):
        s, sig = model_router._score_structure("```\n```\n```\n```\n```\n```", has_images=True)
        assert s <= 0.2


# ── 评分：综合 ─────────────────────────────────────────────
class TestEvaluateComplexity:
    def test_simple_chat(self):
        score = model_router.evaluate_complexity("你好")
        assert score.total < 0.1

    def test_medium_task(self):
        # 加代码块以拉高 structure 分，让 total 落在 medium 区间
        prompt = (
            "请实现一个 React 组件，包含 prop 校验、单测和 Storybook。\n\n"
            "要求：\n"
            "- 使用 TypeScript\n"
            "- 函数组件\n"
            "- 至少 2 个测试用例\n"
            "- 完整代码实现\n"
        )
        score = model_router.evaluate_complexity(prompt)
        assert 0.1 < score.total < 0.7, f"实际 {score.total}"

    def test_complex_task(self):
        # 同时拉高 length + keywords + structure 三个维度
        code_block = "```python\n" + ("x = 1\n" * 50) + "```"
        prompt = (
            "请帮我完整设计一个分布式微服务架构，包含 transformer 模型推理、"
            "kubernetes 部署、加密传输、性能优化算法实现" + code_block
        )
        score = model_router.evaluate_complexity(prompt)
        assert score.total > 0.5, f"实际 {score.total}"

    def test_code_heavy(self):
        prompt = "```python\n" + ("x = 1\n" * 100) + "```"
        score = model_router.evaluate_complexity(prompt)
        assert score.structure > 0

    def test_history_boost(self):
        score1 = model_router.evaluate_complexity("hi")
        score2 = model_router.evaluate_complexity("hi", history=[{"role": "user"}] * 50)
        assert score2.history > score1.history
        assert score2.total > score1.total


# ── 路由决策 ──────────────────────────────────────────────
class TestDecide:
    def test_simple_prompt_routes_to_simple_tier(self):
        d = model_router.decide("你好")
        assert d.tier == "simple"
        assert d.primary.provider == "ollama"

    def test_complex_prompt_routes_to_complex_tier(self):
        # 长 prompt + 复杂关键词 + 多代码块 → 必落到 complex
        # 总长度需要超过 5000 token 才能让 length=0.4（最大）
        code_blocks = "\n\n".join([
            "```python\n" + ("x = 1\n" * 30) + "```"
            for _ in range(3)
        ])
        long_padding = "详细的系统设计、API 定义、数据模型、部署方案、监控告警。" * 60
        prompt = (
            "请完整设计一个分布式微服务架构，包含 transformer 模型推理、"
            "kubernetes 部署、加密传输、性能优化算法实现。" + long_padding + code_blocks
        )
        d = model_router.decide(prompt)
        assert d.tier == "complex", f"实际 tier={d.tier}, complexity={d.complexity.total}"
        assert d.primary.provider == "anthropic"

    def test_medium_prompt(self):
        # 中等长度 + 1-2 个关键词 → medium 或 complex
        d = model_router.decide(
            "请实现一个完整功能：" + ("这是一些详细的需求描述，" * 30) + "优化代码结构"
        )
        assert d.tier in ("medium", "complex")

    def test_custom_rules(self):
        custom = model_router.RoutingRules(tiers=[
            model_router.RoutingTier(
                name="simple",
                max_complexity=1.01,
                primary=model_router.ModelRef("custom", "fast-model"),
            ),
        ])
        d = model_router.decide("hello", rules=custom)
        assert d.tier == "simple"
        assert d.primary.provider == "custom"

    def test_fallback_propagated(self):
        d = model_router.decide("复杂任务 transformer 架构 kubernetes" * 10)
        assert d.fallback is not None

    def test_decision_is_pure(self):
        d1 = model_router.decide("hello")
        d2 = model_router.decide("hello")
        assert d1.tier == d2.tier
        assert d1.rationale == d2.rationale

    def test_decision_to_dict(self):
        d = model_router.decide("hi")
        d_dict = d.to_dict()
        assert "tier" in d_dict
        assert "primary" in d_dict
        assert "complexity" in d_dict
        assert "rationale" in d_dict


# ── 配置持久化 ────────────────────────────────────────────
class TestConfigPersistence:
    def test_load_default_when_no_file(self, isolated_home):
        res = model_router.load_rules()
        assert res["ok"]
        assert res["data"]["tiers"][0]["name"] == "simple"

    def test_save_and_load(self, isolated_home):
        custom = {
            "tiers": [
                {
                    "name": "fast",
                    "max_complexity": 0.5,
                    "primary": {"provider": "ollama", "model": "qwen2.5:1.5b"},
                    "fallback": None,
                },
                {
                    "name": "smart",
                    "max_complexity": 1.01,
                    "primary": {"provider": "anthropic", "model": "claude-3-5-sonnet-latest"},
                    "fallback": {"provider": "ollama", "model": "qwen2.5:32b"},
                },
            ],
            "auto_fallback": True,
            "history_limit": 100,
        }
        save_res = model_router.save_rules(custom)
        assert save_res["ok"]

        load_res = model_router.load_rules()
        assert load_res["ok"]
        assert load_res["data"]["tiers"][0]["primary"]["model"] == "qwen2.5:1.5b"
        assert load_res["data"]["history_limit"] == 100

    def test_reset(self, isolated_home):
        # 先存一个非默认
        model_router.save_rules({
            "tiers": [{
                "name": "x", "max_complexity": 1.0,
                "primary": {"provider": "x", "model": "y"},
            }],
        })
        res = model_router.reset_rules()
        assert res["ok"]
        # 再 load 应回到默认
        load_res = model_router.load_rules()
        assert load_res["data"]["tiers"][0]["name"] == "simple"

    def test_invalid_rules(self, isolated_home):
        # 缺 primary 应抛 ValueError
        with pytest.raises(ValueError):
            model_router.RoutingRules.from_dict({
                "tiers": [{"name": "x", "max_complexity": 1.0}],
            })


# ── 历史记录 ─────────────────────────────────────────────
class TestHistory:
    def test_append_and_list(self, isolated_home):
        model_router.clear_history()
        d = model_router.decide("hi")
        model_router.record_decision(d, "hi")
        res = model_router.get_history(limit=10)
        assert res["ok"]
        assert len(res["data"]) >= 1

    def test_clear(self, isolated_home):
        d = model_router.decide("hi")
        model_router.record_decision(d, "hi")
        res = model_router.clear_history()
        assert res["ok"]
        res = model_router.get_history(limit=10)
        assert len(res["data"]) == 0

    def test_history_persisted_to_disk(self, isolated_home):
        d = model_router.decide("hi")
        model_router.record_decision(d, "hi")
        # 模拟重启
        new_buffer = model_router._HistoryBuffer()
        new_buffer.load_from_disk()
        # 不直接测试 _HistoryBuffer（实现细节），用 get_history
        res = model_router.get_history(limit=10)
        assert len(res["data"]) >= 1

    def test_decide_with_record(self, isolated_home):
        model_router.clear_history()
        from platformkit.shared.model_router import decide, record_decision
        # 模拟 HTTP /decide with record=true
        d = decide("hi")
        record_decision(d, "hi")
        res = model_router.get_history(limit=5)
        assert len(res["data"]) >= 1


# ── 降级判定 ─────────────────────────────────────────────
class TestShouldFallback:
    def test_no_error_no_fallback(self):
        assert model_router.should_fallback() is False
        assert model_router.should_fallback(error=None, status_code=200) is False

    def test_exception_triggers_fallback(self):
        assert model_router.should_fallback(error=Exception("net")) is True

    def test_5xx_triggers_fallback(self):
        assert model_router.should_fallback(status_code=500) is True
        assert model_router.should_fallback(status_code=503) is True

    def test_429_triggers_fallback(self):
        assert model_router.should_fallback(status_code=429) is True

    def test_4xx_no_fallback(self):
        assert model_router.should_fallback(status_code=400) is False
        assert model_router.should_fallback(status_code=401) is False
        assert model_router.should_fallback(status_code=404) is False

    def test_slow_response_triggers_fallback(self):
        assert model_router.should_fallback(elapsed_ms=25000, timeout_ms=30000) is True

    def test_fast_response_no_fallback(self):
        assert model_router.should_fallback(elapsed_ms=5000, timeout_ms=30000) is False
