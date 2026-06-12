"""Platform Shared - 技能市场核心 (TASK-4.5)

技能（Skill）是可复用的提示词 + 模板 + 可选脚本包，对标 CodexMonitor 的
"Skills" 概念 + 一个轻量市场（v1 用内置 catalog + 用户本地安装）。

核心能力:
    1. 技能元数据 schema (skill.yaml 风格 dict)
    2. 官方目录（v1 用内置 SEED_CATALOG，未来可换 GitHub repo / 自建后端）
    3. 搜索 / 分类 / 标签 / 评分
    4. 安装到全局 (~/.yunji/skills/{id}/) 或项目级 (.yunji/skills/{id}/)
    5. 卸载 / 重新安装
    6. 上传自定义技能（写入 ~/.yunji/skills/custom/{id}/）
    7. 给技能打分（写 ratings.json）
    8. 一键应用：把技能的 prompt 模板注入当前 session

**设计原则**（按 AGENTS.md 路由层规则）:
    - 不依赖 FastAPI（路由层做参数解析 + 调本模块）
    - 不依赖第三方库（仅 stdlib）
    - 所有 IO 失败用统一返回结构 `{"ok": bool, "data": ..., "error": str|None}`
    - 不阻塞主线程（文件 IO < 1MB 同步处理，超大技能包走 zip 流式）

**Skill 元数据规范（skill.yaml 兼容）**:
    {
        "id": "react-component-scaffold",        # 唯一标识（小写 + 中划线）
        "name": "React 组件脚手架",                 # 显示名
        "version": "1.2.0",                        # semver
        "author": "YunJi Official",                # 作者
        "category": "scaffold",                    # 分类
        "tags": ["react", "frontend", "tdd"],
        "description": "一键生成带测试的 React 函数组件",
        "long_description": "## 特性\\n- ...",
        "icon": "https://...",                     # 可选
        "repository": "https://github.com/...",    # 可选
        "license": "MIT",
        "min_yunji_version": "2.0.0",
        "rating": 4.7,                             # 0-5
        "rating_count": 128,
        "downloads": 5421,
        "prompt_template": "...",                  # 必填
        "files": {                                 # 技能包附带文件
            "templates/component.tsx": "...",
            "tests/component.test.tsx": "...",
        },
        "dependencies": ["node >= 18"],            # 可选环境依赖
    }

**使用规则**:
    - ✅ 路由层: `from platformkit.shared import skill_market`
    - ❌ 禁止: 在 routes/*.py 中直接读写 skills 目录
"""
from __future__ import annotations

import json
import os
import re
import shutil
import tempfile
import time
import zipfile
import io
from pathlib import Path
from typing import Any, Optional, Iterable


# ── 常量 ────────────────────────────────────────────────────────
SKILL_ID_PATTERN = re.compile(r"^[a-z0-9][a-z0-9\-_]{0,63}$")
VALID_CATEGORIES = {
    "scaffold",       # 脚手架/模板
    "refactor",       # 重构
    "test",           # 测试
    "doc",            # 文档
    "review",         # 代码审查
    "debug",          # 调试
    "deploy",         # 部署
    "data",           # 数据处理
    "ui",             # UI/UX
    "workflow",       # 工作流自动化
    "other",          # 其他
}


def _make_result(ok: bool, data: Any = None, error: Optional[str] = None) -> dict:
    return {"ok": ok, "data": data, "error": error}


