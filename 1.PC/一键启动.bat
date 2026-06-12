@echo off
chcp 65001 >nul 2>&1
title YunJi Debug Console

set "ROOT=%~dp0"
set "APP_DIR=%ROOT%dev\app"
set "WEB_DIR=%ROOT%dev\web"

echo.
echo   ============================================
echo     YunJi SmartIDE v2.0 - Quick Launcher
echo   ============================================
echo.
echo     [1] Web Dev  (Frontend 5173 + Backend 18080)
echo     [2] Web Dev + Auto Browser
echo     [3] Backend API Only (port 18080)
echo     [4] Daemon CLI
echo     [5] Acceptance Test
echo     [6] Check Dependencies
echo     [0] Exit
echo.

set "MODE="
set /p "MODE=   Select [0-6]: "

if "%MODE%"=="0" goto end
if "%MODE%"=="1" goto web
if "%MODE%"=="2" goto web_auto
if "%MODE%"=="3" goto api
if "%MODE%"=="4" goto daemon
if "%MODE%"=="5" goto test
if "%MODE%"=="6" goto check_deps
echo   Invalid: %MODE%
pause
goto end

REM --- check_env ---
:check_env
where python >nul 2>&1 || (
    echo   [ERROR] Python not found!
    pause
    exit /b 1
)
where node >nul 2>&1 || (
    echo   [ERROR] Node not found!
    pause
    exit /b 1
)
echo   [OK] Python found
echo   [OK] Node found
exit /b 0

REM --- wait_backend ---
:wait_backend
echo   Waiting for backend on port %1 ...
set /a _retry=0
:wait_loop
set /a _retry+=1
if %_retry% GTR 30 (
    echo   [WARN] Backend not ready after 30s, starting frontend anyway...
    exit /b 1
)
powershell -Command "try { Invoke-WebRequest -Uri 'http://127.0.0.1:%1/api/health' -TimeoutSec 2 -UseBasicParsing | Out-Null; exit 0 } catch { exit 1 }" >nul 2>&1
if %ERRORLEVEL%==0 (
    echo   [OK] Backend is ready!
    exit /b 0
)
timeout /t 1 /nobreak >nul
goto wait_loop

REM --- [1] Web Dev ---
:web
echo.
call :check_env || goto end

if not exist "%WEB_DIR%\node_modules" (
    echo   Installing npm packages...
    cd /d "%WEB_DIR%"
    call npm install
    if errorlevel 1 (
        echo   [ERROR] npm install failed!
        pause
        goto end
    )
)

echo.
echo   Starting backend on 18080...
start "YunJi-Backend" /d "%APP_DIR%" cmd /k python api_main.py --dev --port 18080

call :wait_backend 18080

echo   Starting frontend on 5173...
start "YunJi-Frontend" /d "%WEB_DIR%" cmd /k npm run dev

echo.
echo   ============================================
echo     Services Started
echo   ============================================
echo.
echo     Backend:   http://127.0.0.1:18080/docs
echo     Frontend:  http://localhost:5173
echo.
echo     - YunJi-Backend window:  Python debug output
echo     - YunJi-Frontend window: Vite debug output
echo     - This window:  Debug console (copy any text)
echo.
echo     Close the backend/frontend windows to stop.
echo.
pause
goto end

REM --- [2] Web Dev + Auto Browser ---
:web_auto
echo.
call :check_env || goto end

if not exist "%WEB_DIR%\node_modules" (
    echo   Installing npm packages...
    cd /d "%WEB_DIR%"
    call npm install
    if errorlevel 1 (
        echo   [ERROR] npm install failed!
        pause
        goto end
    )
)

echo.
echo   Starting backend on 18080...
start "YunJi-Backend" /d "%APP_DIR%" cmd /k python api_main.py --dev --port 18080

call :wait_backend 18080

echo   Starting frontend on 5173...
start "YunJi-Frontend" /d "%WEB_DIR%" cmd /k npm run dev

echo   Waiting for frontend to be ready...
timeout /t 5 /nobreak >nul

echo   Opening browser...
start "" http://localhost:5173

echo.
echo   ============================================
echo     Services Started + Browser Opened
echo   ============================================
echo.
echo     Backend:   http://127.0.0.1:18080/docs
echo     Frontend:  http://localhost:5173
echo.
echo     - YunJi-Backend window:  Python debug output
echo     - YunJi-Frontend window: Vite debug output
echo     - This window:  Debug console (copy any text)
echo.
pause
goto end

REM --- [3] Backend API Only ---
:api
echo.
echo   Starting backend on port 18080...
echo   (All output shown in this window, you can copy)
echo.
cd /d "%APP_DIR%"
python api_main.py --dev --port 18080
goto end

REM --- [4] Daemon CLI ---
:daemon
echo.
echo   ---- Daemon CLI ----
echo   Commands: start / stop / status / restart / info
echo.
cd /d "%APP_DIR%"
python daemon.py info
echo.
set "CMD="
set /p "CMD=daemon> "
if "%CMD%"=="" goto end
python daemon.py %CMD%
echo.
pause
goto end

REM --- [5] Acceptance Test ---
:test
echo.
echo   Launching acceptance test...
if exist "%~dp0acceptance_test.bat" (
    call "%~dp0acceptance_test.bat"
) else (
    echo   acceptance_test.bat not found, skip.
    pause
)
goto end

REM --- [6] Check Dependencies ---
:check_deps
echo.
echo   ============================================
echo     Dependency Check
echo   ============================================
echo.

echo   [Python]
python --version 2>&1
echo.

echo   [Node]
node --version 2>&1
echo.

echo   [npm]
npm --version 2>&1
echo.

echo   [Python Packages]
python -c "import fastapi; print('  fastapi:        OK')" 2>&1
python -c "import uvicorn; print('  uvicorn:        OK')" 2>&1
python -c "import webview;  print('  pywebview:      OK')" 2>&1
python -c "import sse_starlette; print('  sse_starlette:  OK')" 2>&1
python -c "import multipart; print('  python-multipart: OK')" 2>&1
python -c "import pydantic_settings; print('  pydantic_settings: OK')" 2>&1
echo.

echo   [Backend Import Test]
cd /d "%APP_DIR%"
python -c "import api_main; print('  api_main:       OK')" 2>&1
echo.

echo   [Frontend]
if exist "%WEB_DIR%\node_modules" (
    echo   node_modules:   EXISTS
) else (
    echo   node_modules:   MISSING - run npm install first
)
if exist "%WEB_DIR%\node_modules\vite" (
    echo   vite:           INSTALLED
) else (
    echo   vite:           MISSING
)
echo.

echo   [Port Check]
powershell -Command "if (Get-NetTCPConnection -LocalPort 18080 -ErrorAction SilentlyContinue) { Write-Host '  Port 18080:    IN USE' } else { Write-Host '  Port 18080:    FREE' }" 2>&1
powershell -Command "if (Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue) { Write-Host '  Port 5173:     IN USE' } else { Write-Host '  Port 5173:     FREE' }" 2>&1
echo.

echo   ============================================
pause
goto end

REM --- Exit ---
:end
echo.
echo   Press any key to exit...
pause >nul