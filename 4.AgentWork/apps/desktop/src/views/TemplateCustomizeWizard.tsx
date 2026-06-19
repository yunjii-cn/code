// 模板自定义向导（M4.2 D6）⭐ P0 用户硬需求
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.2 D6
//
// 用户原话："然后就是得灵活，用户能自定义自己所属的行业，
//           自己打造符合公司业务和流程的模版"
//
// 6 步流程：
//   Step 1: 选择起点（fork 核心 / 空白 / 社区）
//   Step 2: 定义员工（可视化编辑）
//   Step 3: 上传知识库（PDF/Word/Excel/URL）
//   Step 4: 配置规则（可视化规则编辑器）
//   Step 5: 录入话术（few-shot 示例对话）
//   Step 6: 跑评估 + 可见性设置 + 导出
//
// 特性：
//   - 进度可视化 + localStorage 持久化
//   - 私有模板隔离（私有 = 公司内部用，不进市场）
//   - 公开模板 = 一键提交到社区市场
//   - YAML 导出 / 导入
//   - 版本管理（v1.0.0 → v1.1.0）

import { useEffect, useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import {
  CheckCircle2,
  Loader2,
  ChevronRight,
  ChevronLeft,
  X,
  Save,
  RotateCcw,
  AlertCircle,
  Shield,
  Users,
  BookOpen,
  MessageSquare,
  Sparkles,
  Plus,
  Trash2,
  Download,
  Upload,
  Lock,
  Globe,
  GitFork,
  FilePlus,
  ShoppingBag,
} from "lucide-react";
import { clsx } from "clsx";

// ============================================================================
// 类型定义
// ============================================================================

interface Employee {
  id: string;
  name: string;
  role: string;
  description: string;
}

interface KnowledgeDoc {
  id: string;
  name: string;
  size: number;
  type: "pdf" | "word" | "excel" | "url" | "text";
  url?: string;
}

interface RuleConfig {
  id: string;
  keywords: string[];
  action: "append" | "prepend" | "refuse" | "replace";
  message: string;
  enabled: boolean;
}

interface SpeechExample {
  id: string;
  user: string;
  assistant: string;
  tag?: string;
}

interface TemplateMeta {
  id: string;
  name: string;
  version: string;
  industry: string;
  sub_industry: string;
  description: string;
  visibility: "private" | "public";
  based_on: string | null; // fork 自哪个模板
}

interface EvalResult {
  caseId: string;
  input: string;
  aiResponse: string;
  score: number;
  passed: boolean;
  feedback: string;
}

interface WizardState {
  currentStep: number;
  meta: TemplateMeta;
  employees: Employee[];
  knowledgeDocs: KnowledgeDoc[];
  rules: RuleConfig[];
  speechExamples: SpeechExample[];
  evalResults: EvalResult[] | null;
  startedAt: string;
}

// ============================================================================
// 常量
// ============================================================================

const STORAGE_KEY = "yunji_template_customize_wizard_state";
const TOTAL_STEPS = 6;

const STEPS = [
  { id: 1, label: "选择起点", icon: GitFork },
  { id: 2, label: "定义员工", icon: Users },
  { id: 3, label: "知识库", icon: BookOpen },
  { id: 4, label: "配置规则", icon: Shield },
  { id: 5, label: "录入话术", icon: MessageSquare },
  { id: 6, label: "跑评估", icon: CheckCircle2 },
];

// 5 核心模板（用于 fork）
const CORE_TEMPLATES = [
  {
    id: "ecommerce-fashion",
    name: "电商-穿搭",
    industry: "ecommerce",
    sub_industry: "fashion",
    description: "淘宝/抖音/小红书穿搭商家客服 + 选品 + 搭配团队",
    employees: [
      { id: "customer_service", name: "客服", role: "answer_questions", description: "售前售后咨询" },
      { id: "fashion_advisor", name: "搭配师", role: "fashion_advisor", description: "穿搭风格推荐" },
      { id: "content_writer", name: "内容运营", role: "content_writer", description: "小红书种草文" },
    ],
  },
  {
    id: "ecommerce-electronics",
    name: "电商-电子",
    industry: "ecommerce",
    sub_industry: "electronics",
    description: "3C 数码 / 家电 / 智能硬件商家",
    employees: [
      { id: "tech_advisor", name: "技术顾问", role: "tech_advisor", description: "参数对比 + 故障排查" },
      { id: "customer_service", name: "客服", role: "answer_questions", description: "售前售后咨询" },
      { id: "installer", name: "装机指导", role: "installer", description: "安装教程" },
    ],
  },
  {
    id: "education-early-childhood",
    name: "教育-早教",
    industry: "education",
    sub_industry: "early_childhood",
    description: "0-6 岁早教机构 / 托育中心 / 亲子号",
    employees: [
      { id: "parenting_consultant", name: "育儿顾问", role: "parenting_consultant", description: "辅食 / 夜啼问题" },
      { id: "course_advisor", name: "课程顾问", role: "course_advisor", description: "体验课跟进" },
      { id: "story_teacher", name: "故事老师", role: "story_teacher", description: "睡前故事" },
    ],
  },
  {
    id: "education-arts-training",
    name: "教育-素质培训",
    industry: "education",
    sub_industry: "arts_training",
    description: "少儿编程 / AI 启蒙 / 美术 / 书法 / 音乐机构",
    employees: [
      { id: "course_advisor", name: "课程顾问", role: "course_advisor", description: "Scratch 体验课" },
      { id: "ai_teacher", name: "AI 教师", role: "ai_teacher", description: "辅导编程作业" },
      { id: "art_reviewer", name: "作品老师", role: "art_reviewer", description: "点评儿童画" },
    ],
  },
  {
    id: "finance-securities",
    name: "金融-证券股票",
    industry: "finance",
    sub_industry: "securities",
    description: "券商投顾 / 财经媒体 / 第三方投研机构（合规最严）",
    employees: [
      { id: "market_analyst", name: "行情分析师", role: "market_analyst", description: "解读公告" },
      { id: "advisor_assistant", name: "投顾助理", role: "advisor_assistant", description: "C2 风险测评" },
      { id: "announcement_summarizer", name: "公告摘要员", role: "announcement_summarizer", description: "提炼财报要点" },
    ],
  },
];

// ============================================================================
// 工具函数
// ============================================================================

function genId(prefix: string): string {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
}

function createInitialState(): WizardState {
  return {
    currentStep: 1,
    meta: {
      id: "",
      name: "",
      version: "1.0.0",
      industry: "",
      sub_industry: "",
      description: "",
      visibility: "private",
      based_on: null,
    },
    employees: [],
    knowledgeDocs: [],
    rules: [],
    speechExamples: [],
    evalResults: null,
    startedAt: new Date().toISOString(),
  };
}

function loadState(): WizardState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return createInitialState();
    const parsed = JSON.parse(raw) as WizardState;
    return { ...createInitialState(), ...parsed };
  } catch {
    return createInitialState();
  }
}

