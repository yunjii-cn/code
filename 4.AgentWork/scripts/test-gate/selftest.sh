#!/usr/bin/env bash
# 门禁分类器负向对照（常驻用例）：证明「没有证据不判绿」不是一次性人工验证。
#
# 覆盖：
#   1. 日志不存在                 → exit 2，[GATE] fail=1 — 日志不存在
#   2. 编译失败（零结果行）       → exit 2，[GATE] fail=1 — 未解析到任何测试结果行
#   3. 非 quarantine 用例 FAILED  → exit 2，[FAIL] <name>
#   4. quarantine 用例 FAILED     → exit 0，[WARN] quarantined … result=FAILED
#   5. quarantine 用例 ok         → exit 0，[WARN] quarantined … result=ok
#   6. cargo 非零且无 FAILED 行   → exit 2，[GATE] fail=1 — cargo 退出码
#   7. 干净日志                   → exit 0，[GATE] warn=0 fail=0
#
# 用法：bash scripts/test-gate/selftest.sh
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GATE="$SCRIPT_DIR/gate.sh"
QUARANTINE="$SCRIPT_DIR/quarantine.txt"
TMP="$(mktemp -d -t aw-gate-selftest-XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

pass=0
failed=0

# run_case <名称> <期望退出码> <期望输出子串> <日志文件> [PATH 前缀]
run_case() {
  local name="$1" expected_rc="$2" expected_text="$3" log="$4" path_prefix="${5:-}"
  local out="$TMP/out-$pass-$failed.txt" rc=0
  if [ -n "$path_prefix" ]; then
    PATH="$path_prefix:$PATH" bash "$GATE" --log "$log" > "$out" 2>&1 || rc=$?
  else
    bash "$GATE" --log "$log" > "$out" 2>&1 || rc=$?
  fi
  local ok=1
  [ "$rc" -eq "$expected_rc" ] || ok=0
  grep -qF -- "$expected_text" "$out" || ok=0
  if [ "$ok" -eq 1 ]; then
    printf '[SELFTEST][PASS] %s (rc=%s)\n' "$name" "$rc"
    pass=$((pass + 1))
  else
    printf '[SELFTEST][FAIL] %s 期望 rc=%s 且包含「%s」，实际 rc=%s\n' \
      "$name" "$expected_rc" "$expected_text" "$rc"
    sed -n '1,20p' "$out" | sed 's/^/    /'
    failed=$((failed + 1))
  fi
}

# 从清单取一个真实条目名，避免硬编码测试路径
QNAME="$(awk -F'|' '!/^[[:space:]]*#/ && NF > 1 {gsub(/^[ \t]+|[ \t]+$/, "", $1); print $1; exit}' "$QUARANTINE")"
[ -n "$QNAME" ] || {
  printf '[SELFTEST][FAIL] quarantine.txt 中取不到条目名，无法做 WARN 对照\n' >&2
  exit 2
}

# 1. 日志不存在
run_case "missing-log" 2 "[GATE] fail=1 — 日志不存在" "$TMP/does-not-exist.log"

# 2. 编译失败：有输出、零结果行
mkdir -p "$TMP/bin-compile-fail"
printf '#!/usr/bin/env bash\nprintf "error[E0425]: cannot find value\\n"\nexit 101\n' > "$TMP/bin-compile-fail/cargo"
chmod +x "$TMP/bin-compile-fail/cargo"
run_case "compile-failure-no-results" 2 "[GATE] fail=1 — 未解析到任何测试结果行" "" "$TMP/bin-compile-fail"

# 3. 非 quarantine 失败
printf 'test some::real_failure ... FAILED\n' > "$TMP/real-failed.log"
run_case "non-quarantined-failure" 2 "[FAIL] some::real_failure" "$TMP/real-failed.log"

# 4. quarantine 失败 → 警告但不是通过
printf 'test %s ... FAILED\n' "$QNAME" > "$TMP/quarantined-failed.log"
run_case "quarantined-failure-is-warn" 0 "[WARN] quarantined $QNAME result=FAILED" "$TMP/quarantined-failed.log"

# 5. quarantine 通过 → 仍不记 PASS
printf 'test %s ... ok\n' "$QNAME" > "$TMP/quarantined-ok.log"
run_case "quarantined-ok-is-warn" 0 "[WARN] quarantined $QNAME result=ok" "$TMP/quarantined-ok.log"

# 6. cargo 非零但没有 FAILED 行：有结果行也不能判绿
mkdir -p "$TMP/bin-partial"
printf '#!/usr/bin/env bash\nprintf "test some::ok ... ok\\n"\nexit 1\n' > "$TMP/bin-partial/cargo"
chmod +x "$TMP/bin-partial/cargo"
run_case "cargo-nonzero-without-failed" 2 "[GATE] fail=1 — cargo 退出码" "" "$TMP/bin-partial"

# 7. 干净日志
printf 'test some::one ... ok\n%s\n' "test another::two ... ok" > "$TMP/clean.log"
run_case "clean-log-is-green" 0 "[GATE] warn=0 fail=0" "$TMP/clean.log"

printf '[SELFTEST] pass=%s fail=%s\n' "$pass" "$failed"
[ "$failed" -eq 0 ] || exit 2
