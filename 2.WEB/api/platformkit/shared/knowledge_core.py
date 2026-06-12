"""Platform Shared Knowledge Core - 自进化知识系统核心

2026-06-09 TASK-3.1 引入：Phase 3 W9 自进化知识系统核心
四层知识 + 强度演化（weak → medium → strong）

四层知识:
    L1 纠正 (Corrections)  — "不要用 var，用 const"
    L2 模式 (Patterns)     — "用户连续3次手动测试→提交→部署"
    L3 事实 (Facts)        — "这个项目用 PostgreSQL"
    L4 偏好 (Preferences)  — "我喜欢 Tab 缩进"

强度演化:
    weak   → medium  : 确认 2 次未被纠正
    medium → strong  : 确认 4 次未被纠正
    任何层级可被用户明确确认（confirm() 立即跳 strong）
    注：强度演化基于 confirm_count 累积判定，简单可解释

存储:
    项目级: <project_root>/.yunji/knowledge/{layer}.json
    全局级: ~/.yunji/knowledge/{layer}.json
    每层一个 JSON 文件，简单可读，可手工编辑

使用示例:
    engine = KnowledgeEngine(project_root=Path("/path/to/project"))
    k = engine.add_correction("不要用 var，用 const", source="user")
    engine.confirm(k.id)  # 强度提升

新代码导入:
    from platformkit.shared.knowledge_core import KnowledgeEngine, Knowledge, KnowledgeLayer

设计原则:
    - 零外部依赖（仅 stdlib）
    - 不依赖 FastAPI，可被 daemon 复用
    - 文件锁避免并发冲突（可选）
    - 关键词相关性评分（占位，Phase 4 可替换 embedding）
"""

from __future__ import annotations

import json
import re
import time
import uuid
from dataclasses import asdict, dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Union


class KnowledgeLayer(str, Enum):
    """四层知识层级"""

    CORRECTION = "L1"  # 纠正
    PATTERN = "L2"     # 模式
    FACT = "L3"        # 事实
    PREFERENCE = "L4"  # 偏好


class KnowledgeStrength(str, Enum):
    """知识强度等级（演化模型）"""

    WEAK = "weak"
    MEDIUM = "medium"
    STRONG = "strong"


class KnowledgeScope(str, Enum):
    """知识作用域"""

    PROJECT = "project"  # 项目级：<project_root>/.yunji/knowledge/
    GLOBAL = "global"    # 全局级：~/.yunji/knowledge/


# 强度演化阈值（confirm_count 累积）
STRENGTH_THRESHOLDS: Dict[KnowledgeStrength, int] = {
    KnowledgeStrength.WEAK: 0,    # 默认
    KnowledgeStrength.MEDIUM: 2,  # 确认 2 次
    KnowledgeStrength.STRONG: 4,  # 确认 4 次
}

# 强度排序权重（用于 get_relevant 加权）
STRENGTH_WEIGHTS: Dict[KnowledgeStrength, float] = {
    KnowledgeStrength.WEAK: 1.0,
    KnowledgeStrength.MEDIUM: 1.5,
    KnowledgeStrength.STRONG: 2.0,
}


@dataclass
class Knowledge:
    """单条知识"""

    id: str
    content: str
    layer: KnowledgeLayer
    strength: KnowledgeStrength
    scope: KnowledgeScope
    created_at: float
    updated_at: float
    confirm_count: int = 0
    source: Optional[str] = None  # 来源：'user' / 'ai' / 'error_correction' / 任意标识

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["layer"] = self.layer.value
        d["strength"] = self.strength.value
        d["scope"] = self.scope.value
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "Knowledge":
        return cls(
            id=d["id"],
            content=d["content"],
            layer=KnowledgeLayer(d["layer"]),
            strength=KnowledgeStrength(d["strength"]),
            scope=KnowledgeScope(d["scope"]),
            created_at=d["created_at"],
            updated_at=d["updated_at"],
            confirm_count=d.get("confirm_count", 0),
            source=d.get("source"),
        )


