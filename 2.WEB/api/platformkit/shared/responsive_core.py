"""Platform Shared - 主动感知引擎核心 (Responsive Engine)

2026-06-09 TASK-3.4 引入：Phase 3 W10 主动感知引擎核心

四个 Watcher:
    1. FileChangeWatcher      — 文件变更（mtime polling，无 watchdog 依赖）
    2. CodeQualityPatrol      — 代码质量巡逻（简单规则扫描）
    3. DependencyRiskScanner  — 依赖风险扫描（解析 requirements.txt / package.json）
    4. ProgressTracker        — 上次会话进度提醒（读 .yunji/state.json）

设计原则（按 AGENTS.md）:
    - 不依赖 FastAPI（路由层做参数解析 + 调本模块）
    - 可被 FastAPI app 和远程 daemon 同时调用
    - 软依赖：watchdog 缺失时降级为 mtime polling
    - WebSocket 推送不在本模块（路由层做）
    - 通知 Notification dataclass 统一结构，路由层序列化

使用示例:
    engine = ResponsiveEngine(workspace_path="/path/to/proj")
    notifs = engine.trigger_scan("file_change")
    notifs = engine.trigger_scan("all")  # 全部 watcher

新代码导入:
    from platformkit.shared.responsive_core import ResponsiveEngine, Notification, NotificationType

Phase 4 可选升级:
    - 集成 watchdog 实现实时监听（替换 polling）
    - 集成 pylint/ruff 替代简单规则
    - 接入 NVD API 替代本地 CVE 静态表
"""

from __future__ import annotations

import json
import re
import time
import uuid
from dataclasses import asdict, dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Union


# ──────────── 数据结构 ────────────


class NotificationType(str, Enum):
    """四种感知类型（对应 4 个 Watcher）"""

    FILE_CHANGE = "file_change"
    CODE_QUALITY = "code_quality"
    SECURITY_RISK = "security_risk"
    PROGRESS = "progress"


class Severity(str, Enum):
    """通知严重度"""

    INFO = "info"
    WARNING = "warning"
    ERROR = "error"


@dataclass
class NotificationAction:
    """通知可执行动作"""

    label: str
    action: str  # 动作标识：open_file / run_command / navigate 等
    params: Dict[str, Any] = field(default_factory=dict)


@dataclass
class Notification:
    """感知通知"""

    id: str
    type: NotificationType
    severity: Severity
    title: str
    description: str
    file_path: Optional[str] = None
    actions: List[NotificationAction] = field(default_factory=list)
    created_at: float = field(default_factory=time.time)
    dismissed: bool = False
    source: str = ""  # 哪个 watcher 产出

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["type"] = self.type.value
        d["severity"] = self.severity.value
        d["actions"] = [asdict(a) for a in self.actions]
        d["created_at_iso"] = datetime.fromtimestamp(self.created_at).isoformat(timespec="seconds")
        return d


# ──────────── 基础 Watcher ────────────


class BaseWatcher:
    """所有 Watcher 的基类"""

    type: NotificationType = NotificationType.FILE_CHANGE  # 子类覆盖

    def __init__(self, workspace_path: Path):
        self.workspace_path = workspace_path
        self.last_scan_at: Optional[float] = None

    def scan(self) -> List[Notification]:
        """执行一次扫描，返回 0+ 通知"""
        raise NotImplementedError

    def _new_id(self) -> str:
        return str(uuid.uuid4())


# ──────────── 1. FileChangeWatcher ────────────


# 忽略的目录（避免 node_modules / .git 噪音）
_IGNORE_DIRS = {
    "node_modules", ".git", "__pycache__", ".venv", "venv",
    "dist", "build", ".idea", ".vscode", ".yunji", "target",
    ".next", ".nuxt", "out",
}

# 关注的文件后缀
_WATCH_EXTS = {
    ".py", ".ts", ".tsx", ".js", ".jsx", ".vue", ".go", ".rs",
    ".java", ".kt", ".swift", ".rb", ".php", ".cs", ".cpp", ".c",
    ".h", ".hpp", ".md", ".json", ".yaml", ".yml", ".toml", ".sql",
}


