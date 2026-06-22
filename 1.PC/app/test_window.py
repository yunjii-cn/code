"""逐步创建 MainWindow 定位崩溃点"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from PyQt6.QtWidgets import QApplication, QMainWindow, QWidget, QVBoxLayout, QPushButton
from PyQt6.QtCore import Qt
import webview

app = QApplication(sys.argv)
app.setStyle('Fusion')

# 1. 测试基本 QMainWindow
w = QMainWindow()
w.setWindowTitle("测试1: 基本窗口")
w.resize(800, 600)
print("1: Basic QMainWindow OK")

# 2. 测试 import main
import main as m
print("2: Import main OK")

# 3. 测试 SplashScreen
splash = m.SplashScreen()
print("3: SplashScreen OK")

# 4. 测试 BackendBridge
bridge = m.BackendBridge()
print("4: BackendBridge OK")

# 5. 手动创建最小化 MainWindow
try:
    # 不用 splash，避免 Splash 相关问题
    w2 = QMainWindow()
    w2.setWindowTitle("云集桌面测试")
    w2.resize(800, 600)
    label = QPushButton("🚀 启动 WebView2")
    label.setStyleSheet("font-size: 24px; padding: 20px; color: white; background: #3b82f6; border-radius: 12px;")
    w2.setCentralWidget(label)
    w2.show()
    print("5: Manual window OK, starting event loop...")
    sys.exit(app.exec())
except Exception as e:
    print(f"ERROR: {type(e).__name__}: {e}")
    import traceback
    traceback.print_exc()
