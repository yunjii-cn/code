"""Web Push 路由 - VAPID 密钥 + 订阅管理 + 测试推送

2026-06-09 TASK-4.3 引入：Phase 4 W14 PWA 完整支持

Endpoints:
    GET  /api/push/vapid-public-key    - 获取 VAPID 公钥（前端订阅用）
    POST /api/push/subscribe           - 注册订阅
    POST /api/push/unsubscribe         - 取消订阅（按 endpoint）
    GET  /api/push/subscriptions       - 列出所有订阅（管理用）
    DELETE /api/push/subscriptions/{id} - 删除订阅
    POST /api/push/test                - 发送测试推送
    POST /api/push/notify              - 主动推送通知（内部接口）
"""
from typing import Any, Dict, List, Optional

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from platformkit.shared.push_core import (
    PushError,
    PushPayload,
    PushSender,
    PushSubscription,
    SubscriptionStore,
    VAPIDStore,
    build_push_payload_from_notification,
    get_push_sender,
    should_push,
)


router = APIRouter(prefix="/api/push", tags=["PWA 推送"])


# ──────────── Pydantic 模型 ────────────


class SubscribeRequest(BaseModel):
    endpoint: str = Field(..., min_length=1, description="Push 服务 endpoint URL")
    keys: Dict[str, str] = Field(..., description="包含 p256dh 和 auth")
    expirationTime: Optional[float] = Field(None, description="过期时间戳（毫秒）")
    userAgent: Optional[str] = Field(None, description="用户代理")


class UnsubscribeRequest(BaseModel):
    endpoint: str = Field(..., min_length=1)


class TestPushRequest(BaseModel):
    title: str = Field("云集编程测试", max_length=100)
    body: str = Field("推送通知测试成功！", max_length=200)
    url: str = Field("/", max_length=500)


class NotifyRequest(BaseModel):
    notification: Dict[str, Any] = Field(..., description="感知通知数据")
    min_severity: str = Field("warning", description="最低推送严重度")
    workspace_path: Optional[str] = Field(None, description="工作区路径")


# ──────────── 端点 ────────────


@router.get("/vapid-public-key")
async def get_vapid_public_key():
    """获取 VAPID 公钥（前端 subscribe() 调用）"""
    vapid = VAPIDStore.instance().get_or_generate()
    return {"ok": True, "data": {"publicKey": vapid.public_key}}


@router.get("/status")
async def push_status():
    """推送服务状态"""
    subs = SubscriptionStore.instance()
    vapid = VAPIDStore.instance()
    has_key = vapid.get_or_generate() is not None
    return {
        "ok": True,
        "data": {
            "vapid_initialized": has_key,
            "subscription_count": subs.count(),
            "active_count": len(subs.list_all(enabled_only=True)),
        },
    }


@router.post("/subscribe")
async def subscribe(req: SubscribeRequest):
    """注册推送订阅"""
    try:
        # 构造订阅对象
        data: Dict[str, Any] = {
            "endpoint": req.endpoint,
            "keys": req.keys,
        }
        sub = PushSubscription.from_request(data, user_agent=req.userAgent or "")
        # 保存
        stored = SubscriptionStore.instance().add(sub)
        return {
            "ok": True,
            "data": {
                "id": stored.id,
                "created_at": stored.created_at,
                "message": "订阅成功",
            },
        }
    except PushError as e:
        raise HTTPException(400, str(e))
    except Exception as e:
        raise HTTPException(500, f"订阅失败: {e}")


@router.post("/unsubscribe")
async def unsubscribe(req: UnsubscribeRequest):
    """取消订阅"""
    removed = SubscriptionStore.instance().remove_by_endpoint(req.endpoint)
    return {"ok": True, "data": {"removed": removed}}


@router.get("/subscriptions")
async def list_subscriptions():
    """列出所有订阅（管理用）"""
    subs = SubscriptionStore.instance().list_all()
    items = []
    for s in subs:
        d = s.to_dict()
        # 不返回敏感字段
        d.pop("p256dh", None)
        d.pop("auth", None)
        # endpoint 显示前缀
        if d.get("endpoint"):
            d["endpoint_short"] = d["endpoint"][:50] + "..." if len(d["endpoint"]) > 50 else d["endpoint"]
        items.append(d)
    return {"ok": True, "data": items, "total": len(items)}


@router.delete("/subscriptions/{sub_id}")
async def delete_subscription(sub_id: str):
    """删除订阅"""
    removed = SubscriptionStore.instance().remove(sub_id)
    if not removed:
        raise HTTPException(404, "订阅不存在")
    return {"ok": True}


@router.post("/test")
async def test_push(req: TestPushRequest):
    """发送测试推送"""
    payload = PushPayload(
        title=req.title,
        body=req.body,
        url=req.url,
        tag="yj-test",
    )
    sender = get_push_sender()
    results = sender.send_to_all(payload, throttle_tag="test", throttle_seconds=0)
    if not results:
        return {
            "ok": True,
            "data": {
                "delivered": 0,
                "message": "没有可用的订阅，请先在浏览器启用通知",
            },
        }
    delivered = sum(1 for r in results if r.ok)
    failed = len(results) - delivered
    cleaned = sum(1 for r in results if r.permanent)
    return {
        "ok": True,
        "data": {
            "delivered": delivered,
            "failed": failed,
            "cleaned": cleaned,
            "results": [
                {
                    "sub_id": r.sub_id,
                    "ok": r.ok,
                    "status": r.status,
                    "error": r.error if not r.ok else "",
                }
                for r in results
            ],
        },
    }


@router.post("/notify")
async def push_notification(req: NotifyRequest):
    """主动推送感知通知（供 ResponsiveEngine 等内部调用）"""
    if not should_push(req.notification, req.min_severity):
        return {
            "ok": True,
            "data": {
                "skipped": True,
                "reason": f"severity {req.notification.get('severity')} < {req.min_severity}",
            },
        }

    payload = build_push_payload_from_notification(
        req.notification, workspace_path=req.workspace_path or ""
    )
    # 用 type + id 作为节流 tag
    throttle_tag = f"{req.notification.get('type', 'general')}:{req.notification.get('id', '')[:8]}"
    sender = get_push_sender()
    results = sender.send_to_all(payload, throttle_tag=throttle_tag, throttle_seconds=300)

    delivered = sum(1 for r in results if r.ok)
    return {
        "ok": True,
        "data": {
            "delivered": delivered,
            "throttle_tag": throttle_tag,
        },
    }