# ── 内置官方目录（v1） ─────────────────────────────────────────────
# 未来从 GitHub repo 拉取或自建后端；先用内置 12 个种子技能保证开箱即用。
SEED_CATALOG: list[dict] = [
    {
        "id": "react-component-scaffold",
        "name": "React 函数组件脚手架",
        "version": "1.2.0",
        "author": "YunJi Official",
        "category": "scaffold",
        "tags": ["react", "frontend", "typescript", "tdd"],
        "description": "一键生成带测试 + Storybook 的 React 函数组件",
        "long_description": (
            "## 特性\n"
            "- 完整 TypeScript 类型定义\n"
            "- Vitest + React Testing Library 单测\n"
            "- Storybook 8 故事\n"
            "- a11y 基础检查（axe-core）\n\n"
            "## 使用方式\n"
            "输入组件名 → 自动生成 src/components/{Name}/ 目录。"
        ),
        "icon": "https://api.iconify.design/logos/react.svg",
        "repository": "https://github.com/yunji-official/skills",
        "license": "MIT",
        "min_yunji_version": "2.0.0",
        "rating": 4.8,
        "rating_count": 256,
        "downloads": 12453,
        "prompt_template": (
            "请基于以下需求生成一个 React 函数组件：\n\n"
            "{{requirement}}\n\n"
            "要求：\n"
            "1. TypeScript 严格模式\n"
            "2. 函数组件 + hooks\n"
            "3. 自定义 props 接口\n"
            "4. 至少 2 个 Vitest 单元测试\n"
            "5. Storybook 故事 1 个\n"
        ),
        "files": {
            "templates/Component.tsx": (
                "import { FC } from 'react'\n\n"
                "export interface ComponentProps {\n"
                "  // TODO: define props\n"
                "}\n\n"
                "export const Component: FC<ComponentProps> = (props) => {\n"
                "  return <div data-testid=\"component\">{/* TODO */}</div>\n"
                "}\n"
            ),
            "templates/Component.test.tsx": (
                "import { render } from '@testing-library/react'\n"
                "import { describe, it, expect } from 'vitest'\n"
                "import { Component } from './Component'\n\n"
                "describe('Component', () => {\n"
                "  it('renders without crash', () => {\n"
                "    const { getByTestId } = render(<Component />)\n"
                "    expect(getByTestId('component')).toBeTruthy()\n"
                "  })\n"
                "})\n"
            ),
            "templates/Component.stories.tsx": (
                "import type { Meta, StoryObj } from '@storybook/react'\n"
                "import { Component } from './Component'\n\n"
                "const meta: Meta<typeof Component> = {\n"
                "  title: 'Components/Component',\n"
                "  component: Component,\n"
                "}\n"
                "export default meta\n\n"
                "type Story = StoryObj<typeof Component>\n"
                "export const Default: Story = {}\n"
            ),
        },
        "dependencies": ["node >= 18"],
    },
    {
        "id": "vue3-component-scaffold",
        "name": "Vue 3 组件脚手架",
        "version": "1.1.0",
        "author": "YunJi Official",
        "category": "scaffold",
        "tags": ["vue", "frontend", "typescript", "composition-api"],
        "description": "Vue 3 SFC + Pinia store + Vitest 完整模板",
        "long_description": "## 特性\n- `<script setup lang=\"ts\">`\n- 组合式 API\n- Pinia 状态管理片段\n- Vitest + @vue/test-utils",
        "icon": "https://api.iconify.design/logos/vue.svg",
        "repository": "https://github.com/yunji-official/skills",
        "license": "MIT",
        "min_yunji_version": "2.0.0",
        "rating": 4.7,
        "rating_count": 189,
        "downloads": 8932,
        "prompt_template": (
            "请基于以下需求生成一个 Vue 3 SFC：\n\n"
            "{{requirement}}\n\n"
            "要求：\n"
            "1. `<script setup lang=\"ts\">`\n"
            "2. 组合式 API + ref/reactive\n"
            "3. defineProps + withDefaults\n"
            "4. 至少 1 个 Vitest 组件测试\n"
        ),
        "files": {
            "templates/Component.vue": (
                "<script setup lang=\"ts\">\n"
                "interface Props { msg?: string }\n"
                "const props = withDefaults(defineProps<Props>(), { msg: 'hello' })\n"
                "</script>\n\n"
                "<template>\n"
                "  <div class=\"component\">{{ props.msg }}</div>\n"
                "</template>\n"
            ),
        },
        "dependencies": ["node >= 18"],
    },
    {
        "id": "python-pytest-scaffold",
        "name": "Python pytest 测试脚手架",
        "version": "1.0.0",
        "author": "YunJi Official",
        "category": "test",
        "tags": ["python", "pytest", "tdd"],
        "description": "为指定模块生成 pytest 测试用例",
        "long_description": "## 特性\n- 自动识别函数签名\n- 正常 + 边界 + 异常 三类用例\n- 覆盖率统计",
        "rating": 4.6,
        "rating_count": 145,
        "downloads": 6210,
        "prompt_template": (
            "为以下 Python 模块编写 pytest 测试：\n\n"
            "{{module_path}}\n\n"
            "要求覆盖：\n"
            "1. 正常路径\n"
            "2. 边界条件\n"
            "3. 异常路径\n"
            "4. 使用 parametrize 而非循环\n"
        ),
        "files": {},
        "license": "MIT",
    },
    {
        "id": "api-doc-generator",
        "name": "API 文档自动生成",
        "version": "1.0.0",
        "author": "YunJi Official",
        "category": "doc",
        "tags": ["openapi", "swagger", "api"],
        "description": "从 FastAPI/Express 代码自动生成 OpenAPI 友好文档",
        "rating": 4.5,
        "rating_count": 87,
        "downloads": 3201,
        "prompt_template": (
            "扫描 {{project_path}} 中的 API 路由，生成 OpenAPI 3.0 文档：\n"
            "- 端点列表\n- 请求/响应 schema\n- 鉴权方式"
        ),
        "files": {},
        "license": "MIT",
    },
    {
        "id": "code-review-checklist",
        "name": "代码审查清单",
        "version": "2.0.0",
        "author": "YunJi Official",
        "category": "review",
        "tags": ["review", "quality", "best-practice"],
        "description": "按 12 维度对 PR 进行结构化审查",
        "long_description": "## 12 维度\n1. 命名\n2. 复杂度\n3. 错误处理\n4. 性能\n5. 安全\n6. 测试\n7. 文档\n8. 类型\n9. 并发\n10. 可观测性\n11. 可维护性\n12. 兼容性",
        "rating": 4.9,
        "rating_count": 312,
        "downloads": 15802,
        "prompt_template": (
            "请按以下 12 维度审查以下 diff：\n\n{{diff}}\n\n"
            "输出格式：\n- ✅ 优点\n- ⚠️ 建议\n- ❌ 必须修复"
        ),
        "files": {},
        "license": "MIT",
    },
    {
        "id": "docker-compose-generator",
        "name": "Docker Compose 生成器",
        "version": "1.0.0",
        "author": "YunJi Official",
        "category": "deploy",
        "tags": ["docker", "devops", "compose"],
        "description": "根据服务清单生成 docker-compose.yml",
        "rating": 4.4,
        "rating_count": 76,
        "downloads": 2890,
        "prompt_template": "为 {{services}} 生成 docker-compose.yml：包含网络/卷/健康检查",
        "files": {},
        "license": "MIT",
    },
    {
        "id": "git-commit-message",
        "name": "Git 提交信息生成",
        "version": "1.1.0",
        "author": "YunJi Official",
        "category": "workflow",
        "tags": ["git", "conventional-commits"],
        "description": "Conventional Commits 风格的提交信息生成",
        "rating": 4.7,
        "rating_count": 198,
        "downloads": 9120,
        "prompt_template": (
            "根据以下 diff 生成 Conventional Commits 提交信息：\n\n{{diff}}\n\n"
            "格式：<type>(<scope>): <subject>\n\n<body>\n\n<footer>"
        ),
        "files": {},
        "license": "MIT",
    },
    {
        "id": "bug-root-cause",
        "name": "Bug 根因分析",
        "version": "1.0.0",
        "author": "YunJi Official",
        "category": "debug",
        "tags": ["debug", "analysis"],
        "description": "5 Whys + 鱼骨图根因分析",
        "rating": 4.5,
        "rating_count": 102,
        "downloads": 4180,
        "prompt_template": (
            "对以下 bug 报告进行根因分析：\n\n{{bug_report}}\n\n"
            "输出：\n1. 现象\n2. 5 Whys\n3. 鱼骨图（人/机/料/法/环/测）\n4. 建议修复"
        ),
        "files": {},
        "license": "MIT",
    },
    {
        "id": "sql-query-optimizer",
        "name": "SQL 查询优化",
        "version": "1.0.0",
        "author": "YunJi Official",
        "category": "data",
        "tags": ["sql", "performance"],
        "description": "EXPLAIN + 索引建议",
        "rating": 4.6,
        "rating_count": 134,
        "downloads": 5430,
        "prompt_template": (
            "分析以下 SQL 并给出优化建议：\n\n{{sql}}\n\n"
            "输出：\n1. EXPLAIN 解读\n2. 索引建议\n3. 重写建议"
        ),
        "files": {},
        "license": "MIT",
    },
    {
        "id": "ui-design-review",
        "name": "UI 设计审查",
        "version": "1.0.0",
        "author": "YunJi Official",
        "category": "ui",
        "tags": ["ui", "design", "a11y"],
        "description": "WCAG 2.1 + 视觉一致性审查",
        "rating": 4.3,
        "rating_count": 67,
        "downloads": 1980,
        "prompt_template": (
            "审查以下 UI 代码：\n\n{{code}}\n\n"
            "维度：\n1. WCAG 2.1 AA\n2. 设计令牌一致性\n3. 响应式断点\n4. 加载/错误/空状态"
        ),
        "files": {},
        "license": "MIT",
    },
    {
        "id": "refactor-extract-method",
        "name": "重构：抽取方法",
        "version": "1.0.0",
        "author": "YunJi Official",
        "category": "refactor",
        "tags": ["refactor", "clean-code"],
        "description": "Martin Fowler 经典重构手法",
        "rating": 4.4,
        "rating_count": 89,
        "downloads": 3140,
        "prompt_template": (
            "对以下函数进行 Extract Method 重构：\n\n{{code}}\n\n"
            "要求：\n1. 保持行为不变\n2. 每个新方法 < 20 行\n3. 命名表意\n4. 保留测试可运行"
        ),
        "files": {},
        "license": "MIT",
    },
    {
        "id": "github-issue-triage",
        "name": "GitHub Issue 分流",
        "version": "1.0.0",
        "author": "YunJi Official",
        "category": "workflow",
        "tags": ["github", "triage", "issue"],
        "description": "自动给 Issue 打 label + 指派 + 优先级",
        "rating": 4.5,
        "rating_count": 112,
        "downloads": 4220,
        "prompt_template": (
            "对以下 GitHub Issue 进行分流：\n\n{{issue}}\n\n"
            "输出：\n- type: bug/feature/question/docs\n- priority: P0/P1/P2/P3\n- labels: [...]\n- suggested assignee: ..."
        ),
        "files": {},
        "license": "MIT",
    },
]


