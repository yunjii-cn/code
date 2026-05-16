#!/usr/bin/env python3
import json
import sys
import time
sys.path.insert(0, '.')

from backend import (
    add_zhipu_account,
    list_zhipu_accounts,
    check_zhipu_api
)

# 请在这里填入您的智谱API Key
ZHIPU_API_KEY = "YOUR_API_KEY_HERE"
BASE_URL = "http://127.0.0.1:7780"
ADMIN_KEY = "admin"

def test_add_account():
    """测试添加账户功能"""
    print("=== 测试1: 添加智谱API Key ===")

    result = add_zhipu_account(
        base_url=BASE_URL,
        api_key=ZHIPU_API_KEY,
        admin_key=ADMIN_KEY,
        label="test-key"
    )

    print("添加结果:", result)

    if result.get("ok"):
        print("✅ 账户添加成功")
        return True
    else:
        print("❌ 账户添加失败:", result.get("error", "未知错误"))
        return False

def test_list_accounts():
    """测试列出账户功能"""
    print("\n=== 测试2: 列出智谱账户 ===")

    result = list_zhipu_accounts(
        base_url=BASE_URL,
        admin_key=ADMIN_KEY
    )

    print("列出结果:", result)

    if result.get("ok"):
        accounts = result.get("accounts", [])
        print(f"✅ 找到 {len(accounts)} 个账户")
        for acc in accounts:
            print(f"  - {acc.get('label', 'unknown')}: {acc.get('api_key', 'N/A')[:20]}...")
        return True
    else:
        print("❌ 获取账户列表失败:", result.get("error", "未知错误"))
        return False

def test_check_api():
    """测试API服务状态"""
    print("\n=== 测试3: 检查API服务状态 ===")

    result = check_zhipu_api(
        api_key=ZHIPU_API_KEY,
        base_url=BASE_URL
    )

    print("服务状态:", result)

    if result.get("ok"):
        print("✅ API服务正常")
        return True
    else:
        print("❌ API服务异常:", result.get("error", "未知错误"))
        return False

if __name__ == "__main__":
    print("智谱API测试脚本")
    print("=" * 50)

    if ZHIPU_API_KEY == "YOUR_API_KEY_HERE":
        print("❌ 请先在脚本中设置您的智谱API Key！")
        sys.exit(1)

    # 测试添加账户
    if not test_add_account():
        print("\n❌ 测试失败：无法添加账户")
        sys.exit(1)

    time.sleep(1)

    # 测试列出账户
    test_list_accounts()

    time.sleep(1)

    # 测试API服务
    test_check_api()

    print("\n" + "=" * 50)
    print("测试完成！")
