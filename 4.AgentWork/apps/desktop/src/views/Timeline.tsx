import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Plus, RotateCcw, Clock, AlertCircle, GitBranch } from "lucide-react";
import {
  listSnapshots,
  createSnapshot,
  rollbackSnapshot,
  currentBranch,
  type SnapshotSummary,
} from "@/lib/tauri";
import { cn, formatRelativeTime, snapshotTypeConfig, buildStatusConfig } from "@/lib/utils";
import ConfirmDialog from "@/components/ConfirmDialog";
import { useI18n } from "@/i18n";

export default function Timeline() {
  const navigate = useNavigate();
  const { t } = useI18n();
  const [snapshots, setSnapshots] = useState<SnapshotSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [branchName, setBranchName] = useState<string | null>(null);
  const [repoError, setRepoError] = useState<string | null>(null);

  // 回滚对话框状态
  const [rollbackTarget, setRollbackTarget] = useState<SnapshotSummary | null>(null);
  const [rollbackLoading, setRollbackLoading] = useState(false);
  const [rollbackError, setRollbackError] = useState<string | null>(null);

  useEffect(() => {
    loadSnapshots();
    currentBranch()
      .then(setBranchName)
      .catch((err) => setRepoError(String(err)));
  }, []);

  async function loadSnapshots() {
    try {
      setLoading(true);
      setRepoError(null);
      const data = await listSnapshots(50);
      setSnapshots(data);
    } catch (err) {
      setRepoError(String(err));
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
      alert(`创建快照失败: ${err}`);
    } finally {
      setCreating(false);
    }
  }

  async function handleRollbackConfirm() {
    if (!rollbackTarget) return;
    try {
      setRollbackLoading(true);
      setRollbackError(null);
      await rollbackSnapshot(rollbackTarget.id);
      await loadSnapshots();
      setRollbackTarget(null);
    } catch (err) {
      setRollbackError(String(err));
    } finally {
      setRollbackLoading(false);
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
      <header className="flex items-center justify-between px-4 sm:px-6 py-4 border-b border-zinc-800">
        <div>
          <h1 className="text-lg font-semibold flex items-center gap-2">
            {t("timeline.title")}
            {branchName && (
              <span className="flex items-center gap-1 text-xs font-normal text-zinc-400 bg-zinc-800 px-2 py-0.5 rounded">
                <GitBranch className="w-3 h-3" />
                {branchName}
              </span>
            )}
          </h1>
          <p className="text-sm text-zinc-500">{t("timeline.snapshotCount", { n: snapshots.length })}</p>
        </div>
        <button
          onClick={handleCreateSnapshot}
          disabled={creating || !!repoError}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors"
        >
          <Plus className="w-4 h-4" />
          <span className="text-sm">{creating ? "创建中..." : "创建快照"}</span>
        </button>
      </header>

      {/* 仓库未初始化提示 */}
      {repoError && (
        <div className="m-4 flex items-start gap-3 p-4 rounded-lg bg-amber-950/30 border border-amber-800">
          <AlertCircle className="w-5 h-5 text-amber-400 flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <p className="text-sm text-amber-200 font-medium">仓库未初始化</p>
            <p className="text-xs text-amber-400 mt-1">
              请前往「设置」页面初始化 TimeFlow 仓库后再使用版本控制功能
            </p>
            <button
              onClick={() => navigate("/settings")}
              className="mt-2 text-xs text-amber-300 hover:text-amber-200 underline"
            >
              前往设置 →
            </button>
          </div>
        </div>
      )}

      {/* 时间轴列表 */}
      <div className="flex-1 overflow-auto px-6 py-4">
        {snapshots.length === 0 && !repoError ? (
          <div className="flex flex-col items-center justify-center h-full text-zinc-500">
            <Clock className="w-12 h-12 mb-4 opacity-50" />
            <p>暂无快照</p>
            <p className="text-sm mt-1">修改文件后会自动创建快照，或点击右上角手动创建</p>
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
                        <span className={cn("text-xs", buildCfg.color)}>{buildCfg.label}</span>
                        <span className="text-xs text-zinc-500">
                          {formatRelativeTime(snap.timestamp)}
                        </span>
                      </div>
                      <p className="text-sm text-zinc-200 truncate group-hover:text-white">
                        {snap.message}
                      </p>
                      <p className="text-xs text-zinc-600 mt-0.5 font-mono">{snap.id}</p>
                    </div>

                    {/* 回滚按钮 */}
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setRollbackTarget(snap);
                        setRollbackError(null);
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

      {/* 回滚确认对话框 */}
      <ConfirmDialog
        open={!!rollbackTarget}
        title={t("timeline.rollbackConfirmTitle")}
        variant="warning"
        loading={rollbackLoading}
        confirmText={t("timeline.rollbackConfirm")}
        onConfirm={handleRollbackConfirm}
        onCancel={() => !rollbackLoading && setRollbackTarget(null)}
        description={
          <div className="space-y-2">
            <p>{t("timeline.rollbackConfirmDesc")}</p>
            {rollbackTarget && (
              <div className="bg-zinc-800 rounded p-2 space-y-1">
                <p className="text-xs text-zinc-400">{t("timeline.rollbackSnapshotId")}</p>
                <p className="text-xs font-mono text-zinc-300 break-all">{rollbackTarget.id}</p>
                <p className="text-xs text-zinc-400 mt-2">{t("timeline.rollbackSnapshotMsg")}</p>
                <p className="text-sm text-zinc-200">{rollbackTarget.message}</p>
              </div>
            )}
            <p className="text-xs text-amber-400 bg-amber-950/30 rounded p-2">
              {t("timeline.rollbackTip")}
            </p>
            {rollbackError && (
              <p className="text-xs text-red-400 bg-red-950/30 rounded p-2">
                {t("timeline.rollbackFailPrefix")}{rollbackError}
              </p>
            )}
          </div>
        }
      />
    </div>
  );
}
