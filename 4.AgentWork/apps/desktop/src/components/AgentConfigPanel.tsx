// 右侧栏：当前 Agent / 对话配置面板
// 参考扣子 Agent 设置：模型、技能、工具、置顶、记忆等

import { useState } from "react";
import {
  X,
  Bot,
  MessageSquare,
  Cpu,
  Zap,
  Wrench,
  Pin,
  Brain,
  FolderOpen,
  Plus,
  ChevronRight,
  Sparkles,
} from "lucide-react";
import { clsx } from "clsx";
import { useI18n } from "@/i18n";
import { useTheme } from "@/lib/theme-store";

interface SkillItem {
  id: string;
  name: string;
  desc: string;
  enabled: boolean;
}

interface ToolItem {
  id: string;
  name: string;
  enabled: boolean;
}

interface AgentConfigPanelProps {
  onClose: () => void;
}

const MOCK_SKILLS: SkillItem[] = [
  { id: "1", name: "板块热度分析", desc: "实时分析市场板块热度", enabled: true },
  { id: "2", name: "全维度股票风险雷达", desc: "多维度风险扫描", enabled: true },
  { id: "3", name: "量化股票分析", desc: "量化因子选股分析", enabled: false },
];

const MOCK_TOOLS: ToolItem[] = [
  { id: "file", name: "文件读取", enabled: true },
  { id: "web", name: "网页搜索", enabled: true },
  { id: "code", name: "代码执行", enabled: false },
];