class FileChangeWatcher(BaseWatcher):
    """文件变更感知（mtime polling 实现）

    简化策略：每次 trigger_scan 时遍历工作区，对比文件 mtime 与上次基线。
    新增/修改的文件产出通知。
    Phase 4 可升级为 watchdog 实时监听。
    """

    type = NotificationType.FILE_CHANGE

    def __init__(self, workspace_path: Path):
        super().__init__(workspace_path)
        self._mtime_baseline: Dict[str, float] = {}  # path -> mtime
        self._initialized: bool = False

    def _walk_files(self) -> Dict[str, float]:
        """遍历工作区，{rel_path: mtime}"""
        result: Dict[str, float] = {}
        if not self.workspace_path.exists():
            return result
        try:
            for root, dirs, files in self.workspace_path.walk() if hasattr(self.workspace_path, "walk") else _py_walk(self.workspace_path):
                # 过滤忽略目录
                dirs[:] = [d for d in dirs if d not in _IGNORE_DIRS]
                for f in files:
                    p = Path(root) / f
                    try:
                        rel = str(p.relative_to(self.workspace_path))
                        ext = p.suffix.lower()
                        if ext not in _WATCH_EXTS:
                            continue
                        result[rel] = p.stat().st_mtime
                    except (OSError, ValueError):
                        continue
        except Exception:
            return result
        return result

    def scan(self) -> List[Notification]:
        current = self._walk_files()
        notifications: List[Notification] = []
        # 首次扫描仅建立基线，不产出通知
        if not self._initialized:
            self._mtime_baseline = current
            self._initialized = True
            self.last_scan_at = time.time()
            return notifications
        # 找出新增 / 修改
        new_or_modified = []
        for path, mtime in current.items():
            baseline_mtime = self._mtime_baseline.get(path)
            if baseline_mtime is None or mtime > baseline_mtime:
                new_or_modified.append(path)
        # 删除（不在 current 但在 baseline 中）
        removed = [p for p in self._mtime_baseline if p not in current]
        # 更新基线
        self._mtime_baseline = current
        self.last_scan_at = time.time()
        # 产出通知
        if new_or_modified:
            for p in new_or_modified[:10]:  # 限制单次最多 10 条
                notifications.append(
                    Notification(
                        id=self._new_id(),
                        type=NotificationType.FILE_CHANGE,
                        severity=Severity.INFO,
                        title=f"文件变更：{Path(p).name}",
                        description=f"{p}",
                        file_path=str(self.workspace_path / p),
                        source="FileChangeWatcher",
                        actions=[
                            NotificationAction(label="打开", action="open_file", params={"path": p}),
                        ],
                    )
                )
        if removed:
            for p in removed[:5]:
                notifications.append(
                    Notification(
                        id=self._new_id(),
                        type=NotificationType.FILE_CHANGE,
                        severity=Severity.INFO,
                        title=f"文件删除：{Path(p).name}",
                        description=f"{p}",
                        file_path=str(self.workspace_path / p),
                        source="FileChangeWatcher",
                    )
                )
        return notifications


def _py_walk(root: Path):
    """Python 3.10 兼容的 walk（Path.walk 是 3.12+）"""
    import os
    for r, dirs, files in os.walk(root):
        yield Path(r), dirs, files


# ──────────── 2. CodeQualityPatrol ────────────


