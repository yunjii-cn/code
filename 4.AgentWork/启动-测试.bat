@echo off
REM ============================================================
REM   AgentWork - Test Launcher (Double-click to start Tauri dev)
REM ============================================================

title AgentWork - Test Launcher
color 0B

cd /d "%~dp0"

echo.
echo  ===============================================================
echo           AgentWork - Test Launcher
echo  ===============================================================
echo.
echo  Current dir: %CD%
echo.

timeout /t 3 /nobreak >nul

echo [1/3] Checking environment...

set RUST_PATH=%USERPROFILE%\.cargo\bin
if exist "%RUST_PATH%\cargo.exe" (
    set "PATH=%RUST_PATH%;%PATH%"
    echo [OK] Cargo found
) else (
    echo [ERROR] Cargo not found at %RUST_PATH%
    echo.
    pause
    exit /b 1
)

where node >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Node.js not found
    echo.
    pause
    exit /b 1
)
echo [OK] Node.js found

where pnpm >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    set PKG=npm
    echo [WARN] pnpm not found, using npm
) else (
    set PKG=pnpm
)
echo [OK] Package manager: %PKG%

echo.
echo [2/3] Checking project structure...

if not exist "apps\desktop\package.json" (
    echo [ERROR] apps\desktop\package.json not found
    echo.
    echo Current dir is: %CD%
    echo Expected file: %CD%\apps\desktop\package.json
    echo.
    pause
    exit /b 1
)
echo [OK] apps\desktop\package.json found

if not exist "apps\desktop\src-tauri\Cargo.toml" (
    echo [ERROR] apps\desktop\src-tauri\Cargo.toml not found
    echo.
    pause
    exit /b 1
)
echo [OK] apps\desktop\src-tauri\Cargo.toml found

echo.
echo [3/3] Checking frontend dependencies...

if not exist "apps\desktop\node_modules" (
    echo [INFO] First run, installing dependencies...
    echo.
    cd /d "%~dp0apps\desktop"
    call %PKG% install
    if %ERRORLEVEL% NEQ 0 (
        echo [ERROR] Dependency installation failed
        echo.
        pause
        exit /b 1
    )
    cd /d "%~dp0"
    echo [OK] Dependencies installed
) else (
    echo [OK] node_modules already exists
)

echo.
echo  ===============================================================
echo           Starting Tauri dev mode...
echo           Frontend: http://localhost:1420
echo           Close window or Ctrl+C to exit
echo  ===============================================================
echo.

cd /d "%~dp0apps\desktop"
call %PKG% tauri dev

echo.
echo Tauri dev exited.
pause