def _default_global_root() -> Path:
    """全局知识根目录优先级：
    1. YUNJI_GLOBAL_ROOT 环境变量（测试 / 运维覆盖）
    2. ~/.yunji/knowledge/
    """
    import os
    env_root = os.environ.get("YUNJI_GLOBAL_ROOT")
    if env_root:
        return Path(env_root) / "knowledge"
    return Path.home() / ".yunji" / "knowledge"


def _project_knowledge_dir(project_root: Path) -> Path:
    """项目级知识根目录：<project_root>/.yunji/knowledge/"""
    return project_root / ".yunji" / "knowledge"


def _layer_filename(layer: KnowledgeLayer) -> str:
    """层级 → 文件名（l1.json / l2.json / l3.json / l4.json）"""
    return f"{layer.value.lower()}.json"


def _tokenize(text: str) -> List[str]:
    """简单分词（英文/数字 + 中文按字）。Phase 4 可换成 jieba/BPE"""
    # 英文/数字连续 + 单个中文字
    return re.findall(r"[A-Za-z0-9_]+|[\u4e00-\u9fff]", text.lower())


class KnowledgeEngine:
    """自进化知识引擎

    核心能力：
    - 四层知识增删改查
    - 强度演化（confirm_count 累积触发提升）
    - 跨作用域迁移（promote_to_global）
    - 基于关键词的相关性检索（占位实现，Phase 4 可升级 embedding）
    """

    def __init__(
        self,
        project_root: Optional[Union[Path, str]] = None,
        global_root: Optional[Union[Path, str]] = None,
    ) -> None:
        self.project_root: Optional[Path] = Path(project_root).resolve() if project_root else None
        self.global_root: Optional[Path] = Path(global_root).resolve() if global_root else _default_global_root()

    # ──────────── 路径解析 ────────────

    def _dir_for(self, scope: KnowledgeScope) -> Path:
        if scope == KnowledgeScope.PROJECT:
            if not self.project_root:
                raise ValueError("project_root is required for project-scope knowledge")
            return _project_knowledge_dir(self.project_root)
        return self.global_root

    def _file_for(self, scope: KnowledgeScope, layer: KnowledgeLayer) -> Path:
        return self._dir_for(scope) / _layer_filename(layer)

    # ──────────── 加载 / 保存 ────────────

    def _load_layer(self, scope: KnowledgeScope, layer: KnowledgeLayer) -> List[Knowledge]:
        f = self._file_for(scope, layer)
        if not f.exists():
            return []
        try:
            raw = json.loads(f.read_text(encoding="utf-8"))
            if not isinstance(raw, list):
                return []
            return [Knowledge.from_dict(entry) for entry in raw if isinstance(entry, dict)]
        except (json.JSONDecodeError, KeyError, ValueError):
            # 损坏文件不抛错，返回空列表（容错性）
            return []

    def _save_layer(self, scope: KnowledgeScope, layer: KnowledgeLayer, items: List[Knowledge]) -> None:
        d = self._dir_for(scope)
        d.mkdir(parents=True, exist_ok=True)
        f = self._file_for(scope, layer)
        payload = [k.to_dict() for k in items if k.scope == scope and k.layer == layer]
        f.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")

    def _load_scope(self, scope: KnowledgeScope) -> List[Knowledge]:
        result: List[Knowledge] = []
        for layer in KnowledgeLayer:
            result.extend(self._load_layer(scope, layer))
        return result

    def _save_scope(self, scope: KnowledgeScope, items: List[Knowledge]) -> None:
        """按层级分组后分别保存"""
        by_layer: Dict[KnowledgeLayer, List[Knowledge]] = {layer: [] for layer in KnowledgeLayer}
        for k in items:
            if k.scope == scope:
                by_layer[k.layer].append(k)
        for layer, lst in by_layer.items():
            self._save_layer(scope, layer, lst)

    # ──────────── 增（API 设计文档要求）────────────

    def add_correction(
        self,
        content: str,
        scope: Union[KnowledgeScope, str] = KnowledgeScope.PROJECT,
        strength: Union[KnowledgeStrength, str] = KnowledgeStrength.WEAK,
        source: Optional[str] = None,
    ) -> Knowledge:
        return self._add(content, KnowledgeLayer.CORRECTION, scope, strength, source)

    def add_pattern(
        self,
        content: str,
        scope: Union[KnowledgeScope, str] = KnowledgeScope.PROJECT,
        strength: Union[KnowledgeStrength, str] = KnowledgeStrength.WEAK,
        source: Optional[str] = None,
    ) -> Knowledge:
        return self._add(content, KnowledgeLayer.PATTERN, scope, strength, source)

    def add_fact(
        self,
        content: str,
        scope: Union[KnowledgeScope, str] = KnowledgeScope.PROJECT,
        strength: Union[KnowledgeStrength, str] = KnowledgeStrength.WEAK,
        source: Optional[str] = None,
    ) -> Knowledge:
        return self._add(content, KnowledgeLayer.FACT, scope, strength, source)

    def add_preference(
        self,
        content: str,
        scope: Union[KnowledgeScope, str] = KnowledgeScope.GLOBAL,
        strength: Union[KnowledgeStrength, str] = KnowledgeStrength.WEAK,
        source: Optional[str] = None,
    ) -> Knowledge:
        return self._add(content, KnowledgeLayer.PREFERENCE, scope, strength, source)

    def _add(
        self,
        content: str,
        layer: KnowledgeLayer,
        scope: Union[KnowledgeScope, str],
        strength: Union[KnowledgeStrength, str],
        source: Optional[str],
    ) -> Knowledge:
        scope_e = scope if isinstance(scope, KnowledgeScope) else KnowledgeScope(scope)
        strength_e = strength if isinstance(strength, KnowledgeStrength) else KnowledgeStrength(strength)
        if scope_e == KnowledgeScope.GLOBAL and layer == KnowledgeLayer.CORRECTION:
            # 纠正通常与项目耦合，默认项目级；如需全局显式传
            pass

        now = time.time()
        k = Knowledge(
            id=str(uuid.uuid4()),
            content=content,
            layer=layer,
            strength=strength_e,
            scope=scope_e,
            created_at=now,
            updated_at=now,
            confirm_count=0,
            source=source,
        )
        items = self._load_scope(scope_e)
        items.append(k)
        self._save_scope(scope_e, items)
        return k

    # ──────────── 强度演化 ────────────

    def confirm(self, knowledge_id: str, force_strong: bool = False) -> Knowledge:
        """确认一条知识，提升强度

        Args:
            knowledge_id: 知识 ID
            force_strong: 立即跳到 strong（用户明确"对，就是这样"）
        """
        for scope in KnowledgeScope:
            items = self._load_scope(scope)
            for k in items:
                if k.id == knowledge_id:
                    k.confirm_count += 1
                    k.updated_at = time.time()
                    if force_strong:
                        k.strength = KnowledgeStrength.STRONG
                    else:
                        # 按阈值演化
                        if k.strength == KnowledgeStrength.WEAK and k.confirm_count >= STRENGTH_THRESHOLDS[KnowledgeStrength.MEDIUM]:
                            k.strength = KnowledgeStrength.MEDIUM
                        if k.strength == KnowledgeStrength.MEDIUM and k.confirm_count >= STRENGTH_THRESHOLDS[KnowledgeStrength.STRONG]:
                            k.strength = KnowledgeStrength.STRONG
                    self._save_scope(scope, items)
                    return k
        raise KeyError(f"knowledge not found: {knowledge_id}")

    def deny(self, knowledge_id: str) -> Knowledge:
        """否定/反对一条知识，重置为 weak 并清空 confirm_count"""
        for scope in KnowledgeScope:
            items = self._load_scope(scope)
            for k in items:
                if k.id == knowledge_id:
                    k.strength = KnowledgeStrength.WEAK
                    k.confirm_count = 0
                    k.updated_at = time.time()
                    self._save_scope(scope, items)
                    return k
        raise KeyError(f"knowledge not found: {knowledge_id}")

    def delete(self, knowledge_id: str) -> bool:
        """删除一条知识"""
        for scope in KnowledgeScope:
            items = self._load_scope(scope)
            new_items = [k for k in items if k.id != knowledge_id]
            if len(new_items) != len(items):
                self._save_scope(scope, new_items)
                return True
        return False

    # ──────────── 作用域迁移 ────────────

    def promote_to_global(self, knowledge_id: str) -> Knowledge:
        """项目级 → 全局级"""
        items = self._load_scope(KnowledgeScope.PROJECT)
        for k in items:
            if k.id == knowledge_id:
                k.scope = KnowledgeScope.GLOBAL
                k.updated_at = time.time()
                # 从项目级删除
                items = [x for x in items if x.id != knowledge_id]
                self._save_scope(KnowledgeScope.PROJECT, items)
                # 加入全局级
                global_items = self._load_scope(KnowledgeScope.GLOBAL)
                global_items.append(k)
                self._save_scope(KnowledgeScope.GLOBAL, global_items)
                return k
        raise KeyError(f"knowledge not found: {knowledge_id}")

    def demote_to_project(self, knowledge_id: str, project_root: Path) -> Knowledge:
        """全局级 → 项目级（demote 较少用，先实现备用）"""
        items = self._load_scope(KnowledgeScope.GLOBAL)
        for k in items:
            if k.id == knowledge_id:
                k.scope = KnowledgeScope.PROJECT
                k.updated_at = time.time()
                items = [x for x in items if x.id != knowledge_id]
                self._save_scope(KnowledgeScope.GLOBAL, items)
                project_items = self._load_scope(KnowledgeScope.PROJECT)
                project_items.append(k)
                self._save_scope(KnowledgeScope.PROJECT, project_items)
                return k
        raise KeyError(f"knowledge not found: {knowledge_id}")

    # ──────────── 查询 ────────────

    def get(self, knowledge_id: str) -> Optional[Knowledge]:
        for scope in KnowledgeScope:
            for k in self._load_scope(scope):
                if k.id == knowledge_id:
                    return k
        return None

    def list_by_layer(self, layer: Union[KnowledgeLayer, str]) -> List[Knowledge]:
        layer_e = layer if isinstance(layer, KnowledgeLayer) else KnowledgeLayer(layer)
        result: List[Knowledge] = []
        for scope in KnowledgeScope:
            result.extend(self._load_layer(scope, layer_e))
        return result

    def list_by_strength(self, strength: Union[KnowledgeStrength, str]) -> List[Knowledge]:
        strength_e = strength if isinstance(strength, KnowledgeStrength) else KnowledgeStrength(strength)
        result: List[Knowledge] = []
        for scope in KnowledgeScope:
            for k in self._load_scope(scope):
                if k.strength == strength_e:
                    result.append(k)
        return result

    def list_all(self) -> List[Knowledge]:
        result: List[Knowledge] = []
        for scope in KnowledgeScope:
            result.extend(self._load_scope(scope))
        return result

    # ──────────── 相关性检索（关键词版，占位）────────────

    def get_relevant(self, context: str, top_k: int = 5) -> List[Knowledge]:
        """基于关键词的相关性评分，强度加权

        Phase 4 计划升级为 embedding 相似度。当前实现：
        - 分词（英文/数字/单字中文）
        - 与知识内容交集大小 = 基础分
        - 乘以强度权重
        - 按分数降序返回 top_k
        """
        ctx_tokens = set(_tokenize(context))
        if not ctx_tokens:
            return []
        all_items = self.list_all()
        scored: List[tuple] = []
        for k in all_items:
            k_tokens = set(_tokenize(k.content))
            overlap = len(ctx_tokens & k_tokens)
            if overlap == 0:
                continue
            weight = STRENGTH_WEIGHTS[k.strength]
            score = overlap * weight
            scored.append((score, k.updated_at, k))
        scored.sort(key=lambda x: (x[0], x[1]), reverse=True)
        return [k for _, _, k in scored[:top_k]]

    # ──────────── 统计 ────────────

    def stats(self) -> Dict[str, Any]:
        items = self.list_all()
        by_layer = {layer.value: 0 for layer in KnowledgeLayer}
        by_strength = {s.value: 0 for s in KnowledgeStrength}
        by_scope = {s.value: 0 for s in KnowledgeScope}
        for k in items:
            by_layer[k.layer.value] += 1
            by_strength[k.strength.value] += 1
            by_scope[k.scope.value] += 1
        return {
            "total": len(items),
            "by_layer": by_layer,
            "by_strength": by_strength,
            "by_scope": by_scope,
        }
