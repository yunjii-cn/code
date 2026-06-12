Set-Location $PSScriptRoot
Set-Location ..

Write-Output "============================================================"
Write-Output "  YUNJII-CODE - 环境检测与安装"
Write-Output "============================================================"
Write-Output ""

$scriptDir = $PSScriptRoot
$projectRoot = Get-Location
$nodejsDir = Join-Path $projectRoot "nodejs"
$bunDir = Join-Path $projectRoot "bun"
$nodeVersion = "v24.11.1"
$nodeDistUrl = "https://nodejs.org/dist/$nodeVersion/node-$nodeVersion-win-x64.zip"
$nodeZipPath = Join-Path $nodejsDir "node-$nodeVersion-win-x64.zip"
$nodeExtractPath = Join-Path $nodejsDir "node-$nodeVersion-win-x64"
$nodeExePath = Join-Path $nodeExtractPath "node.exe"
$bunVersion = "1.1.42"
$bunDistUrl = "https://github.com/oven-sh/bun/releases/download/bun-v$bunVersion/bun-windows-x64.zip"
$bunZipPath = Join-Path $bunDir "bun-windows-x64.zip"
$bunExtractPath = Join-Path $bunDir "bun-windows-x64"
$bunExePath = Join-Path $bunExtractPath "bun.exe"
$installStatusFile = Join-Path $projectRoot "env_install_status.json"

$all_ok = $true

function Get-InstallStatus {
    if (Test-Path $installStatusFile) {
        try {
            return Get-Content $installStatusFile -Raw | ConvertFrom-Json
        } catch {
            return @{}
        }
    }
    return @{}
}

function Save-InstallStatus {
    param([hashtable]$status)
    $status | ConvertTo-Json -Depth 10 | Set-Content $installStatusFile -Force
}

$installStatus = Get-InstallStatus

Write-Output "📂 工作目录: $projectRoot"
Write-Output ""

Write-Output "============================================================"
Write-Output "  步骤 1: 检查/安装便携版 Node.js"
Write-Output "============================================================"
Write-Output ""

# 检查 Node.js 是否已安装且版本正确
$nodeInstalled = $false
if (Test-Path $nodeExePath) {
    try {
        $installedVersion = & $nodeExePath --version 2>&1
        if ($installedVersion -eq $nodeVersion) {
            $nodeInstalled = $true
            Write-Output "✅ Node.js $nodeVersion 已安装，跳过下载"
        } else {
            Write-Output "⚠️ 已安装的 Node.js 版本 ($installedVersion) 与要求 ($nodeVersion) 不一致"
            Write-Output "   将重新安装正确版本..."
        }
    } catch {
        Write-Output "⚠️ 无法验证已安装的 Node.js 版本"
    }
}

if (-not $nodeInstalled) {
    if (-not (Test-Path $nodejsDir)) {
        Write-Output "📦 创建 nodejs 目录..."
        New-Item -ItemType Directory -Path $nodejsDir -Force | Out-Null
    }
    
    # 如果 zip 文件已存在且完整，直接解压
    if (Test-Path $nodeZipPath) {
        $zipSize = (Get-Item $nodeZipPath).Length
        Write-Output "📦 检测到已下载的 Node.js zip 文件 ($zipSize 字节)"
        Write-Output "   将直接使用，跳过下载..."
    } else {
        Write-Output "📦 便携版 Node.js 未找到，开始下载..."
        Write-Output "   版本: $nodeVersion"
        Write-Output "   下载地址: $nodeDistUrl"
        Write-Output ""
        
        try {
            Write-Output "⏳ 正在下载 Node.js..."
            Invoke-WebRequest -Uri $nodeDistUrl -OutFile $nodeZipPath -UseBasicParsing
            Write-Output "✅ Node.js 下载完成"
        } catch {
            Write-Output "❌ Node.js 下载失败: $_"
            Write-Output ""
            Write-Output "请手动下载 Node.js 并解压到 nodejs 目录"
            Write-Output "下载地址: https://nodejs.org/"
            $all_ok = $false
        }
    }
    
    if (Test-Path $nodeZipPath) {
        Write-Output ""
        Write-Output "📦 正在解压 Node.js..."
        try {
            # 先删除旧版本（如果存在）
            if (Test-Path $nodeExtractPath) {
                Write-Output "   删除旧版本目录..."
                Remove-Item $nodeExtractPath -Recurse -Force
            }
            
            Expand-Archive -Path $nodeZipPath -DestinationPath $nodejsDir -Force
            Write-Output "✅ Node.js 解压完成"
            
            Write-Output ""
            Write-Output "🧹 清理下载文件..."
            Remove-Item -Path $nodeZipPath -Force
            Write-Output "✅ 清理完成"
            
            $installStatus.nodejs = @{
                version = $nodeVersion
                installedAt = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
            }
            Save-InstallStatus $installStatus
        } catch {
            Write-Output "❌ Node.js 解压失败: $_"
            $all_ok = $false
        }
    }
}

Write-Output ""
Write-Output "📋 验证 Node.js..."
if (Test-Path $nodeExePath) {
    $node_version = & $nodeExePath --version 2>&1
    Write-Output "✅ Node.js 版本: $node_version"
    
    $npmPath = Join-Path $nodeExtractPath "npm.cmd"
    if (Test-Path $npmPath) {
        $npm_version = & $npmPath --version 2>&1
        Write-Output "✅ npm 版本: $npm_version"
    } else {
        Write-Output "❌ npm 未找到"
        $all_ok = $false
    }
} else {
    Write-Output "❌ Node.js 验证失败"
    $all_ok = $false
}

