// 任务看板 UI（W9 M3.3 D4）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 9
//
// 功能：
//   1. 需求输入 → 规划工作流（plan_workflow）
//   2. DAG 可视化（按拓扑层级展示任务节点 + 依赖箭头）
//   3. 任务状态看板（按状态分组：Pending / Running / Verifying / Merged / Failed / Rejected）
//   4. 调度控制台（step / 报告事件 / 重置）
//   5. 调度动作日志
//
// MVP 阶段：使用内置示例 DAG，不实际调用 LLM。
// 用户可手动模拟任务完成/失败/验证，观察 DAG 状态机流转。

import { useEffect, useState, useCallback } from "react";
import {
  ClipboardList,
  Play,
  RefreshCw,
  Loader2,
  CheckCircle2,
  AlertCircle,
  ArrowRight,
  RotateCcw,
  XCircle,
  Clock,
  Cpu,
  GitBranch,
  Activity,
  Layers,
  Zap,
} from "lucide-react";
import {
  planWorkflow,
  getCurrentWorkflow,
  stepWorkflow,
  reportTaskCompleted,
  reportTaskMerged,
  reportTaskVerifyFailed,
  reportTaskFailed,
  resetWorkflow,
  type WorkflowOverview,
  type ScheduleActionInfo,
  type TaskInfo,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";

// 状态配置：颜色 + 图标 + 中文标签
const stateConfig: Record<
  string,
  { label: string; color: string; border: string; icon: typeof Clock }
> = {
  Pending: {
    label: "待执行",
    color: "bg-zinc-700/40 text-zinc-300",
    border: "border-zinc-600",
    icon: Clock,
  },
  Running: {
    label: "执行中",
    color: "bg-blue-950/50 text-blue-300",
    border: "border-blue-500",
    icon: Loader2,
  },
  Verifying: {
    label: "验证中",
    color: "bg-amber-950/50 text-amber-300",
    border: "border-amber-500",
    icon: Activity,
  },
  Merged: {
    label: "已合并",
    color: "bg-green-950/50 text-green-300",
    border: "border-green-500",
    icon: CheckCircle2,
  },
  Failed: {
    label: "失败",
    color: "bg-red-950/50 text-red-300",
    border: "border-red-500",
    icon: AlertCircle,
  },
  Rejected: {
    label: "已拒绝",
    color: "bg-zinc-950 text-zinc-500",
    border: "border-zinc-700",
    icon: XCircle,
  },
};

// 角色颜色配置
const roleColors: Record<string, string> = {
  orchestrator: "bg-purple-950/50 text-purple-300 border-purple-700",
  backend: "bg-blue-950/50 text-blue-300 border-blue-700",
  frontend: "bg-pink-950/50 text-pink-300 border-pink-700",
  tester: "bg-emerald-950/50 text-emerald-300 border-emerald-700",
  fullstack: "bg-indigo-950/50 text-indigo-300 border-indigo-700",
};

function getRoleColor(role: string): string {
  return roleColors[role] ?? "bg-zinc-800 text-zinc-300 border-zinc-700";
}

export default function TaskBoard() {
  const [overview, setOverview] = useState<WorkflowOverview | null>(null);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<{
    type: "success" | "error" | "info";
    text: string;
  } | null>(null);

  // 需求输入
  const [requirement, setRequirement] = useState("添加一个健康检查接口 /api/health");
  const [planning, setPlanning] = useState(false);

  // 最近一次调度动作
  const [lastActions, setLastActions] = useState<ScheduleActionInfo[]>([]);

  const refresh = useCallback(async () => {
    try {
      setLoading(true);
      const data = await getCurrentWorkflow();
      setOverview(data);
    } catch (err) {
      // 首次加载未规划时不显示错误
      console.warn("获取工作流失败:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  function showMessage(type: "success" | "error" | "info", text: string) {
    setMessage({ type, text });
    setTimeout(() => setMessage(null), 4000);
  }

  async function handlePlan() {
    if (planning || !requirement.trim()) return;
    try {
      setPlanning(true);
      setLastActions([]);
      const data = await planWorkflow(requirement.trim());
      setOverview(data);
      showMessage(
        "success",
        `工作流已规划：${data.workflow_id}（${data.total_count} 个任务）`
      );
    } catch (err) {
      showMessage("error", String(err));
    } finally {
      setPlanning(false);
    }
  }

  async function handleStep() {
    if (loading) return;
    try {
      setLoading(true);
      const [data, actions] = await stepWorkflow();
      setOverview(data);
      setLastActions(actions);
      if (actions.length === 0) {
        showMessage("info", "无 ready 任务可调度（等待任务完成或全部已终态）");
      } else {
        showMessage(
          "success",
          `调度产生 ${actions.length} 个动作：${actions
            .map((a) => a.action_type)
            .join(", ")}`
        );
      }
    } catch (err) {
      showMessage("error", String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleReportCompleted(taskId: string) {
    try {
      const data = await reportTaskCompleted(taskId);
      setOverview(data);
      showMessage("info", `任务 ${taskId} 已完成，进入验证`);
    } catch (err) {
      showMessage("error", String(err));
    }
  }

  async function handleReportMerged(taskId: string) {
    try {
      const snapshotId = `snap_${Date.now()}`;
      const data = await reportTaskMerged(taskId, snapshotId);
      setOverview(data);
      showMessage("success", `任务 ${taskId} 验证通过，已合并（${snapshotId}）`);
    } catch (err) {
      showMessage("error", String(err));
    }
  }

  async function handleReportVerifyFailed(taskId: string) {
    try {
      const [data, actions] = await reportTaskVerifyFailed(taskId, [
        "类型检查失败：缺少返回类型注解",
      ]);
      setOverview(data);
      setLastActions(actions);
      if (actions.length > 0) {
        showMessage(
          "info",
          `任务 ${taskId} 验证失败，触发 ${actions.length} 个动作（重试/拒绝）`
        );
      } else {
        showMessage("info", `任务 ${taskId} 验证失败`);
      }
    } catch (err) {
      showMessage("error", String(err));
    }
  }

  async function handleReportFailed(taskId: string) {
    try {
      const data = await reportTaskFailed(taskId, "Agent 执行异常：LLM 调用超时");
      setOverview(data);
      showMessage("info", `任务 ${taskId} 执行失败`);
    } catch (err) {
      showMessage("error", String(err));
    }
  }

  async function handleReset() {
    try {
      await resetWorkflow();
      const data = await getCurrentWorkflow();
      setOverview(data);
      setLastActions([]);
      showMessage("info", "工作流已重置");
    } catch (err) {
      showMessage("error", String(err));
    }
  }

  // 按状态分组任务
  const tasksByState = (overview?.tasks ?? []).reduce<
    Record<string, TaskInfo[]>
  >((acc, task) => {
    const key = task.state.state_type;
    if (!acc[key]) acc[key] = [];
    acc[key].push(task);
    return acc;
  }, {});

  const stateOrder = [
    "Pending",
    "Running",
    "Verifying",
    "Merged",
    "Failed",
    "Rejected",
  ];

  return (
    <div className="flex flex-col h-full">
      {/* 顶部标题栏 */}
      <header className="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <h1 className="text-lg font-semibold flex items-center gap-2">
          <ClipboardList className="w-5 h-5" />
          任务看板
          {overview?.workflow_id && (
            <span className="text-xs text-zinc-500 font-mono ml-2">
              {overview.workflow_id}
            </span>
          )}
        </h1>
        <div className="flex items-center gap-2">
          <button
            onClick={refresh}
            disabled={loading}
            className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-zinc-800 hover:bg-zinc-700 disabled:opacity-50 transition-colors"
          >
            <RefreshCw className={cn("w-3 h-3", loading && "animate-spin")} />
            刷新
          </button>
          {overview && (
            <button
              onClick={handleReset}
              disabled={loading || planning}
              className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-zinc-800 hover:bg-zinc-700 disabled:opacity-50 transition-colors"
            >
              <RotateCcw className="w-3 h-3" />
              重置
            </button>
          )}
        </div>
      </header>

      <div className="flex-1 overflow-auto p-6 space-y-6">
        {/* 消息提示 */}
        {message && (
          <div
            className={cn(
              "flex items-start gap-2 text-sm rounded p-3",
              message.type === "success" && "text-green-400 bg-green-950/30",
              message.type === "error" && "text-red-400 bg-red-950/30",
              message.type === "info" && "text-blue-400 bg-blue-950/30"
            )}
          >
            {message.type === "success" ? (
              <CheckCircle2 className="w-4 h-4 mt-0.5 flex-shrink-0" />
            ) : message.type === "error" ? (
              <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
            ) : (
              <Activity className="w-4 h-4 mt-0.5 flex-shrink-0" />
            )}
            <span className="break-all">{message.text}</span>
          </div>
        )}

        {/* 需求输入 + 规划 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
            <Zap className="w-4 h-4" />
            需求规划
          </h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
            <div className="flex gap-2">
              <input
                type="text"
                value={requirement}
                onChange={(e) => setRequirement(e.target.value)}
                placeholder="输入需求，例如：添加一个健康检查接口 /api/health"
                className="flex-1 bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                onKeyDown={(e) => {
                  if (e.key === "Enter") handlePlan();
                }}
              />
              <button
                onClick={handlePlan}
                disabled={planning || !requirement.trim()}
                className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-sm"
              >
                {planning ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <Zap className="w-4 h-4" />
                )}
                <span>{planning ? "规划中..." : "规划工作流"}</span>
              </button>
            </div>
            <p className="text-xs text-zinc-500">
              MVP 阶段使用内置示例 DAG（task-1 后端实现 → task-2 测试用例），W10+
              接入真实 LLM。
            </p>
          </div>
        </section>

        {/* 进度概览 */}
        {overview && overview.total_count > 0 && (
          <section>
            <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
              <Layers className="w-4 h-4" />
              进度概览
            </h2>
            <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
              <div className="flex items-center justify-between text-sm">
                <span className="text-zinc-300">
                  已完成 {overview.completed_count} / {overview.total_count}
                </span>
                <span
                  className={cn(
                    "font-mono",
                    overview.is_finished
                      ? "text-green-400"
                      : "text-brand-400"
                  )}
                >
                  {overview.progress_percent}%
                  {overview.is_finished && " ✓ 完成"}
                </span>
              </div>
              <div className="h-2 bg-zinc-800 rounded-full overflow-hidden">
                <div
                  className={cn(
                    "h-full transition-all duration-500",
                    overview.is_finished
                      ? "bg-gradient-to-r from-green-500 to-emerald-500"
                      : "bg-gradient-to-r from-brand-500 to-accent-500"
                  )}
                  style={{ width: `${overview.progress_percent}%` }}
                />
              </div>
              {overview.requirement && (
                <p className="text-xs text-zinc-500 pt-1">
                  原始需求：{overview.requirement}
                </p>
              )}
            </div>
          </section>
        )}

        {/* DAG 可视化 */}
        {overview && overview.tasks.length > 0 && (
          <section>
            <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
              <GitBranch className="w-4 h-4" />
              DAG 可视化
            </h2>
            <div className="bg-zinc-900 rounded-lg p-6 overflow-x-auto">
              <DagVisualization tasks={overview.tasks} />
            </div>
          </section>
        )}

        {/* 调度控制台 */}
        {overview && overview.total_count > 0 && (
          <section>
            <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
              <Play className="w-4 h-4" />
              调度控制台
            </h2>
            <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
              <div className="flex items-center gap-2">
                <button
                  onClick={handleStep}
                  disabled={loading || overview.is_finished}
                  className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-sm"
                >
                  {loading ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Play className="w-4 h-4" />
                  )}
                  <span>推进调度（step）</span>
                </button>
                <span className="text-xs text-zinc-500">
                  找出 ready 任务并分配（Pending → Running）
                </span>
              </div>

              {/* 最近一次调度动作 */}
              {lastActions.length > 0 && (
                <div className="bg-zinc-800/50 rounded p-3 space-y-2">
                  <p className="text-xs text-zinc-400 font-medium">
                    本次调度动作（{lastActions.length}）
                  </p>
                  {lastActions.map((action, idx) => (
                    <ActionBadge key={idx} action={action} />
                  ))}
                </div>
              )}
            </div>
          </section>
        )}

        {/* 任务看板（按状态分组） */}
        {overview && overview.tasks.length > 0 && (
          <section>
            <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
              <ClipboardList className="w-4 h-4" />
              任务看板（按状态分组）
            </h2>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {stateOrder.map((stateType) => {
                const tasks = tasksByState[stateType] ?? [];
                if (tasks.length === 0) return null;
                const config = stateConfig[stateType];
                return (
                  <div
                    key={stateType}
                    className={cn(
                      "rounded-lg border p-3",
                      config.color,
                      config.border
                    )}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-xs font-medium flex items-center gap-1">
                        <config.icon
                          className={cn(
                            "w-3 h-3",
                            stateType === "Running" && "animate-spin"
                          )}
                        />
                        {config.label}
                      </span>
                      <span className="text-xs text-zinc-500">
                        {tasks.length}
                      </span>
                    </div>
                    <div className="space-y-2">
                      {tasks.map((task) => (
                        <TaskCard
                          key={task.id}
                          task={task}
                          onReportCompleted={handleReportCompleted}
                          onReportMerged={handleReportMerged}
                          onReportVerifyFailed={handleReportVerifyFailed}
                          onReportFailed={handleReportFailed}
                        />
                      ))}
                    </div>
                  </div>
                );
              })}
            </div>
          </section>
        )}

        {/* 调度动作日志 */}
        {overview && overview.action_log.length > 0 && (
          <section>
            <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
              <Activity className="w-4 h-4" />
              调度日志（最近 {overview.action_log.length} 条）
            </h2>
            <div className="bg-zinc-900 rounded-lg p-3 space-y-1 max-h-60 overflow-auto">
              {[...overview.action_log].reverse().map((entry, idx) => (
                <div
                  key={idx}
                  className="flex items-start gap-2 text-xs py-1 border-b border-zinc-800 last:border-0"
                >
                  <span className="text-zinc-600 font-mono flex-shrink-0">
                    {new Date(entry.timestamp).toLocaleTimeString("zh-CN")}
                  </span>
                  <span
                    className={cn(
                      "px-1.5 py-0.5 rounded flex-shrink-0",
                      entry.action_type === "PlanWorkflow" &&
                        "bg-purple-950/50 text-purple-300",
                      entry.action_type === "AssignTask" &&
                        "bg-blue-950/50 text-blue-300",
                      entry.action_type === "TaskCompleted" &&
                        "bg-amber-950/50 text-amber-300",
                      entry.action_type === "TaskMerged" &&
                        "bg-green-950/50 text-green-300",
                      entry.action_type === "TaskFailed" &&
                        "bg-red-950/50 text-red-300",
                      entry.action_type === "TaskVerifyFailed" &&
                        "bg-red-950/50 text-red-300",
                      entry.action_type === "RetryTask" &&
                        "bg-yellow-950/50 text-yellow-300",
                      entry.action_type === "RejectTask" &&
                        "bg-zinc-950 text-zinc-400"
                    )}
                  >
                    {entry.action_type}
                  </span>
                  <span className="text-zinc-400 break-all">
                    {entry.description}
                  </span>
                </div>
              ))}
            </div>
          </section>
        )}

        {/* 空状态 */}
        {(!overview || overview.total_count === 0) && (
          <div className="flex flex-col items-center justify-center py-12 text-center">
            <ClipboardList className="w-12 h-12 text-zinc-700 mb-3" />
            <p className="text-sm text-zinc-500 mb-1">暂无工作流</p>
            <p className="text-xs text-zinc-600">
              输入需求并点击"规划工作流"开始
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

// DAG 可视化组件（按拓扑层级展示）
function DagVisualization({ tasks }: { tasks: TaskInfo[] }) {
  // 计算每个任务的层级（最长依赖路径）
  const levelMap = new Map<string, number>();
  function getLevel(taskId: string): number {
    if (levelMap.has(taskId)) return levelMap.get(taskId)!;
    const task = tasks.find((t) => t.id === taskId);
    if (!task || task.dependencies.length === 0) {
      levelMap.set(taskId, 0);
      return 0;
    }
    const maxDepLevel = Math.max(
      ...task.dependencies.map((dep) => getLevel(dep))
    );
    const level = maxDepLevel + 1;
    levelMap.set(taskId, level);
    return level;
  }

  tasks.forEach((t) => getLevel(t.id));

  // 按层级分组
  const levels: TaskInfo[][] = [];
  const maxLevel = Math.max(...Array.from(levelMap.values()), 0);
  for (let i = 0; i <= maxLevel; i++) {
    levels.push(
      tasks.filter((t) => levelMap.get(t.id) === i)
    );
  }

  return (
    <div className="flex items-start gap-8 min-w-fit">
      {levels.map((levelTasks, levelIdx) => (
        <div key={levelIdx} className="flex items-center gap-4">
          <div className="flex flex-col gap-3">
            <p className="text-xs text-zinc-600 mb-1">层级 {levelIdx}</p>
            {levelTasks.map((task) => (
              <DagNode key={task.id} task={task} />
            ))}
          </div>
          {levelIdx < levels.length - 1 && (
            <div className="flex flex-col items-center justify-center self-stretch pt-6">
              <ArrowRight className="w-5 h-5 text-zinc-600" />
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

// DAG 节点
function DagNode({ task }: { task: TaskInfo }) {
  const config = stateConfig[task.state.state_type] ?? stateConfig.Pending;
  const Icon = config.icon;

  return (
    <div
      className={cn(
        "rounded-lg border p-3 min-w-[200px] max-w-[260px]",
        config.border,
        "bg-zinc-800/50"
      )}
    >
      <div className="flex items-center justify-between mb-1">
        <span className="text-xs font-mono text-zinc-500">{task.id}</span>
        <span
          className={cn(
            "px-1.5 py-0.5 rounded text-xs flex items-center gap-1",
            config.color
          )}
        >
          <Icon
            className={cn(
              "w-3 h-3",
              task.state.state_type === "Running" && "animate-spin"
            )}
          />
          {config.label}
        </span>
      </div>
      <p className="text-sm text-zinc-200 mb-1">{task.title}</p>
      <div className="flex items-center gap-1">
        <span
          className={cn(
            "px-1.5 py-0.5 rounded text-xs border",
            getRoleColor(task.assignee)
          )}
        >
          <Cpu className="w-2.5 h-2.5 inline mr-1" />
          {task.assignee}
        </span>
        {task.dependencies.length > 0 && (
          <span className="text-xs text-zinc-500">
            ← {task.dependencies.join(", ")}
          </span>
        )}
      </div>
    </div>
  );
}

// 任务卡片（看板视图）
function TaskCard({
  task,
  onReportCompleted,
  onReportMerged,
  onReportVerifyFailed,
  onReportFailed,
}: {
  task: TaskInfo;
  onReportCompleted: (taskId: string) => void;
  onReportMerged: (taskId: string) => void;
  onReportVerifyFailed: (taskId: string) => void;
  onReportFailed: (taskId: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const state = task.state.state_type;

  return (
    <div
      className="bg-zinc-800/70 rounded p-2 cursor-pointer hover:bg-zinc-800 transition-colors"
      onClick={() => setExpanded(!expanded)}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1 min-w-0">
          <p className="text-xs font-mono text-zinc-500">{task.id}</p>
          <p className="text-sm text-zinc-200 truncate">{task.title}</p>
        </div>
        <span
          className={cn(
            "px-1.5 py-0.5 rounded text-xs flex-shrink-0",
            getRoleColor(task.assignee)
          )}
        >
          {task.assignee}
        </span>
      </div>

      {expanded && (
        <div className="mt-2 space-y-1.5 text-xs">
          {task.description && (
            <div>
              <span className="text-zinc-500">描述：</span>
              <span className="text-zinc-300">{task.description}</span>
            </div>
          )}
          {task.acceptance_criteria && (
            <div>
              <span className="text-zinc-500">验收：</span>
              <span className="text-zinc-300">{task.acceptance_criteria}</span>
            </div>
          )}
          {task.dependencies.length > 0 && (
            <div>
              <span className="text-zinc-500">依赖：</span>
              <span className="text-zinc-300">
                {task.dependencies.join(", ")}
              </span>
            </div>
          )}
          <div>
            <span className="text-zinc-500">状态详情：</span>
            <span className="text-zinc-400">{task.state.state_detail}</span>
          </div>
          {task.state.branch && (
            <div>
              <span className="text-zinc-500">分支：</span>
              <span className="text-zinc-400 font-mono">
                {task.state.branch}
              </span>
            </div>
          )}
          {task.state.error && (
            <div>
              <span className="text-zinc-500">错误：</span>
              <span className="text-red-400">{task.state.error}</span>
            </div>
          )}
          {task.state.snapshot_id && (
            <div>
              <span className="text-zinc-500">快照：</span>
              <span className="text-green-400 font-mono">
                {task.state.snapshot_id}
              </span>
            </div>
          )}
        </div>
      )}

      {/* 操作按钮（根据状态显示） */}
      {expanded && (
        <div className="mt-2 flex flex-wrap gap-1" onClick={(e) => e.stopPropagation()}>
          {state === "Running" && (
            <>
              <button
                onClick={() => onReportCompleted(task.id)}
                className="px-2 py-1 text-xs rounded bg-amber-700 hover:bg-amber-600 text-white transition-colors"
              >
                完成执行
              </button>
              <button
                onClick={() => onReportFailed(task.id)}
                className="px-2 py-1 text-xs rounded bg-red-800 hover:bg-red-700 text-white transition-colors"
              >
                执行失败
              </button>
            </>
          )}
          {state === "Verifying" && (
            <>
              <button
                onClick={() => onReportMerged(task.id)}
                className="px-2 py-1 text-xs rounded bg-green-700 hover:bg-green-600 text-white transition-colors"
              >
                验证通过
              </button>
              <button
                onClick={() => onReportVerifyFailed(task.id)}
                className="px-2 py-1 text-xs rounded bg-red-800 hover:bg-red-700 text-white transition-colors"
              >
                验证失败
              </button>
            </>
          )}
          {state === "Failed" && (
            <span className="text-xs text-zinc-500">
              重试 {task.state.retry_count}/3（由 step 触发）
            </span>
          )}
          {state === "Pending" && (
            <span className="text-xs text-zinc-500">
              等待依赖完成（由 step 自动分配）
            </span>
          )}
          {state === "Merged" && (
            <span className="text-xs text-green-500">✓ 已完成</span>
          )}
          {state === "Rejected" && (
            <span className="text-xs text-zinc-500">✗ 已拒绝</span>
          )}
        </div>
      )}
    </div>
  );
}

// 调度动作徽章
function ActionBadge({ action }: { action: ScheduleActionInfo }) {
  const config: Record<
    string,
    { color: string; icon: typeof Play; label: string }
  > = {
    AssignTask: {
      color: "text-blue-300",
      icon: Play,
      label: "分配任务",
    },
    RetryTask: {
      color: "text-yellow-300",
      icon: RotateCcw,
      label: "重试任务",
    },
    RejectTask: {
      color: "text-zinc-400",
      icon: XCircle,
      label: "拒绝任务",
    },
  };
  const cfg = config[action.action_type] ?? {
    color: "text-zinc-300",
    icon: Activity,
    label: action.action_type,
  };
  const Icon = cfg.icon;

  return (
    <div className="flex items-start gap-2 text-xs">
      <Icon className={cn("w-3 h-3 mt-0.5 flex-shrink-0", cfg.color)} />
      <div className="flex-1">
        <span className={cfg.color}>{cfg.label}</span>
        <span className="text-zinc-400 ml-1">
          {action.task_title || action.task_id}
        </span>
        {action.assignee && (
          <span className="text-zinc-500 ml-1">→ {action.assignee}</span>
        )}
        {action.branch && (
          <span className="text-zinc-600 ml-1 font-mono">@{action.branch}</span>
        )}
        {action.reason && (
          <span className="text-red-400 ml-1">（{action.reason}）</span>
        )}
        {action.errors.length > 0 && (
          <span className="text-red-400 ml-1">
            （错误: {action.errors.join("; ")}）
          </span>
        )}
      </div>
    </div>
  );
}
