#!/usr/bin/env bash
# POSIX（Linux）超时"受控集合"清理用例复跑脚本。
#
# 只做可复现：不修改实现、不修改契约。
#
# 用法（在 WSL / Linux 内执行）：
#   bash scripts/posix-timeout-verify/verify.sh check    # 只做环境自检，不编译
#   bash scripts/posix-timeout-verify/verify.sh prepare   # 联网：安装 Rust、生成 vendor、裁剪 workspace
#   bash scripts/posix-timeout-verify/verify.sh run       # 离线：跑四类 POSIX 用例并校验证据行
#   bash scripts/posix-timeout-verify/verify.sh all       # prepare + run
#
# 环境变量：
#   AW_WORK     工作目录，默认 $HOME/.aw-posix-verify
#   AW_VENDOR   vendor 目录，默认 $AW_WORK/vendor
#   AW_OFFLINE  1 = 禁止联网：prepare 遇到缺失 vendor 直接失败，run 始终 --offline
#
# 离线预置清单见 --help 或 check 输出。
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORK_DIR="${AW_WORK:-$HOME/.aw-posix-verify}"
VENDOR_DIR="${AW_VENDOR:-$WORK_DIR/vendor}"
SRC_DIR="$WORK_DIR/src"
LOG_DIR="$WORK_DIR/logs"
TOOLCHAIN_VERSION="1.88.0"
MIN_RUST_VERSION="1.85"
LOG_FILE=""

log() { printf '[INFO] %s\n' "$*"; }
warn() { printf '[WARN] %s\n' "$*" >&2; }

fail() {
  local reason="$1"
  local hint="$2"
  printf '[FAIL] reason=%s\n' "$reason" >&2
  printf '       hint=%s\n' "$hint" >&2
  [ -n "$LOG_FILE" ] && printf '[FAIL] log=%s\n' "$LOG_FILE" >&2
  exit 2
}

offline_requirements() {
  cat <<EOF
离线预置清单（不联网也能复跑，需提前准备好）：
  1. Rust >= ${MIN_RUST_VERSION}（推荐 ${TOOLCHAIN_VERSION}）：cargo / rustc 可用
     找查顺序：PATH -> \$HOME/.cargo/bin -> \$HOME/.rustup/toolchains/<版本>/bin
  2. C 链接器 cc / gcc 与 libc 头文件（Ubuntu：sudo apt install -y gcc libc6-dev）
  3. 依赖 vendor 目录（联网时执行 prepare 生成）：需含 libc-* / serde-* / serde_json-* /
     sha2-* / hex-* / chrono-* / tempfile-*，且每个 crate 带 .cargo-checksum.json
  4. 本仓库 checkout（根 Cargo.toml / Cargo.lock / apps/mcp-bridge）
EOF
}

usage() {
  cat <<EOF
POSIX 超时受控集合清理用例复跑脚本（Linux / WSL）

用法：
  verify.sh check     只做环境自检：平台、Rust 版本、链接器、vendor、workspace
  verify.sh prepare   联网准备：安装 Rust（缺失时）、裁剪 workspace、cargo vendor
  verify.sh run       离线执行：cargo test --offline，校验四类用例与证据行
  verify.sh all       等价于 prepare + run

环境变量：
  AW_WORK=${WORK_DIR}
  AW_VENDOR=${VENDOR_DIR}
  AW_OFFLINE=0|1

$(offline_requirements)
EOF
}

# 判断 $1 >= $2（x.y.z 数字比较）
version_ge() {
  awk -v a="$1" -v b="$2" 'BEGIN {
    split(a, x, "."); split(b, y, ".");
    for (i = 1; i <= 3; i++) {
      if ((x[i] + 0) > (y[i] + 0)) exit 0;
      if ((x[i] + 0) < (y[i] + 0)) exit 1;
    }
    exit 0;
  }'
}

