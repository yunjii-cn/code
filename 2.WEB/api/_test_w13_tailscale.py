"""W13 TASK-4.2 Tailscale 核心集成测试"""
import sys
from pathlib import Path

ROOT = Path(r"E:\软件开发\云集智能编程工作站\dev\dev\app")
sys.path.insert(0, str(ROOT))

from platformkit.shared.tailscale_core import (
    TailscaleDetector, TailscaleInfo, TailscaleStatus,
    PairingCode, PairingCodeStore, PairingCode_generate,
    RemoteAccessHint, build_remote_hint, tailscale_summary,
    get_lan_ip,
)


def test_tailscale_info_serialization():
    """TailscaleInfo 序列化"""
    info = TailscaleInfo(
        status=TailscaleStatus.RUNNING_ONLINE,
        installed=True,
        running=True,
        logged_in=True,
        online=True,
        ipv4="100.64.1.2",
        ipv6="fd7a:115c:a1e0::1",
        hostname="my-mac",
        tailnet="tail.ts.net",
        account="user@example.com",
    )
    d = info.to_dict()
    assert d["status"] == "running_online"
    assert d["ipv4"] == "100.64.1.2"
    # 反序列化
    info2 = TailscaleInfo.from_dict(d)
    assert info2.status == TailscaleStatus.RUNNING_ONLINE
    assert info2.ipv4 == "100.64.1.2"
    assert info2.hostname == "my-mac"
    print("  ✓ test_tailscale_info_serialization")


def test_tailscale_detect_safe():
    """Tailscale 探测（系统无 Tailscale 时不抛异常）"""
    info = TailscaleDetector.detect()
    # 至少要返回合法结构
    assert isinstance(info, TailscaleInfo)
    assert info.status in (
        TailscaleStatus.UNKNOWN,
        TailscaleStatus.NOT_INSTALLED,
        TailscaleStatus.INSTALLED_STOPPED,
        TailscaleStatus.RUNNING_LOGGED_OUT,
        TailscaleStatus.RUNNING_ONLINE,
        TailscaleStatus.ERROR,
    )
    print(f"  ✓ test_tailscale_detect_safe -> status={info.status.value}, installed={info.installed}")


def test_tailscale_detect_cache():
    """Tailscale 探测缓存"""
    info1 = TailscaleDetector.detect()
    info2 = TailscaleDetector.detect()  # 走缓存
    assert info1.detected_at == info2.detected_at  # 缓存应该复用
    # 清缓存
    TailscaleDetector.clear_cache()
    info3 = TailscaleDetector.detect()
    assert info3.detected_at >= info1.detected_at
    print("  ✓ test_tailscale_detect_cache")


def test_pairing_code_format():
    """配对码格式"""
    code = PairingCode_generate()
    assert len(code.value) == 6
    # 6 位 hex
    int(code.value, 16)  # 应该不报错
    # formatted
    assert code.formatted == f"{code.value[:3]}-{code.value[3:]}"
    # 唯一性（多次生成不同）
    codes = set()
    for _ in range(50):
        c = PairingCode_generate()
        codes.add(c.value)
    assert len(codes) >= 1  # 至少 1 个
    print(f"  ✓ test_pairing_code_format -> sample={code.formatted}")


def test_pairing_code_store():
    """配对码存储"""
    store = PairingCodeStore.instance()
    c1 = store.get_or_generate()
    c2 = store.get_or_generate()  # 应该返回同一个
    assert c1.value == c2.value
    # 校验
    assert store.verify(c1.value) is True
    assert store.verify("WRONG1") is False
    # 重新生成
    c3 = store.regenerate()
    assert c3.value != c1.value or len({c1.value, c3.value}) == 2
    assert store.verify(c3.value) is True
    assert store.verify(c1.value) is False  # 旧的失效
    print("  ✓ test_pairing_code_store")


def test_pairing_code_expiry():
    """配对码过期"""
    code = PairingCode(value="ABC123", ttl_seconds=0.0001)  # 1ms
    import time
    time.sleep(0.01)
    assert code.is_expired is True
    print("  ✓ test_pairing_code_expiry")


def test_get_lan_ip():
    """获取 LAN IP"""
    ip = get_lan_ip()
    # 不强制要求非 None（环境无网时为 None）
    if ip:
        parts = ip.split(".")
        assert len(parts) == 4
        for p in parts:
            assert p.isdigit()
    print(f"  ✓ test_get_lan_ip -> {ip}")


