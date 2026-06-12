"""Platform Shared - 工程化持久化核心 (Durable Store)

2026-06-09 TASK-3.7 引入：Phase 3 W11 工程化持久化

设计目标（按 plan）:
    - 6 个文件正确读写
    - state.json 格式稳定
    - 跨会话知识（已由 KnowledgeEngine 实现，本模块仅做 read 代理）
    - 技能可复用（skills/ 目录）
    - 真实进度不被改写成设计散文（checklist 真实状态）

6 个文件清单:
    .yunji/state.json      - 状态机（会话、断点、统计）
    .yunji/memory.md       - 跨会话长期记忆（追加式）
    .yunji/plan.md         - 当前计划（覆盖式）
    .yunji/checklist.md    - 真实进度（markdown 复选框，tick 用标题匹配）
    .yunji/knowledge/      - 知识库（KnowledgeEngine 负责，本模块只读）
    .yunji/skills/         - 可复用技能（每个 .md 文件 = 一个 skill）

外部只读:
    dev/AGENTS.md          - 项目级协作者契约（read_agents() 返回）

设计原则:
    - 不依赖 FastAPI
    - 与 KnowledgeEngine 互不耦合（state.json 引用 knowledge 统计，但不直接调用）
    - 文件失败要容错（损坏的 JSON 不抛错，回退默认值）
    - 时间戳统一用 time.time()（浮点秒）

使用示例:
    store = DurableStore("/path/to/workspace")
    store.append_memory("今天完成了 W9 TASK-3.1-3.3")
    store.update_plan("# Plan\n- [x] W9\n- [ ] W10")
    store.tick_checklist("W10")
    state = store.get_state()
    state["stats"]["total_sessions"] += 1
    store.update_state({"stats": state["stats"]})
"""

from __future__ import annotations

import json
import re
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Union


# ──────────── 常量 ────────────


# state.json 默认结构
_DEFAULT_STATE: Dict[str, Any] = {
    "version": 1,
    "created_at": 0.0,
    "updated_at": 0.0,
    "workspace": "",
    "last_session": {
        "started_at": None,
        "ended_at": None,
        "current_step": None,
        "unfinished_tasks": [],
    },
    "stats": {
        "total_sessions": 0,
        "total_messages": 0,
        "total_tool_calls": 0,
    },
}

# 6 个文件相对路径
FILE_STATE = "state.json"
FILE_MEMORY = "memory.md"
FILE_PLAN = "plan.md"
FILE_CHECKLIST = "checklist.md"
DIR_KNOWLEDGE = "knowledge"
DIR_SKILLS = "skills"

# AGENTS.md 默认查找位置（优先级递减）
_AGENTS_SEARCH_PATHS = [
    "dev/AGENTS.md",
    "AGENTS.md",
    ".yunji/AGENTS.md",
]

# 复选框正则
_CHECKBOX_DONE = re.compile(r"^\s*-\s*\[x\]\s*", re.IGNORECASE)
_CHECKBOX_TODO = re.compile(r"^\s*-\s*\[\s*\]\s*")


# ──────────── 数据模型 ────────────


@dataclass
class SkillEntry:
    name: str
    path: str
    content: str
    size: int


@dataclass
class AgentEntry:
    name: str
    path: str
    content: str
    size: int


@dataclass
class CheckItem:
    id: str  # 标题（去掉前缀方括号）
    title: str
    done: bool
    line_number: int


@dataclass
class StoreStatus:
    """DurableStore 自检状态"""

    workspace: str
    yunji_dir_exists: bool
    files: Dict[str, bool]  # 文件名 → 是否存在
    file_sizes: Dict[str, int]
    last_updated_at: Optional[float]


# ──────────── 引擎 ────────────