# 简单代码异味规则（Phase 4 可换 ruff/pylint）
_CODE_SMELL_RULES = [
    {
        "id": "long-line",
        "pattern": re.compile(r"^.{200,}$"),
        "severity": Severity.WARNING,
        "title": "超长行",
        "description": "单行超过 200 字符，建议拆分",
    },
    {
        "id": "todo-unresolved",
        "pattern": re.compile(r"#\s*TODO|FIXME|XXX", re.IGNORECASE),
        "severity": Severity.INFO,
        "title": "未解决 TODO/FIXME",
        "description": "存在未解决的 TODO/FIXME 标记",
    },
    {
        "id": "print-debug",
        "pattern": re.compile(r"^\s*print\(", re.MULTILINE),
        "severity": Severity.INFO,
        "title": "print 调试语句",
        "description": "生产代码中应使用 logging 替代 print",
    },
    {
        "id": "broad-except",
        "pattern": re.compile(r"except\s*:"),
        "severity": Severity.WARNING,
        "title": "裸 except",
        "description": "except: 会捕获所有异常包括 SystemExit，建议指定异常类型",
    },
    {
        "id": "var-usage-py",
        "pattern": re.compile(r"^\s*var\s+", re.MULTILINE),
        "severity": Severity.WARNING,
        "title": "Python 不应使用 var",
        "description": "Python 没有 var 关键字，用普通赋值",
    },
    {
        "id": "console-log-js",
        "pattern": re.compile(r"^\s*console\.log\(", re.MULTILINE),
        "severity": Severity.INFO,
        "title": "console.log 残留",
        "description": "提交前应移除 console.log",
    },
]


class CodeQualityPatrol(BaseWatcher):
    """代码质量巡逻（简单规则）"""

    type = NotificationType.CODE_QUALITY

    def scan(self) -> List[Notification]:
        notifications: List[Notification] = []
        if not self.workspace_path.exists():
            return notifications
        self.last_scan_at = time.time()
        # 仅扫描源码文件
        try:
            for root, dirs, files in self.workspace_path.walk() if hasattr(self.workspace_path, "walk") else _py_walk(self.workspace_path):
                dirs[:] = [d for d in dirs if d not in _IGNORE_DIRS]
                for f in files:
                    p = Path(root) / f
                    ext = p.suffix.lower()
                    if ext not in {".py", ".ts", ".tsx", ".js", ".jsx", ".vue", ".go", ".rs", ".java", ".kt"}:
                        continue
                    if p.stat().st_size > 500_000:  # > 500KB 跳过
                        continue
                    try:
                        content = p.read_text(encoding="utf-8", errors="ignore")
                    except Exception:
                        continue
                    for rule in _CODE_SMELL_RULES:
                        if rule["pattern"].search(content):
                            rel = str(p.relative_to(self.workspace_path))
                            # 同一文件同一规则只通知一次
                            key = f"{rel}:{rule['id']}"
                            if key in self._seen:
                                continue
                            self._seen.add(key)
                            notifications.append(
                                Notification(
                                    id=self._new_id(),
                                    type=NotificationType.CODE_QUALITY,
                                    severity=rule["severity"],
                                    title=f"[{rule['id']}] {rule['title']}",
                                    description=f"{rel}\n{rule['description']}",
                                    file_path=str(p),
                                    source="CodeQualityPatrol",
                                    actions=[
                                        NotificationAction(label="查看", action="open_file", params={"path": rel}),
                                    ],
                                )
                            )
                            if len(notifications) >= 20:  # 单次扫描最多 20 条
                                self.last_scan_at = time.time()
                                return notifications
        except Exception:
            pass
        return notifications

    def __init__(self, workspace_path: Path):
        super().__init__(workspace_path)
        self._seen: set = set()  # 去重


# ──────────── 3. DependencyRiskScanner ────────────


# 已知问题包（占位。Phase 4 接 NVD/CVE 实时查询）
_KNOWN_RISK_PACKAGES = {
    # 包名（小写） -> 提示
    "requests": "建议固定版本，2.20.0 之前有 CVE-2018-18074",
    "django": "3.2.0 之前有 XSS 漏洞，建议升级",
    "flask": "0.12.0 之前有 cookie 问题",
    "pillow": "8.3.0 之前存在任意代码执行漏洞",
    "pyyaml": "5.1 之前存在反序列化漏洞",
    "log4j": "2.14.1 之前存在严重 RCE（CVE-2021-44228）",
    "axios": "0.21.0 之前存在 SSRF 漏洞",
    "lodash": "4.17.20 之前存在原型链污染",
    "minimist": "1.2.3 之前存在原型链污染",
    "node-fetch": "2.6.6 之前存在信息泄露",
}