require_platform() {
  if [ "$(uname -s)" != "Linux" ]; then
    fail wrong_platform "当前系统 $(uname -s)，本脚本要求 Linux；四类用例含 POSIX 进程组 / setsid，Windows 与 macOS 不适用"
  fi
  command -v setsid >/dev/null 2>&1 \
    || fail missing_setsid "缺少 setsid（util-linux）；第 4 类作用域负例依赖它，不能跳过"
}

require_host_target() {
  local host
  host="$(rustc -vV 2>/dev/null | awk '/^host:/ {print $2}')"
  case "$host" in
    *linux*) log "rustc host=${host}" ;;
    *) fail wrong_target "rustc host=${host:-未知}，不是 linux-*；四类用例必须在 Linux 主机上运行" ;;
  esac
}

find_cargo() {
  CARGO_BIN_DIR=""
  if command -v cargo >/dev/null 2>&1; then
    CARGO_BIN_DIR="$(dirname "$(command -v cargo)")"
  elif [ -x "${HOME:-/nonexistent}/.cargo/bin/cargo" ]; then
    CARGO_BIN_DIR="${HOME}/.cargo/bin"
  else
    local d
    for d in "${HOME:-/nonexistent}"/.rustup/toolchains/*/bin; do
      if [ -x "$d/cargo" ]; then
        CARGO_BIN_DIR="$d"
        break
      fi
    done
  fi
  [ -n "$CARGO_BIN_DIR" ] && export PATH="$CARGO_BIN_DIR:$PATH"
}

require_toolchain() {
  find_cargo
  if [ -z "$CARGO_BIN_DIR" ]; then
    fail rust_missing "未找到 cargo / rustc；联网执行 prepare 安装，离线按 --help 的清单预置 Rust"
  fi
  local ver
  ver="$(rustc --version 2>/dev/null | awk '{print $2}')"
  if [ -z "$ver" ]; then
    fail rust_broken "cargo 在 $CARGO_BIN_DIR，但 rustc 不可用；请检查工具链完整性"
  fi
  if ! version_ge "$ver" "$MIN_RUST_VERSION"; then
    fail rust_too_old "rustc ${ver} < 要求 ${MIN_RUST_VERSION}（仓库 rust-version=1.85；推荐 ${TOOLCHAIN_VERSION}）"
  fi
  log "rustc ${ver}（要求 >= ${MIN_RUST_VERSION}，推荐 ${TOOLCHAIN_VERSION}）"
  require_host_target
}

require_linker() {
  command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1 || command -v clang >/dev/null 2>&1 \
    || fail missing_linker "缺少 C 链接器（cc/gcc/clang）与 libc 头文件；Ubuntu 先 sudo apt install -y gcc libc6-dev"
}

require_repo() {
  local f
  for f in Cargo.toml Cargo.lock apps/mcp-bridge/Cargo.toml apps/mcp-bridge/src/server.rs \
           apps/mcp-bridge/tests/mcp_contract.rs; do
    [ -e "$REPO_ROOT/$f" ] || fail repo_incomplete "仓库缺少 $f（REPO_ROOT=$REPO_ROOT）"
  done
}

install_rust_if_needed() {
  find_cargo
  [ -n "$CARGO_BIN_DIR" ] && return 0
  if [ "${AW_OFFLINE:-0}" = "1" ]; then
    fail rust_missing "离线模式(AW_OFFLINE=1)且未找到 Rust；需预置 cargo/rustc（见 --help 清单）"
  fi
  log "未找到 Rust，尝试安装 ${TOOLCHAIN_VERSION}（需要联网）"
  local rustup_bin=""
  if command -v rustup >/dev/null 2>&1; then
    rustup_bin="$(command -v rustup)"
  elif [ -x "${HOME:-/nonexistent}/.cargo/bin/rustup" ]; then
    rustup_bin="${HOME}/.cargo/bin/rustup"
  fi
  if [ -n "$rustup_bin" ]; then
    "$rustup_bin" toolchain install "$TOOLCHAIN_VERSION" --profile minimal \
      || fail rust_install_failed "rustup toolchain install ${TOOLCHAIN_VERSION} 失败（需联网；离线请预置）"
    "$rustup_bin" default "$TOOLCHAIN_VERSION" >/dev/null 2>&1 || true
  elif command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1; then
    local script="$WORK_DIR/rustup-init.sh"
    mkdir -p "$WORK_DIR"
    if command -v curl >/dev/null 2>&1; then
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$script" \
        || fail rust_install_failed "下载 rustup-init 失败（需联网；离线请预置 Rust）"
    else
      wget -q https://sh.rustup.rs -O "$script" \
        || fail rust_install_failed "下载 rustup-init 失败（需联网；离线请预置 Rust）"
    fi
    sh "$script" -y --profile minimal --default-toolchain "$TOOLCHAIN_VERSION" \
      || fail rust_install_failed "rustup-init 安装失败（需联网；离线请预置 Rust）"
  else
    fail rust_missing "既无 Rust 也无 rustup/curl/wget；离线请按 --help 清单预置 Rust"
  fi
  find_cargo
  [ -n "$CARGO_BIN_DIR" ] || fail rust_install_failed "安装后仍未找到 cargo（检查 \$HOME/.cargo/bin）"
}

write_trimmed_workspace() {
  mkdir -p "$SRC_DIR/apps"
  # 保留 [workspace.package] / [workspace.dependencies]，只把 members 收窄到 mcp-bridge
  awk '
    /^members = \[/ { print "members = [\"apps/mcp-bridge\"]"; skip = 1; next }
    skip && /^\]/ { skip = 0; next }
    !skip { print }
  ' "$REPO_ROOT/Cargo.toml" > "$SRC_DIR/Cargo.toml" \
    || fail workspace_prepare_failed "生成裁剪 Cargo.toml 失败"

  local members
  members="$(awk '/^members = \[/ {print}' "$SRC_DIR/Cargo.toml")"
  case "$members" in
    *apps/mcp-bridge*) ;;
    *) fail workspace_prepare_failed "裁剪 Cargo.toml 未收窄 members（检查根 Cargo.toml 的 members 格式）" ;;
  esac

  cp "$REPO_ROOT/Cargo.lock" "$SRC_DIR/Cargo.lock" \
    || fail workspace_prepare_failed "复制 Cargo.lock 失败"
  rm -rf "$SRC_DIR/apps/mcp-bridge"
  cp -a "$REPO_ROOT/apps/mcp-bridge" "$SRC_DIR/apps/" \
    || fail workspace_prepare_failed "复制 apps/mcp-bridge 失败"
}

write_vendor_config() {
  mkdir -p "$SRC_DIR/.cargo"
  cat > "$SRC_DIR/.cargo/config.toml" <<EOF
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "$VENDOR_DIR"
EOF
}

vendor_complete() {
  local pat
  [ -d "$VENDOR_DIR" ] || return 1
  for pat in libc- serde- serde_json- sha2- hex- chrono- tempfile-; do
    compgen -G "$VENDOR_DIR/$pat*" >/dev/null 2>&1 || return 1
  done
  local first
  first="$(compgen -G "$VENDOR_DIR/libc-*" | head -n1)"
  [ -f "$first/.cargo-checksum.json" ] || return 1
  return 0
}

prepare_vendor() {
  write_trimmed_workspace
  write_vendor_config
  if vendor_complete; then
    log "vendor 已完整，跳过（$VENDOR_DIR）"
    return 0
  fi
  if [ "${AW_OFFLINE:-0}" = "1" ]; then
    fail vendor_missing "离线模式且 vendor 缺失/不完整：$VENDOR_DIR；需预置 vendor 或联网执行 prepare"
  fi
  log "生成 vendor（需要联网）：$VENDOR_DIR"
  local tmp="$VENDOR_DIR.new.$$"
  rm -rf "$tmp"
  (
    cd "$SRC_DIR" || exit 1
    CARGO_NET_OFFLINE=false cargo vendor --versioned-dirs "$tmp" > "$LOG_DIR/vendor.log" 2>&1
  ) || fail vendor_failed "cargo vendor 失败（见 $LOG_DIR/vendor.log）；离线场景请预置 vendor"
  vendor_complete || { rm -rf "$tmp"; fail vendor_incomplete "vendor 生成后仍不完整（见 $LOG_DIR/vendor.log）"; }
  rm -rf "$VENDOR_DIR"
  mv "$tmp" "$VENDOR_DIR" || fail vendor_failed "替换 vendor 目录失败：$VENDOR_DIR"
  write_vendor_config
  log "vendor 就绪：$VENDOR_DIR"
}

run_tests() {
  require_platform
  require_toolchain
  require_linker
  require_repo
  mkdir -p "$LOG_DIR"
  write_trimmed_workspace
  write_vendor_config
  vendor_complete \
    || fail vendor_missing "vendor 缺失/不完整：$VENDOR_DIR；联网执行 prepare，或离线预置（见 --help）"

  LOG_FILE="$LOG_DIR/posix-verify-$(date +%Y%m%d-%H%M%S).log"
  log "运行 POSIX 四类用例（离线，日志 $LOG_FILE）"
  (
    cd "$SRC_DIR" || exit 1
    CARGO_NET_OFFLINE=true CARGO_TARGET_DIR="$WORK_DIR/target" \
      cargo test --offline -p aw-mcp-bridge -- --nocapture
  ) 2>&1 | tee "$LOG_FILE"
  local rc
  rc=${PIPESTATUS[0]}
  [ "$rc" -eq 0 ] || fail test_failed "cargo test 退出码 ${rc}；见日志 $LOG_FILE"

  local missing="" token
  for token in \
    "[evidence] exited pgid=" \
    "[evidence] stopped pgid=" \
    "kill(-pgid,0)_exists=true" \
    "[evidence] stopped after_SIGKILL group_empty=true" \
    "[evidence] setsid escaped_pid=" \
    "escaped_alive=true" \
    "original_group_empty=true" \
    "[evidence] timeout_reason=" \
    "已清理受控集合" \
    "exited_process_group_has_no_members ... ok" \
    "stopped_process_still_counts_as_alive ... ok" \
    "setsid_escape_is_outside_the_observed_group ... ok" \
    "timeout_cleans_controlled_set_including_grandchild ... ok"
  do
    grep -qF -- "$token" "$LOG_FILE" || missing="$missing
  - $token"
  done
  if [ -n "$missing" ]; then
    fail missing_evidence "四类用例证据行未全部命中：$missing"
  fi

  log "四类 POSIX 用例全部通过，证据行齐全"
  log "日志：$LOG_FILE"
}

cmd="${1:-}"
case "$cmd" in
  check)
    require_platform
    require_toolchain
    require_linker
    require_repo
    if vendor_complete; then
      log "vendor 完整：$VENDOR_DIR"
    else
      fail vendor_missing "vendor 缺失/不完整：$VENDOR_DIR；联网执行 prepare，或按 --help 预置"
    fi
    if [ -f "$SRC_DIR/Cargo.toml" ]; then
      log "workspace 已准备：$SRC_DIR"
    else
      warn "workspace 尚未准备（run/all 会自动生成）：$SRC_DIR"
    fi
    log "自检通过：work=$WORK_DIR src=$SRC_DIR vendor=$VENDOR_DIR"
    ;;
  prepare)
    require_platform
    install_rust_if_needed
    require_toolchain
    require_linker
    require_repo
    mkdir -p "$LOG_DIR"
    prepare_vendor
    log "准备完成：$SRC_DIR"
    ;;
  run)
    run_tests
    ;;
  all)
    require_platform
    install_rust_if_needed
    require_toolchain
    require_linker
    require_repo
    mkdir -p "$LOG_DIR"
    prepare_vendor
    run_tests
    ;;
  -h|--help|help|"")
    usage
    ;;
  *)
    usage
    fail bad_usage "未知子命令：$cmd（可用：check / prepare / run / all）"
    ;;
esac
