import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Plus, RotateCcw, Clock } from "lucide-react";
import { listSnapshots, createSnapshot, type SnapshotSummary } from "@/lib/tauri";
import { cn, formatRelativeTime, snapshotTypeConfig, buildStatusConfig } from "@/lib/utils";

export default function Timeline() {
  const navigate = useNavigate();
  const [snapshots, setSnapshots] = useState<SnapshotSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    loadSnapshots();
  }, []);

  async function loadSnapshots() {
    try {
      setLoading(true);
      const data = await listSnapshots(50);
      setSnapshots(data);
    } catch (err) {
      console.error("加载快照失败:", err);
    } finally {
      setLoading(false);
    }
  }

  async function handleCreateSnapshot() {
    try {
      setCreating(true);
      await createSnapshot("手动快照");
      await loadSnapshots();
    } catch (err) {
      console.error("创建快照失败:", err);
    } finally {
      setCreating(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-zinc-500">加载中...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* 顶部栏 */}
      <header className="flex items-center justify-between px-6 py-4 border-b border-zinc-800">
        <div>
          <h1 className="text-lg font-semibold">时间轴</h1>
          <p className="text-sm text-zinc-500">共 {snapshots.length} 个快照</p>
        </div>
        <button
          onClick={handleCreateSnapshot}
          disabled={creating}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors"
        >
          <Plus className="w-4 h-4" />
          <span className="text-sm">{creating ? "创建中..." : "创建快照"}</span>
        </button>
      </header>

      {/* 时间轴列表 */}
      <div className="flex-1 overflow-auto px-6 py-4">
        {snapshots.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-zinc-500">
            <Clock className="w-12 h-12 mb-4 opacity-50" />
            <p>暂无快照</p>
            <p className="text-sm mt-1">修改文件后会自动创建快照</p>
          </div>
        ) : (
          <div className="relative">
            {/* 时间轴竖线 */}
            <div className="absolute left-4 top-2 bottom-2 w-px bg-zinc-800" />

            <div className="space-y-3">
              {snapshots.map((snap) => {
                const typeCfg = snapshotTypeConfig[snap.snapshot_type];
                const buildCfg = buildStatusConfig[snap.build_status];

                return (
                  <div
                    key={snap.id}
                    onClick={() => navigate(`/snapshot/${snap.id}`)}
                    className="relative flex items-start gap-4 p-4 rounded-lg hover:bg-zinc-900 cursor-pointer transition-colors group"
                  >
                    {/* 时间轴节点 */}
                    <div
                      className={cn(
                        "relative z-10 w-3 h-3 rounded-full mt-1.5 ring-4 ring-zinc-950",
                        snap.build_status === "green"
                          ? "bg-green-500"
                          : snap.build_status === "yellow"
                          ? "bg-amber-500"
                          : snap.build_status === "red"
                          ? "bg-red-500"
                          : "bg-zinc-600"
                      )}
                    />

                    {/* 快照信息 */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span
                          className={cn(
                            "px-1.5 py-0.5 rounded text-xs font-medium",
                            typeCfg.color
                          )}
                        >
                          {typeCfg.label}
                        </span>
                        <span className={cn("text-xs", buildCfg.color)}>
                          {buildCfg.label}
                        </span>
                        <span className="text-xs text-zinc-500">
                          {formatRelativeTime(snap.timestamp)}
                        </span>
                      </div>
                      <p className="text-sm text-zinc-200 truncate group-hover:text-white">
                        {snap.message}
                      </p>
                      <p className="text-xs text-zinc-600 mt-0.5 font-mono">
                        {snap.id}
                      </p>
                    </div>

                    {/* 回滚按钮 */}
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        // TODO: 确认对话框
                        console.log("回滚到:", snap.id);
                      }}
                      className="opacity-0 group-hover:opacity-100 p-1.5 rounded hover:bg-zinc-800 transition-all"
                      title="回滚到此版本"
                    >
                      <RotateCcw className="w-4 h-4 text-zinc-400" />
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