class DurableStore:
    """工程化持久化存储

    6 个文件 + 1 个 read-only 入口（AGENTS.md）。
    路径: <workspace>/.yunji/ 下集中存储。
    """

    def __init__(self, workspace_path: Union[Path, str], auto_init: bool = True) -> None:
        self.workspace_path: Path = Path(workspace_path).resolve()
        self.yunji_dir: Path = self.workspace_path / ".yunji"
        if auto_init:
            self.init()

    # ──────────── 初始化 ────────────

    def init(self) -> None:
        """初始化 .yunji/ 目录 + 默认 6 文件"""
        self.yunji_dir.mkdir(parents=True, exist_ok=True)
        (self.yunji_dir / DIR_KNOWLEDGE).mkdir(exist_ok=True)
        (self.yunji_dir / DIR_SKILLS).mkdir(exist_ok=True)
        # state.json
        if not (self.yunji_dir / FILE_STATE).exists():
            state = dict(_DEFAULT_STATE)
            state["workspace"] = str(self.workspace_path)
            state["created_at"] = time.time()
            state["updated_at"] = time.time()
            self._write_json(FILE_STATE, state)
        # memory.md
        if not (self.yunji_dir / FILE_MEMORY).exists():
            header = (
                "# 跨会话长期记忆\n\n"
                f"工作区: `{self.workspace_path}`  \n"
                f"初始化时间: {time.strftime('%Y-%m-%d %H:%M:%S')}  \n\n"
                "---\n\n"
            )
            self._write_text(FILE_MEMORY, header)
        # plan.md
        if not (self.yunji_dir / FILE_PLAN).exists():
            self._write_text(FILE_PLAN, "# 当前计划\n\n暂无，请用 `update_plan()` 写入。\n")
        # checklist.md
        if not (self.yunji_dir / FILE_CHECKLIST).exists():
            self._write_text(FILE_CHECKLIST, "# 项目检查清单\n\n- [ ] 初始化\n")

    # ──────────── state.json ────────────

    def get_state(self) -> Dict[str, Any]:
        """读取 state.json（损坏时返回默认 + 容错）"""
        data = self._read_json(FILE_STATE, default=dict(_DEFAULT_STATE))
        # merge 默认字段，防止早期 state.json 缺字段
        merged = dict(_DEFAULT_STATE)
        merged.update(data)
        return merged

    def update_state(self, updates: Dict[str, Any]) -> Dict[str, Any]:
        """浅合并更新 state.json。深字段（last_session / stats）整体替换传入值"""
        state = self.get_state()
        for k, v in updates.items():
            if isinstance(v, dict) and isinstance(state.get(k), dict):
                # 字典字段做二级合并
                merged = dict(state[k])
                merged.update(v)
                state[k] = merged
            else:
                state[k] = v
        state["updated_at"] = time.time()
        self._write_json(FILE_STATE, state)
        return state

    def mark_session_start(self) -> Dict[str, Any]:
        state = self.get_state()
        return self.update_state({
            "last_session": {
                "started_at": time.time(),
                "ended_at": None,
                "current_step": None,
                "unfinished_tasks": state.get("last_session", {}).get("unfinished_tasks", []),
            },
            "stats": {
                "total_sessions": state.get("stats", {}).get("total_sessions", 0) + 1,
            },
        })

    def mark_session_end(self, unfinished_tasks: Optional[List[str]] = None) -> Dict[str, Any]:
        state = self.get_state()
        last = dict(state.get("last_session", {}))
        last["ended_at"] = time.time()
        last["unfinished_tasks"] = unfinished_tasks if unfinished_tasks is not None else last.get("unfinished_tasks", [])
        return self.update_state({"last_session": last})

    # ──────────── memory.md（追加）────────────

    def append_memory(self, content: str) -> None:
        """追加一条记忆到 memory.md（带时间戳）"""
        if not content.strip():
            return
        ts = time.strftime("%Y-%m-%d %H:%M:%S")
        entry = f"\n## {ts}\n\n{content.strip()}\n"
        path = self.yunji_dir / FILE_MEMORY
        existing = path.read_text(encoding="utf-8") if path.exists() else ""
        self._write_text(FILE_MEMORY, existing + entry)

    def get_memory(self, max_entries: Optional[int] = None) -> str:
        """读取 memory.md，可选返回最近 N 条 ## 段"""
        path = self.yunji_dir / FILE_MEMORY
        if not path.exists():
            return ""
        text = path.read_text(encoding="utf-8")
        if max_entries is None:
            return text
        # 按 ## 切分
        parts = re.split(r"(?m)^## ", text)
        # parts[0] 是头部，之后每项是 ## 后的内容
        if len(parts) <= 1:
            return text
        recent = parts[-max_entries:]
        return "## " + "## ".join(recent)

    # ──────────── plan.md（覆盖）────────────

    def update_plan(self, content: str) -> None:
        self._write_text(FILE_PLAN, content)

    def get_plan(self) -> str:
        path = self.yunji_dir / FILE_PLAN
        return path.read_text(encoding="utf-8") if path.exists() else ""

    # ──────────── checklist.md（tick/reset）────────────

    def tick_checklist(self, item_id: str, done: bool = True) -> bool:
        """勾选/取消勾选一条 checklist 项

        Args:
            item_id: 任务标题（或其前缀）
            done: True 勾选，False 取消
        Returns:
            是否找到并更新
        """
        path = self.yunji_dir / FILE_CHECKLIST
        if not path.exists():
            return False
        lines = path.read_text(encoding="utf-8").splitlines(keepends=False)
        target_re = re.compile(re.escape(item_id), re.IGNORECASE)
        found = False
        for i, line in enumerate(lines):
            stripped = line.lstrip()
            if not (stripped.startswith("- [") and stripped[3] in (" ", "x", "X")):
                continue
            # 提取标题（去掉 `- [] ` 或 `- [x] ` 前缀）
            if _CHECKBOX_DONE.match(line):
                title = _CHECKBOX_DONE.sub("", line).strip()
            elif _CHECKBOX_TODO.match(line):
                title = _CHECKBOX_TODO.sub("", line).strip()
            else:
                continue
            if target_re.search(title):
                indent = line[: len(line) - len(stripped)]
                if done:
                    lines[i] = f"{indent}- [x] {title}"
                else:
                    lines[i] = f"{indent}- [ ] {title}"
                found = True
                break
        if found:
            self._write_text(FILE_CHECKLIST, "\n".join(lines) + "\n")
        return found

    def reset_checklist(self, items: List[str]) -> None:
        """完全重置 checklist（全部未完成）"""
        lines = ["# 项目检查清单\n"]
        for item in items:
            lines.append(f"- [ ] {item}")
        self._write_text(FILE_CHECKLIST, "\n".join(lines) + "\n")

    def get_checklist(self) -> List[CheckItem]:
        """读取 checklist，返回 CheckItem 列表"""
        path = self.yunji_dir / FILE_CHECKLIST
        result: List[CheckItem] = []
        if not path.exists():
            return result
        for i, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if _CHECKBOX_DONE.match(line):
                title = _CHECKBOX_DONE.sub("", line).strip()
                result.append(CheckItem(id=title, title=title, done=True, line_number=i))
            elif _CHECKBOX_TODO.match(line):
                title = _CHECKBOX_TODO.sub("", line).strip()
                result.append(CheckItem(id=title, title=title, done=False, line_number=i))
        return result

    # ──────────── knowledge（只读代理 + 委托）────────────

    def read_knowledge(self) -> Dict[str, Any]:
        """读取 knowledge/ 目录下 4 个 JSON 文件，合并返回

        返回格式:
            {"L1": [...], "L2": [...], "L3": [...], "L4": []}
            其中每项是 Knowledge dict
        """
        result: Dict[str, List[Dict[str, Any]]] = {"L1": [], "L2": [], "L3": [], "L4": []}
        kdir = self.yunji_dir / DIR_KNOWLEDGE
        if not kdir.exists():
            return result
        for layer in result.keys():
            f = kdir / f"{layer.lower()}.json"
            if not f.exists():
                continue
            try:
                data = json.loads(f.read_text(encoding="utf-8"))
                if isinstance(data, list):
                    result[layer] = [x for x in data if isinstance(x, dict)]
            except (json.JSONDecodeError, ValueError):
                continue
        return result

    # ──────────── skills/ ────────────

    def read_skills(self) -> List[SkillEntry]:
        """列出 skills/ 下所有 .md / .markdown 文件"""
        sdir = self.yunji_dir / DIR_SKILLS
        result: List[SkillEntry] = []
        if not sdir.exists():
            return result
        for f in sorted(sdir.rglob("*.md")):
            try:
                content = f.read_text(encoding="utf-8")
            except Exception:
                continue
            try:
                rel = str(f.relative_to(sdir))
            except ValueError:
                rel = f.name
            result.append(SkillEntry(name=rel, path=str(f), content=content, size=len(content)))
        for f in sorted(sdir.rglob("*.markdown")):
            try:
                content = f.read_text(encoding="utf-8")
            except Exception:
                continue
            try:
                rel = str(f.relative_to(sdir))
            except ValueError:
                rel = f.name
            result.append(SkillEntry(name=rel, path=str(f), content=content, size=len(content)))
        return result

    def write_skill(self, name: str, content: str) -> Path:
        """写入一个新 skill（覆盖）"""
        sdir = self.yunji_dir / DIR_SKILLS
        sdir.mkdir(parents=True, exist_ok=True)
        if not name.endswith(".md") and not name.endswith(".markdown"):
            name = name + ".md"
        path = sdir / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return path

    # ──────────── agents（read-only）────────────

    def read_agents(self) -> List[AgentEntry]:
        """读取 AGENTS.md（按优先级路径）"""
        result: List[AgentEntry] = []
        for rel in _AGENTS_SEARCH_PATHS:
            p = self.workspace_path / rel
            if p.exists():
                try:
                    content = p.read_text(encoding="utf-8")
                except Exception:
                    continue
                # name 用文件名（去扩展名）
                name = p.stem
                result.append(AgentEntry(name=name, path=str(p), content=content, size=len(content)))
        return result

    # ──────────── 状态自检 ────────────

    def status(self) -> StoreStatus:
        """自检所有工件状态"""
        files = {
            FILE_STATE: (self.yunji_dir / FILE_STATE).exists(),
            FILE_MEMORY: (self.yunji_dir / FILE_MEMORY).exists(),
            FILE_PLAN: (self.yunji_dir / FILE_PLAN).exists(),
            FILE_CHECKLIST: (self.yunji_dir / FILE_CHECKLIST).exists(),
            f"{DIR_KNOWLEDGE}/": (self.yunji_dir / DIR_KNOWLEDGE).exists(),
            f"{DIR_SKILLS}/": (self.yunji_dir / DIR_SKILLS).exists(),
        }
        sizes: Dict[str, int] = {}
        for fname in [FILE_STATE, FILE_MEMORY, FILE_PLAN, FILE_CHECKLIST]:
            p = self.yunji_dir / fname
            if p.exists():
                sizes[fname] = p.stat().st_size
        # state.updated_at
        state = self.get_state()
        return StoreStatus(
            workspace=str(self.workspace_path),
            yunji_dir_exists=self.yunji_dir.exists(),
            files=files,
            file_sizes=sizes,
            last_updated_at=state.get("updated_at"),
        )

    # ──────────── 内部 IO ────────────

    def _read_json(self, name: str, default: Any) -> Any:
        p = self.yunji_dir / name
        if not p.exists():
            return default
        try:
            return json.loads(p.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, ValueError, OSError):
            return default

    def _write_json(self, name: str, data: Any) -> None:
        p = self.yunji_dir / name
        p.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")

    def _write_text(self, name: str, text: str) -> None:
        p = self.yunji_dir / name
        p.write_text(text, encoding="utf-8")
