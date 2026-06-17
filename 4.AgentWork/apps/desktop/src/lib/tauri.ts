// Tauri 后端命令封装
// 前端通过这些函数调用 Rust 后端

import { invoke } from "@tauri-apps/api/core";

// ===== 类型定义 =====

export interface SnapshotSummary {
  id: string;
  timestamp: string;
  message: string;
  snapshot_type: "wip" | "progress" | "candidate" | "release";
  build_status: "green" | "yellow" | "red" | "unknown";
  author: string;
}

export interface FileChange {
  path: string;
  status: "added" | "modified" | "deleted";
  additions: number;
  deletions: number;
}

export interface SnapshotDetail {
  id: string;
  parent_id: string | null;
  timestamp: string;
  message: string;
  ai_summary: string | null;
  snapshot_type: "wip" | "progress" | "candidate" | "release";
  build_status: "green" | "yellow" | "red" | "unknown";
  author: string;
  files_changed: FileChange[];
}

export interface AppInfo {
  name: string;
  version: string;
  description: string;
}

export interface SystemInfo {
  os: string;
  arch: string;
  cpu_count: number;
  total_memory: number;
}

// ===== 命令调用 =====

export async function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

export async function getSystemInfo(): Promise<SystemInfo> {
  return invoke<SystemInfo>("get_system_info");
}

export async function listSnapshots(limit?: number): Promise<SnapshotSummary[]> {
  return invoke<SnapshotSummary[]>("list_snapshots", { limit });
}

export async function getSnapshotDetail(id: string): Promise<SnapshotDetail> {
  return invoke<SnapshotDetail>("get_snapshot_detail", { id });
}

export async function createSnapshot(message?: string): Promise<string> {
  return invoke<string>("create_snapshot", { message });
}

export async function rollbackSnapshot(id: string): Promise<boolean> {
  return invoke<boolean>("rollback_snapshot", { id });
}
