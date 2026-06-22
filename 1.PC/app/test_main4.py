import sys
print("1: Creating QApplication...")
from PyQt6.QtWidgets import QApplication
app = QApplication(sys.argv)
app.setStyle('Fusion')
print("2: QApplication OK")

print("3: Import webview...")
import webview
print("4: webview OK")

print("5: Creating pywebview window...")
w = webview.create_window("Test", html="<h1>Hello WebView2</h1>", width=800, height=600)
print("6: Starting webview...")
webview.start()
print("7: Done")
