import os
import json
import time
import asyncio
import logging
from pathlib import Path
from typing import Optional

logger = logging.getLogger("zhipu2api")

DATA_DIR = Path(os.environ.get("ZHIPU_DATA_DIR", str(Path(__file__).parent / "data")))
DATA_DIR.mkdir(parents=True, exist_ok=True)

ACCOUNTS_FILE = DATA_DIR / "accounts.json"
API_KEYS_FILE = DATA_DIR / "api_keys.json"

PORT = int(os.environ.get("PORT", "7780"))
ADMIN_KEY = os.environ.get("ADMIN_KEY", "admin")
BASE_URL = os.environ.get("ZHIPU_BASE_URL", "https://open.bigmodel.cn/api/paas/v4")

MODEL_MAP = {
    "gpt-4o": "glm-4-plus",
    "gpt-4o-mini": "glm-4-flash",
    "claude-sonnet-4-5": "glm-4-plus",
    "claude-3-haiku": "glm-4-flash",
    "deepseek-chat": "glm-4-flash",
}


def _load_json(path: Path, default=None):
    try:
        if path.exists():
            with open(path, "r", encoding="utf-8") as f:
                return json.load(f)
    except Exception:
        pass
    return default if default is not None else {}


def _save_json(path: Path, data):
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


class KeyPool:
    def __init__(self):
        self._lock = asyncio.Lock()
        self._accounts = []
        self._load()

    def _load(self):
        data = _load_json(ACCOUNTS_FILE, {"accounts": []})
        self._accounts = data.get("accounts", [])

    def _save(self):
        _save_json(ACCOUNTS_FILE, {"accounts": self._accounts})

    async def add(self, api_key: str, label: str = ""):
        async with self._lock:
            for acc in self._accounts:
                if acc.get("api_key") == api_key:
                    return False
            self._accounts.append({
                "api_key": api_key,
                "label": label or api_key[:8] + "...",
                "valid": True,
                "status": "valid",
                "rate_limited_until": 0,
                "consecutive_failures": 0,
                "last_used": 0,
                "total_requests": 0,
            })
            self._save()
            return True

    async def remove(self, api_key: str):
        async with self._lock:
            self._accounts = [a for a in self._accounts if a.get("api_key") != api_key]
            self._save()
            return True

    async def acquire(self) -> Optional[dict]:
        async with self._lock:
            now = time.time()
            ready = []
            for acc in self._accounts:
                if not acc.get("valid", True):
                    continue
                if acc.get("rate_limited_until", 0) > now:
                    continue
                ready.append(acc)
            if not ready:
                return None
            ready.sort(key=lambda a: (a.get("total_requests", 0), a.get("last_used", 0)))
            acc = ready[0]
            acc["last_used"] = now
            acc["total_requests"] = acc.get("total_requests", 0) + 1
            self._save()
            return acc

    async def mark_rate_limited(self, api_key: str, cooldown: float = 60):
        async with self._lock:
            for acc in self._accounts:
                if acc.get("api_key") == api_key:
                    acc["rate_limited_until"] = time.time() + cooldown
                    acc["consecutive_failures"] = acc.get("consecutive_failures", 0) + 1
                    self._save()
                    break

    async def mark_invalid(self, api_key: str):
        async with self._lock:
            for acc in self._accounts:
                if acc.get("api_key") == api_key:
                    acc["valid"] = False
                    acc["status"] = "invalid"
                    self._save()
                    break

    async def mark_valid(self, api_key: str):
        async with self._lock:
            for acc in self._accounts:
                if acc.get("api_key") == api_key:
                    acc["valid"] = True
                    acc["status"] = "valid"
                    acc["consecutive_failures"] = 0
                    acc["rate_limited_until"] = 0
                    self._save()
                    break

    async def list_all(self):
        async with self._lock:
            return [dict(a) for a in self._accounts]

    async def count(self):
        async with self._lock:
            total = len(self._accounts)
            valid = sum(1 for a in self._accounts if a.get("valid", True))
            return total, valid


key_pool = KeyPool()
