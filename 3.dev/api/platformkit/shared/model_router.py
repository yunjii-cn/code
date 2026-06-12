"""Platform Shared - 智能模型路由核心 (TASK-4.6)

**目标**（按 TASK-4.6 验收）：
    - 简单对话走本地（Ollama）
    - 复杂任务自动走云端（Claude / GPT）
    - 云端失败时自动降级到本地
    - 路由规则用户可配置

**设计原则**:
    - 路由决策是**纯函数**（可测试、可重放）
    - 实际网络请求不在本模块（路由层做）
    - 配置持久化到 ~/.yunji/model-router.json
    - 不依赖 FastAPI / httpx 等外部库

**复杂度评分**（0.0 - 1.0）:
    1. **长度分**（0-0.4）: token 估算（中文字符 × 1.5 + 英文词 × 1.3）
       - < 100 token = 0
       - 100-500 = 0.1
       - 500-2000 = 0.2
       - 2000-5000 = 0.3
       - > 5000 = 0.4
    2. **关键词分**（0-0.4）: 复杂任务关键词命中
       - 强信号: "重构", "架构", "设计", "完整", "深入", "全栈", "transformer", "kubernetes", "distributed"
       - 中信号: "实现", "写一个", "分析", "优化", "调试", "算法", "加密"
       - 弱信号: "改", "看", "加", "删", "是", "什么"
       每个强信号 +0.2，中信号 +0.1，弱信号 -0.05（噪音）封顶 0.4
    3. **结构分**（0-0.2）:
       - 代码块 ``` 数量 > 0: +0.05 每个，最高 0.15
       - 多模态（有图片）: +0.15
       - 多文件路径（/path 出现 > 2 次）: +0.05
    4. **会话长度分**（0-0.1）: 历史消息数 / 20，封顶 0.1

**路由规则**:
    {
        "tiers": [
            {"name": "simple", "max_complexity": 0.3,
             "primary": {"provider": "ollama", "model": "qwen2.5:7b"},
             "fallback": {"provider": "ollama", "model": "qwen2.5:3b"}},
            {"name": "medium", "max_complexity": 0.7,
             "primary": {"provider": "ollama", "model": "qwen2.5:32b"},
             "fallback": {"provider": "openai", "model": "gpt-4o-mini"}},
            {"name": "complex", "max_complexity": 1.01,
             "primary": {"provider": "anthropic", "model": "claude-3-5-sonnet"},
             "fallback": {"provider": "ollama", "model": "qwen2.5:32b"}},
        ],
        "auto_fallback": true,
        "history_limit": 200,
    }

**降级链**:
    primary 失败（超时/5xx/网络）→ fallback 失败 → error

**使用规则**:
    - ✅ 路由层: `from platformkit.shared import model_router`
    - ✅ 业务层: `decision = model_router.decide(prompt, history, rules)`
    - ❌ 禁止: 在 routes/*.py 中重新实现评分逻辑
"""
from __future__ import annotations

import json
import os
import re
import time
from collections import deque
from dataclasses import dataclass, asdict, field
from pathlib import Path
from threading import Lock
from typing import Optional


# ── 常量 ────────────────────────────────────────────────────────
_STRONG_KEYWORDS = (
    "重构", "架构", "设计", "完整", "深入", "全栈", "分布式",
    "transformer", "kubernetes", "distributed", "architecture",
    "微服务", "concurrency", "并发", "算法", "算法设计",
    "database schema", "数据库设计", "compiler", "编译器",
    "operating system", "操作系统", "from scratch", "从头实现",
)
_MEDIUM_KEYWORDS = (
    "实现", "写一个", "分析", "优化", "调试", "加密", "解密",
    "refactor", "implement", "design", "build", "create", "develop",
    "complex", "复杂", "完整代码", "完整方案", "系统设计",
    "性能", "performance", "benchmark", "压测",
    "migration", "迁移", "安全", "security", "漏洞", "vulnerability",
)
_WEAK_KEYWORDS = (
    "改", "看", "加", "删", "是", "什么", "你好", "hi", "hello",
    "thanks", "谢谢", "ok", "好的",
)
_CODE_FENCE_PATTERN = re.compile(r"```")
_FILE_PATH_PATTERN = re.compile(r"(?:^|\s)(/[\w./\-]+|\.{1,2}/[\w./\-]+|[A-Z]:\\[\w.\\\-]+)", re.MULTILINE)


