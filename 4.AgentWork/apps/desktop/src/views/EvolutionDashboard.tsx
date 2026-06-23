// 进化仪表盘（M6.2）
//
// 可视化展示 AI 员工的自进化情况：
//   - 进化指数、记忆数量、会话数量、技能掌握度
//   - 成功率趋势图
//   - 最近进化事件时间轴
//
// MVP：前端使用本地模拟数据 + 后端 get_evolution_stats 兜底

import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  Brain,
  Cpu,
  FileDown,
  FileUp,
  GitBranch,
  Plus,
  RefreshCw,
  Sparkles,
  Trash2,
  TrendingUp,
  Wrench,
  X,
  Zap,
} from "lucide-react";
import {
  getEvolutionStats,
  type EvolutionStats,
  listMemories,
  deleteMemory,
  addMemory,
  exportEvolutionPack,
  importEvolutionPack,
  type MemoryEntryInfo,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";

/** 趋势数据点 */
interface TrendPoint {
  label: string;
  value: number;
}

/** 模拟趋势（后续替换为后端真实时序数据） */
const MOCK_TREND: TrendPoint[] = [
  { label: "W1", value: 42 },
  { label: "W2", value: 48 },
  { label: "W3", value: 55 },
  { label: "W4", value: 61 },
  { label: "W5", value: 68 },
  { label: "W6", value: 74 },
  { label: "W7", value: 82 },
];

/** 模拟事件 */
const MOCK_EVENTS = [
  { id: 1, time: "10:23", text: "客服员工学会了「退换货流程」技能", type: "skill" },
  { id: 2, time: "09:41", text: "开发员工记住了项目代码风格偏好", type: "memory" },
  { id: 3, time: "昨天", text: "销售员工完成 12 次客户跟进，成功率 91%", type: "session" },
  { id: 4, time: "昨天", text: "系统根据反馈自动优化了 prompt 权重", type: "evolve" },
  { id: 5, time: "3 天前", text: "教育助教新增「启发式引导」话术示例", type: "skill" },
];

const TYPE_ICON: Record<string, React.ElementType> = {
  skill: Wrench,
  memory: Brain,
  session: Activity,
  evolve: Zap,
};

const TYPE_COLOR: Record<string, string> = {
  skill: "text-blue-400 bg-blue-950/40 border-blue-800",
  memory: "text-purple-400 bg-purple-950/40 border-purple-800",
  session: "text-green-400 bg-green-950/40 border-green-800",
  evolve: "text-amber-400 bg-amber-950/40 border-amber-800",
};

/** 数字动画卡片 */
function StatCard({
  icon: Icon,
  label,
  value,
  unit,
  trend,
  color,
}: {
  icon: React.ElementType;
  label: string;
  value: number;
  unit?: string;
  trend?: number;
  color: string;
}) {
  return (
    <div className="relative overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 backdrop-blur-sm transition-all hover:border-zinc-700 hover:bg-zinc-900">
      <div
        className={cn(
          "absolute -right-4 -top-4 h-20 w-20 rounded-full blur-2xl opacity-20",
          color
        )}
      />
      <div className="relative flex items-start justify-between">
        <div>
          <p className="text-xs text-zinc-500">{label}</p>
          <p className="mt-1 text-2xl font-semibold tracking-tight text-zinc-100">
            {value.toLocaleString()}
            {unit && (
              <span className="ml-1 text-sm font-normal text-zinc-500">{unit}</span>
            )}
          </p>
          {trend !== undefined && (
            <p
              className={cn(
                "mt-1 flex items-center gap-1 text-xs",
                trend >= 0 ? "text-green-400" : "text-red-400"
              )}
            >
              <TrendingUp className="w-3 h-3" />
              {trend >= 0 ? "+" : ""}
              {trend}% 较上周
            </p>
          )}
        </div>
        <div
          className={cn(
            "flex h-10 w-10 items-center justify-center rounded-lg border",
            color.replace("blur-2xl", ""),
            "bg-opacity-10"
          )}
        >
          <Icon className={cn("h-5 w-5", color.replace("bg-", "text-").split(" ")[0])} />
        </div>
      </div>
    </div>
  );
}

/** 简单 SVG 折线图 */
function TrendChart({ data }: { data: TrendPoint[] }) {
  const max = Math.max(...data.map((d) => d.value), 100);
  const width = 600;
  const height = 160;
  const padding = 24;
  const chartW = width - padding * 2;
  const chartH = height - padding * 2;

  const points = useMemo(() => {
    return data.map((d, i) => {
      const x = padding + (i / (data.length - 1)) * chartW;
      const y = padding + chartH - (d.value / max) * chartH;
      return { x, y, label: d.label, value: d.value };
    });
  }, [data, max]);

  const pathD = useMemo(() => {
    return points
      .map((p, i) => `${i === 0 ? "M" : "L"} ${p.x} ${p.y}`)
      .join(" ");
  }, [points]);

  const areaD = useMemo(() => {
    const first = points[0];
    const last = points[points.length - 1];
    return `${pathD} L ${last.x} ${height - padding} L ${first.x} ${height - padding} Z`;
  }, [pathD, points]);

  return (
    <div className="w-full overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/60 p-4">
      <div className="mb-4 flex items-center justify-between">
        <h3 className="text-sm font-medium text-zinc-300 flex items-center gap-2">
          <Activity className="w-4 h-4 text-brand-400" />
          进化趋势
        </h3>
        <span className="text-xs text-zinc-500">近 7 周成功率 (%)</span>
      </div>
      <svg viewBox={`0 0 ${width} ${height}`} className="w-full" preserveAspectRatio="none">
        <defs>
          <linearGradient id="trendGradient" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#3b82f6" stopOpacity="0.35" />
            <stop offset="100%" stopColor="#3b82f6" stopOpacity="0" />
          </linearGradient>
        </defs>
        <path d={areaD} fill="url(#trendGradient)" />
        <path d={pathD} fill="none" stroke="#3b82f6" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" />
        {points.map((p, i) => (
          <g key={i}>
            <circle cx={p.x} cy={p.y} r="4" fill="#18181b" stroke="#3b82f6" strokeWidth="2" />
            <text x={p.x} y={height - 6} textAnchor="middle" fill="#71717a" fontSize="10">
              {p.label}
            </text>
            <text x={p.x} y={p.y - 10} textAnchor="middle" fill="#a1a1aa" fontSize="10">
              {p.value}%
            </text>
          </g>
        ))}
      </svg>
    </div>
  );
}

/** 事件时间轴 */
function EvolutionTimeline() {
  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4">
      <h3 className="mb-4 text-sm font-medium text-zinc-300 flex items-center gap-2">
        <GitBranch className="w-4 h-4 text-brand-400" />
        最近进化事件
      </h3>
      <div className="space-y-4">
        {MOCK_EVENTS.map((event, idx) => {
          const Icon = TYPE_ICON[event.type] || Sparkles;
          return (
            <div key={event.id} className="flex gap-3">
              <div className="flex flex-col items-center">
                <div
                  className={cn(
                    "flex h-8 w-8 items-center justify-center rounded-full border",
                    TYPE_COLOR[event.type]
                  )}
                >
                  <Icon className="h-4 w-4" />
                </div>
                {idx < MOCK_EVENTS.length - 1 && (
                  <div className="mt-2 h-full w-px bg-zinc-800" />
                )}
              </div>
              <div className="pb-4">
                <p className="text-xs text-zinc-500">{event.time}</p>
                <p className="mt-0.5 text-sm text-zinc-300">{event.text}</p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default function EvolutionDashboard() {
  const [stats, setStats] = useState<EvolutionStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [memories, setMemories] = useState<MemoryEntryInfo[]>([]);
  const [activeEmployee, setActiveEmployee] = useState("customer_service");
  const [showAddMemory, setShowAddMemory] = useState(false);
  const [newMemoryContent, setNewMemoryContent] = useState("");
  const [newMemoryKind, setNewMemoryKind] = useState("Fact");
  async function refresh() {
    setLoading(true);
    try {
      const data = await getEvolutionStats();
      setStats(data);
    } catch {
      setStats({ evolution_index: 78, memory_count: 156, session_count: 42, skill_mastery: 85, success_rate: 91, active_employees: 6 });
    }
    try {
      const mems = await listMemories(activeEmployee);
      setMemories(mems);
    } catch {
      setMemories([]);
    }
    setLoading(false);
  }

  useEffect(() => { refresh(); }, [activeEmployee]);

  const data = stats ?? { evolution_index: 78, memory_count: 156, session_count: 42, skill_mastery: 85, success_rate: 91, active_employees: 6 };

  async function handleAddMemory() {
    if (!newMemoryContent.trim()) return;
    try {
      await addMemory(activeEmployee, newMemoryContent.trim(), newMemoryKind);
      setNewMemoryContent("");
      setShowAddMemory(false);
      refresh();
    } catch (e) { console.error(e); }
  }

  async function handleDeleteMemory(id: string) {
    try {
      await deleteMemory(id);
      refresh();
    } catch (e) { console.error(e); }
  }

  async function handleExport() {
    try {
      const json = await exportEvolutionPack(activeEmployee);
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${activeEmployee}_evolution_${new Date().toISOString().slice(0, 10)}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) { console.error(e); }
  }

  async function handleImport() {
    try {
      const [fileHandle] = await (window as any).showOpenFilePicker({ types: [{ description: "进化包", accept: { "application/json": [".json"] } }] });
      const file = await fileHandle.getFile();
      const text = await file.text();
      const result = await importEvolutionPack(text);
      alert(result);
      refresh();
    } catch (e) { /* user cancelled */ }
  }

  return (
    <div className="flex flex-col h-full">
      <header className="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <h1 className="text-lg font-semibold flex items-center gap-2">
          <Sparkles className="w-5 h-5 text-brand-400" />
          进化仪表盘
        </h1>
        <div className="flex items-center gap-2">
          <select
            value={activeEmployee}
            onChange={(e) => setActiveEmployee(e.target.value)}
            className="bg-zinc-800 text-xs rounded-lg px-2 py-1 border border-zinc-700 focus:border-brand-500 focus:outline-none"
          >
            {["customer_service","developer","assistant","education_teacher","sales_followup","finance_auditor"].map((id) => (
              <option key={id} value={id}>{id}</option>
            ))}
          </select>
          <button onClick={handleExport} className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-zinc-800 hover:bg-zinc-700 transition-colors" title="导出进化包">
            <FileDown className="w-3 h-3" />
          </button>
          <button onClick={handleImport} className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-zinc-800 hover:bg-zinc-700 transition-colors" title="导入进化包">
            <FileUp className="w-3 h-3" />
          </button>
          <button onClick={refresh} disabled={loading} className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-zinc-800 hover:bg-zinc-700 disabled:opacity-50 transition-colors">
            <RefreshCw className={`w-3 h-3 ${loading ? "animate-spin" : ""}`} />
            刷新
          </button>
        </div>
      </header>

      <div className="flex-1 overflow-auto p-6 space-y-6">
        {/* 指标卡片区 */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          <StatCard icon={Sparkles} label="进化指数" value={data.evolution_index} unit="/100" trend={12} color="bg-brand-500" />
          <StatCard icon={Brain} label="记忆数量" value={data.memory_count} trend={8} color="bg-purple-500" />
          <StatCard icon={Activity} label="会话数量" value={data.session_count} trend={23} color="bg-green-500" />
          <StatCard icon={Wrench} label="技能掌握度" value={data.skill_mastery} unit="%" trend={5} color="bg-blue-500" />
          <StatCard icon={TrendingUp} label="任务成功率" value={data.success_rate} unit="%" trend={3} color="bg-emerald-500" />
          <StatCard icon={Cpu} label="活跃员工" value={data.active_employees} color="bg-amber-500" />
        </div>

        {/* 趋势 + 事件 */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <TrendChart data={MOCK_TREND} />
          <EvolutionTimeline />
        </div>

        {/* 记忆管理面板 */}
        <div className="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-medium text-zinc-300 flex items-center gap-2">
              <Brain className="w-4 h-4 text-purple-400" />
              记忆管理（{memories.length} 条）
            </h3>
            <button onClick={() => setShowAddMemory(!showAddMemory)} className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-brand-600 hover:bg-brand-700 transition-colors">
              <Plus className="w-3 h-3" />
              添加记忆
            </button>
          </div>

          {showAddMemory && (
            <div className="mb-3 p-3 rounded-lg bg-zinc-800/50 space-y-2">
              <div className="flex gap-2">
                <select value={newMemoryKind} onChange={(e) => setNewMemoryKind(e.target.value)} className="bg-zinc-900 text-xs rounded px-2 py-1 border border-zinc-700">
                  <option value="Fact">事实</option>
                  <option value="Preference">偏好</option>
                  <option value="Procedure">流程</option>
                </select>
                <input
                  type="text"
                  value={newMemoryContent}
                  onChange={(e) => setNewMemoryContent(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleAddMemory()}
                  placeholder="输入记忆内容..."
                  className="flex-1 bg-zinc-900 text-xs rounded px-2 py-1 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                  autoFocus
                />
                <button onClick={handleAddMemory} className="px-2 py-1 text-xs rounded bg-brand-600 hover:bg-brand-700">保存</button>
                <button onClick={() => setShowAddMemory(false)} className="px-2 py-1 text-xs rounded bg-zinc-700 hover:bg-zinc-600"><X className="w-3 h-3" /></button>
              </div>
            </div>
          )}

          {memories.length === 0 ? (
            <p className="text-sm text-zinc-500 text-center py-4">暂无记忆，点击"添加记忆"开始记录</p>
          ) : (
            <div className="space-y-2 max-h-64 overflow-auto">
              {memories.map((mem) => (
                <div key={mem.id} className="flex items-start justify-between p-2 rounded-lg bg-zinc-800/30 hover:bg-zinc-800/60 transition-colors">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className={cn(
                        "text-xs px-1.5 py-0.5 rounded",
                        mem.kind === "Fact" && "bg-blue-950/40 text-blue-400",
                        mem.kind === "Preference" && "bg-purple-950/40 text-purple-400",
                        mem.kind === "Procedure" && "bg-amber-950/40 text-amber-400"
                      )}>
                        {{ Fact: "事实", Preference: "偏好", Procedure: "流程" }[mem.kind] || mem.kind}
                      </span>
                      <span className="text-xs text-zinc-500">优先级 {mem.priority}</span>
                    </div>
                    <p className="text-sm text-zinc-300 mt-1 truncate">{mem.content}</p>
                  </div>
                  <button onClick={() => handleDeleteMemory(mem.id)} className="p-1 text-zinc-500 hover:text-red-400 transition-colors flex-shrink-0">
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* 能力雷达占位 */}
        <div className="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4">
          <h3 className="mb-3 text-sm font-medium text-zinc-300 flex items-center gap-2">
            <Cpu className="w-4 h-4 text-brand-400" />
            多维度能力评估
          </h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {[
              { label: "代码能力", score: 88 },
              { label: "沟通表达", score: 92 },
              { label: "工具调用", score: 79 },
              { label: "学习速度", score: 85 },
            ].map((item) => (
              <div key={item.label} className="rounded-lg bg-zinc-800/50 p-3">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs text-zinc-400">{item.label}</span>
                  <span className="text-xs font-medium text-brand-400">{item.score}</span>
                </div>
                <div className="h-1.5 w-full rounded-full bg-zinc-700">
                  <div className="h-1.5 rounded-full bg-brand-500 transition-all duration-700" style={{ width: `${item.score}%` }} />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
