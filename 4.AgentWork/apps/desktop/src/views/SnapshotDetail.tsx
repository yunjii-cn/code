import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { ArrowLeft, RotateCcw, FileCode, Brain } from "lucide-react";
import { getSnapshotDetail, type SnapshotDetail } from "@/lib/tauri";
import { cn, formatRelativeTime, snapshotTypeConfig, buildStatusConfig } from "@/lib/utils";

export default function SnapshotDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [snapshot, setSnapshot] = useState<SnapshotDetail | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (id) loadDetail(id);
  }, [id]);

  async function loadDetail(snapId: string) {
    try {
      setLoading(true);
      const data = await getSnapshotDetail(snapId);
      setSnapshot(data);
    } catch (err) {
      console.error("加载快照详情失败:", err);
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-zinc-500">加载中...</div>
      </div>
    );
  }

  if (!snapshot) {
    return (
      <div className="flex flex-col items-center justify-center h-full">
        <p className="text-zinc-500">快照不存在</p>
        <button
          onClick={() => navigate("/timeline")}
          className="mt-4 text-brand-400 hover:text-brand-300"
        >
          返回时间轴
        </button>
      </div>
    );
  }

  const typeCfg = snapshotTypeConfig[snapshot.snapshot_type];
  const buildCfg = buildStatusConfig[snapshot.build_status];

  return (
    <div className="flex flex-col h-full">
      {/* 顶部栏 */}
      <header className="flex items-center gap-4 px-6 py-4 border-b border-zinc-800">
        <button
          onClick={() => navigate("/timeline")}
          className="p-1.5 rounded-lg hover:bg-zinc-800 transition-colors"
        >
          <ArrowLeft className="w-5 h-5" />
        </button>
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <h1 className="text-lg font-semibold">{snapshot.message}</h1>
            <span
              className={cn(
                "px-1.5 py-0.5 rounded text-xs font-medium",
                typeCfg.color
              )}
            >
              {typeCfg.label}
            </span>
          </div>
          <p className="text-xs text-zinc-500 mt-0.5 font-mono">{snapshot.id}</p>
        </div>
        <button
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-amber-600 hover:bg-amber-700 transition-colors"
          onClick={() => {
            // TODO: 确认对话框
            console.log("回滚到:", snapshot.id);
          }}
        >
          <RotateCcw className="w-4 h-4" />
          <span className="text-sm">回滚到此版本</span>
        </button>
      </header>

      {/* 内容区 */}
      <div className="flex-1 overflow-auto p-6 space-y-6">
        {/* 元数据 */}
        <section className="grid grid-cols-2 gap-4">
          <div className="bg-zinc-900 rounded-lg p-4">
            <p className="text-xs text-zinc-500 mb-1">时间</p>
            <p className="text-sm">{formatRelativeTime(snapshot.timestamp)}</p>
          </div>
          <div className="bg-zinc-900 rounded-lg p-4">
            <p className="text-xs text-zinc-500 mb-1">构建状态</p>
            <p className={cn("text-sm", buildCfg.color)}>{buildCfg.label}</p>
          </div>
          <div className="bg-zinc-900 rounded-lg p-4">
            <p className="text-xs text-zinc-500 mb-1">父快照</p>
            <p className="text-sm font-mono">{snapshot.parent_id || "无"}</p>
          </div>
          <div className="bg-zinc-900 rounded-lg p-4">
            <p className="text-xs text-zinc-500 mb-1">作者</p>
            <p className="text-sm">{snapshot.author}</p>
          </div>
        </section>

        {/* AI 摘要 */}
        {snapshot.ai_summary && (
          <section className="bg-zinc-900 rounded-lg p-4">
            <div className="flex items-center gap-2 mb-2">
              <Brain className="w-4 h-4 text-accent-400" />
              <h2 className="text-sm font-medium text-accent-400">AI 变更摘要</h2>
            </div>
            <p className="text-sm text-zinc-300 leading-relaxed">
              {snapshot.ai_summary}
            </p>
          </section>
        )}

        {/* 文件变更 */}
        <section>
          <h2 className="text-sm font-medium mb-3 flex items-center gap-2">
            <FileCode className="w-4 h-4" />
            文件变更 ({snapshot.files_changed.length})
          </h2>
          <div className="bg-zinc-900 rounded-lg overflow-hidden">
            {snapshot.files_changed.map((file, idx) => (
              <div
                key={file.path}
                className={cn(
                  "flex items-center gap-3 px-4 py-3",
                  idx > 0 && "border-t border-zinc-800"
                )}
              >
                <span
                  className={cn(
                    "w-2 h-2 rounded-full",
                    file.status === "added"
                      ? "bg-green-500"
                      : file.status === "modified"
                      ? "bg-amber-500"
                      : "bg-red-500"
                  )}
                />
                <span className="flex-1 text-sm font-mono text-zinc-300 truncate">
                  {file.path}
                </span>
                <span className="text-xs text-zinc-500">
                  +{file.additions} -{file.deletions}
                </span>
              </div>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
