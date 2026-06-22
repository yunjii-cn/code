import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

print("1: Starting step-by-step import...")

# 逐段执行 main.py 的代码
with open('main.py', 'r', encoding='utf-8') as f:
    source = f.read()

# 分割为模块级代码和 main() 部分
lines = source.split('\n')

# 找到 if __name__ == "__main__"
main_idx = None
for i, line in enumerate(lines):
    if line.strip() == 'if __name__ == "__main__":':
        main_idx = i
        break

if main_idx:
    module_code = '\n'.join(lines[:main_idx])
    print(f"2: Module code: {main_idx} lines")
    
    try:
        print("3: Executing module-level code...")
        exec(compile(module_code, 'main.py', 'exec'))
        print("4: Module-level code OK")
    except Exception as e:
        print(f"4: ERROR: {type(e).__name__}: {e}")
        import traceback
        traceback.print_exc()
