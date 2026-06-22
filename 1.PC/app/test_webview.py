"""最小化启动测试"""
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from PyQt6.QtWidgets import QApplication, QMainWindow, QLabel
from PyQt6.QtCore import Qt
import webview

app = QApplication(sys.argv)
app.setStyle('Fusion')

w = QMainWindow()
w.setWindowTitle("云集桌面 - WebView2 测试")
w.resize(800, 600)
label = QLabel("PyQt6 启动成功！点击按钮测试 WebView2")
label.setAlignment(Qt.AlignmentFlag.AlignCenter)
label.setStyleSheet("color: white; font-size: 24px; background: #1a1a1a;")
w.setCentralWidget(label)
w.show()

print("[TEST] PyQt6 窗口启动成功")
sys.exit(app.exec())
