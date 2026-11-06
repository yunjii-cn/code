# 契约提案：`runs.prune` 状态过滤 / 分档（未实现）

> 状态：**提案文档，未实现、未改契约版本**。契约仍为 **v1.20**；本文不是契约正文，也不构成对 PM 的承诺。
> 结论先行：**推荐"状态多值白名单"（`status: ["failed", "blocked"]`），不推荐服务端分档常量**；`status` 必须由调用方**显式传入**才生效，缺省行为与 v1.20 完全一致（不过滤状态、不删最新 attempt、`older_than_days` 仍必填）。

## 1. 背景与约束

- 当前 `runs.prune` 只有三个删除条件：`older_than_days`（必填）、非最新 `attempt`、`handback.json` 有可解析 `finishedAt`；另有 `limit`（缺省 100）、`dry_run`（缺省 true）、`confirm`（真删必须 true）。
- 分档需求（`ok=30` / `failed=180` / `blocked=365`）目前标注为**未实现、需另立契约**；PM 已定口径是**统一 365、不分档**。
- 核心纪律：**宁可不删**。任何新增过滤维度都不能让"没写 = 按推荐值删"成立；不能让旧服务端忽略新参数后删得更多。

## 2. 参数形态：状态多值 vs 分档常量

### 2.1 方案对比

| 维度 | A. 状态多值白名单（推荐） | B. 服务端分档常量 |
|---|---|---|
| 调用形态 | `status: ["failed","blocked"]` | `tier: "failed"` 或 `tier: "standard"` |
| 语义透明度 | 高：删哪些状态一眼可见 | 低：档位到状态的映射藏在服务端/契约里 |
| 与 `older_than_days` 的关系 | 两个独立条件，AND 组合 | 档位隐含阈值，和显式阈值可能冲突 |
| 默认生效风险 | 无：缺省即不过滤 | 高：档位容易被当成"默认策略"静默生效 |
| 扩展性 | 加状态值即可；`failed` 后续可细分 `blocked_reason` | 每加一档都要改契约与映射 |
| 旧服务端兼容风险 | 旧版会忽略 `status` → **删得更多**，需能力门禁 | 旧版会忽略 `tier` → 同样删得更多 |
| PM 侧计算 | PM 自己决定"哪些状态算旧" | PM 依赖 AW 的档位定义 |

### 2.2 推荐

**采用 A：`status` 为可选字符串数组，白名单语义；不新增 `tier` 参数。** 理由：

1. 删除范围可被调用方逐项审计，符合"回执不说谎、删除可复核"。
2. 不引入"档位 → 阈值"的隐式映射，`older_than_days` 仍是唯一年龄阈值，避免两套阈值打架。
3. 分档需求若未来确有必要，**在 PM 侧展开为状态数组**（例如 `ok` → `["ok"]`，`failed` → `["failed"]`），AW 不把业务分档写进删除接口。
4. 不提供 `tier` 也意味着**不存在"默认档位"**，从接口形态上消除"分档静默生效"的可能。

### 2.3 参数定义（提案）

| 参数 | 类型 | 必填 | 缺省 | 语义 |
|---|---|---|---|---|
| `status` | `string[]` | 否 | **缺省 = 不过滤**（与 v1.20 一致） | 白名单；仅 `handback.status` **精确等于**数组内某值才进入候选 |
| `older_than_days` | integer `1..=3650` | 是 | 无（缺省 `invalid_retention`） | 年龄阈值；与 `status` 为 **AND** |
| `limit` | integer `1..=100` | 否 | 100 | 过滤后的候选批次上限 |
| `dry_run` | boolean | 否 | true | 预览；零写入 |
| `confirm` | boolean | 否 | false | 真删必须 `dry_run=false` + `confirm=true` |

`status` 取值集合：`ok` / `failed` / `blocked`（与 `handback.status` 现有三值一一对应；**大小写敏感**）。`blocked` 下的 `timeout` 等细分**不在本提案范围**，不新增 `blocked_reason` 过滤。

校验规则（失败即整单拒绝，不删任何目录、不写审计）：

