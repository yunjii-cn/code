@echo off
title YunJi Desktop

set "HERE=%~dp0"
cd /d "%HERE%"

echo.
echo ============================================================
echo   YunJi Desktop (Free) - Starting...
echo ============================================================
echo.

if not exist "app\main.py" (
    echo   [ERROR] main.py not found
    pause
    exit /b 1
)

cd "app"

if not exist "uv\uv.exe" (
    echo   uv not found. Run: 一键环境部署.bat
    pause
    exit /b 1
)

echo   Starting...
"uv\uv.exe" run main.py

if errorlevel 1 (
    echo   Start failed ^(code %errorlevel%^)
    pause
)