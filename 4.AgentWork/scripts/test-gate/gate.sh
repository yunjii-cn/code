#!/usr/bin/env bash
# 默认门禁（POSIX）：跑 aw-mcp-bridge 测试，或对已有日志做分类。
#
# 与 scripts/test-gate/quarantine.txt 配合：清单里的用例按 [WARN] 呈现，
# 既不算通过也不阻断；其余用例失败一律 [FAIL] 并以 2 退出。
#
# 用法：
#   gate.sh                       # 跑 cargo test --offline -p aw-mcp-bridge -- --nocapture
#   gate.sh --test-threads 16     # 追加测试二进制参数
#   gate.sh --log <file>          # 只分类已有日志，不重跑
#   gate.sh --race-repeat 12      # 可选：竞态用例连跑 12 次（默认 1 = 不连跑；也可用 AW_RACE_REPEAT）
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# 清单路径可用环境变量覆盖：selftest.sh 用固定 fixture，避免负向对照依赖生产清单是否非空。
QUARANTINE_FILE="${AW_GATE_QUARANTINE_FILE:-$SCRIPT_DIR/quarantine.txt}"
RACE_REPEAT="${AW_RACE_REPEAT:-1}"
LOG_FILE=""
RUN_ARGS=()

while [ "$#" -gt 0 ]; do
  case "$1" in
    --log)
      LOG_FILE="${2:-}"
      shift 2
      ;;
    --log=*)
      LOG_FILE="${1#--log=}"
      shift
      ;;
    --race-repeat)
      RACE_REPEAT="${2:-1}"
      shift 2
      ;;
    --race-repeat=*)
      RACE_REPEAT="${1#--race-repeat=}"
      shift
      ;;
    --)
      shift
      RUN_ARGS+=("$@")
      break
      ;;
    *)
      RUN_ARGS+=("$1")
      shift
      ;;
  esac
done

case "$RACE_REPEAT" in
  '' | *[!0-9]*)
    printf '[GATE] fail=1 — --race-repeat 必须是非负整数（收到 %s）\n' "$RACE_REPEAT" >&2
    exit 2
    ;;
esac
if [ "$RACE_REPEAT" -lt 1 ]; then
  printf '[GATE] fail=1 — --race-repeat 必须 >= 1（收到 %s）\n' "$RACE_REPEAT" >&2
  exit 2
fi

cargo_rc=0
LOG_ONLY=0
[ -n "$LOG_FILE" ] && LOG_ONLY=1
if [ -z "$LOG_FILE" ]; then
  LOG_FILE="$(mktemp -t aw-gate-XXXXXX.log)"
  printf '[INFO] 运行测试（日志 %s）\n' "$LOG_FILE"
  (
    cd "$REPO_ROOT" || exit 1
    cargo test --offline -p aw-mcp-bridge -- --nocapture "${RUN_ARGS[@]}"
  ) 2>&1 | tee "$LOG_FILE"
  cargo_rc=${PIPESTATUS[0]}
fi

# 日志不存在/不可读时严禁判绿：没有证据就不算通过
if [ ! -r "$LOG_FILE" ]; then
  printf '[GATE] fail=1 — 日志不存在或不可读：%s\n' "$LOG_FILE" >&2
  exit 2
fi

# 解析结果行：test <full::name> ... ok|FAILED|ignored
results="$(grep -E '^test .+ \.\.\. (ok|FAILED|ignored)$' "$LOG_FILE" || true)"

# 一条结果行都没有：编译/链接失败、被过滤或日志损坏，均不可判绿
if [ -z "$results" ]; then
  printf '[GATE] fail=1 — 未解析到任何测试结果行（cargo_rc=%s，日志 %s）\n' "$cargo_rc" "$LOG_FILE" >&2
  exit 2
fi

# 清单条目（名字|平台|负责人|自版本|触发条件|观测频率）
quarantine_lines() {
  awk '!/^[[:space:]]*#/ && NF > 1' "$QUARANTINE_FILE"
}

quarantine_names() {
  quarantine_lines | awk -F'|' '{print $1}'
}

case "$(uname -s)" in
  Linux*) PLATFORM="linux" ;;
  Darwin*) PLATFORM="macos" ;;
  *) PLATFORM="posix" ;;
esac

is_quarantined() {
  quarantine_names | grep -qxF -- "$1"
}

# 是否观测到该用例的某个结果
observed() {
  grep -qxF -- "test $1 ... $2" <<<"$results"
}

warn_count=0
fatal_count=0

