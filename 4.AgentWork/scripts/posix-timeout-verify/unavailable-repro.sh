#!/usr/bin/env bash
# 复现驱动（v1.50）：在可控负载下反复跑「建作用域 → 观测 → 终止」，统计
# `Unavailable` 的出现频率、原因与观测并发峰值，输出频率表。
#
# 用法（在 WSL / Linux 内执行）：
#   bash scripts/posix-timeout-verify/unavailable-repro.sh
#   ROUNDS=100 CONFIGS="1:0 8:16 16:64" bash scripts/posix-timeout-verify/unavailable-repro.sh
#
# 环境变量：
#   ROUNDS    每个观测线程的轮数（默认 50）
#   CONFIGS   "<workers>:<load_threads>" 列表（默认 "1:0 4:0 8:16 16:64"）
#   AW_WORK   工作目录（默认 $HOME/.aw-posix-verify，与 verify.sh 共用）
#   AW_VENDOR vendor 目录（默认 $AW_WORK/vendor）
#
# 前提：已按 verify.sh 完成 prepare（vendor 完整、裁剪 workspace 就绪）。
# 本脚本会先调用 verify.sh prepare 把当前源码同步进裁剪 workspace。
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORK_DIR="${AW_WORK:-$HOME/.aw-posix-verify}"
SRC_DIR="$WORK_DIR/src"
LOG_DIR="$WORK_DIR/logs"
ROUNDS="${ROUNDS:-50}"
CONFIGS="${CONFIGS:-1:0 4:0 8:16 16:64}"
STAMP="$(date +%Y%m%d-%H%M%S)"
FAIL=0

log() { printf '[INFO] %s\n' "$*"; }
fail() {
  printf '[FAIL] reason=%s\n' "$1" >&2
  [ -n "${2:-}" ] && printf '       hint=%s\n' "$2" >&2
  exit 2
}

[ "$(uname -s)" = "Linux" ] || fail wrong_platform "本脚本要求 Linux（/proc 与 cgroup v2 观测路径）"

# 复用 verify.sh 的工具链查找与 workspace 同步，避免两套裁剪逻辑漂移
bash "$REPO_ROOT/scripts/posix-timeout-verify/verify.sh" prepare \
  || fail prepare_failed "verify.sh prepare 失败；先按 verify.sh --help 准备离线环境"

CARGO_BIN_DIR=""
if command -v cargo >/dev/null 2>&1; then
  CARGO_BIN_DIR="$(dirname "$(command -v cargo)")"
elif [ -x "${HOME:-/nonexistent}/.cargo/bin/cargo" ]; then
  CARGO_BIN_DIR="${HOME}/.cargo/bin"
else
  for d in "${HOME:-/nonexistent}"/.rustup/toolchains/*/bin; do
    if [ -x "$d/cargo" ]; then CARGO_BIN_DIR="$d"; break; fi
  done
fi
[ -n "$CARGO_BIN_DIR" ] || fail rust_missing "未找到 cargo；先执行 verify.sh prepare"
export PATH="$CARGO_BIN_DIR:$PATH"

mkdir -p "$LOG_DIR"
TABLE="$LOG_DIR/unavailable-repro-$STAMP.table.txt"
: > "$TABLE"

log "每线程轮数=$ROUNDS 配置=[$CONFIGS] 表格=$TABLE"
for config in $CONFIGS; do
  workers="${config%%:*}"
  threads="${config##*:}"
  [ -n "$workers" ] && [ -n "$threads" ] || fail bad_config "CONFIGS 条目必须是 <workers>:<load_threads>，实际：$config"
  log_file="$LOG_DIR/unavailable-repro-$STAMP-w${workers}-l$threads.log"
  log "运行 workers=$workers load_threads=$threads rounds_per_worker=$ROUNDS（日志 $log_file）"
  (
    cd "$SRC_DIR" || exit 1
    AW_REPRO_ROUNDS="$ROUNDS" AW_REPRO_WORKERS="$workers" AW_REPRO_LOAD_THREADS="$threads" \
      CARGO_NET_OFFLINE=true CARGO_TARGET_DIR="$WORK_DIR/target" \
      cargo test --offline -p aw-mcp-bridge --lib \
        unavailable_frequency_under_controlled_load -- --ignored --nocapture
  ) > "$log_file" 2>&1
  rc=$?
  if [ "$rc" -ne 0 ]; then
    printf '[FAIL] workers=%s load_threads=%s cargo_rc=%s（见 %s）\n' "$workers" "$threads" "$rc" "$log_file" >&2
    FAIL=1
    continue
  fi
  grep -E '^\[repro\]\[' "$log_file" >> "$TABLE"
done

printf '\n[table] workers | load_threads | rounds | cleaned | unavailable | still_alive | observe_fail | rate | peak_obs | max_loadavg\n'
printf '[table] ------- | ------------ | ------ | ------- | ----------- | ----------- | ------------ | ---- | -------- | -----------\n'
while IFS= read -r line; do
  case "$line" in
    "[repro][row]"*)
      get() { printf '%s\n' "$line" | sed -nE "s/.*[[:space:]]$1=([^ ]+).*/\\1/p"; }
      printf '[table] %7s | %12s | %6s | %7s | %11s | %11s | %12s | %4s | %8s | %11s\n' \
        "$(get workers)" "$(get load_threads)" "$(get rounds)" "$(get cleaned)" "$(get unavailable)" \
        "$(get still_alive)" "$(get observe_fail)" "$(get rate)" "$(get peak_obs)" \
        "$(get max_loadavg)"
      ;;
    "[repro][reasons]"*)
      printf '%s\n' "$line"
      ;;
    "[repro][side]"* | "[repro][env]"* | "[repro][verdict]"*)
      printf '%s\n' "$line"
      ;;
  esac
done < "$TABLE"

[ "$FAIL" -eq 0 ] || fail repro_run_failed "至少一组负载运行失败；见 $LOG_DIR/unavailable-repro-$STAMP-load*.log"
log "完成；表格与原始日志在 $LOG_DIR（前缀 unavailable-repro-$STAMP）"
