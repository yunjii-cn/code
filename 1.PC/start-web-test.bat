@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul 2>&1
title YunJi Code - Web Dev and Build

echo ============================================================
echo   YunJi Code - Web Dev and Build
echo ============================================================
echo.
echo   [1] Start dev servers (Frontend + Backend)
echo   [2] Build EXE (Full: Frontend + PyInstaller)
echo   [3] Build EXE (Skip frontend, PyInstaller only)
echo.
set /p MODE="Select mode (1/2/3): "

if "!MODE!"=="1" goto :dev
if "!MODE!"=="2" goto :build
if "!MODE!"=="3" goto :build_skip
echo Invalid selection.
pause
goto :end

:dev
echo.
echo ============================================================
echo   Starting dev servers...
echo ============================================================
echo.

set "ROOT=%~dp0"
set "APP_DIR=%ROOT%dev\app"
set "WEB_DIR=%ROOT%dev\web"

echo [1/4] Checking Python...
where python >nul 2>&1
if !errorlevel! neq 0 (
    echo   [ERROR] Python not found
    goto :error
)
for /f "tokens=*" %%v in ('python --version 2^>^&1') do echo   [OK] %%v
echo.

echo [2/4] Checking Node.js...
where node >nul 2>&1
if !errorlevel! neq 0 (
    echo   [ERROR] Node.js not found
    goto :error
)
for /f "tokens=*" %%v in ('node --version 2^>^&1') do echo   [OK] Node.js %%v
echo.

echo [3/4] Checking web dependencies...
if not exist "%WEB_DIR%\node_modules" (
    echo   Installing npm packages...
    pushd "%WEB_DIR%"
    call npm install
    if !errorlevel! neq 0 (
        echo   [ERROR] npm install failed
        popd
        goto :error
    )
    popd
    echo   [OK] npm packages installed
) else (
    echo   [OK] node_modules exists
)
echo.

echo [4/4] Checking Python dependencies...
python -c "import fastapi" >nul 2>&1
if !errorlevel! neq 0 (
    echo   Installing Python packages...
    pip install fastapi uvicorn pywebview -q
    if !errorlevel! neq 0 (
        echo   [ERROR] pip install failed
        goto :error
    )
    echo   [OK] Python packages installed
) else (
    echo   [OK] Python packages exist
)
echo.

echo   [Backend] Starting FastAPI on port 18080...
start "YunJi-API" cmd /k "cd /d %ROOT%dev\app && python api_main.py --dev"
timeout /t 3 /nobreak >nul

echo   [Frontend] Starting Vite dev server on port 5173...
pushd "%WEB_DIR%"
start "YunJi-Vite" cmd /k "npm run dev"
popd
timeout /t 5 /nobreak >nul

echo.
echo ============================================================
echo   Dev servers started!
echo   Frontend:  http://localhost:5173
echo   Backend:   http://localhost:18080
echo   API Docs:  http://localhost:18080/docs
echo ============================================================
echo.
echo   Press any key to close this window...
pause >nul
goto :end

:build
echo.
echo ============================================================
echo   Building EXE (full)...
echo ============================================================
echo.
python build/build_web.py
goto :end_pause

:build_skip
echo.
echo ============================================================
echo   Building EXE (skip frontend)...
echo ============================================================
echo.
python build/build_web.py --skip-frontend
goto :end_pause

:error
echo.
echo   Failed. Check errors above.
echo.
pause
goto :end

:end_pause
echo.
pause

:end
endlocal