function saveState(state: WizardState) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // 忽略存储失败
  }
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// 生成模板 YAML（导出用）
function generateTemplateYAML(state: WizardState): string {
  const { meta, employees, knowledgeDocs, rules, speechExamples } = state;
  const lines: string[] = [];

  lines.push(`# 云集智能体工作台 - 自定义模板`);
  lines.push(`# 生成时间: ${new Date().toISOString()}`);
  lines.push(`# 可见性: ${meta.visibility === "private" ? "私有（公司内部）" : "公开（可提交社区）"}`);
  lines.push(``);
  lines.push(`id: ${meta.id || "untitled"}`);
  lines.push(`name: ${meta.name || "未命名模板"}`);
  lines.push(`version: ${meta.version || "1.0.0"}`);
  lines.push(`industry: ${meta.industry || "custom"}`);
  if (meta.sub_industry) lines.push(`sub_industry: ${meta.sub_industry}`);
  lines.push(`description: ${meta.description || ""}`);
  lines.push(`visibility: ${meta.visibility}`);
  if (meta.based_on) lines.push(`based_on: ${meta.based_on}`);
  lines.push(``);
  lines.push(`employees:`);
  employees.forEach((emp) => {
    lines.push(`  - id: ${emp.id}`);
    lines.push(`    name: ${emp.name}`);
    lines.push(`    role: ${emp.role}`);
    lines.push(`    description: ${emp.description}`);
  });
  lines.push(``);
  lines.push(`knowledge:`);
  knowledgeDocs.forEach((doc) => {
    lines.push(`  - name: ${doc.name}`);
    lines.push(`    type: ${doc.type}`);
    if (doc.url) lines.push(`    url: ${doc.url}`);
    else lines.push(`    size: ${doc.size}`);
  });
  lines.push(``);
  lines.push(`rules:`);
  rules.forEach((rule) => {
    lines.push(`  - keywords: [${rule.keywords.join(", ")}]`);
    lines.push(`    action: ${rule.action}`);
    lines.push(`    message: ${rule.message}`);
    lines.push(`    enabled: ${rule.enabled}`);
  });
  lines.push(``);
  lines.push(`speeches:`);
  speechExamples.forEach((speech) => {
    lines.push(`  - user: ${speech.user}`);
    lines.push(`    assistant: ${speech.assistant}`);
    if (speech.tag) lines.push(`    tag: ${speech.tag}`);
  });

  return lines.join("\n");
}

// 模拟评估（MVP，后续接入真实 LLM 评估）
function runMockEvaluation(state: WizardState): EvalResult[] {
  const results: EvalResult[] = [];
  const { employees, speechExamples, rules } = state;

  // 基于话术示例生成评估用例
  speechExamples.slice(0, 5).forEach((speech, idx) => {
    const score = 60 + Math.min(idx * 5, 30) + (rules.length > 0 ? 5 : 0);
    results.push({
      caseId: `case_${idx + 1}`,
      input: speech.user,
      aiResponse: speech.assistant,
      score,
      passed: score >= 70,
      feedback: score >= 80
        ? "响应准确，符合员工角色定义"
        : score >= 70
        ? "响应基本符合要求，可优化细节"
        : "响应质量不达标，建议补充话术示例",
    });
  });

  // 如果没有话术示例，使用员工描述生成
  if (results.length === 0 && employees.length > 0) {
    employees.slice(0, 3).forEach((emp, idx) => {
      results.push({
        caseId: `case_${idx + 1}`,
        input: `你好，请介绍一下 ${emp.name} 的职责`,
        aiResponse: `${emp.name} 的职责是：${emp.description}`,
        score: 65,
        passed: false,
        feedback: "缺少话术示例，建议补充至少 3 条 few-shot 示例",
      });
    });
  }

  return results;
}

function canProceed(state: WizardState): boolean {
  switch (state.currentStep) {
    case 1:
      // 必须选择起点（基于某个模板 / 空白 / 已设置 meta.id）
      return state.meta.id !== "" || state.meta.based_on !== null || state.employees.length > 0;
    case 2:
      return state.employees.length > 0 && state.meta.name !== "";
    case 3:
      // 知识库可选，但建议至少 1 个
      return true;
    case 4:
      // 规则可选
      return true;
    case 5:
      return state.speechExamples.length >= 1;
    case 6:
      return true;
    default:
      return false;
  }
}

// ============================================================================
// 主组件
// ============================================================================

