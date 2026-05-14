import sys, os, importlib.util
spec = importlib.util.spec_from_file_location("bv", os.path.join(os.path.dirname(__file__), "build-version.py"))
bv = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bv)

release_dir = bv.BUILD_DIR / "云集智能编程工作站v2026.05.14.0354"
print(f"Release dir: {release_dir}")
exe_name = f"{release_dir.name}.exe"
print(f"EXE exists: {(release_dir / exe_name).exists()}")

print("Step 3: 打包后处理...")
bv.post_build(release_dir)

print("Step 4: 清理...")
bv.cleanup()

print("Step 5: 记录版本...")
changes = ["API服务注册表+按模型名自动路由", "一键启动全部服务", "服务类型识别", "旧虚拟环境迁移", "三目录架构修正", "PYTHONDONTWRITEBYTECODE子进程传递"]
release_name = release_dir.name
bv.record_version(release_name, changes)

print("Step 6: 部署到 dev/...")
bv._deploy_to_dev(release_dir)

print("Done!")
