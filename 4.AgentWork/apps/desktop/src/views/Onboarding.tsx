// 欢迎向导（Onboarding）
// 首次启动引导：欢迎 → 选行业 → 配员工 → 试运行 → 完成
// 通过 localStorage 标记 yunji-onboarded 判断是否首次

import { useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  Sparkles,
  ShoppingBag,
  Cpu,
  Palette,
  Baby,
  TrendingUp,
  Wand2,
  CheckCircle2,
  Loader2,
  ArrowRight,
  ArrowLeft,
  SkipForward,
  Users,
  Play,
} from "lucide-react";
import { clsx } from "clsx";
import { useI18n } from "@/i18n";
import {
  loadTeamTemplate,
  getTeamConfig,
  executeTeamTask,
  type RoleInfo,
} from "@/lib/tauri";

const ONBOARDED_KEY = "yunji-onboarded";

/** 判断是否已完成向导 */
export function isOnboarded(): boolean {
  try {
    return localStorage.getItem(ONBOARDED_KEY) === "1";
  } catch {
    return false;
  }
}

/** 标记向导完成 */
export function markOnboarded() {
  try {
    localStorage.setItem(ONBOARDED_KEY, "1");
  } catch {
    // ignore
  }
}

/** 重置向导状态（重新运行） */
export function resetOnboarding() {
  try {
    localStorage.removeItem(ONBOARDED_KEY);
  } catch {
    // ignore
  }
}

type Step = 0 | 1 | 2 | 3 | 4;

interface IndustryOption {
  id: string;
  templateName: string;
  labelKey: string;
  icon: typeof ShoppingBag;
  accent: string;
}

const INDUSTRIES: IndustryOption[] = [
  {
    id: "ecommerce-fashion",
    templateName: "ecommerce-fashion",
    labelKey: "onboarding.industry.ecommerce-fashion",
    icon: ShoppingBag,
    accent: "from-pink-500 to-rose-500",
  },
  {
    id: "ecommerce-electronics",
    templateName: "ecommerce-electronics",
    labelKey: "onboarding.industry.ecommerce-electronics",
    icon: Cpu,
    accent: "from-blue-500 to-cyan-500",
  },
  {
    id: "education-arts",
    templateName: "education-arts-training",
    labelKey: "onboarding.industry.education-arts",
    icon: Palette,
    accent: "from-purple-500 to-fuchsia-500",
  },
  {
    id: "education-early",
    templateName: "education-early-childhood",
    labelKey: "onboarding.industry.education-early",
    icon: Baby,
    accent: "from-amber-500 to-orange-500",
  },
  {
    id: "finance-securities",
    templateName: "finance-securities",
    labelKey: "onboarding.industry.finance-securities",
    icon: TrendingUp,
    accent: "from-emerald-500 to-teal-500",
  },
  {
    id: "custom",
    templateName: "",
    labelKey: "onboarding.industry.custom",
    icon: Wand2,
    accent: "from-zinc-500 to-zinc-600",
  },
];

