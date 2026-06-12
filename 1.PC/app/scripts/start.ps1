Set-Location $PSScriptRoot
Set-Location ..

$projectRoot = Get-Location

Write-Output "============================================================"
Write-Output "  YUNJII-CODE - 启动脚本"
Write-Output "============================================================"
Write-Output ""

Write-Output "📂 工作目录: $projectRoot"
Write-Output ""

python main.py

Write-Output ""
Write-Output "软件已退出"
Read-Host "按回车键退出" | Out-Null
