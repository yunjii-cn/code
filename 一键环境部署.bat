@echo off
chcp 65001 >nul
title 一键环境部署 (Python + uv + Node + Bun)
color 0D
cd /d "%~dp0"

echo.
echo  ===============================================================
echo   🛠️  一键环境部署 (便携版，不污染全局)
echo  ===============================================================
echo.

if not exist "1.PC\app\uv\uv.exe" (
    echo  [1/4] 安装 uv (Python 包管理器)...
    mkdir "1.PC\app\uv" 2>nul
    powershell -Command "& {Invoke-WebRequest -Uri 'https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-pc-windows-msvc.zip' -OutFile 'uv.zip'; Expand-Archive 'uv.zip' -DestinationPath '1.PC\app\uv' -Force; Remove-Item 'uv.zip'}"
    echo  ✅ uv 安装完成
) else (
    echo  ✅ uv 已安装
)

if not exist "1.PC\app\python\python.exe" (
    echo  [2/4] 安装便携版 Python 3.12...
    powershell -Command "& {Invoke-WebRequest -Uri 'https://www.python.org/ftp/python/3.12.7/python-3.12.7-embed-amd64.zip' -OutFile 'python.zip'; Expand-Archive 'python.zip' -DestinationPath '1.PC\app\python' -Force; Remove-Item 'python.zip'}"
    echo  ✅ Python 安装完成
) else (
    echo  ✅ Python 已安装
)

if not exist "1.PC\app\nodejs\node.exe" (
    echo  [3/4] 安装便携版 Node.js 20...
    powershell -Command "& {Invoke-WebRequest -Uri 'https://nodejs.org/dist/v20.18.0/node-v20.18.0-win-x64.zip' -OutFile 'node.zip'; Expand-Archive 'node.zip' -DestinationPath '1.PC\app\nodejs' -Force; Remove-Item 'node.zip'}"
    echo  ✅ Node.js 安装完成
) else (
    echo  ✅ Node.js 已安装
)

if not exist "1.PC\app\bun\bun.exe" (
    echo  [4/4] 安装便携版 Bun...
    powershell -Command "& {Invoke-WebRequest -Uri 'https://github.com/oven-sh/bun/releases/latest/download/bun-windows-x64.zip' -OutFile 'bun.zip'; Expand-Archive 'bun.zip' -DestinationPath '1.PC\app\bun' -Force; Remove-Item 'bun.zip'}"
    echo  ✅ Bun 安装完成
) else (
    echo  ✅ Bun 已安装
)

echo.
echo  ===============================================================
echo  ✅ 所有便携环境就绪 (全部在 1.PC/app/，不污染系统)
echo  ===============================================================
echo.
pause