Write-Output ""
Write-Output "============================================================"
Write-Output "  步骤 2: 检查/安装便携版 Bun"
Write-Output "============================================================"
Write-Output ""

# 检查 Bun 是否已安装且版本正确
$bunInstalled = $false
if (Test-Path $bunExePath) {
    try {
        $installedVersion = & $bunExePath --version 2>&1
        if ($installedVersion -like "*$bunVersion*") {
            $bunInstalled = $true
            Write-Output "✅ Bun $bunVersion 已安装，跳过下载"
        } else {
            Write-Output "⚠️ 已安装的 Bun 版本 ($installedVersion) 与要求 ($bunVersion) 不一致"
            Write-Output "   将重新安装正确版本..."
        }
    } catch {
        Write-Output "⚠️ 无法验证已安装的 Bun 版本"
    }
}

if (-not $bunInstalled) {
    if (-not (Test-Path $bunDir)) {
        Write-Output "📦 创建 bun 目录..."
        New-Item -ItemType Directory -Path $bunDir -Force | Out-Null
    }
    
    # 如果 zip 文件已存在且完整，直接解压
    if (Test-Path $bunZipPath) {
        $zipSize = (Get-Item $bunZipPath).Length
        Write-Output "📦 检测到已下载的 Bun zip 文件 ($zipSize 字节)"
        Write-Output "   将直接使用，跳过下载..."
    } else {
        Write-Output "📦 便携版 Bun 未找到，开始下载..."
        Write-Output "   版本: $bunVersion"
        Write-Output "   下载地址: $bunDistUrl"
        Write-Output ""
        
        try {
            Write-Output "⏳ 正在下载 Bun..."
            Invoke-WebRequest -Uri $bunDistUrl -OutFile $bunZipPath -UseBasicParsing
            Write-Output "✅ Bun 下载完成"
        } catch {
            Write-Output "❌ Bun 下载失败: $_"
            Write-Output ""
            Write-Output "请手动下载 Bun 并解压到 bun 目录"
            Write-Output "下载地址: https://bun.sh/"
            $all_ok = $false
        }
    }
    
    if (Test-Path $bunZipPath) {
        Write-Output ""
        Write-Output "📦 正在解压 Bun..."
        try {
            # 先删除旧版本（如果存在）
            if (Test-Path $bunExtractPath) {
                Write-Output "   删除旧版本目录..."
                Remove-Item $bunExtractPath -Recurse -Force
            }
            
            Expand-Archive -Path $bunZipPath -DestinationPath $bunDir -Force
            Write-Output "✅ Bun 解压完成"
            
            Write-Output ""
            Write-Output "🧹 清理下载文件..."
            Remove-Item -Path $bunZipPath -Force
            Write-Output "✅ 清理完成"
            
            $installStatus.bun = @{
                version = $bunVersion
                installedAt = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
            }
            Save-InstallStatus $installStatus
        } catch {
            Write-Output "❌ Bun 解压失败: $_"
            $all_ok = $false
        }
    }
}

Write-Output ""
Write-Output "📋 验证 Bun..."
if (Test-Path $bunExePath) {
    $bun_version = & $bunExePath --version 2>&1
    Write-Output "✅ Bun 版本: $bun_version"
} else {
    Write-Output "❌ Bun 验证失败"
    $all_ok = $false
}

Write-Output ""
Write-Output "============================================================"
Write-Output "  步骤 3: 检查/安装项目依赖"
Write-Output "============================================================"
Write-Output ""

Write-Output "📋 检测项目依赖..."
if (Test-Path "package.json") {
    # 检查 tsx 是否已安装（Node.js 启动 TypeScript 文件所必需）
    $tsxPath = Join-Path $projectRoot "node_modules\tsx\dist\loader.mjs"
    $needInstall = $false
    
    if (-not (Test-Path "node_modules")) {
        Write-Output "⚠️ node_modules 目录不存在"
        $needInstall = $true
    } elseif (-not (Test-Path $tsxPath)) {
        Write-Output "⚠️ tsx 包未安装（Node.js 运行 TypeScript 必需）"
        $needInstall = $true
    } else {
        Write-Output "✅ node_modules 目录存在"
        Write-Output "✅ tsx 包已安装"
    }
    
    if ($needInstall) {
        $npmCmd = Join-Path $nodeExtractPath "npm.cmd"
        if (Test-Path $npmCmd) {
            Write-Output "📦 正在安装项目依赖..."
            & $npmCmd install
            if ($LASTEXITCODE -eq 0) {
                Write-Output "✅ 项目依赖安装完成"
            } else {
                Write-Output "❌ 项目依赖安装失败"
                $all_ok = $false
            }
        } else {
            Write-Output "⚠️ npm 未找到，跳过依赖安装"
        }
    }
}

Write-Output ""
Write-Output "============================================================"
if ($all_ok) {
    Write-Output "✅ 环境安装完成！"
    Write-Output ""
    Write-Output "下一步："
    Write-Output "1. 点击启动按钮运行服务"
    Write-Output "2. 或运行 scripts/2、start-app.ps1"
} else {
    Write-Output "⚠️ 部分环境未就绪"
}
Write-Output "============================================================"
Write-Output ""