export default function Onboarding() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [step, setStep] = useState<Step>(0);
  const [selectedIndustry, setSelectedIndustry] = useState<string | null>(null);
  const [roles, setRoles] = useState<RoleInfo[]>([]);
  const [loadingRoles, setLoadingRoles] = useState(false);
  const [trialTask, setTrialTask] = useState("");
  const [trialRunning, setTrialRunning] = useState(false);
  const [trialError, setTrialError] = useState<string | null>(null);

  const stepLabels = [
    t("onboarding.step.welcome"),
    t("onboarding.step.industry"),
    t("onboarding.step.employee"),
    t("onboarding.step.trial"),
    t("onboarding.step.done"),
  ];

  function handleSkip() {
    markOnboarded();
    navigate("/chat", { replace: true });
  }

  function handleFinish() {
    markOnboarded();
    navigate("/chat", { replace: true });
  }

  async function handleIndustrySelect(industryId: string) {
    setSelectedIndustry(industryId);
    const opt = INDUSTRIES.find((i) => i.id === industryId);
    if (!opt || !opt.templateName) {
      // 自定义：直接进入试运行
      setStep(3);
      return;
    }
    setLoadingRoles(true);
    setRoles([]);
    try {
      await loadTeamTemplate(opt.templateName);
      const config = await getTeamConfig();
      setRoles(config);
    } catch (err) {
      console.error("加载行业模板失败:", err);
    } finally {
      setLoadingRoles(false);
      setStep(2);
    }
  }

  async function handleTrialRun() {
    if (!trialTask.trim() || trialRunning) return;
    const firstRole = roles[0];
    if (!firstRole) {
      setStep(4);
      return;
    }
    try {
      setTrialRunning(true);
      setTrialError(null);
      await executeTeamTask(firstRole.id, trialTask.trim(), "auto");
      setStep(4);
    } catch (err) {
      setTrialError(String(err));
    } finally {
      setTrialRunning(false);
    }
  }

  return (
    <div className="min-h-screen flex flex-col bg-zinc-950 text-zinc-50">
      {/* 顶部进度条 */}
      <header className="px-8 py-5 border-b border-zinc-800 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Sparkles className="w-5 h-5 text-brand-400" />
          <span className="text-sm font-medium text-zinc-300">{t("onboarding.title")}</span>
        </div>
        <div className="flex items-center gap-2">
          {stepLabels.map((label, i) => (
            <div key={i} className="flex items-center gap-2">
              <div
                className={clsx(
                  "flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs transition-colors",
                  i === step
                    ? "bg-brand-600 text-white"
                    : i < step
                    ? "bg-brand-950/50 text-brand-300"
                    : "bg-zinc-900 text-zinc-500"
                )}
              >
                {i < step && <CheckCircle2 className="w-3 h-3" />}
                <span>{label}</span>
              </div>
              {i < stepLabels.length - 1 && <div className="w-4 h-px bg-zinc-700" />}
            </div>
          ))}
        </div>
        <button
          onClick={handleSkip}
          className="text-xs text-zinc-500 hover:text-zinc-300 transition-colors flex items-center gap-1"
        >
          <SkipForward className="w-3.5 h-3.5" />
          {t("onboarding.skip")}
        </button>
      </header>

      {/* 步骤内容 */}
      <div className="flex-1 overflow-auto flex items-center justify-center p-8">
        <div className="w-full max-w-3xl">
          {step === 0 && (
            <StepWelcome t={t} onStart={() => setStep(1)} />
          )}

          {step === 1 && (
            <StepIndustry
              t={t}
              selected={selectedIndustry}
              onSelect={handleIndustrySelect}
              onBack={() => setStep(0)}
            />
          )}

          {step === 2 && (
            <StepEmployee
              t={t}
              roles={roles}
              loading={loadingRoles}
              onBack={() => setStep(1)}
              onNext={() => setStep(3)}
            />
          )}

          {step === 3 && (
            <StepTrial
              t={t}
              task={trialTask}
              setTask={setTrialTask}
              running={trialRunning}
              error={trialError}
              onBack={() => setStep(2)}
              onRun={handleTrialRun}
              onSkip={() => setStep(4)}
            />
          )}

          {step === 4 && (
            <StepDone t={t} onFinish={handleFinish} />
          )}
        </div>
      </div>
    </div>
  );
}

// ===== 步骤组件 =====

function StepWelcome({
  t,
  onStart,
}: {
  t: (k: string) => string;
  onStart: () => void;
}) {
  return (
    <div className="text-center space-y-6">
      <div className="inline-flex items-center justify-center w-20 h-20 rounded-2xl bg-gradient-to-br from-brand-500 to-brand-700 shadow-lg shadow-brand-900/50">
        <Sparkles className="w-10 h-10 text-white" />
      </div>
      <div className="space-y-3">
        <h1 className="text-3xl font-bold">{t("onboarding.welcome")}</h1>
        <p className="text-zinc-400 max-w-xl mx-auto leading-relaxed">
          {t("onboarding.welcomeDesc")}
        </p>
      </div>
      <button
        onClick={onStart}
        className="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-brand-600 hover:bg-brand-700 transition-colors text-white font-medium shadow-lg shadow-brand-900/30"
      >
        {t("onboarding.start")}
        <ArrowRight className="w-4 h-4" />
      </button>
    </div>
  );
}

function StepIndustry({
  t,
  selected,
  onSelect,
  onBack,
}: {
  t: (k: string) => string;
  selected: string | null;
  onSelect: (id: string) => void;
  onBack: () => void;
}) {
  return (
    <div className="space-y-6">
      <div className="text-center space-y-2">
        <h2 className="text-2xl font-bold">{t("onboarding.industry.title")}</h2>
        <p className="text-zinc-400 max-w-xl mx-auto text-sm">
          {t("onboarding.industry.desc")}
        </p>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {INDUSTRIES.map((opt) => {
          const Icon = opt.icon;
          const isActive = selected === opt.id;
          return (
            <button
              key={opt.id}
              onClick={() => onSelect(opt.id)}
              className={clsx(
                "group relative p-5 rounded-xl border text-left transition-all overflow-hidden",
                isActive
                  ? "border-brand-500 bg-brand-950/30"
                  : "border-zinc-800 bg-zinc-900 hover:border-zinc-600"
              )}
            >
              <div
                className={clsx(
                  "inline-flex items-center justify-center w-10 h-10 rounded-lg bg-gradient-to-br mb-3",
                  opt.accent
                )}
              >
                <Icon className="w-5 h-5 text-white" />
              </div>
              <p className="text-sm font-medium text-zinc-100">{t(opt.labelKey)}</p>
              {isActive && (
                <CheckCircle2 className="absolute top-3 right-3 w-4 h-4 text-brand-400" />
              )}
            </button>
          );
        })}
      </div>
      <div className="flex justify-start">
        <button
          onClick={onBack}
          className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          {t("common.back")}
        </button>
      </div>
    </div>
  );
}