# ── 数据结构 ────────────────────────────────────────────────────
@dataclass
class ModelRef:
    provider: str
    model: str

    def to_dict(self) -> dict:
        return asdict(self)

    @classmethod
    def from_dict(cls, d: dict) -> "ModelRef":
        provider = str(d.get("provider", "") or "").strip()
        model = str(d.get("model", "") or "").strip()
        if not provider:
            raise ValueError("ModelRef.provider 不能为空")
        if not model:
            raise ValueError("ModelRef.model 不能为空")
        return cls(provider=provider, model=model)


@dataclass
class RoutingTier:
    name: str
    max_complexity: float
    primary: ModelRef
    fallback: Optional[ModelRef] = None

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "max_complexity": self.max_complexity,
            "primary": self.primary.to_dict(),
            "fallback": self.fallback.to_dict() if self.fallback else None,
        }

    @classmethod
    def from_dict(cls, d: dict) -> "RoutingTier":
        return cls(
            name=str(d.get("name", "")),
            max_complexity=float(d.get("max_complexity", 1.0)),
            primary=ModelRef.from_dict(d.get("primary") or {}),
            fallback=ModelRef.from_dict(d["fallback"]) if d.get("fallback") else None,
        )


@dataclass
class RoutingRules:
    tiers: list[RoutingTier] = field(default_factory=list)
    auto_fallback: bool = True
    history_limit: int = 200

    def to_dict(self) -> dict:
        return {
            "tiers": [t.to_dict() for t in self.tiers],
            "auto_fallback": self.auto_fallback,
            "history_limit": self.history_limit,
        }

    @classmethod
    def from_dict(cls, d: dict) -> "RoutingRules":
        tiers_raw = d.get("tiers") or []
        tiers = []
        for t in tiers_raw:
            try:
                tiers.append(RoutingTier.from_dict(t))
            except (KeyError, ValueError, TypeError) as e:
                raise ValueError(f"无效 tier: {e}") from e
        if not tiers:
            raise ValueError("至少需要一个 tier")
        return cls(
            tiers=tiers,
            auto_fallback=bool(d.get("auto_fallback", True)),
            history_limit=int(d.get("history_limit", 200)),
        )


