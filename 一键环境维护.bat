@echo off
setlocal enabledelayedexpansion
title YunJi Env Maintainer

set "ROOT=%~dp0"
set "ROOT=%ROOT:~0,-1%"
set "APP_DIR=%ROOT%\3.dev\api"
set "WEB_DIR=%ROOT%\3.dev\ui"

:: Portable paths
set "UV_EXE=%ROOT%\1.PC\app\uv\uv.exe"
set "PY_EXE=%ROOT%\1.PC\app\python\python.exe"
set "NODE_EXE=%ROOT%\1.PC\app\nodejs\node.exe"
set "BUN_EXE=%ROOT%\1.PC\app\bun\bun.exe"

:menu
cls
echo.
echo ============================================================
echo   YunJi v2.0 - One-Click Env Maintainer
echo ============================================================
echo.

:: Status display
echo   --- Status ---
echo.

:: UV
if exist "%UV_EXE%" (
    for %%F in ("%UV_EXE%") do echo   [OK]  uv         ^(%%~zF bytes^)
) else (
    echo   [  ]  uv         not installed
)

:: Python (portable)
if exist "%PY_EXE%" (
    echo   [OK]  Python     ^(portable, 1.PC\app\python^)
) else (
    echo   [  ]  Python     ^(portable^) not installed
)

:: System Python
where python >nul 2>&1 && (
    for /f "tokens=*" %%v in ('python --version 2^>^&1') do (
        set "SYS_PY=%%v"
        for /f "tokens=*" %%p in ('where python 2^>^&1') do set "SYS_PY_PATH=%%p"
    )
    echo   [OK]  System     !SYS_PY!  ^(!SYS_PY_PATH!^)
) || (
    echo   [  ]  System     Python not in PATH
)

:: Node.js (portable)
if exist "%NODE_EXE%" (
    echo   [OK]  Node.js    ^(portable, 1.PC\app\nodejs^)
) else (
    echo   [  ]  Node.js    ^(portable^) not installed
)

:: System Node
where node >nul 2>&1 && (
    for /f "tokens=*" %%v in ('node --version 2^>^&1') do set "SYS_NODE=%%v"
    echo   [OK]  System     Node.js !SYS_NODE!
) || (
    echo   [  ]  System     Node.js not in PATH
)

:: Bun
if exist "%BUN_EXE%" (
    echo   [OK]  Bun        ^(portable^)
) else (
    echo   [  ]  Bun        not installed
)

:: Frontend node_modules
if exist "%WEB_DIR%\node_modules" (
    echo   [OK]  Frontend   node_modules installed
) else (
    echo   [  ]  Frontend   node_modules missing
)

:: Backend deps
if exist "%APP_DIR%\api_main.py" (
    echo   [OK]  Backend    3.dev\api\api_main.py
) else (
    echo   [  ]  Backend    api_main.py missing!
)

:: Project dirs
echo.
echo   --- Project Dirs ---
echo.
echo   Root:      %ROOT%
echo   Backend:   %APP_DIR%
echo   Frontend:  %WEB_DIR%

echo.
echo ============================================================
echo.
echo   [1] Deploy ALL  (uv + Python + Node + Bun)
echo   [2] Deploy uv only
echo   [3] Deploy Python portable only
echo   [4] Deploy Node.js portable only
echo   [5] Deploy Bun only
echo   [6] Install frontend npm packages
echo   [7] Install backend pip packages
echo   [8] Force reinstall ALL (clean + redeploy)
echo   [9] Quick fix (reinstall frontend + backend deps only)
echo.
echo   [0] Exit
echo.
set "MODE="
set /p "MODE=   Select [0-9]: "
echo.

if "%MODE%"=="0" goto end
if "%MODE%"=="1" goto deploy_all
if "%MODE%"=="2" goto deploy_uv
if "%MODE%"=="3" goto deploy_python
if "%MODE%"=="4" goto deploy_node
if "%MODE%"=="5" goto deploy_bun
if "%MODE%"=="6" goto fix_frontend
if "%MODE%"=="7" goto fix_backend
if "%MODE%"=="8" goto force_reinstall
if "%MODE%"=="9" goto quick_fix
echo   Invalid option
pause
goto menu

:deploy_all
call :deploy_uv_func
call :deploy_python_func
call :deploy_node_func
call :deploy_bun_func
call :fix_frontend
call :fix_backend
echo.
echo   [DONE] All components deployed.
pause
goto menu

:deploy_uv
call :deploy_uv_func
pause
goto menu

:deploy_python
call :deploy_python_func
pause
goto menu

:deploy_node
call :deploy_node_func
pause
goto menu

:deploy_bun
call :deploy_bun_func
pause
goto menu

:fix_frontend
if not exist "%WEB_DIR%\" (
    echo   [ERROR] Frontend dir not found: %WEB_DIR%
    echo   Make sure 3.dev\ui\ exists.
    pause
    goto menu
)
pushd "%WEB_DIR%" 2>nul
echo   Installing npm packages...
call npm install 2>nul
if !errorlevel! neq 0 (
    echo   [WARN] npm install failed, trying with --legacy-peer-deps...
    call npm install --legacy-peer-deps 2>nul
)
popd
echo   [OK] Frontend deps installed.
pause
goto menu

