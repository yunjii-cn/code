@echo off
REM ============================================================
REM   AgentWork - Dev Launcher (v2 - stable)
REM   Pure ASCII, no for /f parsing, no chcp
REM ============================================================

title AgentWork - Dev Launcher
color 0B

cd /d "%~dp0"

echo.
echo  ===============================================================
echo           AgentWork (Yunji AI Agent Workbench)
echo  ===============================================================
echo.

REM ---------- 1. Env detection ----------
echo [1/4] Checking environment...

where cargo >nul 2>&1
if %ERRORLEVEL% NEQ 0 goto NO_CARGO

where node >nul 2>&1
if %ERRORLEVEL% NEQ 0 goto NO_NODE

where pnpm >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [WARN]  pnpm not found, will use npm
    set PKG=npm
    goto PKG_OK
)
set PKG=pnpm
goto PKG_OK

:NO_CARGO
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
) else (
    echo [ERROR] Rust/Cargo not found
    echo         Please install Rust 1.88+ from https://rustup.rs
    goto END_ERROR
)

where node >nul 2>&1
if %ERRORLEVEL% NEQ 0 goto NO_NODE

where pnpm >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    set PKG=npm
) else (
    set PKG=pnpm
)
goto PKG_OK

:NO_NODE
echo [ERROR] Node.js not found
echo         Please install Node.js 20+ from https://nodejs.org
goto END_ERROR

:PKG_OK
echo        OK cargo / node / %PKG%

echo.

REM ---------- 2. Project structure ----------
echo [2/4] Checking project structure...

if not exist "apps\desktop\package.json" (
    echo [ERROR] apps\desktop\package.json not found
    echo         Are you running from the 4.AgentWork root?
    goto END_ERROR
)
if not exist "apps\desktop\src-tauri\Cargo.toml" (
    echo [ERROR] apps\desktop\src-tauri\Cargo.toml not found
    goto END_ERROR
)
if not exist "platformkit\crates\agent-team\src\lib.rs" (
    echo [ERROR] platformkit crates not found
    goto END_ERROR
)
echo        OK apps\desktop + platformkit\crates\agent-team

echo.

REM ---------- 3. Dependencies ----------
echo [3/4] Checking dependencies...

if not exist "apps\desktop\node_modules" (
    echo [INFO]  First run, installing frontend dependencies...
    cd /d "%~dp0apps\desktop"
    call %PKG% install
    if %ERRORLEVEL% NEQ 0 (
        echo [ERROR] Frontend dependency installation failed
        cd /d "%~dp0"
        goto END_ERROR
    )
    cd /d "%~dp0"
    echo        OK Frontend dependencies installed
) else (
    echo        OK Frontend dependencies ready
)

echo.

REM ---------- 4. Menu ----------
:MENU
echo [4/4] Select startup mode
echo.
echo  ---------------------------------------------------------------
echo   [1] Desktop dev (RECOMMENDED)  - Tauri dev with HMR
echo   [2] Frontend only              - Vite dev server
echo   [3] Desktop production build   - Tauri build to .exe
echo   [4] Frontend production build  - Vite build
echo   [5] Run tests                  - cargo test + tsc
echo   [6] Run clippy                 - cargo clippy
echo   [0] Exit
echo  ---------------------------------------------------------------
echo.
set /p CHOICE=Choose [0-6]:

if "%CHOICE%"=="1" goto MODE_TAURI_DEV
if "%CHOICE%"=="2" goto MODE_VITE_DEV
if "%CHOICE%"=="3" goto MODE_TAURI_BUILD
if "%CHOICE%"=="4" goto MODE_VITE_BUILD
if "%CHOICE%"=="5" goto MODE_TEST
if "%CHOICE%"=="6" goto MODE_CLIPPY
if "%CHOICE%"=="0" goto END_OK

echo Invalid choice
goto MENU

REM ============================================================
REM  Mode 1: Tauri dev
REM ============================================================
:MODE_TAURI_DEV
echo.
echo [INFO] Starting Tauri dev mode...
echo        First compile takes 1-2 minutes
echo        Frontend: http://localhost:1420
echo        Close window or Ctrl+C to exit
echo.
cd /d "%~dp0apps\desktop"
call %PKG% tauri dev
goto END_OK

REM ============================================================
REM  Mode 2: Vite dev
REM ============================================================
:MODE_VITE_DEV
echo.
echo [INFO] Starting Vite dev server...
echo        http://localhost:1420
echo        Ctrl+C to exit
echo.
cd /d "%~dp0apps\desktop"
call %PKG% dev
goto END_OK

REM ============================================================
REM  Mode 3: Tauri build
REM ============================================================
:MODE_TAURI_BUILD
echo.
echo [INFO] Building Tauri production package...
echo        This takes 3-5 minutes
echo.
cd /d "%~dp0apps\desktop"
call %PKG% tauri build
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Build failed
    pause
)
echo.
echo [OK] Build complete!
echo     Output: apps\desktop\src-tauri\target\release\bundle\
pause
goto END_OK

REM ============================================================
REM  Mode 4: Vite build
REM ============================================================
:MODE_VITE_BUILD
echo.
echo [INFO] Building Vite frontend...
echo.
cd /d "%~dp0apps\desktop"
call %PKG% build
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Build failed
    pause
)
echo.
echo [OK] Build complete! Output: apps\desktop\dist\
pause
goto END_OK

REM ============================================================
REM  Mode 5: Run tests
REM ============================================================
:MODE_TEST
echo.
echo [INFO] Running Rust unit tests...
echo.
cd /d "%~dp0"
call cargo test -p agent-team
echo.
echo [INFO] Running TypeScript type check...
cd /d "%~dp0apps\desktop"
call npx tsc --noEmit
echo.
echo [OK] Tests complete
pause
goto MENU

REM ============================================================
REM  Mode 6: Run clippy
REM ============================================================
:MODE_CLIPPY
echo.
echo [INFO] Running cargo clippy...
echo.
cd /d "%~dp0"
call cargo clippy -p agent-team -p aw-desktop --all-targets
echo.
echo [OK] Clippy check complete
pause
goto MENU

REM ============================================================
REM  End
REM ============================================================
:END_ERROR
echo.
echo Press any key to close...
pause >nul
exit /b 1

:END_OK
exit /b 0