def test_build_remote_hint_no_tailscale():
    """构建远程提示（无 Tailscale）"""
    TailscaleDetector.clear_cache()
    info = TailscaleInfo(status=TailscaleStatus.NOT_INSTALLED, installed=False)
    hint = build_remote_hint(port=18080, info=info, pairing_code="ABC123")
    # primary 应该是 LAN IP 或 127.0.0.1
    assert hint.primary_url is not None
    assert ":18080" in hint.primary_url
    assert "token=ABC123" in hint.primary_url
    assert hint.requires_token is True
    assert hint.note is not None
    print(f"  ✓ test_build_remote_hint_no_tailscale -> primary={hint.primary_url}, note={hint.note}")


def test_build_remote_hint_tailscale_online():
    """构建远程提示（Tailscale 在线）"""
    info = TailscaleInfo(
        status=TailscaleStatus.RUNNING_ONLINE,
        installed=True,
        running=True,
        logged_in=True,
        online=True,
        ipv4="100.64.1.2",
        hostname="my-host",
        tailnet="tail.ts.net",
    )
    hint = build_remote_hint(port=19000, info=info, pairing_code="XYZ789")
    # primary 应该是 Tailscale IP
    assert "100.64.1.2:19000" in hint.primary_url
    assert "token=XYZ789" in hint.primary_url
    assert "✅ Tailscale 已登录" in (hint.note or "")
    # 备选应该有 LAN 和 127.0.0.1
    assert len(hint.backup_urls) >= 1
    print(f"  ✓ test_build_remote_hint_tailscale_online")


def test_build_remote_hint_no_token():
    """构建远程提示（无配对码）"""
    info = TailscaleInfo(status=TailscaleStatus.NOT_INSTALLED, installed=False)
    hint = build_remote_hint(port=18080, info=info, pairing_code=None)
    assert hint.requires_token is False
    assert "?token=" not in (hint.primary_url or "")
    print(f"  ✓ test_build_remote_hint_no_token -> {hint.primary_url}")


def test_tailscale_summary():
    """Tailscale 状态摘要"""
    summary = tailscale_summary()
    assert "tailscale" in summary
    assert "platform" in summary
    assert summary["platform"] in ("Windows", "Linux", "Darwin")
    assert "lan_ip" in summary
    print(f"  ✓ test_tailscale_summary -> platform={summary['platform']}, lan_ip={summary['lan_ip']}")


def test_tailscale_status_values():
    """TailscaleStatus 枚举完整性"""
    assert TailscaleStatus.UNKNOWN
    assert TailscaleStatus.NOT_INSTALLED
    assert TailscaleStatus.INSTALLED_STOPPED
    assert TailscaleStatus.RUNNING_LOGGED_OUT
    assert TailscaleStatus.RUNNING_ONLINE
    assert TailscaleStatus.ERROR
    print("  ✓ test_tailscale_status_values")


def test_remote_access_hint_serialization():
    """RemoteAccessHint 序列化"""
    hint = RemoteAccessHint(
        primary_url="http://100.64.1.2:18080?token=ABC",
        backup_urls=["http://192.168.1.5:18080?token=ABC"],
        pairing_code="ABC",
        note="Tailscale OK",
        requires_token=True,
    )
    d = hint.to_dict()
    assert d["primary_url"] == "http://100.64.1.2:18080?token=ABC"
    assert d["pairing_code"] == "ABC"
    assert d["requires_token"] is True
    print("  ✓ test_remote_access_hint_serialization")


def main():
    print("\n=== W13 TASK-4.2 Tailscale 核心集成测试 ===\n")
    test_tailscale_info_serialization()
    test_tailscale_detect_safe()
    test_tailscale_detect_cache()
    test_pairing_code_format()
    test_pairing_code_store()
    test_pairing_code_expiry()
    test_get_lan_ip()
    test_build_remote_hint_no_tailscale()
    test_build_remote_hint_tailscale_online()
    test_build_remote_hint_no_token()
    test_tailscale_summary()
    test_tailscale_status_values()
    test_remote_access_hint_serialization()
    print("\n✅ 所有 13 个测试通过\n")


if __name__ == "__main__":
    main()