# ── 路径解析 ──────────────────────────────────────────────────────
def _global_skills_dir() -> Path:
    """全局技能目录 ~/.yunji/skills/"""
    home = Path.home()
    p = home / ".yunji" / "skills"
    p.mkdir(parents=True, exist_ok=True)
    return p


def _project_skills_dir(workspace_path: str) -> Path:
    """项目级技能目录 .yunji/skills/"""
    p = Path(workspace_path) / ".yunji" / "skills"
    p.mkdir(parents=True, exist_ok=True)
    return p


def _ratings_file() -> Path:
    """全局评分数据 ~/.yunji/skills/.ratings.json"""
    p = _global_skills_dir() / ".ratings.json"
    if not p.exists():
        p.write_text("{}", encoding="utf-8")
    return p


def _resolve_skills_dir(scope: str, workspace_path: str = "") -> Optional[Path]:
    if scope == "global":
        return _global_skills_dir()
    if scope == "project":
        if not workspace_path:
            return None
        return _project_skills_dir(workspace_path)
    return None


# ── 元数据校验 ────────────────────────────────────────────────────
def _validate_skill(meta: dict) -> tuple[bool, str]:
    """校验技能元数据是否符合 schema。返回 (ok, error_message)。"""
    if not isinstance(meta, dict):
        return False, "元数据必须是 dict"
    sid = meta.get("id", "")
    if not isinstance(sid, str) or not SKILL_ID_PATTERN.match(sid):
        return False, f"id 必须匹配 {SKILL_ID_PATTERN.pattern}"
    if not meta.get("name") or not isinstance(meta["name"], str):
        return False, "缺少 name"
    if not meta.get("version") or not isinstance(meta["version"], str):
        return False, "缺少 version"
    category = meta.get("category", "other")
    if category not in VALID_CATEGORIES:
        return False, f"category 必须是 {sorted(VALID_CATEGORIES)} 之一"
    if not meta.get("prompt_template") or not isinstance(meta["prompt_template"], str):
        return False, "缺少 prompt_template"
    # rating / rating_count 允许缺省（自动 0/0）
    if "rating" in meta and not isinstance(meta["rating"], (int, float)):
        return False, "rating 必须是数字"
    if "files" in meta and not isinstance(meta["files"], dict):
        return False, "files 必须是 dict"
    return True, ""


