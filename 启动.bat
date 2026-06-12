@echo off
chcp 65001 >nul
title 云集智能 - 总入口
color 0B
cd /d "%~dp0"
:menu
cls
echo.
echo  ===============================================================
echo           云集智能 - 3 产品线总入口
echo  ===============================================================
echo.
echo   [1] 🟢 云集桌面     (Free)     - PyQt6 桌面版 / 永久免费
echo   [2] 🟡 云集 Web     (Pro)      - 跨端版 / ¥9.9 月
echo   [3] 🔵 云集旗舰     (Business) - 全功能 / ¥99 月
echo.
echo   [4] 📖 产品线矩阵   (doc/产品线矩阵.md)
echo   [5] 🛠️ 一键环境部署
echo   [6] 🔧 环境维护
echo.
echo   [0] 退出
echo  ===============================================================
echo.
set /p choice=请选择 [0-6]: 

if "%choice%"=="1" start "" "1.PC\启动-桌面.bat" & exit /b
if "%choice%"=="2" start "" "2.WEB\启动-Web.bat" & exit /b
if "%choice%"=="3" start "" "3.dev\启动-旗舰.bat" & exit /b
if "%choice%"=="4" start notepad "doc\产品线矩阵.md" & goto menu
if "%choice%"=="5" call "一键环境部署.bat" & goto menu
if "%choice%"=="6" call "一键环境维护.bat" & goto menu
if "%choice%"=="0" exit /b

echo 无效选项
pause
goto menu