"""Platform Shared - Git 子进程包装器

通过 subprocess 包装 `git` CLI，提供：
    - 单文件回退（git restore / git checkout）
    - 增强 diff（带行号信息的 hunk 解析）

**设计原则**:
    - 不依赖 FastAPI（路由层做参数解析 + 调本模块）
    - 不依赖第三方库（仅 stdlib subprocess + re）
    - 统一返回 dict: {"ok": bool, "data": Any, "error": str|None}

**使用规则**:
    - ✅ 路由层: `from platformkit.shared.git_core import restore_file`
    - ❌ 禁止: 在 routes/*.py 中直接 subprocess.run(["git", ...])
"""
from __future__ import annotations

import os
import re
import subprocess
from typing import Optional


DEFAULT_TIMEOUT_S = 15
CREATE_NO_WINDOW = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0


def _make_result(ok: bool, data=None, error: Optional[str] = None, stderr: str = "", returncode: int = 0) -> dict:
    return {"ok": ok, "data": data, "error": error, "stderr": stderr, "returncode": returncode}


def _run_git(args: list[str], cwd: Optional[str] = None, timeout_s: int = DEFAULT_TIMEOUT_S) -> dict:
    """底层 git 命令执行器。"""
    if not cwd or not os.path.isdir(cwd):
        return _make_result(ok=False, error=f"项目路径无效: {cwd}")
    try:
        proc = subprocess.run(
            ["git", *args],
            capture_output=True,
            text=True,
            timeout=timeout_s,
            cwd=cwd,
            creationflags=CREATE_NO_WINDOW,
        )
        stderr = (proc.stderr or "").strip()
        if proc.returncode != 0:
            return _make_result(
                ok=False,
                returncode=proc.returncode,
                stderr=stderr,
                error=stderr or f"git 命令失败，退出码 {proc.returncode}",
            )
        return _make_result(ok=True, data=proc.stdout, stderr=stderr, returncode=0)
    except subprocess.TimeoutExpired:
        return _make_result(ok=False, error=f"git 命令超时（>{timeout_s}s）")
    except FileNotFoundError:
        return _make_result(ok=False, error="git 未安装")
    except Exception as e:
        return _make_result(ok=False, error=str(e))


def _validate_path(cwd: str, file_path: str) -> Optional[str]:
    """校验 file_path 在 cwd 范围内（防止 path traversal）。返回错误信息或 None。"""
    if not file_path:
        return "文件路径不能为空"
    if file_path.startswith("/") or file_path.startswith("\\"):
        return "文件路径必须为相对路径"
    abs_cwd = os.path.abspath(cwd)
    abs_target = os.path.abspath(os.path.join(abs_cwd, file_path))
    # Windows 不区分大小写
    cmp_cwd = abs_cwd.lower() if os.name == "nt" else abs_cwd
    cmp_target = abs_target.lower() if os.name == "nt" else abs_target
    if not cmp_target.startswith(cmp_cwd + os.sep) and cmp_target != cmp_cwd:
        return f"文件路径越界: {file_path}"
    return None


def restore_file(cwd: str, file_path: str, staged: bool = False) -> dict:
    """单文件回退到 HEAD 版本（git restore）。

    Args:
        cwd: 项目根目录
        file_path: 相对路径
        staged: True = 只取消暂存（保留 working tree 修改），False = 完全回退

    Returns:
        {"ok", "error"|None, "stderr": str}
    """
    err = _validate_path(cwd, file_path)
    if err:
        return _make_result(ok=False, error=err)
    if staged:
        # git restore --staged <file> 取消暂存
        args = ["restore", "--staged", file_path]
    else:
        # git restore <file> 丢弃 working tree 修改
        args = ["restore", file_path]
    res = _run_git(args, cwd=cwd)
    if not res["ok"]:
        return res
    return _make_result(ok=True, data={"restored": file_path, "staged_only": staged})


# unified diff hunk header: @@ -oldStart,oldCount +newStart,newCount @@
HUNK_HEADER_RE = re.compile(
    r"^@@\s+-(\d+)(?:,(\d+))?\s+\+(\d+)(?:,(\d+))?\s+@@(.*)$"
)


def get_file_diff(
    cwd: str,
    file_path: str,
    staged: bool = False,
    context_lines: int = 3,
) -> dict:
    """获取单个文件的 diff（带行号信息的增强格式）。

    Args:
        cwd: 项目根目录
        file_path: 相对路径
        staged: True = 已暂存（git diff --cached），False = 未暂存
        context_lines: diff context 行数

    Returns:
        {
            "ok": True,
            "data": {
                "file": str,
                "raw": str,           # 原始 unified diff
                "hunks": [            # 解析后的 hunk 列表
                    {
                        "oldStart": int, "oldLines": int,
                        "newStart": int, "newLines": int,
                        "header": str,
                        "lines": [
                            {"type": "add"|"remove"|"context", "text": str,
                             "oldLineNo": int|None, "newLineNo": int|None}
                        ]
                    }
                ],
                "stats": {"additions": int, "deletions": int}
            }
        }
    """
    err = _validate_path(cwd, file_path)
    if err:
        return _make_result(ok=False, error=err)
    args = ["diff", f"--unified={context_lines}", "--", file_path]
    if staged:
        args.insert(1, "--cached")
    res = _run_git(args, cwd=cwd, timeout_s=15)
    if not res["ok"]:
        return res
    raw = res["data"] or ""
    hunks: list[dict] = []
    cur_hunk: Optional[dict] = None
    old_line = 0
    new_line = 0
    additions = 0
    deletions = 0
    for line in raw.splitlines():
        m = HUNK_HEADER_RE.match(line)
        if m:
            if cur_hunk is not None:
                hunks.append(cur_hunk)
            old_start = int(m.group(1))
            old_count = int(m.group(2) or "1")
            new_start = int(m.group(3))
            new_count = int(m.group(4) or "1")
            section = m.group(5) or ""
            cur_hunk = {
                "oldStart": old_start,
                "oldLines": old_count,
                "newStart": new_start,
                "newLines": new_count,
                "header": section.strip(),
                "lines": [],
            }
            old_line = old_start
            new_line = new_start
            continue
        if cur_hunk is None:
            continue
        if line.startswith("+"):
            cur_hunk["lines"].append({
                "type": "add",
                "text": line[1:],
                "oldLineNo": None,
                "newLineNo": new_line,
            })
            new_line += 1
            additions += 1
        elif line.startswith("-"):
            cur_hunk["lines"].append({
                "type": "remove",
                "text": line[1:],
                "oldLineNo": old_line,
                "newLineNo": None,
            })
            old_line += 1
            deletions += 1
        elif line.startswith(" "):
            cur_hunk["lines"].append({
                "type": "context",
                "text": line[1:],
                "oldLineNo": old_line,
                "newLineNo": new_line,
            })
            old_line += 1
            new_line += 1
    if cur_hunk is not None:
        hunks.append(cur_hunk)
    return _make_result(
        ok=True,
        data={
            "file": file_path,
            "raw": raw,
            "hunks": hunks,
            "stats": {"additions": additions, "deletions": deletions},
        },
    )


__all__ = ["restore_file", "get_file_diff"]
