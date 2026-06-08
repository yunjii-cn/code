Set-Location $PSScriptRoot
Set-Location ..

Write-Output "============================================================"
Write-Output "  YUNJII-CODE - 启动桌面应用"
Write-Output "============================================================"
Write-Output ""

$scriptDir = $PSScriptRoot
$projectRoot = Get-Location
$nodejsDir = Join-Path $projectRoot "nodejs"
$bunDir = Join-Path $projectRoot "bun"
$nodeVersion = "v24.11.1"
$bunVersion = "1.1.42"
$nodeExtractPath = Join-Path $nodejsDir "node-$nodeVersion-win-x64"
$nodeExePath = Join-Path $nodeExtractPath "node.exe"
$npmPath = Join-Path $nodeExtractPath "npm.cmd"
$bunExtractPath = Join-Path $bunDir "bun-windows-x64"
$bunExePath = Join-Path $bunExtractPath "bun.exe"

function Find-Bun {
    $paths = @()
    
    if (Test-Path $bunExePath) {
        $paths += $bunExePath
    }
    
    $paths += @(
        "bun",
        "$env:APPDATA\bun\bun.exe"
    )
    
    foreach ($path in $paths) {
        try {
            if (Test-Path $path) {
                return $path
            }
            $cmd = Get-Command $path -ErrorAction SilentlyContinue
            if ($cmd) {
                return $cmd.Source
            }
        } catch {
            continue
        }
    }
    return $null
}

function Find-Npm {
    $paths = @()
    
    if (Test-Path $npmPath) {
        $paths += $npmPath
    }
    
    $paths += @(
        "npm",
        "$env:APPDATA\npm\npm.cmd",
        "C:\Program Files\nodejs\npm.cmd",
        "C:\Program Files (x86)\nodejs\npm.cmd"
    )
    
    foreach ($path in $paths) {
        try {
            if (Test-Path $path) {
                return $path
            }
            $cmd = Get-Command $path -ErrorAction SilentlyContinue
            if ($cmd) {
                return $cmd.Source
            }
        } catch {
            continue
        }
    }
    return $null
}

if (Test-Path "package.json") {
    Write-Output "🚀 启动应用..."
    
    $foundBunPath = Find-Bun
    
    if (-not $foundBunPath) {
        Write-Output "❌ 未找到 Bun"
        Write-Output ""
        Write-Output "请运行部署维护功能安装便携版 Bun"
        Write-Output "或手动下载 Bun 并安装"
        Write-Output "下载地址：https://bun.sh/"
        Write-Output ""
        Write-Output "应用已退出"
        exit 1
    }
    
    Write-Output "✅ 使用 Bun: $foundBunPath"
    
    # 把便携版 Node.js 和 Bun 都添加到 PATH
    if (Test-Path $nodeExtractPath) {
        Write-Output "✅ 使用便携版 Node.js: $nodeExtractPath"
        $env:PATH = "$nodeExtractPath;$env:PATH"
    }
    
    if (Test-Path $bunExtractPath) {
        Write-Output "✅ 使用便携版 Bun: $bunExtractPath"
        $env:PATH = "$bunExtractPath;$env:PATH"
    }
    
    if (-not (Test-Path "node_modules")) {
        Write-Output "📦 首次运行，正在安装依赖..."
        
        Write-Output "   配置 Bun 国内镜像..."
        & $foundBunPath config set registry https://registry.npmmirror.com
        
        & $foundBunPath install
        if ($LASTEXITCODE -ne 0) {
            Write-Output "❌ 依赖安装失败"
            Write-Output ""
            Write-Output "应用已退出"
            exit 1
        }
        Write-Output "✅ 依赖安装完成"
    }
    
    Write-Output ""
    Write-Output "💻 启动桌面版..."
    
    # 先检查是否需要构建
    $desktopDistPath = Join-Path $projectRoot "desktop\dist"
    if (-not (Test-Path $desktopDistPath)) {
        Write-Output "📦 首次运行，正在构建前端..."
        & $foundBunPath run desktop:build
    }
    
    $foundNpmPath = Find-Npm
    $electronPath = Join-Path $projectRoot "node_modules\.bin\electron.cmd"
    $electronJsPath = Join-Path $projectRoot "node_modules\electron\cli.js"
    $desktopMainPath = Join-Path $projectRoot "desktop\main.cjs"
    
    if (Test-Path $electronPath) {
        Write-Output "✅ 启动 Electron..."
        & $electronPath $desktopMainPath
    } elseif (Test-Path $electronJsPath -and $foundNpmPath) {
        Write-Output "✅ 启动 Electron..."
        & $foundNpmPath run desktop
    } else {
        Write-Output "📦 正在构建并启动..."
        & $foundBunPath run desktop
    }
} else {
    Write-Output "❌ 未找到 package.json 文件"
}

Write-Output ""
Write-Output "应用已退出"