:fix_backend
if not exist "%APP_DIR%\" (
    echo   [ERROR] Backend dir not found: %APP_DIR%
    echo   Make sure 3.dev\api\ exists.
    pause
    goto menu
)
pushd "%APP_DIR%" 2>nul
where python >nul 2>&1 && (
    echo   Installing Python deps via system pip...
    pip install fastapi uvicorn python-multipart sse-starlette pydantic-settings --quiet 2>nul
)
if exist "%UV_EXE%" (
    echo   Installing Python deps via uv...
    "%UV_EXE%" pip install fastapi uvicorn python-multipart sse-starlette pydantic-settings --quiet 2>nul
)
popd
echo   [OK] Backend deps installed.
pause
goto menu

:force_reinstall
echo   *** Force Reinstall ***
echo   This will delete and reinstall all portable tools.
echo.
set /p "CONFIRM=   Type YES to confirm: "
if not "%CONFIRM%"=="YES" (
    echo   Cancelled.
    pause
    goto menu
)
echo.
if exist "%ROOT%\1.PC\app\uv" rd /s /q "%ROOT%\1.PC\app\uv"
if exist "%ROOT%\1.PC\app\python" rd /s /q "%ROOT%\1.PC\app\python"
if exist "%ROOT%\1.PC\app\nodejs" rd /s /q "%ROOT%\1.PC\app\nodejs"
if exist "%ROOT%\1.PC\app\bun" rd /s /q "%ROOT%\1.PC\app\bun"
if exist "%WEB_DIR%\node_modules" rd /s /q "%WEB_DIR%\node_modules"
call :deploy_all
pause
goto menu

:quick_fix
echo   Quick fix: refreshing frontend + backend deps...
call :fix_frontend
call :fix_backend
echo   [DONE] Quick fix complete.
pause
goto menu

:: ===== Deploy Functions =====

:deploy_uv_func
if exist "%UV_EXE%" (
    echo   [SKIP] uv already installed.
    exit /b 0
)
echo   Downloading uv...
mkdir "%ROOT%\1.PC\app\uv" 2>nul
powershell -NoProfile -Command "& {try{Invoke-WebRequest -Uri 'https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-pc-windows-msvc.zip' -OutFile 'uv.zip'; Expand-Archive 'uv.zip' -DestinationPath '%ROOT%\1.PC\app\uv' -Force; Remove-Item 'uv.zip'; Write-Host '  [OK] uv installed'}catch{Write-Host '  [FAIL] uv download error'}}" 2>nul
exit /b 0

:deploy_python_func
if exist "%PY_EXE%" (
    echo   [SKIP] Python portable already installed.
    exit /b 0
)
echo   Downloading Python 3.12 portable...
mkdir "%ROOT%\1.PC\app\python" 2>nul
powershell -NoProfile -Command "& {try{Invoke-WebRequest -Uri 'https://www.python.org/ftp/python/3.12.7/python-3.12.7-embed-amd64.zip' -OutFile 'python.zip'; Expand-Archive 'python.zip' -DestinationPath '%ROOT%\1.PC\app\python' -Force; Remove-Item 'python.zip'; Write-Host '  [OK] Python installed'}catch{Write-Host '  [FAIL] Python download error'}}" 2>nul
exit /b 0

:deploy_node_func
if exist "%NODE_EXE%" (
    echo   [SKIP] Node.js portable already installed.
    exit /b 0
)
echo   Downloading Node.js 20 portable...
mkdir "%ROOT%\1.PC\app\nodejs" 2>nul
powershell -NoProfile -Command "& {try{Invoke-WebRequest -Uri 'https://nodejs.org/dist/v20.18.0/node-v20.18.0-win-x64.zip' -OutFile 'node.zip'; Expand-Archive 'node.zip' -DestinationPath '%ROOT%\1.PC\app\nodejs' -Force; Remove-Item 'node.zip'; Write-Host '  [OK] Node.js installed'}catch{Write-Host '  [FAIL] Node.js download error'}}" 2>nul
exit /b 0

:deploy_bun_func
if exist "%BUN_EXE%" (
    echo   [SKIP] Bun already installed.
    exit /b 0
)
echo   Downloading Bun...
mkdir "%ROOT%\1.PC\app\bun" 2>nul
powershell -NoProfile -Command "& {try{Invoke-WebRequest -Uri 'https://github.com/oven-sh/bun/releases/latest/download/bun-windows-x64.zip' -OutFile 'bun.zip'; Expand-Archive 'bun.zip' -DestinationPath '%ROOT%\1.PC\app\bun' -Force; Remove-Item 'bun.zip'; Write-Host '  [OK] Bun installed'}catch{Write-Host '  [FAIL] Bun download error'}}" 2>nul
exit /b 0

:end
echo.
echo   Press any key to exit...
pause >nul
exit /b 0