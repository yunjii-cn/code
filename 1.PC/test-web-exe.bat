@echo off
chcp 65001 >nul 2>&1
title YunJi Code

echo.
echo ========================================
echo   YunJi Code - Desktop
echo ========================================
echo.

cd /d "%~dp0"

for %%f in (*.exe) do (
    echo   Starting...
    start "" "%%f"
    goto :end
)

echo   Launcher not found. Please build first:
echo     python build/build_stub.py
echo     python build/build_web.py
echo.

:end
pause