- 非数组、空数组、含非字符串、含未知值、含重复值 → `invalid_retention: status 必须是非空数组，元素只能取 ok/failed/blocked`；
- 重复值先去重再判定，不视为错误（去重后仍非空）；
- `status: null` 视为非法，不当作"不过滤"（避免"写错即放开"）。

## 3. 与 `older_than_days` / `limit` 的组合语义

候选判定顺序（短路，任一不满足即 skipped，不进入候选）：

1. 目录是合法 `task_id`、非软链、非最新 attempt；
2. `dispatch.json` 可读；
3. `handback.json` 可读，`finishedAt` 存在且可解析、非未来时间；
4. `age_days >= older_than_days`；
5. **`handback.status` 在 `status` 白名单内**（`status` 缺省时跳过本步）。

`limit` 在**过滤完成之后**生效：`candidates_total` = 全部命中数，`truncated = candidates_total > limit`，批次按 `task_id` 升序、`attempt` 升序取前 `limit` 个。**不存在"先取 100 再过滤导致漏删/少删"的语义**。

`skipped` 新增原因 `status_mismatch`（状态不在白名单）；`handback` 缺 `status` 或值不认识 → `status_unknown`，**永不进入候选**（宁可不删）。`skipped` 只增不减，旧原因不变。

组合示例（提案口径）：

| `status` | `older_than_days` | 结果 |
|---|---|---|
| 缺省 | 365 | 与 v1.20 完全一致：所有状态、非最新、≥365 天 |
| `["failed"]` | 365 | 仅 `failed` 且 ≥365 天、非最新 |
| `["failed","blocked"]` | 180 | 仅 `failed`/`blocked` 且 ≥180 天、非最新 |
| `["ok"]` | 30 | 仅 `ok` 且 ≥30 天、非最新（PM 显式选择，不是默认） |

## 4. 兼容性与能力门禁

### 4.1 对既有调用方

- 不传 `status` 时，候选集合、`candidates_total`、`limit`、`dry_run`、`confirm`、审计记录**全部与 v1.20 相同**；旧调用方零改动。
- 新字段是加法：响应新增 `status_filter`（缺省 `null`，显式时回显数组），`skipped` 新增两个原因；不删除/不改名任何既有字段。
- `runs.list` / `receipt.get` / `task.execute` 不受影响。

### 4.2 旧服务端风险（必须写进发布门槛）

当前 `additionalProperties` 策略是**忽略未知参数**。若 PM 对旧版 AW 传 `status`，旧版会**静默忽略并按"不过滤"删除**——这是"宁可不删"的直接反例。因此：

1. **能力声明**：新版必须在 `tools/list` 的 `runs.prune.inputSchema.properties.status` 中声明该参数（schema 即能力位）。
2. **PM 侧门禁**：PM 只有在确认服务端 schema 含 `status`（或契约版本 ≥ 引入版本）时才允许携带 `status`；不满足则**不得调用真删**，只能 dry-run 或退回旧参数。
3. **禁止平滑降级**：不允许"旧服务端忽略 status 也照删"；发现 schema 不含 `status` 时，PM 必须把该服务端视为不支持状态过滤。
4. 引入版本建议为 **v1.21**（契约升版、`tools/list` 描述与审计同步）；未升版前本文不生效。

## 5. "宁可不删"在分档下如何不被削弱

1. **没有默认档位、没有默认状态**：`status` 缺省 = 不过滤，等价 v1.20；要收窄必须显式写白名单。
2. **分档不落在 AW**：`ok=30` / `failed=180` / `blocked=365` 只是 PM 侧的展开规则；AW 只接受 `status` 数组 + 单一 `older_than_days`，不存在"传 tier 就按推荐阈值删"。
3. **年龄阈值仍必填**：`status` 不能替代 `older_than_days`；缺省仍 `invalid_retention`。
4. **最新 attempt 永不删**：与状态过滤正交，任何白名单都不能选中最新 attempt。
5. **未知/缺失状态不删**：`status_unknown` / `status_mismatch` 只跳过，不猜测、不按 `failed` 处理。
6. **两段式删除不变**：`dry_run` 缺省 true；真删必须显式 `dry_run=false` + `confirm=true`；`confirm` 缺失不删、不写审计。
7. **先记账后删除不变**：plan 行写不进就一个目录都不删；结果 `partial` 仍 `isError=true`。
8. **审计可复核**：`runs/prune.jsonl` 的 plan/result 行必须记录生效的 `status_filter`、`older_than_days`、`limit`、`batch_id`，事后可逐条核对"为什么这个 attempt 被删/没被删"。
9. **失败即整单拒绝**：`status` 形态非法时不做"尽力而为"的删减，直接 `invalid_retention`，零删除、零审计。
10. **旧服务端不静默放开**：见 §4.2，能力门禁不满足就不允许带 `status` 真删。

