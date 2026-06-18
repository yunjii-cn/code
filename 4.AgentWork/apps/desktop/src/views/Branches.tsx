import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  GitBranch,
  Plus,
  Trash2,
  CheckCircle2,
  AlertCircle,
} from "lucide-react";
import {
  listBranches,
  createBranch,
  switchBranch,
  deleteBranch,
  type BranchInfo,
} from "@/lib/tauri";
import { cn, formatRelativeTime } from "@/lib/utils";
import ConfirmDialog from "@/components/ConfirmDialog";

export default function Branches() {
  const navigate = useNavigate();
  const [branches, setBranches] = useState<BranchInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [repoError, setRepoError] = useState<string | null>(null);

  // 新建分支对话框
  const [createOpen, setCreateOpen] = useState(false);
  const [newBranchName, setNewBranchName] = useState("");
  const [createLoading, setCreateLoading] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // 切换分支对话框
  const [switchTarget, setSwitchTarget] = useState<BranchInfo | null>(null);
  const [switchLoading, setSwitchLoading] = useState(false);
  const [switchError, setSwitchError] = useState<string | null>(null);

  // 删除分支对话框
  const [deleteTarget, setDeleteTarget] = useState<BranchInfo | null>(null);
  const [deleteLoading, setDeleteLoading] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  useEffect(() => {
    loadBranches();
  }, []);

  async function loadBranches() {
    try {
      setLoading(true);
      setRepoError(null);
      const data = await listBranches();
      setBranches(data);
    } catch (err) {
      setRepoError(String(err));
      console.error("加载分支失败:", err);
    } finally {
      setLoading(false);
    }
  }

  async function handleCreateConfirm() {
    if (!newBranchName.trim()) {
      setCreateError("请输入分支名");
      return;
    }
    try {
      setCreateLoading(true);
      setCreateError(null);
      await createBranch(newBranchName.trim());
      await loadBranches();
      setCreateOpen(false);
      setNewBranchName("");
    } catch (err) {
      setCreateError(String(err));
    } finally {
      setCreateLoading(false);
    }
  }

  async function handleSwitchConfirm() {
    if (!switchTarget) return;
    try {
      setSwitchLoading(true);
      setSwitchError(null);
      await switchBranch(switchTarget.name);
      await loadBranches();
      setSwitchTarget(null);
    } catch (err) {
      setSwitchError(String(err));
    } finally {
      setSwitchLoading(false);
    }
  }

  async function handleDeleteConfirm() {
    if (!deleteTarget) return;
    try {
      setDeleteLoading(true);
      setDeleteError(null);
      await deleteBranch(deleteTarget.name);
      await loadBranches();
      setDeleteTarget(null);
    } catch (err) {
      setDeleteError(String(err));
    } finally {
      setDeleteLoading(false);
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
          <h1 className="text-lg font-semibold flex items-center gap-2">
            <GitBranch className="w-5 h-5" />
            分支管理
          </h1>
          <p className="text-sm text-zinc-500">共 {branches.length} 个分支</p>
        </div>
        <button
          onClick={() => {
            setCreateOpen(true);
            setCreateError(null);
            setNewBranchName("");
          }}
          disabled={!!repoError}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors"
        >
          <Plus className="w-4 h-4" />
          <span className="text-sm">新建分支</span>
        </button>
      </header>

      {/* 仓库未初始化提示 */}
      {repoError && (
        <div className="m-4 flex items-start gap-3 p-4 rounded-lg bg-amber-950/30 border border-amber-800">
          <AlertCircle className="w-5 h-5 text-amber-400 flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <p className="text-sm text-amber-200 font-medium">仓库未初始化</p>
            <p className="text-xs text-amber-400 mt-1">
              请前往「设置」页面初始化 TimeFlow 仓库后再使用分支功能
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

      {/* 分支列表 */}
      <div className="flex-1 overflow-auto p-6">
        {branches.length === 0 && !repoError ? (
          <div className="flex flex-col items-center justify-center h-full text-zinc-500">
            <GitBranch className="w-12 h-12 mb-4 opacity-50" />
            <p>暂无分支</p>
          </div>
        ) : (
          <div className="bg-zinc-900 rounded-lg overflow-hidden">
            {branches.map((branch, idx) => (
              <div
                key={branch.name}
                className={cn(
                  "flex items-center gap-3 px-4 py-3",
                  idx > 0 && "border-t border-zinc-800"
                )}
              >
                {/* 当前分支标记 */}
                <div className="w-5">
                  {branch.is_current && (
                    <CheckCircle2 className="w-4 h-4 text-green-400" />
                  )}
                </div>

                {/* 分支名 */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <GitBranch className="w-4 h-4 text-zinc-500" />
                    <span className="text-sm font-medium text-zinc-200">
                      {branch.name}
                    </span>
                    {branch.is_current && (
                      <span className="text-xs px-1.5 py-0.5 rounded bg-green-950 text-green-400">
                        当前
                      </span>
                    )}
                  </div>
                  <div className="text-xs text-zinc-500 mt-0.5 flex items-center gap-3">
                    <span>创建于 {formatRelativeTime(branch.created_at)}</span>
                    <span className="font-mono">HEAD: {branch.head.slice(0, 12)}</span>
                  </div>
                </div>

                {/* 操作按钮 */}
                <div className="flex items-center gap-1">
                  {!branch.is_current && (
                    <button
                      onClick={() => {
                        setSwitchTarget(branch);
                        setSwitchError(null);
                      }}
                      className="px-3 py-1.5 text-xs rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors"
                      title="切换到此分支"
                    >
                      切换
                    </button>
                  )}
                  {!branch.is_current && branch.name !== "main" && (
                    <button
                      onClick={() => {
                        setDeleteTarget(branch);
                        setDeleteError(null);
                      }}
                      className="p-1.5 rounded-lg hover:bg-red-950 text-zinc-400 hover:text-red-400 transition-colors"
                      title="删除分支"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* 新建分支对话框 */}
      <ConfirmDialog
        open={createOpen}
        title="新建分支"
        variant="info"
        loading={createLoading}
        confirmText="创建"
        onConfirm={handleCreateConfirm}
        onCancel={() => !createLoading && setCreateOpen(false)}
        description={
          <div className="space-y-3">
            <p>从当前 HEAD 创建新分支：</p>
            <input
              type="text"
              value={newBranchName}
              onChange={(e) => setNewBranchName(e.target.value)}
              placeholder="分支名，例如：feature/login"
              autoFocus
              className="w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreateConfirm();
              }}
            />
            {createError && (
              <p className="text-xs text-red-400 bg-red-950/30 rounded p-2">
                {createError}
              </p>
            )}
          </div>
        }
      />

      {/* 切换分支确认对话框 */}
      <ConfirmDialog
        open={!!switchTarget}
        title="切换分支"
        variant="warning"
        loading={switchLoading}
        confirmText="确认切换"
        onConfirm={handleSwitchConfirm}
        onCancel={() => !switchLoading && setSwitchTarget(null)}
        description={
          <div className="space-y-2">
            <p>确定要切换到以下分支吗？工作区文件将被恢复到该分支的状态。</p>
            {switchTarget && (
              <div className="bg-zinc-800 rounded p-2">
                <p className="text-sm text-zinc-200 flex items-center gap-2">
                  <GitBranch className="w-4 h-4" />
                  {switchTarget.name}
                </p>
              </div>
            )}
            <p className="text-xs text-amber-400 bg-amber-950/30 rounded p-2">
              ⚠️ 未提交的变更可能会丢失，建议先创建快照。
            </p>
            {switchError && (
              <p className="text-xs text-red-400 bg-red-950/30 rounded p-2">
                切换失败：{switchError}
              </p>
            )}
          </div>
        }
      />

      {/* 删除分支确认对话框 */}
      <ConfirmDialog
        open={!!deleteTarget}
        title="删除分支"
        variant="danger"
        loading={deleteLoading}
        confirmText="确认删除"
        onConfirm={handleDeleteConfirm}
        onCancel={() => !deleteLoading && setDeleteTarget(null)}
        description={
          <div className="space-y-2">
            <p>确定要删除以下分支吗？此操作不可撤销。</p>
            {deleteTarget && (
              <div className="bg-zinc-800 rounded p-2">
                <p className="text-sm text-zinc-200 flex items-center gap-2">
                  <GitBranch className="w-4 h-4" />
                  {deleteTarget.name}
                </p>
              </div>
            )}
            <p className="text-xs text-red-400 bg-red-950/30 rounded p-2">
              ⚠️ 分支指针将被删除，但快照历史会保留。
            </p>
            {deleteError && (
              <p className="text-xs text-red-400 bg-red-950/30 rounded p-2">
                删除失败：{deleteError}
              </p>
            )}
          </div>
        }
      />
    </div>
  );
}
