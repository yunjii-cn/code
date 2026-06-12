#!/usr/bin/env python3
"""
智谱API功能测试脚本

使用方法：
1. 将您的智谱API Key填入 ZHIPU_API_KEY 变量
2. 运行脚本：.\uv\uv.exe run python test_zhipu_add_account.py
"""

import json
import sys
import time
sys.path.insert(0, '.')

from backend import add_zhipu_account, list_zhipu_accounts

# 请在这里填入您的智谱API Key
ZHIPU_API_KEY = "YOUR_API_KEY_HERE"
BASE_URL = "http://127.0.0.1:7780"
ADMIN_KEY = "admin"

def main():
    print("=" * 60)
    print("智谱API添加账户测试")
    print("=" * 60)
    print()

    # 检查是否设置了API Key
    if ZHIPU_API_KEY == "YOUR_API_KEY_HERE":
        print("❌ 错误：请先在脚本中设置您的智谱API Key！")
        print()
        print("设置方法：")
        print("1. 打开本脚本文件")
        print("2. 找到第15行: ZHIPU_API_KEY = 'YOUR_API_KEY_HERE'")
        print("3. 将 'YOUR_API_KEY_HERE' 替换为您的真实API Key")
        print("4. 重新运行本脚本")
        print()
        return False

    # 1. 测试添加账户
    print("步骤1: 添加智谱API Key到服务端...")
    print("API Key:", ZHIPU_API_KEY[:20] + "...")

    result = add_zhipu_account(
        base_url=BASE_URL,
        api_key=ZHIPU_API_KEY,
        admin_key=ADMIN_KEY,
        label="测试账户"
    )

    print("添加结果:", json.dumps(result, indent=2, ensure_ascii=False))
    print()

    if not result.get("ok"):
        print("❌ 添加失败!")
        print("错误信息:", result.get("error", "未知错误"))
        return False

    print("✅ 添加成功!")
    print()

    # 等待一下让服务端保存
    time.sleep(1)

    # 2. 验证账户是否保存
    print("步骤2: 验证账户是否保存...")
    result = list_zhipu_accounts(
        base_url=BASE_URL,
        admin_key=ADMIN_KEY
    )

    print("查询结果:", json.dumps(result, indent=2, ensure_ascii=False))
    print()

    if result.get("ok"):
        accounts = result.get("data", {}).get("accounts", [])
        print("✅ 服务端已保存", len(accounts), "个账户")
        for acc in accounts:
            print(f"  - {acc.get('label', 'unknown')}: {acc.get('api_key', 'N/A')[:20]}...")
        print()
        print("=" * 60)
        print("测试成功！智谱API Key已正确保存到服务端。")
        print("现在您可以在应用中正常使用智谱AI对话功能了。")
        print("=" * 60)
        return True
    else:
        print("❌ 验证失败!")
        return False

if __name__ == "__main__":
    try:
        success = main()
        sys.exit(0 if success else 1)
    except Exception as e:
        print()
        print("=" * 60)
        print("❌ 测试过程中发生异常:")
        print(str(e))
        print("=" * 60)
        import traceback
        traceback.print_exc()
        sys.exit(1)
