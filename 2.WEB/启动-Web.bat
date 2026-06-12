@echo off
title YunJi Web Pro

set "HERE=%~dp0"
cd /d "%HERE%"

echo.
echo ============================================================
echo   YunJi Web (Pro) - Starting...
echo ============================================================
echo.

if not exist "api\api_main.py" (
    echo   [ERROR] api_main.py not found
    pause
    exit /b 1
)

:: Backend
echo   [1/3] Starting backend on 18080...
cd api
if exist "..\..\1.PC\app\uv\uv.exe" (
    start "YunJi-Web-API" cmd /c ..\..\1.PC\app\uv\uv.exe run api_main.py --port 18080
) else (
    start "YunJi-Web-API" cmd /c python api_main.py --port 18080
)
cd ..

ping -n 4 127.0.0.1 >nul

:: Frontend
echo   [2/3] Starting frontend on 5173...
if not exist "ui\node_modules" (
    echo        Installing npm packages...
    cd ui
    call npm install --silent
    cd ..
)
cd ui
start "YunJi-Web-Vite" cmd /c npm run dev
cd ..

ping -n 3 127.0.0.1 >nul

echo   [3/3] Opening browser...
start "" http://localhost:5173

echo.
echo ============================================================
echo   Backend:  http://127.0.0.1:18080/docs
echo   Frontend: http://localhost:5173
echo ============================================================
echo.
pause