@dataclass
class ComplexityScore:
    total: float
    length: float
    keywords: float
    structure: float
    history: float
    matched_keywords: list[str] = field(default_factory=list)
    signals: list[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class RoutingDecision:
    tier: str
    primary: ModelRef
    fallback: Optional[ModelRef]
    complexity: ComplexityScore
    rationale: str
    timestamp: float = field(default_factory=lambda: time.time())

    def to_dict(self) -> dict:
        return {
            "tier": self.tier,
            "primary": self.primary.to_dict(),
            "fallback": self.fallback.to_dict() if self.fallback else None,
            "complexity": self.complexity.to_dict(),
            "rationale": self.rationale,
            "timestamp": self.timestamp,
        }


# ── 默认规则 ────────────────────────────────────────────────────
def _default_rules() -> RoutingRules:
    return RoutingRules(tiers=[
        RoutingTier(
            name="simple",
            max_complexity=0.3,
            primary=ModelRef(provider="ollama", model="qwen2.5:3b"),
            fallback=ModelRef(provider="ollama", model="qwen2.5:1.5b"),
        ),
        RoutingTier(
            name="medium",
            max_complexity=0.7,
            primary=ModelRef(provider="ollama", model="qwen2.5:7b"),
            fallback=ModelRef(provider="zhipu", model="glm-4-flash"),
        ),
        RoutingTier(
            name="complex",
            max_complexity=1.01,
            primary=ModelRef(provider="anthropic", model="claude-3-5-sonnet-latest"),
            fallback=ModelRef(provider="ollama", model="qwen2.5:32b"),
        ),
    ], auto_fallback=True, history_limit=200)


DEFAULT_RULES: RoutingRules = _default_rules()


# ── 工具 ────────────────────────────────────────────────────────
def _make_result(ok: bool, data=None, error: Optional[str] = None) -> dict:
    return {"ok": ok, "data": data, "error": error}


def _config_path() -> Path:
    p = Path.home() / ".yunji" / "model-router.json"
    p.parent.mkdir(parents=True, exist_ok=True)
    return p


def _history_path() -> Path:
    p = Path.home() / ".yunji" / "model-router-history.jsonl"
    p.parent.mkdir(parents=True, exist_ok=True)
    return p


# ── 复杂度评分 ──────────────────────────────────────────────────
def _estimate_tokens(text: str) -> int:
    """粗略估算 token 数：中文字符 × 1.5，英文词 × 1.3，其它字符 × 0.5。"""
    if not text:
        return 0
    chinese = sum(1 for c in text if "\u4e00" <= c <= "\u9fff")
    # 英文词 = 连续 a-zA-Z
    english_words = len(re.findall(r"[a-zA-Z]+", text))
    other = max(0, len(text) - chinese - sum(len(w) for w in re.findall(r"[a-zA-Z]+", text)))
    return int(chinese * 1.5 + english_words * 1.3 + other * 0.5)


def _score_length(prompt: str) -> tuple[float, int]:
    tokens = _estimate_tokens(prompt)
    if tokens < 100:
        return 0.0, tokens
    if tokens < 500:
        return 0.1, tokens
    if tokens < 2000:
        return 0.2, tokens
    if tokens < 5000:
        return 0.3, tokens
    return 0.4, tokens


def _score_keywords(prompt: str) -> tuple[float, list[str]]:
    p = prompt.lower()
    matched: list[str] = []
    score = 0.0
    # 强信号 +0.2
    for kw in _STRONG_KEYWORDS:
        if kw.lower() in p:
            matched.append(kw)
            score += 0.2
    # 中信号 +0.1
    for kw in _MEDIUM_KEYWORDS:
        if kw.lower() in p:
            matched.append(kw)
            score += 0.1
    # 弱信号（噪音）-0.05
    for kw in _WEAK_KEYWORDS:
        if kw in p:
            score -= 0.05
    # 封顶
    score = max(0.0, min(0.4, score))
    return score, matched


def _score_structure(prompt: str, has_images: bool = False) -> tuple[float, list[str]]:
    signals: list[str] = []
    score = 0.0
    # 代码块
    fences = _CODE_FENCE_PATTERN.findall(prompt)
    if len(fences) >= 2:
        n_blocks = len(fences) // 2
        score += min(0.15, n_blocks * 0.05)
        signals.append(f"{n_blocks} code blocks")
    # 多模态
    if has_images:
        score += 0.15
        signals.append("multimodal(image)")
    # 文件路径
    paths = _FILE_PATH_PATTERN.findall(prompt)
    if len(paths) > 2:
        score += 0.05
        signals.append(f"{len(paths)} file paths")
    score = min(0.2, score)
    return score, signals


def _score_history(history: list) -> float:
    """会话长度分：消息数 / 20，封顶 0.1。"""
    n = len(history or [])
    return min(0.1, n / 20.0 / 10.0)  # 200 条消息 ≈ 0.1


def evaluate_complexity(
    prompt: str,
    history: Optional[list] = None,
    has_images: bool = False,
) -> ComplexityScore:
    """评估输入的复杂度（0-1）。"""
    history = history or []
    length, tokens = _score_length(prompt)
    keywords, matched = _score_keywords(prompt)
    structure, signals = _score_structure(prompt, has_images=has_images)
    hist = _score_history(history)
    total = length + keywords + structure + hist
    total = max(0.0, min(1.0, total))
    return ComplexityScore(
        total=round(total, 3),
        length=round(length, 3),
        keywords=round(keywords, 3),
        structure=round(structure, 3),
        history=round(hist, 3),
        matched_keywords=matched,
        signals=signals,
    )


# ── 路由决策 ────────────────────────────────────────────────────
def decide(
    prompt: str,
    rules: Optional[RoutingRules] = None,
    history: Optional[list] = None,
    has_images: bool = False,
) -> RoutingDecision:
    """根据 prompt + 规则 + 历史做出路由决策（纯函数）。

    Args:
        prompt: 用户输入
        rules: 路由规则（None = 用默认）
        history: 历史消息列表（仅用于评分）
        has_images: 是否带图片

    Returns:
        RoutingDecision
    """
    rules = rules or DEFAULT_RULES
    complexity = evaluate_complexity(prompt, history=history, has_images=has_images)

    chosen: Optional[RoutingTier] = None
    for tier in rules.tiers:
        if complexity.total <= tier.max_complexity:
            chosen = tier
            break
    if chosen is None and rules.tiers:
        chosen = rules.tiers[-1]

    if chosen is None:
        # 兜底
        chosen = DEFAULT_RULES.tiers[-1]

    # 构造理由
    reasons: list[str] = []
    if complexity.length > 0:
        reasons.append(f"length={complexity.length:.2f}")
    if complexity.matched_keywords:
        reasons.append(f"keywords=[{','.join(complexity.matched_keywords[:3])}]")
    if complexity.signals:
        reasons.append(f"signals=[{','.join(complexity.signals)}]")
    rationale = f"tier={chosen.name}, complexity={complexity.total:.2f} ({'; '.join(reasons) or 'baseline'})"

    return RoutingDecision(
        tier=chosen.name,
        primary=chosen.primary,
        fallback=chosen.fallback,
        complexity=complexity,
        rationale=rationale,
    )


# ── 配置持久化 ──────────────────────────────────────────────────
def load_rules() -> dict:
    """从 ~/.yunji/model-router.json 加载规则。"""
    p = _config_path()
    if not p.exists():
        return _make_result(True, DEFAULT_RULES.to_dict())
    try:
        text = p.read_text(encoding="utf-8")
        raw = json.loads(text)
        rules = RoutingRules.from_dict(raw)
        return _make_result(True, rules.to_dict())
    except (json.JSONDecodeError, OSError, ValueError) as e:
        return _make_result(False, error=f"加载规则失败: {e}")


def save_rules(rules_dict: dict) -> dict:
    """保存规则到 ~/.yunji/model-router.json。"""
    try:
        rules = RoutingRules.from_dict(rules_dict)
    except (ValueError, TypeError) as e:
        return _make_result(False, error=f"规则格式错误: {e}")
    p = _config_path()
    try:
        p.write_text(json.dumps(rules.to_dict(), ensure_ascii=False, indent=2), encoding="utf-8")
        return _make_result(True, rules.to_dict())
    except OSError as e:
        return _make_result(False, error=f"保存失败: {e}")


def reset_rules() -> dict:
    """重置为默认规则。"""
    try:
        p = _config_path()
        if p.exists():
            p.unlink()
    except OSError:
        pass
    return _make_result(True, DEFAULT_RULES.to_dict())


# ── 历史记录 ────────────────────────────────────────────────────
class _HistoryBuffer:
    """线程安全的环形历史缓冲（FIFO，限制条数）。"""
    def __init__(self, limit: int = 200):
        self._lock = Lock()
        self._buf: deque = deque(maxlen=limit)
        self._path = _history_path()

    def append(self, record: dict) -> None:
        with self._lock:
            self._buf.append(record)
            # 追加到磁盘
            try:
                with self._path.open("a", encoding="utf-8") as f:
                    f.write(json.dumps(record, ensure_ascii=False) + "\n")
            except OSError:
                pass

    def list(self) -> list[dict]:
        with self._lock:
            return list(self._buf)

    def clear(self) -> None:
        with self._lock:
            self._buf.clear()
            try:
                if self._path.exists():
                    self._path.unlink()
            except OSError:
                pass

    def load_from_disk(self, limit: int = 200) -> None:
        """冷启动时从磁盘读回最近 N 条。"""
        if not self._path.exists():
            return
        try:
            lines = self._path.read_text(encoding="utf-8").splitlines()[-limit:]
            with self._lock:
                self._buf.clear()
                for line in lines:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        self._buf.append(json.loads(line))
                    except json.JSONDecodeError:
                        continue
        except OSError:
            pass


_history: _HistoryBuffer = _HistoryBuffer()
_history.load_from_disk()


def record_decision(decision: RoutingDecision, prompt_preview: str = "") -> dict:
    """把路由决策写入历史。"""
    record = {
        **decision.to_dict(),
        "prompt_preview": prompt_preview[:200],
    }
    _history.append(record)
    return _make_result(True, record)


def get_history(limit: int = 50) -> dict:
    """读取最近 N 条路由历史。"""
    items = _history.list()
    if limit > 0:
        items = items[-limit:]
    items.reverse()  # 最新的在前
    return _make_result(True, items)


def clear_history() -> dict:
    """清空历史。"""
    _history.clear()
    return _make_result(True, [])


# ── 降级辅助 ────────────────────────────────────────────────────
def should_fallback(
    error: Optional[BaseException] = None,
    status_code: Optional[int] = None,
    elapsed_ms: Optional[int] = None,
    timeout_ms: int = 30000,
) -> bool:
    """判断是否应该降级（不抛错就 false）。

    触发降级的条件：
        - 网络/超时异常
        - HTTP 5xx
        - HTTP 429 (rate limit)
        - 响应时间 > 80% timeout
    """
    if error is not None:
        return True
    if status_code is not None:
        if status_code >= 500 or status_code == 429:
            return True
    if elapsed_ms is not None and timeout_ms > 0:
        if elapsed_ms > int(timeout_ms * 0.8):
            return True
    return False


__all__ = [
    "ModelRef",
    "RoutingTier",
    "RoutingRules",
    "ComplexityScore",
    "RoutingDecision",
    "DEFAULT_RULES",
    "evaluate_complexity",
    "decide",
    "load_rules",
    "save_rules",
    "reset_rules",
    "record_decision",
    "get_history",
    "clear_history",
    "should_fallback",
    "_estimate_tokens",
]