function StepEmployee({
  t,
  roles,
  loading,
  onBack,
  onNext,
}: {
  t: (k: string) => string;
  roles: RoleInfo[];
  loading: boolean;
  onBack: () => void;
  onNext: () => void;
}) {
  return (
    <div className="space-y-6">
      <div className="text-center space-y-2">
        <div className="inline-flex items-center justify-center w-12 h-12 rounded-xl bg-zinc-800 mb-2">
          <Users className="w-6 h-6 text-brand-400" />
        </div>
        <h2 className="text-2xl font-bold">{t("onboarding.employee.title")}</h2>
        <p className="text-zinc-400 max-w-xl mx-auto text-sm">
          {t("onboarding.employee.desc")}
        </p>
      </div>

      {loading ? (
        <div className="flex flex-col items-center justify-center py-12 text-zinc-400">
          <Loader2 className="w-8 h-8 animate-spin mb-3" />
          <p className="text-sm">{t("onboarding.employee.loading")}</p>
        </div>
      ) : roles.length === 0 ? (
        <div className="text-center py-8 text-zinc-500 text-sm">
          {t("onboarding.employee.none")}
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {roles.map((role) => (
            <div
              key={role.id}
              className="p-4 rounded-lg border border-zinc-800 bg-zinc-900 space-y-2"
            >
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-zinc-100">{role.name}</span>
                <span className="text-xs text-zinc-500">{role.id}</span>
              </div>
              <p className="text-xs text-zinc-400 leading-relaxed">{role.description}</p>
              <div className="flex items-center gap-2 text-xs">
                <span className="px-2 py-0.5 rounded bg-zinc-800 text-zinc-300">
                  {role.model}
                </span>
                {role.model_fallback.length > 0 && (
                  <span className="text-zinc-500">
                    {t("team.fallback")} {role.model_fallback.join(", ")}
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="flex justify-between">
        <button
          onClick={onBack}
          className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          {t("common.back")}
        </button>
        <button
          onClick={onNext}
          disabled={loading}
          className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-white"
        >
          {t("common.next")}
          <ArrowRight className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}

function StepTrial({
  t,
  task,
  setTask,
  running,
  error,
  onBack,
  onRun,
  onSkip,
}: {
  t: (k: string) => string;
  task: string;
  setTask: (v: string) => void;
  running: boolean;
  error: string | null;
  onBack: () => void;
  onRun: () => void;
  onSkip: () => void;
}) {
  return (
    <div className="space-y-6">
      <div className="text-center space-y-2">
        <div className="inline-flex items-center justify-center w-12 h-12 rounded-xl bg-zinc-800 mb-2">
          <Play className="w-6 h-6 text-brand-400" />
        </div>
        <h2 className="text-2xl font-bold">{t("onboarding.trial.title")}</h2>
        <p className="text-zinc-400 max-w-xl mx-auto text-sm">
          {t("onboarding.trial.desc")}
        </p>
      </div>

      <div className="space-y-3">
        <textarea
          value={task}
          onChange={(e) => setTask(e.target.value)}
          placeholder={t("onboarding.trial.placeholder")}
          rows={3}
          className="w-full bg-zinc-900 text-sm rounded-lg px-4 py-3 border border-zinc-700 focus:border-brand-500 focus:outline-none resize-none"
        />
        {error && (
          <div className="text-sm text-red-400 bg-red-950/30 rounded p-3">{error}</div>
        )}
      </div>

      <div className="flex justify-between">
        <button
          onClick={onBack}
          className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          {t("common.back")}
        </button>
        <div className="flex items-center gap-2">
          <button
            onClick={onSkip}
            className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
          >
            {t("onboarding.trial.skip")}
          </button>
          <button
            onClick={onRun}
            disabled={!task.trim() || running}
            className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-white"
          >
            {running ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Play className="w-4 h-4" />
            )}
            {running ? t("onboarding.trial.running") : t("onboarding.trial.run")}
          </button>
        </div>
      </div>
    </div>
  );
}

function StepDone({
  t,
  onFinish,
}: {
  t: (k: string) => string;
  onFinish: () => void;
}) {
  return (
    <div className="text-center space-y-6">
      <div className="inline-flex items-center justify-center w-20 h-20 rounded-2xl bg-gradient-to-br from-emerald-500 to-teal-600 shadow-lg shadow-emerald-900/30">
        <CheckCircle2 className="w-10 h-10 text-white" />
      </div>
      <div className="space-y-3">
        <h1 className="text-3xl font-bold">{t("onboarding.done.title")}</h1>
        <p className="text-zinc-400 max-w-xl mx-auto leading-relaxed">
          {t("onboarding.done.desc")}
        </p>
      </div>
      <button
        onClick={onFinish}
        className="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-brand-600 hover:bg-brand-700 transition-colors text-white font-medium shadow-lg shadow-brand-900/30"
      >
        {t("onboarding.finish")}
        <ArrowRight className="w-4 h-4" />
      </button>
    </div>
  );
}
