Set-Location $PSScriptRoot
Set-Location ..

Write-Output "============================================================"
Write-Output "  YUNJII-CODE - 启动脚本"
Write-Output "============================================================"
Write-Output ""

Write-Output "🚀 启动 YUNJII-CODE..."
Write-Output ""

python main.py

Write-Output ""
Write-Output "软件已退出"
Read-Host "按回车键退出" | Out-Null
