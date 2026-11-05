// 更新入口可用性判定（纯函数，便于断言）
// 约定：发布流（versions.json）不可达或不存在时一律 fail-closed——禁用入口并给出原因。

export interface UpdateCheckOutcome {
  ok: boolean;
  error?: string | null;
}

export interface UpdateAvailability {
  disabled: boolean;
  reasonKey: "update.publishStreamMissing" | null;
  detail: string;
}

export function resolveUpdateAvailability(
  outcome: UpdateCheckOutcome
): UpdateAvailability {
  if (outcome.ok) {
    return { disabled: false, reasonKey: null, detail: "" };
  }
  return {
    disabled: true,
    reasonKey: "update.publishStreamMissing",
    detail: (outcome.error ?? "").trim(),
  };
}