## 6. 需要的用例清单（实现时先红后绿，只增不减）

### 6.1 schema / 参数校验

1. `tools/list` 的 `runs.prune.inputSchema` 声明 `status`（`type: array`、`items.enum`、`minItems: 1`、`uniqueItems: true`），描述写明"缺省不过滤"。
2. 缺省 `status`：与 v1.20 行为逐字段一致（回归用例，锁定兼容性）。
3. `status: ["failed"]`：只删 `failed`，`ok`/`blocked` 进 `skipped` 且原因为 `status_mismatch`。
4. `status: ["ok","blocked"]`：多值并集，`failed` 不删。
5. `status: []` → `invalid_retention`，零删除、零审计。
6. `status: ["unknown"]` / 大小写不符（`["Failed"]`）→ `invalid_retention`。
7. `status: "failed"`（字符串而非数组）→ `invalid_retention`。
8. `status: null` → `invalid_retention`（不得当作不过滤）。
9. 重复值 `["failed","failed"]` → 去重后正常执行，`status_filter` 回显去重结果。

### 6.2 组合语义

10. `status + older_than_days` 为 AND：`failed` 但未满阈值 → `not_old_enough`，不删。
11. `status` 不能选中最新 attempt：最新 `failed` 仍 `latest_attempt_retained`。
12. `handback` 缺 `status` → `status_unknown`，不删。
13. `handback.status` 为未知字符串 → `status_unknown`，不删。
14. `limit` 在过滤后生效：构造 3 个命中 + `limit:1`，`candidates_total=3`、`truncated=true`、批次 1 个。
15. `limit` 边界 `1` / `100` 通过，`0` / `101` → `invalid_retention`。

### 6.3 安全与审计

16. dry-run + `status`：零写入、不创建 `.prune-trash`、`audit_written=false`（沿用现有断言口径）。
17. 真删缺 `confirm`：`confirm_required`，零删除、零审计，`status` 合法也不放行。
18. 真删 `dry_run=false` + `confirm=true`：plan/result 两行都含 `status_filter`、`older_than_days`、`limit`、`batch_id`。
19. 混合状态批次：`["ok","failed"]` 下两类都被删，`blocked` 保留；`deleted_total` 与 `failed_total` 分别正确。
20. `partial` 失败路径：单个目录重命名失败 → `partial` + `isError=true`，其余仍按白名单删除，审计可追溯。
21. 旧客户端不传 `status`：`runs.prune` 响应**不出现** `status_filter` 以外的结构变化；旧回执/旧 `prune.jsonl` 可被 `receipt.get` 原样读取。

### 6.4 能力门禁（发布前手工检查项）

22. 旧版服务端（schema 无 `status`）收到带 `status` 的调用：记录"不得真删"的检查结果（当前实现会忽略未知参数，因此这条作为**发布阻断项**，必须由 PM 侧门禁保证）。
23. 新版 `tools/list` 与契约 §8.9 描述一致：`status` 缺省不过滤、无档位、无默认阈值。

## 7. 非目标

- 不引入 `tier` 参数或任何服务端分档映射；
- 不按 `blocked_reason`（`timeout` / `invalid_cwd` 等）细分过滤；
- 不做定时/启动自动清理；
- 不改"只删非最新 attempt、只认 `finishedAt`、缺字段不删、先记账后删除"四条既有纪律；
- 本文不改实现、不改契约版本；若 PM 接受，另开实现批次并升 **v1.21**。
