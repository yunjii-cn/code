# 门禁分类器负向对照（Windows 常驻用例），语义与 selftest.sh 一致。
#
# 用法：powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-gate/selftest.ps1
param(
    [string]$Gate = "",
    [string]$QuarantineFile = ""
)

$ErrorActionPreference = "Continue"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $Gate) { $Gate = Join-Path $ScriptDir "gate.ps1" }
if (-not $QuarantineFile) { $QuarantineFile = Join-Path $ScriptDir "quarantine.txt" }

$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("aw-gate-selftest-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $Tmp -Force | Out-Null

$pass = 0
$failed = 0

function Invoke-GateCase {
    param(
        [string]$Name,
        [int]$ExpectedRc,
        [string]$ExpectedText,
        [string]$LogFile,
        [string]$PathPrefix = ""
    )
    $out = Join-Path $Tmp ("out-" + [guid]::NewGuid().ToString("N") + ".txt")
    $oldPath = $env:PATH
    if ($PathPrefix) { $env:PATH = $PathPrefix + ";" + $oldPath }
    try {
        if ($LogFile) {
            & powershell -NoProfile -ExecutionPolicy Bypass -File $Gate -LogFile $LogFile -SkipSelfTest 2>&1 | Out-File -FilePath $out -Encoding utf8
        }
        else {
            & powershell -NoProfile -ExecutionPolicy Bypass -File $Gate -SkipSelfTest 2>&1 | Out-File -FilePath $out -Encoding utf8
        }
        $rc = $LASTEXITCODE
    }
    finally {
        $env:PATH = $oldPath
    }
    $text = Get-Content -Raw -Path $out -ErrorAction SilentlyContinue
    $ok = ($rc -eq $ExpectedRc) -and ($text -and $text.Contains($ExpectedText))
    if ($ok) {
        Write-Host "[SELFTEST][PASS] $Name (rc=$rc)"
        $script:pass++
    }
    else {
        Write-Host "[SELFTEST][FAIL] $Name 期望 rc=$ExpectedRc 且包含「$ExpectedText」，实际 rc=$rc" -ForegroundColor Red
        ($text -split "`n" | Select-Object -First 20) | ForEach-Object { Write-Host "    $_" }
        $script:failed++
    }
}

$qname = $null
foreach ($line in Get-Content -Path $QuarantineFile) {
    if ($line -match '^\s*#' -or $line.Trim() -eq '') { continue }
    $qname = $line.Split('|')[0].Trim()
    if ($qname) { break }
}
if (-not $qname) {
    Write-Host "[SELFTEST][FAIL] quarantine.txt 中取不到条目名，无法做 WARN 对照" -ForegroundColor Red
    exit 2
}

# 1. 日志不存在
Invoke-GateCase -Name "missing-log" -ExpectedRc 2 -ExpectedText "[GATE] fail=1 — 日志不存在" `
    -LogFile (Join-Path $Tmp "does-not-exist.log")

# 2. 编译失败：有输出、零结果行（假 cargo，退出 101）
$fakeCompile = Join-Path $Tmp "bin-compile-fail"
New-Item -ItemType Directory -Path $fakeCompile -Force | Out-Null
Set-Content -Path (Join-Path $fakeCompile "cargo.cmd") -Encoding ascii -Value @(
    "@echo off",
    "echo error[E0425]: cannot find value",
    "exit /b 101"
)
Invoke-GateCase -Name "compile-failure-no-results" -ExpectedRc 2 `
    -ExpectedText "[GATE] fail=1 — 未解析到任何测试结果行" -PathPrefix $fakeCompile

# 3. 非 quarantine 失败
$realFailed = Join-Path $Tmp "real-failed.log"
Set-Content -Path $realFailed -Encoding utf8 -Value "test some::real_failure ... FAILED"
Invoke-GateCase -Name "non-quarantined-failure" -ExpectedRc 2 `
    -ExpectedText "[FAIL] some::real_failure" -LogFile $realFailed

# 4. quarantine 失败 → 警告但不是通过
$qFailed = Join-Path $Tmp "quarantined-failed.log"
Set-Content -Path $qFailed -Encoding utf8 -Value ("test " + $qname + " ... FAILED")
Invoke-GateCase -Name "quarantined-failure-is-warn" -ExpectedRc 0 `
    -ExpectedText ("[WARN] quarantined " + $qname + " result=FAILED") -LogFile $qFailed

# 5. quarantine 通过 → 仍不记 PASS
$qOk = Join-Path $Tmp "quarantined-ok.log"
Set-Content -Path $qOk -Encoding utf8 -Value ("test " + $qname + " ... ok")
Invoke-GateCase -Name "quarantined-ok-is-warn" -ExpectedRc 0 `
    -ExpectedText ("[WARN] quarantined " + $qname + " result=ok") -LogFile $qOk

# 6. cargo 非零但没有 FAILED 行：有结果行也不能判绿
$fakePartial = Join-Path $Tmp "bin-partial"
New-Item -ItemType Directory -Path $fakePartial -Force | Out-Null
Set-Content -Path (Join-Path $fakePartial "cargo.cmd") -Encoding ascii -Value @(
    "@echo off",
    "echo test some::ok ... ok",
    "exit /b 1"
)
Invoke-GateCase -Name "cargo-nonzero-without-failed" -ExpectedRc 2 `
    -ExpectedText "[GATE] fail=1 — cargo 退出码" -PathPrefix $fakePartial

# 7. 干净日志
$clean = Join-Path $Tmp "clean.log"
Set-Content -Path $clean -Encoding utf8 -Value @("test some::one ... ok", "test another::two ... ok")
Invoke-GateCase -Name "clean-log-is-green" -ExpectedRc 0 -ExpectedText "[GATE] warn=0 fail=0" -LogFile $clean

Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
Write-Host "[SELFTEST] pass=$pass fail=$failed"
if ($failed -ne 0) { exit 2 }
