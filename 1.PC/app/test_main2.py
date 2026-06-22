import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

print("1: Import main module (without calling main)...")
import main as m

print("2: Calling _ensure_single_instance...")
m._ensure_single_instance()
print("3: _ensure_single_instance OK")

print("4: Creating QApplication...")
from PyQt6.QtWidgets import QApplication
from PyQt6.QtCore import Qt
app = QApplication(sys.argv)
app.setStyle('Fusion')

print("5: QApplication OK")

print("6: Creating MainWindow...")
splash = m.SplashScreen()
from PyQt6.QtCore import QRect
screen = app.primaryScreen().geometry()
x = (screen.width() - splash.width()) // 2
y = (screen.height() - splash.height()) // 2
splash.move(x, y)
splash.show()
splash.repaint()
app.processEvents()

window = m.MainWindow(splash=splash)
window.resize(1260, 860)
print("7: MainWindow OK, showing...")
window.show()

print("8: Starting event loop...")
sys.exit(app.exec())
