#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
EXE 入口点 - 云集智能编程工作站

职责:
1. 冻结模式下抑制 stdout/stderr 避免崩溃
2. 设置 sys.path 确保 onefile 模式下能导入 backend 等模块
3. 防篡改校验 (EXE 完整性校验)
4. 导入并调用 main.main()（杀同名单实例控制在 main.py 入口统一处理）
"""
import sys
import os
import ctypes
import ctypes.wintypes
import hashlib

# ── 冻结模式下抑制输出 ──
if sys.platform == 'win32' and getattr(sys, 'frozen', False):
    class _NullWriter:
        def write(self, *args, **kwargs):
            return 0
        def flush(self, *args, **kwargs):
            pass
        def isatty(self):
            return False
    sys.stdout = _NullWriter()
    sys.stderr = _NullWriter()

# ── 设置 sys.path (onefile 模式) ──
if getattr(sys, 'frozen', False) and hasattr(sys, '_MEIPASS'):
    _meipass = sys._MEIPASS
    if _meipass not in sys.path:
        sys.path.insert(0, _meipass)


# ══════════════════════════════════════════════════════════════
# 防篡改机制 - EXE 完整性校验
# ══════════════════════════════════════════════════════════════

BRAND_NAME = "云集智能编程工作站"
INTEGRITY_FILE = ".yunji.integrity"


def _compute_exe_hash(exe_path: str) -> str:
    """计算 EXE 文件的 SHA256 哈希"""
    sha256 = hashlib.sha256()
    try:
        with open(exe_path, 'rb') as f:
            # 分块读取大文件
            while chunk := f.read(65536):
                sha256.update(chunk)
        return sha256.hexdigest()
    except Exception:
        return ""


def _get_integrity_path(exe_dir: str) -> str:
    """获取完整性校验文件路径"""
    return os.path.join(exe_dir, INTEGRITY_FILE)


def _save_exe_hash(exe_path: str, exe_dir: str) -> bool:
    """保存 EXE 哈希到完整性文件"""
    exe_hash = _compute_exe_hash(exe_path)
    if not exe_hash:
        return False
    integrity_path = _get_integrity_path(exe_dir)
    try:
        with open(integrity_path, 'w', encoding='utf-8') as f:
            f.write(exe_hash)
        return True
    except Exception:
        return False


def _verify_exe_hash(exe_path: str, exe_dir: str) -> bool:
    """验证 EXE 哈希是否匹配"""
    integrity_path = _get_integrity_path(exe_dir)
    if not os.path.exists(integrity_path):
        # 首次运行，保存哈希
        return _save_exe_hash(exe_path, exe_dir)
    
    try:
        with open(integrity_path, 'r', encoding='utf-8') as f:
            stored_hash = f.read().strip()
        current_hash = _compute_exe_hash(exe_path)
        return stored_hash == current_hash
    except Exception:
        return True  # 读取失败时放行，避免误报


def _check_integrity() -> bool:
    """
    检查 EXE 完整性
    返回 True 表示通过，False 表示被篡改
    """
    if not getattr(sys, 'frozen', False):
        # 开发模式不校验
        return True
    
    exe_path = sys.executable
    exe_dir = os.path.dirname(exe_path)
    exe_name = os.path.basename(exe_path)
    
    # 只校验正式命名的 EXE
    if not exe_name.startswith(BRAND_NAME):
        return True
    
    return _verify_exe_hash(exe_path, exe_dir)


def _show_tamper_warning():
    """显示篡改警告"""
    try:
        ctypes.windll.user32.MessageBoxW(
            0,
            f"{BRAND_NAME} 检测到程序文件被修改，可能存在安全风险。\n\n请重新安装官方版本。",
            "安全警告",
            0x10  # MB_ICONERROR
        )
    except Exception:
        pass


# ══════════════════════════════════════════════════════════════
# 单实例控制
# ══════════════════════════════════════════════════════════════
# 单实例控制统一在 main.py 的 _ensure_single_instance() 里实现:
#   - 杀同名前缀的旧 EXE
#   - 创建 mutex 占位（防 race）
# launcher.py 不再重复处理，main() 调 _main.main() 时自动执行


def main():
    """EXE/dev 共用入口"""
    # 防篡改校验
    if not _check_integrity():
        _show_tamper_warning()
        sys.exit(1)

    import main as _main
    _main.main()


if __name__ == "__main__":
    main()