# ── 目录（catalog）操作 ────────────────────────────────────────────
def list_catalog(category: str = "", search: str = "", tag: str = "") -> dict:
    """列出官方目录中的技能，可按 category / search / tag 过滤。"""
    try:
        results: list[dict] = []
        q = search.lower().strip()
        t = tag.lower().strip()
        for skill in SEED_CATALOG:
            if category and skill.get("category") != category:
                continue
            if t and t not in [x.lower() for x in skill.get("tags", [])]:
                continue
            if q:
                haystack = " ".join([
                    skill.get("id", ""),
                    skill.get("name", ""),
                    skill.get("description", ""),
                    " ".join(skill.get("tags", [])),
                ]).lower()
                if q not in haystack:
                    continue
            results.append(skill)
        results.sort(key=lambda s: (-s.get("rating", 0), -s.get("downloads", 0)))
        return _make_result(True, results)
    except Exception as e:
        return _make_result(False, error=f"列出目录失败: {e}")


def get_skill(skill_id: str) -> dict:
    """获取官方目录中某个技能的详情。"""
    if not SKILL_ID_PATTERN.match(skill_id):
        return _make_result(False, error="无效的技能 ID")
    for skill in SEED_CATALOG:
        if skill["id"] == skill_id:
            return _make_result(True, skill)
    return _make_result(False, error=f"官方目录中没有 {skill_id}")


