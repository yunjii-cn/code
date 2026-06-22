import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

print("1: Creating QApplication...")
from PyQt6.QtWidgets import QApplication
app = QApplication(sys.argv)
app.setStyle('Fusion')
print("2: QApplication OK")

print("3: Import main...")
import main as m
print("4: Import main OK")

print("5: Creating MainWindow...")
splash = m.SplashScreen()
window = m.MainWindow(splash=splash)
window.resize(1260, 860)
window.show()
print("6: Starting event loop...")
sys.exit(app.exec())