export default function AgentConfigPanel({ onClose }: AgentConfigPanelProps) {
  const { t } = useI18n();
  const { isLight } = useTheme();
  const [model, setModel] = useState("glm5.2");
  const [pinned, setPinned] = useState(false);
  const [memory, setMemory] = useState(true);
  const [skills, setSkills] = useState<SkillItem[]>(MOCK_SKILLS);
  const [tools, setTools] = useState<ToolItem[]>(MOCK_TOOLS);

  const toggleSkill = (id: string) => {
    setSkills((prev) =>
      prev.map((s) => (s.id === id ? { ...s, enabled: !s.enabled } : s))
    );
  };

  const toggleTool = (id: string) => {
    setTools((prev) =>
      prev.map((t) => (t.id === id ? { ...t, enabled: !t.enabled } : t))
    );
  };

  return (
    <div className="flex flex-col h-full bg-zinc-900 border-l border-zinc-800">
      {/* 顶部标题 */}
      <header className="flex items-center justify-between px-4 py-3 border-b border-zinc-800 flex-shrink-0">
        <div className="flex items-center gap-2">
          <Bot className="w-4 h-4 text-brand-400" />
          <h2 className="text-sm font-semibold text-zinc-200">
            {t("agentConfig.title")}
          </h2>
        </div>
        <button
          onClick={onClose}
          className="p-1 rounded-lg hover:bg-zinc-800 text-zinc-500 hover:text-zinc-300 transition-colors"
        >
          <X className="w-4 h-4" />
        </button>
      </header>

      <div className="flex-1 overflow-y-auto p-4 space-y-5">
        {/* Agent 信息卡片 */}
        <div className="flex items-center gap-3 p-3 rounded-xl bg-zinc-800/40 border border-zinc-800">
          <div className="w-12 h-12 rounded-full bg-gradient-to-br from-brand-500 to-brand-700 flex items-center justify-center flex-shrink-0">
            <Bot className="w-6 h-6 text-white" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-1.5">
              <span className="text-sm font-semibold text-zinc-100 truncate">
                {t("agentConfig.defaultAgent")}
              </span>
              <span className="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/15 text-emerald-400 border border-emerald-500/20">
                {t("agentConfig.online")}
              </span>
            </div>
            <p className="text-[11px] text-zinc-500 truncate">
              {t("agentConfig.agentDesc")}
            </p>
          </div>
        </div>

        {/* 模型设置 */}
        <section className="space-y-2">
          <div className="flex items-center gap-2 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
            <Cpu className="w-3.5 h-3.5" />
            <span>{t("agentConfig.model")}</span>
          </div>
          <div className="p-3 rounded-xl bg-zinc-800/40 border border-zinc-800 space-y-3">
            <div>
              <label className="text-[11px] text-zinc-500 mb-1.5 block">
                {t("agentConfig.currentModel")}
              </label>
              <select
                value={model}
                onChange={(e) => setModel(e.target.value)}
                className={clsx(
                  "w-full text-sm rounded-lg border px-3 py-2 outline-none transition-colors",
                  isLight
                    ? "bg-white border-zinc-300 text-zinc-900"
                    : "bg-zinc-950 border-zinc-700 text-zinc-100"
                )}
              >
                <option value="glm5.2">GLM-5.2</option>
                <option value="qwen2.5-coder">Qwen2.5-Coder</option>
                <option value="gpt-4o-mini">GPT-4o-mini</option>
                <option value="mock">Mock 测试</option>
              </select>
            </div>
            <button className="w-full flex items-center justify-between px-3 py-2 rounded-lg bg-zinc-800/60 hover:bg-zinc-800 text-zinc-300 text-sm transition-colors">
              <span>{t("agentConfig.modelParams")}</span>
              <ChevronRight className="w-4 h-4 text-zinc-500" />
            </button>
          </div>
        </section>

        {/* 已启用技能 */}
        <section className="space-y-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
              <Zap className="w-3.5 h-3.5" />
              <span>{t("agentConfig.skills")}</span>
            </div>
            <button className="p-1 rounded-md hover:bg-zinc-800 text-zinc-500 hover:text-zinc-300 transition-colors">
              <Plus className="w-3.5 h-3.5" />
            </button>
          </div>
          <div className="space-y-1">
            {skills.map((skill) => (
              <div
                key={skill.id}
                className="flex items-start gap-2.5 p-2.5 rounded-lg bg-zinc-800/40 border border-zinc-800 hover:border-zinc-700 transition-colors"
              >
                <input
                  type="checkbox"
                  checked={skill.enabled}
                  onChange={() => toggleSkill(skill.id)}
                  className="mt-0.5 w-4 h-4 rounded border-zinc-600 text-brand-600 bg-zinc-900 focus:ring-brand-600 focus:ring-offset-0"
                />
                <div className="flex-1 min-w-0">
                  <div className="text-sm text-zinc-200">{skill.name}</div>
                  <div className="text-[11px] text-zinc-500 truncate">
                    {skill.desc}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </section>

        {/* 已启用工具 */}
        <section className="space-y-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
              <Wrench className="w-3.5 h-3.5" />
              <span>{t("agentConfig.tools")}</span>
            </div>
            <button className="p-1 rounded-md hover:bg-zinc-800 text-zinc-500 hover:text-zinc-300 transition-colors">
              <Plus className="w-3.5 h-3.5" />
            </button>
          </div>
          <div className="space-y-1">
            {tools.map((tool) => (
              <label
                key={tool.id}
                className="flex items-center justify-between p-2.5 rounded-lg bg-zinc-800/40 border border-zinc-800 hover:border-zinc-700 transition-colors cursor-pointer"
              >
                <span className="text-sm text-zinc-200">{tool.name}</span>
                <div
                  onClick={() => toggleTool(tool.id)}
                  className={clsx(
                    "w-9 h-5 rounded-full transition-colors relative",
                    tool.enabled ? "bg-brand-600" : "bg-zinc-700"
                  )}
                >
                  <span
                    className={clsx(
                      "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform",
                      tool.enabled ? "translate-x-4" : "translate-x-0"
                    )}
                  />
                </div>
              </label>
            ))}
          </div>
        </section>

        {/* 已授权文件夹 */}
        <section className="space-y-2">
          <div className="flex items-center gap-2 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
            <FolderOpen className="w-3.5 h-3.5" />
            <span>{t("agentConfig.folders")}</span>
          </div>
          <div className="p-3 rounded-xl bg-zinc-800/40 border border-zinc-800">
            <p className="text-[11px] text-zinc-500 mb-2">
              {t("agentConfig.foldersDesc")}
            </p>
            <button className="flex items-center gap-1.5 text-xs text-brand-400 hover:text-brand-300 transition-colors">
              <Plus className="w-3.5 h-3.5" />
              <span>{t("agentConfig.addFolder")}</span>
            </button>
          </div>
        </section>

        {/* 其他设置 */}
        <section className="space-y-2">
          <div className="flex items-center gap-2 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
            <MessageSquare className="w-3.5 h-3.5" />
            <span>{t("agentConfig.chatSettings")}</span>
          </div>
          <div className="p-1 rounded-xl bg-zinc-800/40 border border-zinc-800 space-y-0.5">
            <ToggleRow
              icon={Pin}
              label={t("agentConfig.pinChat")}
              desc={t("agentConfig.pinChatDesc")}
              enabled={pinned}
              onToggle={() => setPinned((v) => !v)}
            />
            <ToggleRow
              icon={Brain}
              label={t("agentConfig.memory")}
              desc={t("agentConfig.memoryDesc")}
              enabled={memory}
              onToggle={() => setMemory((v) => !v)}
            />
          </div>
        </section>

        {/* 占位提示 */}
        <div className="p-3 rounded-xl bg-brand-600/10 border border-brand-700/20 flex items-start gap-2">
          <Sparkles className="w-4 h-4 text-brand-400 flex-shrink-0 mt-0.5" />
          <p className="text-[11px] text-zinc-500 leading-relaxed">
            {t("agentConfig.comingSoon")}
          </p>
        </div>
      </div>
    </div>
  );
}

function ToggleRow({
  icon: Icon,
  label,
  desc,
  enabled,
  onToggle,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  desc: string;
  enabled: boolean;
  onToggle: () => void;
}) {
  return (
    <div className="flex items-center justify-between p-2.5 rounded-lg hover:bg-zinc-800/40 transition-colors">
      <div className="flex items-center gap-2.5 min-w-0">
        <Icon className="w-4 h-4 text-zinc-500 flex-shrink-0" />
        <div className="min-w-0">
          <div className="text-sm text-zinc-200">{label}</div>
          <div className="text-[11px] text-zinc-500 truncate">{desc}</div>
        </div>
      </div>
      <button
        onClick={onToggle}
        className={clsx(
          "w-9 h-5 rounded-full transition-colors relative flex-shrink-0",
          enabled ? "bg-brand-600" : "bg-zinc-700"
        )}
      >
        <span
          className={clsx(
            "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform",
            enabled ? "translate-x-4" : "translate-x-0"
          )}
        />
      </button>
    </div>
  );
}
