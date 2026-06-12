@echo off
chcp 65001 >nul 2>&1
title YunJi SmartIDE - Dev Mode

echo ============================================
echo   YunJi SmartIDE - Development Mode
echo ============================================
echo.

cd /d "%~dp0"

if not exist "app\main.py" (
    echo [ERROR] app\main.py not found
    pause
    exit /b 1
)

if not exist "data" mkdir data
if not exist "temp" mkdir temp

set "UV=%~dp0app\uv\uv.exe"
set "VENV=%~dp0data\.venv"

if not exist "%VENV%\Scripts\python.exe" (
    echo [INFO] Creating venv...
    "%UV%" venv "%VENV%" --python 3.13
    if %ERRORLEVEL% NEQ 0 (
        echo [ERROR] Failed to create venv
        pause
        exit /b 1
    )
    echo [INFO] Installing dependencies...
    "%UV%" pip install --python "%VENV%\Scripts\python.exe" PyQt6 PyQt6-WebEngine
)

echo [INFO] Starting...
echo.

cd /d "%~dp0app"
"%VENV%\Scripts\python.exe" main.py

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] Exit code: %ERRORLEVEL%
    cd /d "%~dp0"
    pause
)
