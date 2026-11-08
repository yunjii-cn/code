# 默认门禁（Windows）：跑 aw-mcp-bridge 测试，或对已有日志做分类。
#
# 与 scripts/test-gate/quarantine.txt 配合：清单里的用例按 WARN 呈现，
# 既不算通过也不阻断；其余用例失败一律 FAIL 并以 2 退出。
#
# 用法：
#   powershell -File scripts/test-gate/gate.ps1
#   powershell -File scripts/test-gate/gate.ps1 -TestThreads 8
#   powershell -File scripts/test-gate/gate.ps1 -LogFile <file>
#   powershell -File scripts/test-gate/gate.ps1 -RaceRepeat 12   # 可选：竞态用例连跑 12 次（默认 1 = 不连跑）
#   powershell -File scripts/test-gate/gate.ps1 -SkipSelfTest   # 仅供 selftest.ps1 内部调用
param(
    [string]$LogFile = "",
    [int]$TestThreads = 0,
    [string]$Package = "aw-mcp-bridge",
    # 清单路径可覆盖：selftest.ps1 用固定 fixture，避免负向对照依赖生产清单是否非空。
    [string]$QuarantineFile = "",
    # 竞态用例连跑次数：1 = 默认（主套件里已跑 1 次，不额外耗时）；>1 才连跑。
    [int]$RaceRepeat = 1,
    [switch]$SkipSelfTest
)

$ErrorActionPreference = "Continue"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "../..")).Path
$LogOnly = $LogFile -ne ""
if ($RaceRepeat -lt 1) {
    Write-Host "[GATE] fail=1 — -RaceRepeat 必须 >= 1（收到 $RaceRepeat）" -ForegroundColor Red
    exit 2
}
if (-not $QuarantineFile) { $QuarantineFile = Join-Path $ScriptDir "quarantine.txt" }

# v1.50 常驻负向对照：门禁自己先过一遍「无证据不判绿」的用例，否则本次结果不可信。
if (-not $SkipSelfTest) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "selftest.ps1") -Gate $PSCommandPath
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[GATE] fail=1 — 门禁分类器负向对照未通过；先修 gate.ps1 / selftest.ps1，本次测试结论不可信" -ForegroundColor Red
        exit 2
    }
}

$cargoRc = 0
if (-not $LogFile) {
    $LogFile = Join-Path $env:TEMP ("aw-gate-" + [guid]::NewGuid().ToString("N") + ".log")
    Write-Host "[INFO] 运行测试（日志 $LogFile）"
    $cargoArgs = @("test", "--offline", "-p", $Package, "--", "--nocapture")
    if ($TestThreads -gt 0) { $cargoArgs += @("--test-threads=$TestThreads") }
    Push-Location $RepoRoot
    try {
        & cargo @cargoArgs 2>&1 | Tee-Object -FilePath $LogFile
        $cargoRc = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }
}

# 日志不存在/不可读时严禁判绿：没有证据就不算通过
if (-not (Test-Path -Path $LogFile -PathType Leaf)) {
    Write-Host "[GATE] fail=1 — 日志不存在或不可读：$LogFile" -ForegroundColor Red
    exit 2
}

$results = @(Get-Content -Path $LogFile | Where-Object { $_ -match '^test .+ \.\.\. (ok|FAILED|ignored)$' })

# 一条结果行都没有：编译/链接失败、被过滤或日志损坏，均不可判绿
if ($results.Count -eq 0) {
    Write-Host "[GATE] fail=1 — 未解析到任何测试结果行（cargo_rc=$cargoRc，日志 $LogFile）" -ForegroundColor Red
    exit 2
}

$quarantine = @()
$quarantinePlatforms = @{}
if (Test-Path $QuarantineFile) {
    foreach ($entry in Get-Content -Path $QuarantineFile) {
        if ($entry.Trim() -eq "" -or $entry.TrimStart().StartsWith("#")) { continue }
        $parts = $entry -split '\|'
        if ($parts.Count -lt 2) { continue }
        $quarantine += $parts[0]
        $quarantinePlatforms[$parts[0]] = $parts[1]
    }
}