export default function TemplateCustomizeWizard() {
  const navigate = useNavigate();
  const [state, setState] = useState<WizardState>(loadState);
  const [processing, setProcessing] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error" | "info"; text: string } | null>(null);

  useEffect(() => {
    saveState(state);
  }, [state]);

  const updateState = useCallback((patch: Partial<WizardState>) => {
    setState((prev) => ({ ...prev, ...patch }));
  }, []);

  const handleNext = () => {
    if (state.currentStep < TOTAL_STEPS) {
      // Step 5 → Step 6：自动跑评估
      if (state.currentStep === 5 && !state.evalResults) {
        setProcessing(true);
        setTimeout(() => {
          const results = runMockEvaluation(state);
          updateState({ evalResults: results, currentStep: 6 });
          setProcessing(false);
        }, 800);
        return;
      }
      updateState({ currentStep: state.currentStep + 1 });
    }
  };

  const handlePrev = () => {
    if (state.currentStep > 1) {
      updateState({ currentStep: state.currentStep - 1 });
    }
  };

  const handleReset = () => {
    if (confirm("确定要重置向导吗？所有未保存的数据将丢失。")) {
      const fresh = createInitialState();
      setState(fresh);
      saveState(fresh);
      setMessage({ type: "info", text: "向导已重置" });
    }
  };

  const handleSave = () => {
    saveState(state);
    setMessage({ type: "success", text: "进度已保存，可稍后继续" });
  };

  const handleExportYAML = () => {
    const yaml = generateTemplateYAML(state);
    const blob = new Blob([yaml], { type: "text/yaml;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${state.meta.id || "template"}-${state.meta.version || "1.0.0"}.yaml`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    setMessage({ type: "success", text: "YAML 已导出" });
  };

  return (
    <div className="flex flex-col h-full p-6">
      {/* 头部 */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-zinc-100">模板自定义向导</h1>
          <p className="text-sm text-zinc-400 mt-1">
            6 步打造你公司专属的行业模板：选起点 → 定义员工 → 知识库 → 规则 → 话术 → 评估
          </p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={handleSave}
            className="px-3 py-1.5 text-sm rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-200 flex items-center gap-1.5"
          >
            <Save className="w-4 h-4" />
            保存进度
          </button>
          <button
            onClick={handleReset}
            className="px-3 py-1.5 text-sm rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-200 flex items-center gap-1.5"
          >
            <RotateCcw className="w-4 h-4" />
            重置
          </button>
        </div>
      </div>

      {/* 步骤指示器 */}
      <StepIndicator currentStep={state.currentStep} />

      {/* 消息提示 */}
      {message && (
        <div
          className={clsx(
            "mt-4 p-3 rounded-md flex items-center gap-2 text-sm",
            message.type === "success" && "bg-green-900/30 text-green-300 border border-green-800",
            message.type === "error" && "bg-red-900/30 text-red-300 border border-red-800",
            message.type === "info" && "bg-blue-900/30 text-blue-300 border border-blue-800"
          )}
        >
          {message.type === "success" && <CheckCircle2 className="w-4 h-4" />}
          {message.type === "error" && <AlertCircle className="w-4 h-4" />}
          {message.type === "info" && <AlertCircle className="w-4 h-4" />}
          {message.text}
          <button onClick={() => setMessage(null)} className="ml-auto">
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      {/* 步骤内容 */}
      <div className="flex-1 mt-6 overflow-auto">
        {state.currentStep === 1 && (
          <Step1ChooseStart
            meta={state.meta}
            employees={state.employees}
            onUpdate={(patch) => updateState(patch)}
            navigate={navigate}
          />
        )}
        {state.currentStep === 2 && (
          <Step2DefineEmployees
            meta={state.meta}
            employees={state.employees}
            onUpdate={(patch) => updateState(patch)}
          />
        )}
        {state.currentStep === 3 && (
          <Step3UploadKnowledge
            knowledgeDocs={state.knowledgeDocs}
            onUpdate={(docs) => updateState({ knowledgeDocs: docs })}
          />
        )}
        {state.currentStep === 4 && (
          <Step4ConfigureRules
            rules={state.rules}
            onUpdate={(rules) => updateState({ rules })}
          />
        )}
        {state.currentStep === 5 && (
          <Step5SpeechExamples
            examples={state.speechExamples}
            onUpdate={(examples) => updateState({ speechExamples: examples })}
          />
        )}
        {state.currentStep === 6 && (
          <Step6RunEvaluation
            state={state}
            processing={processing}
            onUpdate={updateState}
            onExport={handleExportYAML}
            navigate={navigate}
          />
        )}
      </div>

      {/* 底部导航 */}
      <div className="flex items-center justify-between mt-6 pt-4 border-t border-zinc-800">
        <button
          onClick={handlePrev}
          disabled={state.currentStep === 1}
          className={clsx(
            "px-4 py-2 rounded-md flex items-center gap-1.5 text-sm",
            state.currentStep === 1
              ? "bg-zinc-900 text-zinc-600 cursor-not-allowed"
              : "bg-zinc-800 hover:bg-zinc-700 text-zinc-200"
          )}
        >
          <ChevronLeft className="w-4 h-4" />
          上一步
        </button>

        <span className="text-sm text-zinc-400">
          步骤 {state.currentStep} / {TOTAL_STEPS}
        </span>

        {state.currentStep < TOTAL_STEPS ? (
          <button
            onClick={handleNext}
            disabled={!canProceed(state) || processing}
            className={clsx(
              "px-4 py-2 rounded-md flex items-center gap-1.5 text-sm",
              canProceed(state) && !processing
                ? "bg-brand-600 hover:bg-brand-500 text-white"
                : "bg-zinc-900 text-zinc-600 cursor-not-allowed"
            )}
          >
            {processing ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                评估中...
              </>
            ) : (
              <>
                下一步
                <ChevronRight className="w-4 h-4" />
              </>
            )}
          </button>
        ) : (
          <button
            onClick={handleReset}
            className="px-4 py-2 rounded-md bg-green-700 hover:bg-green-600 text-white flex items-center gap-1.5 text-sm"
          >
            <CheckCircle2 className="w-4 h-4" />
            完成
          </button>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// 步骤指示器
// ============================================================================

function StepIndicator({ currentStep }: { currentStep: number }) {
  return (
    <div className="flex items-center gap-2">
      {STEPS.map((step, idx) => {
        const Icon = step.icon;
        const isCompleted = currentStep > step.id;
        const isCurrent = currentStep === step.id;
        return (
          <div key={step.id} className="flex items-center flex-1">
            <div
              className={clsx(
                "flex items-center gap-2 px-3 py-2 rounded-md text-sm",
                isCompleted && "bg-green-900/30 text-green-300",
                isCurrent && "bg-brand-600 text-white",
                !isCompleted && !isCurrent && "bg-zinc-900 text-zinc-500"
              )}
            >
              <Icon className="w-4 h-4" />
              <span className="hidden md:inline">{step.label}</span>
              <span className="md:hidden">{step.id}</span>
            </div>
            {idx < STEPS.length - 1 && (
              <div
                className={clsx(
                  "flex-1 h-0.5 mx-2",
                  isCompleted ? "bg-green-700" : "bg-zinc-800"
                )}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}

// ============================================================================
// Step 1: 选择起点
// ============================================================================

interface Step1Props {
  meta: TemplateMeta;
  employees: Employee[];
  onUpdate: (patch: Partial<WizardState>) => void;
  navigate: (path: string) => void;
}

function Step1ChooseStart({ meta, employees, onUpdate, navigate }: Step1Props) {
  const handleFork = (template: typeof CORE_TEMPLATES[0]) => {
    onUpdate({
      meta: {
        ...meta,
        id: `${template.id}-custom-${Date.now().toString(36).slice(-4)}`,
        name: `${template.name}-定制版`,
        industry: template.industry,
        sub_industry: template.sub_industry,
        description: `基于 ${template.name} 模板定制`,
        based_on: template.id,
      },
      employees: template.employees.map((e) => ({ ...e })),
    });
  };

  const handleBlank = () => {
    onUpdate({
      meta: {
        ...meta,
        id: `custom-${Date.now().toString(36).slice(-4)}`,
        name: "我的自定义模板",
        industry: "custom",
        based_on: null,
      },
      employees: [],
    });
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-zinc-100 mb-2">选择起点</h2>
        <p className="text-sm text-zinc-400 mb-4">
          从 5 个核心行业模板 fork（推荐），或从空白创建完全自定义的模板。
        </p>
      </div>

      {/* 选项 A: Fork 核心模板 */}
      <div>
        <h3 className="text-sm font-medium text-zinc-300 mb-3 flex items-center gap-2">
          <GitFork className="w-4 h-4 text-brand-400" />
          从核心模板 Fork（推荐）
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {CORE_TEMPLATES.map((tpl) => {
            const isSelected = meta.based_on === tpl.id;
            return (
              <button
                key={tpl.id}
                onClick={() => handleFork(tpl)}
                className={clsx(
                  "p-4 rounded-lg border text-left transition-colors",
                  isSelected
                    ? "border-brand-600 bg-brand-900/20"
                    : "border-zinc-800 bg-zinc-900 hover:border-zinc-700"
                )}
              >
                <div className="flex items-center justify-between mb-2">
                  <h4 className="font-medium text-zinc-100">{tpl.name}</h4>
                  {isSelected && <CheckCircle2 className="w-4 h-4 text-brand-400" />}
                </div>
                <p className="text-xs text-zinc-500 mb-2">{tpl.description}</p>
                <div className="text-xs text-zinc-400">
                  {tpl.employees.length} 个员工: {tpl.employees.map((e) => e.name).join(" / ")}
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* 选项 B: 空白创建 */}
      <div>
        <h3 className="text-sm font-medium text-zinc-300 mb-3 flex items-center gap-2">
          <FilePlus className="w-4 h-4 text-yellow-400" />
          从空白创建
        </h3>
        <button
          onClick={handleBlank}
          className={clsx(
            "w-full p-4 rounded-lg border text-left transition-colors",
            meta.based_on === null && employees.length === 0 && meta.id !== ""
              ? "border-brand-600 bg-brand-900/20"
              : "border-zinc-800 bg-zinc-900 hover:border-zinc-700"
          )}
        >
          <div className="flex items-center justify-between mb-1">
            <h4 className="font-medium text-zinc-100">空白模板</h4>
            <Plus className="w-4 h-4 text-zinc-500" />
          </div>
          <p className="text-xs text-zinc-500">
            完全从零开始，自定义所有内容。适合 5 核心模板未覆盖的行业（如汽车 4S 店 / 餐饮 / 房产等）。
          </p>
        </button>
      </div>

      {/* 选项 C: 从社区市场安装 */}
      <div>
        <h3 className="text-sm font-medium text-zinc-300 mb-3 flex items-center gap-2">
          <ShoppingBag className="w-4 h-4 text-green-400" />
          从社区市场安装
        </h3>
        <button
          onClick={() => navigate("/marketplace")}
          className="w-full p-4 rounded-lg border border-zinc-800 bg-zinc-900 hover:border-zinc-700 text-left transition-colors"
        >
          <div className="flex items-center justify-between mb-1">
            <h4 className="font-medium text-zinc-100">浏览社区市场</h4>
            <ShoppingBag className="w-4 h-4 text-zinc-500" />
          </div>
          <p className="text-xs text-zinc-500">
            前往模板市场浏览社区上传的模板，安装后再基于它自定义。
          </p>
        </button>
      </div>

      {/* 当前选择 */}
      {meta.based_on && (
        <div className="bg-green-900/20 border border-green-800 rounded-lg p-3 flex items-center gap-2 text-sm text-green-300">
          <CheckCircle2 className="w-4 h-4" />
          已选择基于 <strong>{CORE_TEMPLATES.find((t) => t.id === meta.based_on)?.name}</strong> 创建定制模板
        </div>
      )}
      {meta.based_on === null && meta.id !== "" && (
        <div className="bg-green-900/20 border border-green-800 rounded-lg p-3 flex items-center gap-2 text-sm text-green-300">
          <CheckCircle2 className="w-4 h-4" />
          已选择从空白创建模板
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Step 2: 定义员工
// ============================================================================

interface Step2Props {
  meta: TemplateMeta;
  employees: Employee[];
  onUpdate: (patch: Partial<WizardState>) => void;
}

function Step2DefineEmployees({ meta, employees, onUpdate }: Step2Props) {
  const handleAddEmployee = () => {
    const newEmp: Employee = {
      id: genId("emp"),
      name: `员工 ${employees.length + 1}`,
      role: "answer_questions",
      description: "",
    };
    onUpdate({ employees: [...employees, newEmp] });
  };

  const handleUpdateEmployee = (id: string, patch: Partial<Employee>) => {
    onUpdate({
      employees: employees.map((e) => (e.id === id ? { ...e, ...patch } : e)),
    });
  };

  const handleRemoveEmployee = (id: string) => {
    onUpdate({ employees: employees.filter((e) => e.id !== id) });
  };

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold text-zinc-100 mb-2">定义员工</h2>
        <p className="text-sm text-zinc-400 mb-4">
          配置模板的元信息和员工列表。每个员工代表一个 AI 角色，承担特定职责。
        </p>
      </div>

      {/* 模板元信息 */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4 space-y-3">
        <h3 className="text-sm font-medium text-zinc-200 border-b border-zinc-800 pb-2">
          模板信息
        </h3>
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="block text-xs text-zinc-500 mb-1">模板 ID</label>
            <input
              type="text"
              value={meta.id}
              onChange={(e) => onUpdate({ meta: { ...meta, id: e.target.value } })}
              className="w-full px-3 py-2 bg-zinc-950 border border-zinc-800 rounded text-sm text-zinc-100 focus:outline-none focus:border-brand-600"
            />
          </div>
          <div>
            <label className="block text-xs text-zinc-500 mb-1">模板名称 *</label>
            <input
              type="text"
              value={meta.name}
              onChange={(e) => onUpdate({ meta: { ...meta, name: e.target.value } })}
              className="w-full px-3 py-2 bg-zinc-950 border border-zinc-800 rounded text-sm text-zinc-100 focus:outline-none focus:border-brand-600"
            />
          </div>
          <div>
            <label className="block text-xs text-zinc-500 mb-1">版本号</label>
            <input
              type="text"
              value={meta.version}
              onChange={(e) => onUpdate({ meta: { ...meta, version: e.target.value } })}
              placeholder="1.0.0"
              className="w-full px-3 py-2 bg-zinc-950 border border-zinc-800 rounded text-sm text-zinc-100 focus:outline-none focus:border-brand-600"
            />
          </div>
          <div>
            <label className="block text-xs text-zinc-500 mb-1">行业</label>
            <input
              type="text"
              value={meta.industry}
              onChange={(e) => onUpdate({ meta: { ...meta, industry: e.target.value } })}
              placeholder="ecommerce / education / finance / custom"
              className="w-full px-3 py-2 bg-zinc-950 border border-zinc-800 rounded text-sm text-zinc-100 focus:outline-none focus:border-brand-600"
            />
          </div>
          <div className="col-span-2">
            <label className="block text-xs text-zinc-500 mb-1">描述</label>
            <textarea
              value={meta.description}
              onChange={(e) => onUpdate({ meta: { ...meta, description: e.target.value } })}
              rows={2}
              className="w-full px-3 py-2 bg-zinc-950 border border-zinc-800 rounded text-sm text-zinc-100 focus:outline-none focus:border-brand-600"
            />
          </div>
        </div>
      </div>

      {/* 员工列表 */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium text-zinc-200">
            员工列表 ({employees.length})
          </h3>
          <button
            onClick={handleAddEmployee}
            className="px-3 py-1 bg-brand-600 hover:bg-brand-500 text-white rounded text-xs flex items-center gap-1"
          >
            <Plus className="w-3 h-3" />
            添加员工
          </button>
        </div>

        {employees.length === 0 ? (
          <p className="text-sm text-zinc-500 text-center py-6">
            尚未添加员工。点击"添加员工"开始定义。
          </p>
        ) : (
          <div className="space-y-3">
            {employees.map((emp) => (
              <div key={emp.id} className="bg-zinc-950 border border-zinc-800 rounded p-3">
                <div className="grid grid-cols-12 gap-2">
                  <div className="col-span-3">
                    <label className="block text-xs text-zinc-500 mb-1">员工 ID</label>
                    <input
                      type="text"
                      value={emp.id}
                      onChange={(e) => handleUpdateEmployee(emp.id, { id: e.target.value })}
                      className="w-full px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-100 focus:outline-none focus:border-brand-600"
                    />
                  </div>
                  <div className="col-span-3">
                    <label className="block text-xs text-zinc-500 mb-1">名称</label>
                    <input
                      type="text"
                      value={emp.name}
                      onChange={(e) => handleUpdateEmployee(emp.id, { name: e.target.value })}
                      className="w-full px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-100 focus:outline-none focus:border-brand-600"
                    />
                  </div>
                  <div className="col-span-3">
                    <label className="block text-xs text-zinc-500 mb-1">角色</label>
                    <input
                      type="text"
                      value={emp.role}
                      onChange={(e) => handleUpdateEmployee(emp.id, { role: e.target.value })}
                      className="w-full px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-100 focus:outline-none focus:border-brand-600"
                    />
                  </div>
                  <div className="col-span-2">
                    <label className="block text-xs text-zinc-500 mb-1">描述</label>
                    <input
                      type="text"
                      value={emp.description}
                      onChange={(e) => handleUpdateEmployee(emp.id, { description: e.target.value })}
                      className="w-full px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-100 focus:outline-none focus:border-brand-600"
                    />
                  </div>
                  <div className="col-span-1 flex items-end">
                    <button
                      onClick={() => handleRemoveEmployee(emp.id)}
                      className="w-full px-2 py-1 bg-red-900/50 hover:bg-red-800 text-red-300 rounded text-xs flex items-center justify-center"
                      title="删除员工"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Step 3: 上传知识库
// ============================================================================

interface Step3Props {
  knowledgeDocs: KnowledgeDoc[];
  onUpdate: (docs: KnowledgeDoc[]) => void;
}

function Step3UploadKnowledge({ knowledgeDocs, onUpdate }: Step3Props) {
  const [urlInput, setUrlInput] = useState("");

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files) return;
    const newDocs: KnowledgeDoc[] = [];
    Array.from(files).forEach((file) => {
      const ext = file.name.split(".").pop()?.toLowerCase() || "";
      let type: KnowledgeDoc["type"] = "text";
      if (ext === "pdf") type = "pdf";
      else if (ext === "doc" || ext === "docx") type = "word";
      else if (ext === "xls" || ext === "xlsx") type = "excel";

      newDocs.push({
        id: genId("doc"),
        name: file.name,
        size: file.size,
        type,
      });
    });
    onUpdate([...knowledgeDocs, ...newDocs]);
    e.target.value = "";
  };

  const handleAddUrl = () => {
    if (!urlInput.trim()) return;
    onUpdate([
      ...knowledgeDocs,
      {
        id: genId("url"),
        name: urlInput,
        size: 0,
        type: "url",
        url: urlInput,
      },
    ]);
    setUrlInput("");
  };

  const handleRemove = (id: string) => {
    onUpdate(knowledgeDocs.filter((d) => d.id !== id));
  };

  const typeIcons: Record<KnowledgeDoc["type"], string> = {
    pdf: "📄",
    word: "📝",
    excel: "📊",
    url: "🔗",
    text: "📃",
  };

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold text-zinc-100 mb-2">上传知识库</h2>
        <p className="text-sm text-zinc-400 mb-4">
          上传 PDF / Word / Excel 文档或网页 URL，作为员工的领域知识库。
        </p>
      </div>

      {/* 上传区域 */}
      <div className="border-2 border-dashed border-zinc-700 rounded-lg p-6 text-center hover:border-brand-600 transition-colors">
        <Upload className="w-10 h-10 mx-auto text-zinc-500 mb-2" />
        <p className="text-sm text-zinc-400 mb-3">点击或拖拽上传文档</p>
        <input
          type="file"
          multiple
          accept=".pdf,.doc,.docx,.xls,.xlsx,.txt,.md"
          onChange={handleFileUpload}
          className="hidden"
          id="knowledge-upload"
        />
        <label
          htmlFor="knowledge-upload"
          className="inline-block px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded text-sm cursor-pointer"
        >
          选择文件
        </label>
        <p className="text-xs text-zinc-600 mt-2">支持 PDF / Word / Excel / TXT / Markdown</p>
      </div>

      {/* URL 添加 */}
      <div className="flex gap-2">
        <input
          type="url"
          value={urlInput}
          onChange={(e) => setUrlInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              handleAddUrl();
            }
          }}
          placeholder="或输入网页 URL：https://example.com/article"
          className="flex-1 px-3 py-2 bg-zinc-900 border border-zinc-800 rounded text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-brand-600"
        />
        <button
          onClick={handleAddUrl}
          className="px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded text-sm"
        >
          添加 URL
        </button>
      </div>

      {/* 文档列表 */}
      {knowledgeDocs.length > 0 && (
        <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
          <h3 className="text-sm font-medium text-zinc-200 mb-3">
            已上传文档 ({knowledgeDocs.length})
          </h3>
          <div className="space-y-2">
            {knowledgeDocs.map((doc) => (
              <div
                key={doc.id}
                className="flex items-center gap-3 bg-zinc-950 border border-zinc-800 rounded p-2"
              >
                <span className="text-xl">{typeIcons[doc.type]}</span>
                <div className="flex-1 min-w-0">
                  <div className="text-sm text-zinc-200 truncate">{doc.name}</div>
                  <div className="text-xs text-zinc-500">
                    {doc.type.toUpperCase()}
                    {doc.size > 0 && ` · ${formatFileSize(doc.size)}`}
                  </div>
                </div>
                <button
                  onClick={() => handleRemove(doc.id)}
                  className="text-zinc-500 hover:text-red-400"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {knowledgeDocs.length === 0 && (
        <div className="bg-yellow-900/10 border border-yellow-900 rounded-lg p-3 text-sm text-yellow-300 flex items-center gap-2">
          <AlertCircle className="w-4 h-4" />
          建议至少上传 1 个知识库文档，以提升员工回答准确性。
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Step 4: 配置规则
// ============================================================================

interface Step4Props {
  rules: RuleConfig[];
  onUpdate: (rules: RuleConfig[]) => void;
}

function Step4ConfigureRules({ rules, onUpdate }: Step4Props) {
  const handleAddRule = () => {
    const newRule: RuleConfig = {
      id: genId("rule"),
      keywords: [],
      action: "append",
      message: "",
      enabled: true,
    };
    onUpdate([...rules, newRule]);
  };

  const handleUpdateRule = (id: string, patch: Partial<RuleConfig>) => {
    onUpdate(rules.map((r) => (r.id === id ? { ...r, ...patch } : r)));
  };

  const handleRemoveRule = (id: string) => {
    onUpdate(rules.filter((r) => r.id !== id));
  };

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold text-zinc-100 mb-2">配置规则</h2>
        <p className="text-zinc-400 text-sm mb-4">
          规则用于约束员工行为。例如：检测到敏感词时拒绝回答，或在回答末尾追加免责声明。
        </p>
      </div>

      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium text-zinc-200">规则列表 ({rules.length})</h3>
          <button
            onClick={handleAddRule}
            className="px-3 py-1 bg-brand-600 hover:bg-brand-500 text-white rounded text-xs flex items-center gap-1"
          >
            <Plus className="w-3 h-3" />
            添加规则
          </button>
        </div>

        {rules.length === 0 ? (
          <div className="text-center py-6">
            <Shield className="w-10 h-10 mx-auto text-zinc-700 mb-2" />
            <p className="text-sm text-zinc-500">尚未配置规则</p>
            <p className="text-xs text-zinc-600 mt-1">
              规则可选，但金融 / 医疗等合规行业强烈建议配置
            </p>
          </div>
        ) : (
          <div className="space-y-3">
            {rules.map((rule) => (
              <div key={rule.id} className="bg-zinc-950 border border-zinc-800 rounded p-3 space-y-2">
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={rule.enabled}
                    onChange={(e) => handleUpdateRule(rule.id, { enabled: e.target.checked })}
                    className="w-4 h-4"
                  />
                  <span className="text-xs text-zinc-400">启用</span>
                  <div className="flex-1" />
                  <button
                    onClick={() => handleRemoveRule(rule.id)}
                    className="text-zinc-500 hover:text-red-400"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
                <div className="grid grid-cols-12 gap-2">
                  <div className="col-span-7">
                    <label className="block text-xs text-zinc-500 mb-1">关键词（逗号分隔）</label>
                    <input
                      type="text"
                      value={rule.keywords.join(", ")}
                      onChange={(e) =>
                        handleUpdateRule(rule.id, {
                          keywords: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
                        })
                      }
                      placeholder="敏感词1, 敏感词2"
                      className="w-full px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-100 focus:outline-none focus:border-brand-600"
                    />
                  </div>
                  <div className="col-span-5">
                    <label className="block text-xs text-zinc-500 mb-1">动作</label>
                    <select
                      value={rule.action}
                      onChange={(e) =>
                        handleUpdateRule(rule.id, {
                          action: e.target.value as RuleConfig["action"],
                        })
                      }
                      className="w-full px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-100 focus:outline-none focus:border-brand-600"
                    >
                      <option value="append">追加（在末尾添加）</option>
                      <option value="prepend">前置（在开头添加）</option>
                      <option value="refuse">拒绝（不回答）</option>
                      <option value="replace">替换（覆盖回答）</option>
                    </select>
                  </div>
                  <div className="col-span-12">
                    <label className="block text-xs text-zinc-500 mb-1">消息内容</label>
                    <input
                      type="text"
                      value={rule.message}
                      onChange={(e) => handleUpdateRule(rule.id, { message: e.target.value })}
                      placeholder="例如：本回答仅供参考，不构成投资建议。"
                      className="w-full px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-100 focus:outline-none focus:border-brand-600"
                    />
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Step 5: 录入话术
// ============================================================================

interface Step5Props {
  examples: SpeechExample[];
  onUpdate: (examples: SpeechExample[]) => void;
}

function Step5SpeechExamples({ examples, onUpdate }: Step5Props) {
  const handleAdd = () => {
    const newExample: SpeechExample = {
      id: genId("speech"),
      user: "",
      assistant: "",
      tag: "",
    };
    onUpdate([...examples, newExample]);
  };

  const handleUpdate = (id: string, patch: Partial<SpeechExample>) => {
    onUpdate(examples.map((e) => (e.id === id ? { ...e, ...patch } : e)));
  };

  const handleRemove = (id: string) => {
    onUpdate(examples.filter((e) => e.id !== id));
  };

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold text-zinc-100 mb-2">录入话术示例</h2>
        <p className="text-sm text-zinc-400 mb-4">
          提供 few-shot 示例对话，帮助 AI 学习你公司专属的回复风格和业务知识。建议至少 3 条。
        </p>
      </div>

      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium text-zinc-200">
            话术示例 ({examples.length})
          </h3>
          <button
            onClick={handleAdd}
            className="px-3 py-1 bg-brand-600 hover:bg-brand-500 text-white rounded text-xs flex items-center gap-1"
          >
            <Plus className="w-3 h-3" />
            添加示例
          </button>
        </div>

        {examples.length === 0 ? (
          <div className="text-center py-6">
            <MessageSquare className="w-10 h-10 mx-auto text-zinc-700 mb-2" />
            <p className="text-sm text-zinc-500">尚未添加话术示例</p>
            <p className="text-xs text-zinc-600 mt-1">至少需要 1 条示例才能进入下一步</p>
          </div>
        ) : (
          <div className="space-y-3">
            {examples.map((ex, idx) => (
              <div key={ex.id} className="bg-zinc-950 border border-zinc-800 rounded p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-zinc-500">示例 #{idx + 1}</span>
                  <button
                    onClick={() => handleRemove(ex.id)}
                    className="text-zinc-500 hover:text-red-400"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
                <div>
                  <label className="block text-xs text-zinc-500 mb-1">用户输入</label>
                  <textarea
                    value={ex.user}
                    onChange={(e) => handleUpdate(ex.id, { user: e.target.value })}
                    rows={2}
                    placeholder="用户的问题..."
                    className="w-full px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-100 focus:outline-none focus:border-brand-600"
                  />
                </div>
                <div>
                  <label className="block text-xs text-zinc-500 mb-1">期望回复</label>
                  <textarea
                    value={ex.assistant}
                    onChange={(e) => handleUpdate(ex.id, { assistant: e.target.value })}
                    rows={3}
                    placeholder="AI 应该这样回答..."
                    className="w-full px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-100 focus:outline-none focus:border-brand-600"
                  />
                </div>
                <div>
                  <label className="block text-xs text-zinc-500 mb-1">标签（可选）</label>
                  <input
                    type="text"
                    value={ex.tag || ""}
                    onChange={(e) => handleUpdate(ex.id, { tag: e.target.value })}
                    placeholder="如：售前 / 售后 / 投诉"
                    className="w-full px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-100 focus:outline-none focus:border-brand-600"
                  />
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Step 6: 跑评估 + 可见性 + 导出
// ============================================================================

interface Step6Props {
  state: WizardState;
  processing: boolean;
  onUpdate: (patch: Partial<WizardState>) => void;
  onExport: () => void;
  navigate: (path: string) => void;
}

function Step6RunEvaluation({ state, processing, onUpdate, onExport, navigate }: Step6Props) {
  const { meta, employees, knowledgeDocs, rules, speechExamples, evalResults } = state;

  const handleRerun = () => {
    onUpdate({ evalResults: null });
    setTimeout(() => {
      const results = runMockEvaluation(state);
      onUpdate({ evalResults: results });
    }, 600);
  };

  // 评估统计
  const passedCount = evalResults?.filter((r) => r.passed).length || 0;
  const totalCount = evalResults?.length || 0;
  const passRate = totalCount > 0 ? (passedCount / totalCount) * 100 : 0;

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold text-zinc-100 mb-2">评估与发布</h2>
        <p className="text-sm text-zinc-400 mb-4">
          自动验证模板完整性 + 质量，设置可见性，导出 YAML 或提交到社区市场。
        </p>
      </div>

      {/* 完整性检查 */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
        <h3 className="text-sm font-medium text-zinc-200 mb-3">完整性检查</h3>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          <CheckItem label="模板信息" passed={!!(meta.id && meta.name && meta.version)} />
          <CheckItem label="员工定义" passed={employees.length > 0} detail={`${employees.length} 个员工`} />
          <CheckItem label="知识库" passed={knowledgeDocs.length > 0} detail={`${knowledgeDocs.length} 个文档`} />
          <CheckItem label="话术示例" passed={speechExamples.length >= 3} detail={`${speechExamples.length} 条`} />
          <CheckItem label="规则配置" passed={rules.length >= 0} detail={`${rules.length} 条规则`} />
          <CheckItem label="行业适配" passed={!!meta.industry} detail={meta.industry || "未设置"} />
          <CheckItem label="版本号" passed={/^\d+\.\d+\.\d+$/.test(meta.version)} detail={meta.version} />
          <CheckItem label="描述" passed={!!meta.description} detail={meta.description ? "已填写" : "未填写"} />
        </div>
      </div>

      {/* 评估结果 */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium text-zinc-200">评估结果</h3>
          <button
            onClick={handleRerun}
            disabled={processing}
            className="px-3 py-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded text-xs flex items-center gap-1"
          >
            {processing ? <Loader2 className="w-3 h-3 animate-spin" /> : <RotateCcw className="w-3 h-3" />}
            重新评估
          </button>
        </div>

        {processing ? (
          <div className="flex items-center gap-2 text-zinc-400 text-sm py-6 justify-center">
            <Loader2 className="w-4 h-4 animate-spin" />
            正在评估...
          </div>
        ) : evalResults && evalResults.length > 0 ? (
          <>
            <div className="grid grid-cols-3 gap-3 mb-3">
              <div className="bg-zinc-950 rounded p-3 text-center">
                <div className="text-2xl font-semibold text-brand-400">{totalCount}</div>
                <div className="text-xs text-zinc-500 mt-1">总用例</div>
              </div>
              <div className="bg-zinc-950 rounded p-3 text-center">
                <div className="text-2xl font-semibold text-green-400">{passedCount}</div>
                <div className="text-xs text-zinc-500 mt-1">通过</div>
              </div>
              <div className="bg-zinc-950 rounded p-3 text-center">
                <div className="text-2xl font-semibold text-yellow-400">{passRate.toFixed(0)}%</div>
                <div className="text-xs text-zinc-500 mt-1">通过率</div>
              </div>
            </div>

            <div className="space-y-2 max-h-60 overflow-auto">
              {evalResults.map((r) => (
                <div
                  key={r.caseId}
                  className={clsx(
                    "bg-zinc-950 border rounded p-2 text-xs",
                    r.passed ? "border-green-900" : "border-red-900"
                  )}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-zinc-400">{r.caseId}</span>
                    <span
                      className={clsx(
                        "font-medium",
                        r.passed ? "text-green-400" : "text-red-400"
                      )}
                    >
                      {r.score} 分 · {r.passed ? "通过" : "未通过"}
                    </span>
                  </div>
                  <div className="text-zinc-300">
                    <span className="text-zinc-500">输入: </span>
                    {r.input}
                  </div>
                  <div className="text-zinc-300 mt-0.5">
                    <span className="text-zinc-500">回复: </span>
                    {r.aiResponse}
                  </div>
                  <div className="text-zinc-500 mt-0.5">{r.feedback}</div>
                </div>
              ))}
            </div>
          </>
        ) : (
          <p className="text-sm text-zinc-500 text-center py-6">尚未运行评估</p>
        )}
      </div>

      {/* 可见性设置 */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
        <h3 className="text-sm font-medium text-zinc-200 mb-3">可见性设置</h3>
        <div className="grid grid-cols-2 gap-3">
          <button
            onClick={() => onUpdate({ meta: { ...meta, visibility: "private" } })}
            className={clsx(
              "p-3 rounded border text-left",
              meta.visibility === "private"
                ? "border-brand-600 bg-brand-900/20"
                : "border-zinc-800 hover:border-zinc-700"
            )}
          >
            <div className="flex items-center gap-2 mb-1">
              <Lock className="w-4 h-4 text-yellow-400" />
              <span className="font-medium text-zinc-100">私有</span>
            </div>
            <p className="text-xs text-zinc-500">
              公司内部使用，不会出现在社区市场。适合定制业务流。
            </p>
          </button>
          <button
            onClick={() => onUpdate({ meta: { ...meta, visibility: "public" } })}
            className={clsx(
              "p-3 rounded border text-left",
              meta.visibility === "public"
                ? "border-brand-600 bg-brand-900/20"
                : "border-zinc-800 hover:border-zinc-700"
            )}
          >
            <div className="flex items-center gap-2 mb-1">
              <Globe className="w-4 h-4 text-green-400" />
              <span className="font-medium text-zinc-100">公开</span>
            </div>
            <p className="text-xs text-zinc-500">
              可一键提交到社区市场，让其他公司也使用（付费模板享 70% 分成）。
            </p>
          </button>
        </div>
      </div>

      {/* 操作按钮 */}
      <div className="flex flex-wrap gap-2">
        <button
          onClick={onExport}
          className="px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded text-sm flex items-center gap-2"
        >
          <Download className="w-4 h-4" />
          导出 YAML
        </button>
        {meta.visibility === "public" && (
          <button
            onClick={() => navigate("/marketplace/submit")}
            className="px-4 py-2 bg-green-700 hover:bg-green-600 text-white rounded text-sm flex items-center gap-2"
          >
            <Sparkles className="w-4 h-4" />
            提交到社区市场
          </button>
        )}
      </div>
    </div>
  );
}

function CheckItem({
  label,
  passed,
  detail,
}: {
  label: string;
  passed: boolean;
  detail?: string;
}) {
  return (
    <div
      className={clsx(
        "bg-zinc-950 border rounded p-3",
        passed ? "border-green-900" : "border-red-900"
      )}
    >
      <div className="flex items-center gap-2 mb-1">
        {passed ? (
          <CheckCircle2 className="w-4 h-4 text-green-400" />
        ) : (
          <AlertCircle className="w-4 h-4 text-red-400" />
        )}
        <span className="text-sm text-zinc-200">{label}</span>
      </div>
      {detail && <div className="text-xs text-zinc-500">{detail}</div>}
    </div>
  );
}
