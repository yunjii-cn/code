// 工具函数

import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/** 合并 Tailwind 类名 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** 格式化时间戳为相对时间 */
export function formatRelativeTime(timestamp: string): string {
  const now = new Date();
  const time = new Date(timestamp);
  const diff = now.getTime() - time.getTime();

  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return "刚刚";
  if (minutes < 60) return `${minutes} 分钟前`;
  if (hours < 24) return `${hours} 小时前`;
  if (days < 30) return `${days} 天前`;
  return time.toLocaleDateString("zh-CN");
}

/** 快照类型标签配置 */
export const snapshotTypeConfig = {
  wip: { label: "WIP", color: "bg-zinc-600 text-zinc-200" },
  progress: { label: "进度", color: "bg-blue-600 text-white" },
  candidate: { label: "候选", color: "bg-amber-600 text-white" },
  release: { label: "正式", color: "bg-green-600 text-white" },
} as const;

/** 构建状态图标配置 */
export const buildStatusConfig = {
  green: { label: "✓ 通过", color: "text-green-400" },
  yellow: { label: "⚠ 部分通过", color: "text-amber-400" },
  red: { label: "✗ 失败", color: "text-red-400" },
  unknown: { label: "未知", color: "text-zinc-500" },
} as const;