# 逐条结果分类：quarantine 命中 → WARN（无论 ok / FAILED）；其余 FAILED → FAIL
while IFS= read -r line; do
  [ -n "$line" ] || continue
  name="$(sed -nE 's/^test (.+) \.\.\. (ok|FAILED|ignored)$/\1/p' <<<"$line")"
  state="$(sed -nE 's/^test (.+) \.\.\. (ok|FAILED|ignored)$/\2/p' <<<"$line")"
  [ -n "$name" ] || continue
  if is_quarantined "$name"; then
    printf '[WARN] quarantined %s result=%s — 已知不稳定，不计为通过（见 scripts/test-gate/quarantine.txt）\n' "$name" "$state"
    if [ "$state" = "FAILED" ]; then
      warn_count=$((warn_count + 1))
    fi
  elif [ "$state" = "FAILED" ]; then
    printf '[FAIL] %s\n' "$name"
    fatal_count=$((fatal_count + 1))
  fi
done <<<"$results"

# 清单里的用例没被观测到（被过滤 / 改名）也要出声，避免清单悄悄失效
while IFS='|' read -r name platforms _rest; do
  [ -n "$name" ] || continue
  case "$platforms" in
    *any* | *"$PLATFORM"*) ;;
    *) continue ;;
  esac
  if ! observed "$name" ok && ! observed "$name" FAILED && ! observed "$name" ignored; then
    printf '[WARN] quarantine entry not observed: %s（被过滤或改名，清单可能失效）\n' "$name"
  fi
done < <(quarantine_lines)

if [ "$fatal_count" -gt 0 ]; then
  printf '[GATE] warn=%s fail=%s — 存在非 quarantine 失败\n' "$warn_count" "$fatal_count" >&2
  exit 2
fi
# 有 FAILED 行却全部落在 quarantine，cargo 仍会返回非零；那是预期行为，不额外判死。
# 反之 cargo 非零但没有任何 FAILED 行，说明是编译/链接或运行器错误，不能吞掉。
if [ "$cargo_rc" -ne 0 ] && ! grep -qE '^test .+ \.\.\. FAILED$' "$LOG_FILE"; then
  printf '[GATE] fail=1 — cargo 退出码 %s，且日志中无 FAILED 行（疑似编译/链接失败）\n' "$cargo_rc" >&2
  exit 2
fi

# 可选竞态连跑：默认 1 = 不额外跑（主套件已含 1 次）。判定以「test result: ok. 1 passed」为准，
# 不能只看退出码——--exact 名字写错时 cargo 以 0 个用例、退出码 0 返回，只看退出码会得到假绿。
RACE_TARGET="server::prune_claim_race_tests::prune_concurrent_claimers_delete_once_others_lost_race"
if [ "$RACE_REPEAT" -le 1 ]; then
  printf '[RACE] repeat=1（默认，未启用连跑）\n'
elif [ "$LOG_ONLY" -eq 1 ]; then
  printf '[RACE] repeat=%s 被忽略：--log 只分类已有日志，不跑测试\n' "$RACE_REPEAT"
else
  race_ok=0
  race_bad=0
  i=1
  while [ "$i" -le "$RACE_REPEAT" ]; do
    race_out="$( (cd "$REPO_ROOT" || exit 1; cargo test --offline -p aw-mcp-bridge --lib -- --exact "$RACE_TARGET") 2>&1 )"
    race_rc=$?
    if [ "$race_rc" -eq 0 ] && printf '%s' "$race_out" | grep -q 'test result: ok\. 1 passed'; then
      race_ok=$((race_ok + 1))
      printf '[RACE] run %s/%s ok\n' "$i" "$RACE_REPEAT"
    else
      race_bad=$((race_bad + 1))
      printf '[FAIL] race run %s/%s（rc=%s，未见 1 passed）\n' "$i" "$RACE_REPEAT" "$race_rc" >&2
    fi
    i=$((i + 1))
  done
  printf '[RACE] repeat=%s ok=%s bad=%s target=%s\n' "$RACE_REPEAT" "$race_ok" "$race_bad" "$RACE_TARGET"
  if [ "$race_bad" -gt 0 ]; then
    printf '[GATE] fail=%s — 竞态连跑未全绿\n' "$race_bad" >&2
    exit 2
  fi
fi

printf '[GATE] warn=%s fail=0 — 非 quarantine 用例全部通过；已知不稳定按 WARN 呈现，不代表全绿结论\n' "$warn_count"
exit 0