class DependencyRiskScanner(BaseWatcher):
    """依赖风险扫描（requirements.txt / package.json / Cargo.toml）"""

    type = NotificationType.SECURITY_RISK

    def scan(self) -> List[Notification]:
        notifications: List[Notification] = []
        self.last_scan_at = time.time()
        if not self.workspace_path.exists():
            return notifications
        # 解析 requirements.txt
        req_file = self.workspace_path / "requirements.txt"
        if req_file.exists():
            notifications.extend(self._scan_requirements(req_file))
        # 解析 package.json
        pkg_file = self.workspace_path / "package.json"
        if pkg_file.exists():
            notifications.extend(self._scan_package_json(pkg_file))
        # 解析 Cargo.toml
        cargo_file = self.workspace_path / "Cargo.toml"
        if cargo_file.exists():
            notifications.extend(self._scan_cargo(cargo_file))
        return notifications

    def _scan_requirements(self, f: Path) -> List[Notification]:
        result: List[Notification] = []
        try:
            content = f.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            return result
        for line in content.splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            # 解析包名（去版本约束）
            m = re.match(r"^([A-Za-z0-9_.\-]+)", line)
            if not m:
                continue
            pkg = m.group(1).lower()
            if pkg in _KNOWN_RISK_PACKAGES:
                result.append(
                    Notification(
                        id=self._new_id(),
                        type=NotificationType.SECURITY_RISK,
                        severity=Severity.WARNING,
                        title=f"依赖风险：{pkg}",
                        description=_KNOWN_RISK_PACKAGES[pkg],
                        file_path=str(f),
                        source="DependencyRiskScanner",
                        actions=[
                            NotificationAction(label="升级", action="run_command", params={"cmd": f"pip install -U {pkg}"}),
                        ],
                    )
                )
        return result

    def _scan_package_json(self, f: Path) -> List[Notification]:
        result: List[Notification] = []
        try:
            data = json.loads(f.read_text(encoding="utf-8"))
        except Exception:
            return result
        deps: Dict[str, str] = {}
        for key in ("dependencies", "devDependencies", "peerDependencies"):
            section = data.get(key) or {}
            if isinstance(section, dict):
                deps.update({k: v for k, v in section.items() if isinstance(k, str)})
        for pkg in deps:
            if pkg.lower() in _KNOWN_RISK_PACKAGES:
                result.append(
                    Notification(
                        id=self._new_id(),
                        type=NotificationType.SECURITY_RISK,
                        severity=Severity.WARNING,
                        title=f"依赖风险：{pkg}",
                        description=_KNOWN_RISK_PACKAGES[pkg],
                        file_path=str(f),
                        source="DependencyRiskScanner",
                        actions=[
                            NotificationAction(label="升级", action="run_command", params={"cmd": f"npm update {pkg}"}),
                        ],
                    )
                )
        return result

    def _scan_cargo(self, f: Path) -> List[Notification]:
        # Cargo.toml 简单正则（不引入 toml 解析器）
        result: List[Notification] = []
        try:
            content = f.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            return result
        for m in re.finditer(r'^\s*([A-Za-z0-9_\-]+)\s*=\s*["{]', content, re.MULTILINE):
            pkg = m.group(1).lower()
            if pkg in _KNOWN_RISK_PACKAGES:
                result.append(
                    Notification(
                        id=self._new_id(),
                        type=NotificationType.SECURITY_RISK,
                        severity=Severity.WARNING,
                        title=f"依赖风险：{pkg}",
                        description=_KNOWN_RISK_PACKAGES[pkg],
                        file_path=str(f),
                        source="DependencyRiskScanner",
                    )
                )
        return result


# ──────────── 4. ProgressTracker ────────────


