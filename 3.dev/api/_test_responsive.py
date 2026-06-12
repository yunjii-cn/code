"""responsive_core 集成测试 (TASK-3.4)"""
import sys
import os
import pathlib
import tempfile
import time

# 避免污染 ~/.yunji/
os.environ["YUNJI_GLOBAL_ROOT"] = str(pathlib.Path(tempfile.gettempdir()) / "yj_responsive_test")

sys.path.insert(0, '.')

from platformkit.shared.responsive_core import (
    ResponsiveEngine, Notification, NotificationAction, NotificationType, Severity,
    FileChangeWatcher, CodeQualityPatrol, DependencyRiskScanner, ProgressTracker,
)

print('imports OK')

with tempfile.TemporaryDirectory() as tmp:
    workspace = pathlib.Path(tmp) / "demo_project"
    workspace.mkdir()

    # 1) 创建一些文件
    (workspace / "src").mkdir()
    # main.py 留给 file_change 测试
    (workspace / "src" / "main.py").write_text("def hello():\n    return 42\n", encoding="utf-8")
    # bad.py 给 code_quality 留 TODO/print 不动
    (workspace / "src" / "bad.py").write_text(
        "def bad():\n"
        "    # TODO: 处理错误\n"
        "    print('debug')\n"
        "    return 1\n",
        encoding="utf-8",
    )
    (workspace / "src" / "util.py").write_text("def fetch():\n    pass\n", encoding="utf-8")
    (workspace / "requirements.txt").write_text("requests==2.18.0\nflask==0.10.0\n# numpy etc\n", encoding="utf-8")
    (workspace / "package.json").write_text(
        json_dumps_safe := '{"name": "demo", "dependencies": {"axios": "0.20.0", "lodash": "4.17.10"}}',
        encoding="utf-8",
    )
    (workspace / ".yunji").mkdir()
    (workspace / ".yunji" / "state.json").write_text(
        '{"last_session": {"unfinished_tasks": [{"title": "完成登录模块", "description": "实现 OAuth2 流程"}, {"title": "写测试"}]}}',
        encoding="utf-8",
    )

    # 启动引擎（预热 file_change 基线）
    engine = ResponsiveEngine(workspace)
    start_result = engine.start()
    print('start:', start_result)
    assert start_result["ok"] is True
    assert start_result["running"] is True
    assert engine.is_running() is True

    # 第一次扫描 file_change 应该不产通知（基线已建立）
    fc1 = engine.trigger_scan("file_change")
    print('file_change first scan:', len(fc1), 'notifs')
    assert len(fc1) == 0, "首次扫描应不产通知"

    # 改一个文件
    time.sleep(0.1)
    (workspace / "src" / "main.py").write_text("def hello():\n    return 'updated'\n", encoding="utf-8")
    fc2 = engine.trigger_scan("file_change")
    print('file_change after change:', len(fc2), 'notifs, first title:', fc2[0].title if fc2 else None)
    assert len(fc2) > 0, "改文件后应产通知"
    assert any("main.py" in n.title for n in fc2)
    assert fc2[0].type == NotificationType.FILE_CHANGE

    # code_quality
    cq = engine.trigger_scan("code_quality")
    print('code_quality:', len(cq), 'notifs')
    assert len(cq) > 0, "应发现 TODO / print 等问题"
    assert any(n.severity == Severity.WARNING for n in cq) or any(n.severity == Severity.INFO for n in cq)
    assert any("TODO" in n.title for n in cq)
    assert any("print" in n.title for n in cq)

    # security_risk
    sr = engine.trigger_scan("security_risk")
    print('security_risk:', len(sr), 'notifs')
    assert len(sr) > 0, "应发现 requests / axios / lodash 风险"
    assert any("requests" in n.title for n in sr)
    assert any("axios" in n.title for n in sr)
    assert all(n.type == NotificationType.SECURITY_RISK for n in sr)

    # progress
    pr = engine.trigger_scan("progress")
    print('progress:', len(pr), 'notifs')
    assert len(pr) >= 2, "应有 2 条未完成任务"
    assert any("登录模块" in n.title for n in pr)

    # all
    all_n = engine.trigger_scan("all")
    print('all scan:', len(all_n), 'notifs')

    # status
    status = engine.list_watcher_status()
    print('status:', status)
    assert "file_change" in status
    assert "code_quality" in status

    # stop
    stop_result = engine.stop()
    assert stop_result["ok"] is True
    assert engine.is_running() is False

    # 非法 scan_type
    bad = engine.trigger_scan("invalid_type")
    assert bad == []

print('ALL OK')