$warnCount = 0
$fatalCount = 0
foreach ($line in $results) {
    if ($line -match '^test (.+) \.\.\. (ok|FAILED|ignored)$') {
        $name = $Matches[1]
        $state = $Matches[2]
        if ($quarantine -contains $name) {
            Write-Host "[WARN] quarantined $name result=$state — 已知不稳定，不计为通过（见 scripts/test-gate/quarantine.txt）"
            if ($state -eq "FAILED") { $warnCount++ }
        }
        elseif ($state -eq "FAILED") {
            Write-Host "[FAIL] $name"
            $fatalCount++
        }
    }
}

foreach ($name in $quarantine) {
    $platforms = $quarantinePlatforms[$name]
    if ($platforms -and -not ($platforms -match 'any' -or $platforms -match 'windows')) { continue }
    $hit = $results | Where-Object { $_ -match ("^test " + [regex]::Escape($name) + " \.\.\. (ok|FAILED|ignored)$") }
    if (-not $hit) {
        Write-Host "[WARN] quarantine entry not observed: $name（被过滤或改名，清单可能失效）"
    }
}

if ($fatalCount -gt 0) {
    Write-Host "[GATE] warn=$warnCount fail=$fatalCount — 存在非 quarantine 失败" -ForegroundColor Red
    exit 2
}
# 有 FAILED 行却全部落在 quarantine，cargo 仍会返回非零；那是预期行为，不额外判死。
# 反之 cargo 非零但没有任何 FAILED 行，说明是编译/链接或运行器错误，不能吞掉。
$hasFailedLine = @($results | Where-Object { $_ -match '\.\.\. FAILED$' }).Count -gt 0
if ($cargoRc -ne 0 -and -not $hasFailedLine) {
    Write-Host "[GATE] fail=1 — cargo 退出码 $cargoRc，且日志中无 FAILED 行（疑似编译/链接失败）" -ForegroundColor Red
    exit 2
}

# 可选竞态连跑：默认 1 = 不额外跑（主套件已含 1 次）。判定以「test result: ok. 1 passed」为准，
# 不能只看退出码——--exact 名字写错时 cargo 以 0 个用例、退出码 0 返回，只看退出码会得到假绿。
$raceTarget = "server::prune_claim_race_tests::prune_concurrent_claimers_delete_once_others_lost_race"
if ($RaceRepeat -le 1) {
    Write-Host "[RACE] repeat=1（默认，未启用连跑）"
}
elseif ($LogOnly) {
    Write-Host "[RACE] repeat=$RaceRepeat 被忽略：-LogFile 只分类已有日志，不跑测试"
}
else {
    $raceOk = 0
    $raceBad = 0
    $raceArgs = @("test", "--offline", "-p", $Package, "--lib", "--", "--exact", $raceTarget)
    for ($i = 1; $i -le $RaceRepeat; $i++) {
        Push-Location $RepoRoot
        try {
            $raceOut = (& cargo @raceArgs 2>&1 | Out-String)
            $raceRc = $LASTEXITCODE
        }
        finally {
            Pop-Location
        }
        if ($raceRc -eq 0 -and $raceOut -match 'test result: ok\. 1 passed') {
            $raceOk++
            Write-Host "[RACE] run $i/$RaceRepeat ok"
        }
        else {
            $raceBad++
            Write-Host "[FAIL] race run $i/$RaceRepeat（rc=$raceRc，未见 1 passed）" -ForegroundColor Red
        }
    }
    Write-Host "[RACE] repeat=$RaceRepeat ok=$raceOk bad=$raceBad target=$raceTarget"
    if ($raceBad -gt 0) {
        Write-Host "[GATE] fail=$raceBad — 竞态连跑未全绿" -ForegroundColor Red
        exit 2
    }
}
Write-Host "[GATE] warn=$warnCount fail=0 — 非 quarantine 用例全部通过；已知不稳定按 WARN 呈现，不代表全绿结论" -ForegroundColor Yellow
exit 0
