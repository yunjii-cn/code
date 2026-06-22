import sys
print('1: start')
import clr_loader
print('2: clr_loader OK')
import pythonnet
print('3: pythonnet OK')
pythonnet.load('coreclr')
print('4: coreclr loaded')
import webview
print('5: webview OK')
