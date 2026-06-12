@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul 2>&1
title 云集智能编程工作站 - Web开发

set "ROOT=%~dp0"
set "ROOT=%ROOT:~0,-1%"
set "APP_DIR=%ROOT%\app"
set "WEB_DIR=%ROOT%\web"

echo.
echo   云集智能编程工作站 - Web 开发模式
echo   ========================================
echo.

:: ── 检查 Python ──
where python >nul 2>&1
if %errorlevel% neq 0 (
    echo   [X] Python 未安装
    pause & exit /b 1
)

:: ── 检查 FastAPI 依赖 ──
python -c "import fastapi" >nul 2>&1
if %errorlevel% neq 0 (
    echo   [!] 安装 Python 依赖...
    if exist "%APP_DIR%\uv\uv.exe" (
        "%APP_DIR%\uv\uv.exe" pip install --system fastapi uvicorn >nul 2>&1
    ) else (
        pip install fastapi uvicorn --quiet >nul 2>&1
    )
)
python -c "import fastapi; import uvicorn" >nul 2>&1
if %errorlevel% neq 0 (
    echo   [X] Python 依赖安装失败
    pause & exit /b 1
)

:: ── 检查 Node.js ──
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo   [X] Node.js 未安装
    pause & exit /b 1
)

:: ── 检查前端依赖 ──
if not exist "%WEB_DIR%\node_modules" (
    echo   [!] 安装前端依赖...
    cd /d "%WEB_DIR%"
    call npm install --silent >nul 2>&1
    if %errorlevel% neq 0 (
        echo   [X] npm install 失败
        pause & exit /b 1
    )
)

:: ── 启动后端 ──
echo   [*] 启动后端 API (端口 18080)...
cd /d "%APP_DIR%"
start "" /MIN cmd /c "title YunJi-API && python api_main.py --dev --port 18080"

:: ── 启动前端 ──
echo   [*] 启动前端 Vite (端口 5173)...
cd /d "%WEB_DIR%"
start "" /MIN cmd /c "title YunJi-Vite && npm run dev"

:: ── 等待服务就绪 ──
echo   [*] 等待服务就绪...
set "API_READY="
set "VITE_READY="
for /L %%i in (1,1,30) do (
    timeout /t 1 /nobreak >nul

    if not defined API_READY (
        powershell -NoProfile -Command "$r=try{(Invoke-WebRequest 'http://127.0.0.1:18080/api/health' -UseBasicParsing -TimeoutSec 2).StatusCode}catch{0};Write-Host $r" 2>nul > "%TEMP%\yunji_api_check.txt"
        set /p API_STATUS=<"%TEMP%\yunji_api_check.txt"
        if "!API_STATUS!"=="200" set "API_READY=1"
    )

    if not defined VITE_READY (
        powershell -NoProfile -Command "$r=try{(Invoke-WebRequest 'http://localhost:5173' -UseBasicParsing -TimeoutSec 2).StatusCode}catch{0};Write-Host $r" 2>nul > "%TEMP%\yunji_vite_check.txt"
        set /p VITE_STATUS=<"%TEMP%\yunji_vite_check.txt"
        if "!VITE_STATUS!"=="200" set "VITE_READY=1"
    )

    if defined API_READY if defined VITE_READY goto :servers_ready
)

:servers_ready
if not defined API_READY  echo   [!] 后端启动较慢，请稍候...
if not defined VITE_READY echo   [!] 前端启动较慢，请稍候...
timeout /t 2 /nobreak >nul
del "%TEMP%\yunji_api_check.txt" "%TEMP%\yunji_vite_check.txt" 2>nul

:: ── 打开浏览器 ──
echo   [>] 打开浏览器 http://localhost:5173
start "" http://localhost:5173

echo   ========================================
echo   [OK] 开发环境已启动！
echo   API:  http://127.0.0.1:18080/docs
echo   ========================================
echo.
echo   本窗口即将关闭，服务将在后台继续运行。
echo   关闭服务请手动关闭 YunJi-API 和 YunJi-Vite 窗口。
timeout /t 3 /nobreak >nul
exit