def list_categories() -> dict:
    """列出所有有效分类 + 每个分类的技能数（仅官方目录）。"""
    try:
        counts: dict[str, int] = {c: 0 for c in VALID_CATEGORIES}
        for skill in SEED_CATALOG:
            cat = skill.get("category", "other")
            if cat in counts:
                counts[cat] += 1
            else:
                counts[cat] = 1
        return _make_result(True, {
            "categories": [
                {"id": "all", "label": "全部", "count": len(SEED_CATALOG)},
                *[{"id": c, "label": c, "count": counts.get(c, 0)} for c in sorted(counts.keys())],
            ],
            "total": len(SEED_CATALOG),
        })
    except Exception as e:
        return _make_result(False, error=f"列出分类失败: {e}")


# ── 安装 / 卸载 ───────────────────────────────────────────────────
def install_skill(
    skill_id: str,
    scope: str = "global",
    workspace_path: str = "",
) -> dict:
    """从官方目录安装技能到全局或项目目录。

    Returns:
        {"ok": bool, "data": {"path": str, "files_written": int, "scope": str}, "error": str|None}
    """
    if not SKILL_ID_PATTERN.match(skill_id):
        return _make_result(False, error="无效的技能 ID")
    if scope not in ("global", "project"):
        return _make_result(False, error=f"scope 必须是 'global' 或 'project'，收到 {scope!r}")
    target_dir = _resolve_skills_dir(scope, workspace_path)
    if target_dir is None:
        return _make_result(False, error="项目级安装需要 workspace_path")

    meta_res = get_skill(skill_id)
    if not meta_res["ok"]:
        return meta_res
    meta = meta_res["data"]

    try:
        skill_dir = target_dir / skill_id
        skill_dir.mkdir(parents=True, exist_ok=True)
        # 写 meta.yaml
        meta_path = skill_dir / "skill.yaml"
        meta_path.write_text(
            _dump_yaml(meta),
            encoding="utf-8",
        )
        # 写附带文件
        files_written = 0
        for rel_path, content in (meta.get("files") or {}).items():
            file_path = skill_dir / rel_path
            file_path.parent.mkdir(parents=True, exist_ok=True)
            file_path.write_text(content, encoding="utf-8")
            files_written += 1
        # 写 install.json（追踪安装时间/来源）
        (skill_dir / "install.json").write_text(
            json.dumps({
                "skill_id": skill_id,
                "version": meta.get("version", ""),
                "scope": scope,
                "installed_at": int(time.time()),
                "source": "official_catalog",
            }, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        return _make_result(True, {
            "path": str(skill_dir),
            "files_written": files_written,
            "scope": scope,
        })
    except Exception as e:
        return _make_result(False, error=f"安装失败: {e}")


def uninstall_skill(
    skill_id: str,
    scope: str = "global",
    workspace_path: str = "",
) -> dict:
    """从指定目录卸载技能。"""
    if not SKILL_ID_PATTERN.match(skill_id):
        return _make_result(False, error="无效的技能 ID")
    target_dir = _resolve_skills_dir(scope, workspace_path)
    if target_dir is None:
        return _make_result(False, error="项目级卸载需要 workspace_path")
    skill_dir = target_dir / skill_id
    if not skill_dir.exists():
        return _make_result(False, error=f"技能未安装: {skill_id}")
    try:
        shutil.rmtree(skill_dir)
        return _make_result(True, {"path": str(skill_dir), "scope": scope})
    except Exception as e:
        return _make_result(False, error=f"卸载失败: {e}")


def list_installed(
    scope: str = "global",
    workspace_path: str = "",
) -> dict:
    """列出已安装的技能。"""
    target_dir = _resolve_skills_dir(scope, workspace_path)
    if target_dir is None:
        return _make_result(False, error="项目级查询需要 workspace_path")
    if not target_dir.exists():
        return _make_result(True, [])
    results: list[dict] = []
    try:
        for entry in sorted(target_dir.iterdir()):
            if not entry.is_dir() or entry.name.startswith("."):
                continue
            install_json = entry / "install.json"
            meta_yaml = entry / "skill.yaml"
            info: dict = {"id": entry.name, "path": str(entry)}
            if install_json.exists():
                try:
                    info.update(json.loads(install_json.read_text(encoding="utf-8")))
                except Exception:
                    pass
            if meta_yaml.exists():
                # 从 yaml 中取 name/category 等（轻量解析：只取 name/desc 字段）
                try:
                    text = meta_yaml.read_text(encoding="utf-8")
                    name_m = re.search(r"^name:\s*(.+)$", text, re.MULTILINE)
                    desc_m = re.search(r"^description:\s*(.+)$", text, re.MULTILINE)
                    if name_m:
                        info["name"] = name_m.group(1).strip().strip('"')
                    if desc_m:
                        info["description"] = desc_m.group(1).strip().strip('"')
                except Exception:
                    pass
            info["scope"] = scope
            results.append(info)
    except Exception as e:
        return _make_result(False, error=f"列出已安装失败: {e}")
    return _make_result(True, results)


# ── 上传（自定义技能） ─────────────────────────────────────────────
def upload_skill(
    meta: dict,
    scope: str = "global",
    workspace_path: str = "",
) -> dict:
    """上传（创建）一个自定义技能。写入 ~/.yunji/skills/custom/{id}/。

    Args:
        meta: 技能元数据（必须含 id/name/version/category/prompt_template）
    """
    ok, err = _validate_skill(meta)
    if not ok:
        return _make_result(False, error=f"元数据校验失败: {err}")
    target_dir = _resolve_skills_dir(scope, workspace_path)
    if target_dir is None:
        return _make_result(False, error="项目级上传需要 workspace_path")
    try:
        skill_dir = target_dir / "custom" / meta["id"]
        skill_dir.mkdir(parents=True, exist_ok=True)
        (skill_dir / "skill.yaml").write_text(
            _dump_yaml({**meta, "author": meta.get("author", "User") or "User"}),
            encoding="utf-8",
        )
        files_written = 0
        for rel_path, content in (meta.get("files") or {}).items():
            file_path = skill_dir / rel_path
            file_path.parent.mkdir(parents=True, exist_ok=True)
            file_path.write_text(content, encoding="utf-8")
            files_written += 1
        (skill_dir / "install.json").write_text(
            json.dumps({
                "skill_id": meta["id"],
                "version": meta.get("version", "0.1.0"),
                "scope": scope,
                "installed_at": int(time.time()),
                "source": "user_upload",
            }, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        return _make_result(True, {
            "path": str(skill_dir),
            "files_written": files_written,
            "scope": scope,
        })
    except Exception as e:
        return _make_result(False, error=f"上传失败: {e}")


# ── 评分 ──────────────────────────────────────────────────────────
def rate_skill(skill_id: str, score: float) -> dict:
    """给技能打分（1-5）。累加到全局 ratings.json。"""
    if not SKILL_ID_PATTERN.match(skill_id):
        return _make_result(False, error="无效的技能 ID")
    if not isinstance(score, (int, float)) or not (1 <= score <= 5):
        return _make_result(False, error="score 必须是 1-5 之间的数字")
    try:
        rf = _ratings_file()
        data: dict = {}
        if rf.exists():
            data = json.loads(rf.read_text(encoding="utf-8"))
        entry = data.get(skill_id, {"sum": 0.0, "count": 0})
        entry["sum"] = float(entry.get("sum", 0.0)) + float(score)
        entry["count"] = int(entry.get("count", 0)) + 1
        data[skill_id] = entry
        rf.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
        avg = entry["sum"] / entry["count"] if entry["count"] else 0
        return _make_result(True, {
            "skill_id": skill_id,
            "average": round(avg, 2),
            "count": entry["count"],
        })
    except Exception as e:
        return _make_result(False, error=f"评分失败: {e}")


def get_user_rating(skill_id: str) -> dict:
    """获取某技能的用户评分聚合信息。"""
    if not SKILL_ID_PATTERN.match(skill_id):
        return _make_result(False, error="无效的技能 ID")
    try:
        rf = _ratings_file()
        data: dict = {}
        if rf.exists():
            data = json.loads(rf.read_text(encoding="utf-8"))
        entry = data.get(skill_id, {"sum": 0.0, "count": 0})
        avg = entry["sum"] / entry["count"] if entry["count"] else 0
        return _make_result(True, {
            "skill_id": skill_id,
            "average": round(avg, 2),
            "count": entry["count"],
        })
    except Exception as e:
        return _make_result(False, error=f"获取评分失败: {e}")


# ── 应用技能（注入 prompt 模板） ───────────────────────────────────
def apply_skill(skill_id: str, variables: dict, scope: str = "global",
                workspace_path: str = "") -> dict:
    """把技能的 prompt_template 与 variables 合并，返回渲染后的 prompt。"""
    target_dir = _resolve_skills_dir(scope, workspace_path)
    if target_dir is None:
        return _make_result(False, error="项目级应用需要 workspace_path")
    # 优先项目级
    candidates: list[Path] = []
    if scope == "project":
        candidates.append(target_dir / skill_id)
        candidates.append(_global_skills_dir() / skill_id)
    else:
        candidates.append(target_dir / skill_id)
        candidates.append(_global_skills_dir() / "custom" / skill_id)

    for skill_dir in candidates:
        if not skill_dir.exists():
            continue
        yaml_path = skill_dir / "skill.yaml"
        if not yaml_path.exists():
            continue
        try:
            text = yaml_path.read_text(encoding="utf-8")
            template_m = re.search(r"^prompt_template:\s*\|\s*$", text, re.MULTILINE)
            if not template_m:
                continue
            # 提取 | 后的多行块
            lines = text.splitlines()
            start_idx = None
            for i, line in enumerate(lines):
                if line.startswith("prompt_template:") and line.rstrip().endswith("|"):
                    start_idx = i + 1
                    break
            if start_idx is None:
                continue
            template_lines = []
            for line in lines[start_idx:]:
                # 缩进至少 2 空格
                if line.startswith("  ") or line.strip() == "":
                    template_lines.append(line[2:] if line.startswith("  ") else line)
                else:
                    break
            template = "\n".join(template_lines).strip()
        except Exception:
            continue

        try:
            rendered = template
            for k, v in (variables or {}).items():
                rendered = rendered.replace("{{" + k + "}}", str(v))
            return _make_result(True, {
                "rendered_prompt": rendered,
                "skill_id": skill_id,
                "scope": "project" if skill_dir.parent.name != "custom" and scope == "project" else "global",
                "path": str(skill_dir),
            })
        except Exception as e:
            return _make_result(False, error=f"渲染模板失败: {e}")

    return _make_result(False, error=f"技能 {skill_id} 未安装")


# ── 导出 zip ─────────────────────────────────────────────────────
def export_skill_zip(skill_id: str, scope: str = "global",
                     workspace_path: str = "") -> dict:
    """把已安装的技能打包为 zip（base64 编码），用于分享。"""
    import base64
    if not SKILL_ID_PATTERN.match(skill_id):
        return _make_result(False, error="无效的技能 ID")
    target_dir = _resolve_skills_dir(scope, workspace_path)
    if target_dir is None:
        return _make_result(False, error="项目级导出需要 workspace_path")
    skill_dir = target_dir / skill_id
    if not skill_dir.exists():
        return _make_result(False, error=f"技能未安装: {skill_id}")
    try:
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zf:
            for root, _, files in os.walk(skill_dir):
                for fn in files:
                    fp = Path(root) / fn
                    arc = fp.relative_to(skill_dir)
                    zf.write(fp, arcname=str(arc))
        b64 = base64.b64encode(buf.getvalue()).decode("ascii")
        return _make_result(True, {
            "filename": f"{skill_id}.zip",
            "size": len(buf.getvalue()),
            "base64": b64,
        })
    except Exception as e:
        return _make_result(False, error=f"导出失败: {e}")


# ── 工具 ──────────────────────────────────────────────────────────
def _dump_yaml(meta: dict) -> str:
    """把 dict 序列化为简易 YAML（仅支持一阶 + | 多行字符串 + 嵌套 files）。

    不用 PyYAML 以保持 0 依赖。生产可换 ruamel.yaml。
    """
    lines: list[str] = []
    for k, v in meta.items():
        if k == "files" and isinstance(v, dict):
            lines.append("files:")
            for fk, fv in v.items():
                lines.append(f"  {fk}: |")
                for fl in str(fv).splitlines():
                    lines.append(f"    {fl}")
            continue
        if k == "prompt_template" and isinstance(v, str):
            lines.append("prompt_template: |")
            for pl in v.splitlines():
                lines.append(f"  {pl}")
            continue
        if isinstance(v, list):
            lines.append(f"{k}:")
            for item in v:
                lines.append(f"  - {item}")
            continue
        if isinstance(v, str) and "\n" in v:
            lines.append(f"{k}: |")
            for sl in v.splitlines():
                lines.append(f"  {sl}")
            continue
        lines.append(f"{k}: {v}")
    return "\n".join(lines) + "\n"


# ── 统计 ──────────────────────────────────────────────────────────
def get_stats() -> dict:
    """技能市场总览统计。"""
    try:
        global_installed = 0
        gd = _global_skills_dir()
        if gd.exists():
            global_installed = sum(1 for e in gd.iterdir() if e.is_dir() and not e.name.startswith("."))
        total_rating = sum(s.get("rating", 0) for s in SEED_CATALOG)
        total_downloads = sum(s.get("downloads", 0) for s in SEED_CATALOG)
        return _make_result(True, {
            "official_count": len(SEED_CATALOG),
            "global_installed": global_installed,
            "avg_rating": round(total_rating / len(SEED_CATALOG), 2) if SEED_CATALOG else 0,
            "total_downloads": total_downloads,
            "categories": len(VALID_CATEGORIES),
        })
    except Exception as e:
        return _make_result(False, error=f"统计失败: {e}")


__all__ = [
    "SEED_CATALOG",
    "VALID_CATEGORIES",
    "SKILL_ID_PATTERN",
    "list_catalog",
    "get_skill",
    "list_categories",
    "install_skill",
    "uninstall_skill",
    "list_installed",
    "upload_skill",
    "rate_skill",
    "get_user_rating",
    "apply_skill",
    "export_skill_zip",
    "get_stats",
]