class ProgressTracker(BaseWatcher):
    """上次会话进度追踪

    读 .yunji/state.json 中的 last_session 字段，若有未完成任务则发提醒。
    """

    type = NotificationType.PROGRESS

    def scan(self) -> List[Notification]:
        notifications: List[Notification] = []
        self.last_scan_at = time.time()
        state_file = self.workspace_path / ".yunji" / "state.json"
        if not state_file.exists():
            return notifications
        try:
            data = json.loads(state_file.read_text(encoding="utf-8"))
        except Exception:
            return notifications
        last = data.get("last_session")
        if not last or not isinstance(last, dict):
            return notifications
        unfinished = last.get("unfinished_tasks") or []
        if not isinstance(unfinished, list) or len(unfinished) == 0:
            return notifications
        # 取前 5 条未完成任务
        for task in unfinished[:5]:
            if not isinstance(task, dict):
                continue
            title = task.get("title") or task.get("name") or "未完成任务"
            desc = task.get("description") or ""
            notifications.append(
                Notification(
                    id=self._new_id(),
                    type=NotificationType.PROGRESS,
                    severity=Severity.INFO,
                    title=f"继续：{title}",
                    description=desc,
                    source="ProgressTracker",
                    actions=[
                        NotificationAction(label="继续", action="navigate", params={"target": "/chat?resume=1"}),
                    ],
                )
            )
        return notifications


# ──────────── 引擎 ────────────


class ResponsiveEngine:
    """主动感知引擎主控

    持有 4 个 watcher 实例，提供 start/stop/trigger_scan 接口。
    不启动后台线程（按需扫描），启动/停止仅做初始化/清理。
    """

    WATCHER_CLASSES: Dict[NotificationType, type] = {
        NotificationType.FILE_CHANGE: FileChangeWatcher,
        NotificationType.CODE_QUALITY: CodeQualityPatrol,
        NotificationType.SECURITY_RISK: DependencyRiskScanner,
        NotificationType.PROGRESS: ProgressTracker,
    }

    def __init__(self, workspace_path: Union[Path, str]):
        self.workspace_path: Path = Path(workspace_path).resolve()
        self._watchers: Dict[NotificationType, BaseWatcher] = {}
        self._running = False
        self._init_watchers()

    def _init_watchers(self) -> None:
        for ntype, cls in self.WATCHER_CLASSES.items():
            try:
                self._watchers[ntype] = cls(self.workspace_path)
            except Exception as e:
                # 单个 watcher 失败不影响其他
                print(f"[ResponsiveEngine] 初始化 {ntype.value} 失败: {e}")

    def start(self) -> Dict[str, Any]:
        """启动引擎（仅做标记；watcher 启动基线/资源）"""
        self._running = True
        # 先预热 file_change watcher 的基线（首次扫描不产通知）
        fc = self._watchers.get(NotificationType.FILE_CHANGE)
        if fc:
            try:
                fc.scan()
            except Exception:
                pass
        return {"ok": True, "running": True, "watchers": [t.value for t in self._watchers.keys()]}

    def stop(self) -> Dict[str, Any]:
        """停止引擎"""
        self._running = False
        return {"ok": True, "running": False}

    def is_running(self) -> bool:
        return self._running

    def trigger_scan(self, scan_type: str = "all") -> List[Notification]:
        """触发一次扫描

        Args:
            scan_type: 'all' / 'file_change' / 'code_quality' / 'security_risk' / 'progress'
        """
        if scan_type == "all":
            target_types = list(self._watchers.keys())
        else:
            try:
                target_types = [NotificationType(scan_type)]
            except ValueError:
                return []
        all_notifs: List[Notification] = []
        for ntype in target_types:
            watcher = self._watchers.get(ntype)
            if not watcher:
                continue
            try:
                notifs = watcher.scan()
                all_notifs.extend(notifs)
            except Exception as e:
                # 单个 watcher 异常不影响其他
                print(f"[ResponsiveEngine] {ntype.value} scan 失败: {e}")
        return all_notifs

    def list_watcher_status(self) -> Dict[str, Any]:
        """查看各 watcher 状态"""
        result: Dict[str, Any] = {}
        for ntype, w in self._watchers.items():
            result[ntype.value] = {
                "last_scan_at": w.last_scan_at,
            }
        return result
