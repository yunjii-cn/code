// 培训数据导入向导（M4.1 D5）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.1 D5
//
// 5 步流程：
//   Step 1: 选择员工
//   Step 2: 上传文档（PDF/Word/Excel/URL）
//   Step 3: 上传话术示例
//   Step 4: 配置规则
//   Step 5: 跑评估
//
// 特性：
//   - 进度可视化
//   - 可中断 + 恢复（localStorage 保存进度）

import { useEffect, useState, useCallback } from "react";
import {
  User,
  FileText,
  MessageSquare,
  Shield,
  CheckCircle2,
  Loader2,
  ChevronRight,
  ChevronLeft,
  X,
  Save,
  RotateCcw,
  Upload,
  Plus,
  Trash2,
  Play,
  AlertCircle,
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

interface SpeechExample {
  id: string;
  user: string;
  assistant: string;
  tag?: string;
}

interface RuleConfig {
  id: string;
  keywords: string[];
  action: "append" | "prepend" | "refuse" | "replace";
  message: string;
  enabled: boolean;
}

interface UploadedDoc {
  id: string;
  name: string;
  size: number;
  type: "pdf" | "word" | "excel" | "url" | "text";
  status: "pending" | "processing" | "done" | "error";
  progress: number;
}

interface WizardState {
  currentStep: number;
  selectedEmployee: string | null;
  uploadedDocs: UploadedDoc[];
  speechExamples: SpeechExample[];
  rules: RuleConfig[];
  evalResults: EvalResult[] | null;
  startedAt: string;
}

interface EvalResult {
  caseId: string;
  input: string;
  aiResponse: string;
  score: number;
  passed: boolean;
  feedback: string;
}

// ============================================================================
// 常量
// ============================================================================

const STORAGE_KEY = "yunji_training_wizard_state";
const TOTAL_STEPS = 5;

const STEPS = [
  { id: 1, label: "选择员工", icon: User },
  { id: 2, label: "上传文档", icon: FileText },
  { id: 3, label: "话术示例", icon: MessageSquare },
  { id: 4, label: "配置规则", icon: Shield },
  { id: 5, label: "跑评估", icon: CheckCircle2 },
];

// 内置员工列表（MVP，后续从后端加载）
const BUILTIN_EMPLOYEES: Employee[] = [
  { id: "customer_service", name: "电商客服", role: "answer_questions", description: "电商客服咨询，友好风格" },
  { id: "finance_advisor", name: "金融顾问", role: "finance_advisor", description: "银行理财顾问，专业风格" },
  { id: "medical_consult", name: "医疗咨询", role: "medical_consult", description: "医院在线咨询，沉稳风格" },
  { id: "developer", name: "开发工程师", role: "write_code", description: "软件开发，技术风格" },
  { id: "assistant", name: "通用助手", role: "answer_questions", description: "通用问答助手" },
];

// ============================================================================
// 主组件
// ============================================================================

export default function TrainingWizard() {
  const [state, setState] = useState<WizardState>(loadState);
  const [processing, setProcessing] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error" | "info"; text: string } | null>(null);

  // 保存状态到 localStorage
  useEffect(() => {
    saveState(state);
  }, [state]);

  const updateState = useCallback((patch: Partial<WizardState>) => {
    setState((prev) => ({ ...prev, ...patch }));
  }, []);

  const handleNext = () => {
    if (state.currentStep < TOTAL_STEPS) {
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

  return (
    <div className="flex flex-col h-full p-6">
      {/* 头部 */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-zinc-100">AI 员工培训向导</h1>
          <p className="text-sm text-zinc-400 mt-1">
            5 步完成 AI 员工培训：选员工 → 上传文档 → 话术示例 → 配置规则 → 跑评估
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
          <Step1SelectEmployee
            selected={state.selectedEmployee}
            onSelect={(id) => updateState({ selectedEmployee: id })}
          />
        )}
        {state.currentStep === 2 && (
          <Step2UploadDocs
            docs={state.uploadedDocs}
            onUpdate={(updater) =>
              updateState({ uploadedDocs: updater(state.uploadedDocs) })
            }
            processing={processing}
            setProcessing={setProcessing}
          />
        )}
        {state.currentStep === 3 && (
          <Step3SpeechExamples
            examples={state.speechExamples}
            onUpdate={(examples) => updateState({ speechExamples: examples })}
          />
        )}
        {state.currentStep === 4 && (
          <Step4ConfigureRules
            rules={state.rules}
            onUpdate={(rules) => updateState({ rules })}
          />
        )}
        {state.currentStep === 5 && (
          <Step5RunEvaluation
            state={state}
            onUpdate={updateState}
            processing={processing}
            setProcessing={setProcessing}
            setMessage={setMessage}
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
            disabled={!canProceed(state)}
            className={clsx(
              "px-4 py-2 rounded-md flex items-center gap-1.5 text-sm",
              canProceed(state)
                ? "bg-brand-600 hover:bg-brand-500 text-white"
                : "bg-zinc-900 text-zinc-600 cursor-not-allowed"
            )}
          >
            下一步
            <ChevronRight className="w-4 h-4" />
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
    <div className="flex items-center justify-between">
      {STEPS.map((step, idx) => {
        const isCompleted = currentStep > step.id;
        const isCurrent = currentStep === step.id;
        const Icon = step.icon;
        return (
          <div key={step.id} className="flex items-center flex-1">
            <div className="flex flex-col items-center gap-1">
              <div
                className={clsx(
                  "w-10 h-10 rounded-full flex items-center justify-center transition-colors",
                  isCompleted && "bg-green-700 text-white",
                  isCurrent && "bg-brand-600 text-white",
                  !isCompleted && !isCurrent && "bg-zinc-800 text-zinc-500"
                )}
              >
                {isCompleted ? <CheckCircle2 className="w-5 h-5" /> : <Icon className="w-5 h-5" />}
              </div>
              <span
                className={clsx(
                  "text-xs",
                  isCurrent ? "text-zinc-200" : "text-zinc-500"
                )}
              >
                {step.label}
              </span>
            </div>
            {idx < STEPS.length - 1 && (
              <div
                className={clsx(
                  "flex-1 h-0.5 mx-2",
                  currentStep > step.id ? "bg-green-700" : "bg-zinc-800"
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
// Step 1: 选择员工
// ============================================================================

function Step1SelectEmployee({
  selected,
  onSelect,
}: {
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <div>
      <h2 className="text-lg font-semibold text-zinc-100 mb-4">选择要培训的 AI 员工</h2>
      <div className="grid grid-cols-2 gap-3">
        {BUILTIN_EMPLOYEES.map((emp) => (
          <button
            key={emp.id}
            onClick={() => onSelect(emp.id)}
            className={clsx(
              "p-4 rounded-lg border text-left transition-colors",
              selected === emp.id
                ? "border-brand-500 bg-brand-900/20"
                : "border-zinc-800 bg-zinc-900 hover:border-zinc-700"
            )}
          >
            <div className="flex items-center gap-3">
              <div
                className={clsx(
                  "w-10 h-10 rounded-lg flex items-center justify-center",
                  selected === emp.id ? "bg-brand-600" : "bg-zinc-800"
                )}
              >
                <User className="w-5 h-5 text-white" />
              </div>
              <div>
                <div className="font-medium text-zinc-100">{emp.name}</div>
                <div className="text-xs text-zinc-400">{emp.description}</div>
              </div>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}

// ============================================================================
// Step 2: 上传文档
// ============================================================================

function Step2UploadDocs({
  docs,
  onUpdate,
  processing,
  setProcessing,
}: {
  docs: UploadedDoc[];
  onUpdate: (updater: (docs: UploadedDoc[]) => UploadedDoc[]) => void;
  processing: boolean;
  setProcessing: (v: boolean) => void;
}) {
  const handleAddUrl = () => {
    const url = prompt("输入文档 URL：");
    if (!url) return;
    const newDoc: UploadedDoc = {
      id: `doc_${Date.now()}`,
      name: url,
      size: 0,
      type: "url",
      status: "pending",
      progress: 0,
    };
    onUpdate((prev) => [...prev, newDoc]);
    simulateProcessing(newDoc.id, onUpdate, setProcessing);
  };

  const handleAddFile = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".pdf,.docx,.xlsx,.txt,.md";
    input.multiple = true;
    input.onchange = (e) => {
      const files = (e.target as HTMLInputElement).files;
      if (!files) return;
      const newDocs: UploadedDoc[] = [];
      for (const file of Array.from(files)) {
        const ext = file.name.split(".").pop()?.toLowerCase() || "";
        const type: UploadedDoc["type"] =
          ext === "pdf" ? "pdf" :
          ext === "docx" || ext === "doc" ? "word" :
          ext === "xlsx" || ext === "xls" ? "excel" :
          "text";
        newDocs.push({
          id: `doc_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
          name: file.name,
          size: file.size,
          type,
          status: "pending",
          progress: 0,
        });
      }
      onUpdate((prev) => [...prev, ...newDocs]);
      newDocs.forEach((d) => simulateProcessing(d.id, onUpdate, setProcessing));
    };
    input.click();
  };

  const handleRemove = (id: string) => {
    onUpdate((prev) => prev.filter((d) => d.id !== id));
  };

  return (
    <div>
      <h2 className="text-lg font-semibold text-zinc-100 mb-4">上传知识库文档</h2>
      <p className="text-sm text-zinc-400 mb-4">
        支持 PDF / Word / Excel / TXT / Markdown / URL。文档将自动分块（500 字/chunk）并生成 embedding 索引。
      </p>

      <div className="flex gap-2 mb-4">
        <button
          onClick={handleAddFile}
          disabled={processing}
          className="px-3 py-1.5 text-sm rounded-md bg-brand-600 hover:bg-brand-500 text-white flex items-center gap-1.5 disabled:opacity-50"
        >
          <Upload className="w-4 h-4" />
          选择文件
        </button>
        <button
          onClick={handleAddUrl}
          disabled={processing}
          className="px-3 py-1.5 text-sm rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-200 flex items-center gap-1.5 disabled:opacity-50"
        >
          <Plus className="w-4 h-4" />
          添加 URL
        </button>
      </div>

      <div className="space-y-2">
        {docs.length === 0 && (
          <div className="text-center py-12 text-zinc-500 text-sm">
            暂无文档，点击上方按钮添加
          </div>
        )}
        {docs.map((doc) => (
          <div
            key={doc.id}
            className="flex items-center gap-3 p-3 rounded-md bg-zinc-900 border border-zinc-800"
          >
            <FileText className="w-5 h-5 text-zinc-400 flex-shrink-0" />
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="text-sm text-zinc-200 truncate">{doc.name}</span>
                <span className="text-xs text-zinc-500 uppercase">{doc.type}</span>
              </div>
              <div className="flex items-center gap-2 mt-1">
                {doc.status === "processing" && (
                  <>
                    <div className="flex-1 h-1 bg-zinc-800 rounded-full overflow-hidden">
                      <div
                        className="h-full bg-brand-600 transition-all"
                        style={{ width: `${doc.progress}%` }}
                      />
                    </div>
                    <span className="text-xs text-zinc-500">{doc.progress}%</span>
                  </>
                )}
                {doc.status === "done" && (
                  <span className="text-xs text-green-400 flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3" />
                    已处理
                  </span>
                )}
                {doc.status === "error" && (
                  <span className="text-xs text-red-400">处理失败</span>
                )}
                {doc.status === "pending" && (
                  <span className="text-xs text-zinc-500">等待中</span>
                )}
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
  );
}

// 模拟文档处理
function simulateProcessing(
  docId: string,
  onUpdate: (updater: (docs: UploadedDoc[]) => UploadedDoc[]) => void,
  setProcessing: (v: boolean) => void
) {
  setProcessing(true);
  let progress = 0;
  const interval = setInterval(() => {
    progress += Math.random() * 20;
    if (progress >= 100) {
      progress = 100;
      clearInterval(interval);
      onUpdate((docs) =>
        docs.map((d) =>
          d.id === docId ? { ...d, status: "done" as const, progress: 100 } : d
        )
      );
      setProcessing(false);
    } else {
      onUpdate((docs) =>
        docs.map((d) =>
          d.id === docId
            ? { ...d, status: "processing" as const, progress: Math.floor(progress) }
            : d
        )
      );
    }
  }, 300);
}

// ============================================================================
// Step 3: 话术示例
// ============================================================================

function Step3SpeechExamples({
  examples,
  onUpdate,
}: {
  examples: SpeechExample[];
  onUpdate: (examples: SpeechExample[]) => void;
}) {
  const handleAdd = () => {
    const newExample: SpeechExample = {
      id: `ex_${Date.now()}`,
      user: "",
      assistant: "",
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
    <div>
      <h2 className="text-lg font-semibold text-zinc-100 mb-4">录入话术示例</h2>
      <p className="text-sm text-zinc-400 mb-4">
        提供 3-10 个示例对话，AI 员工会通过 few-shot 学习你的风格。示例越典型，效果越好。
      </p>

      <button
        onClick={handleAdd}
        className="mb-4 px-3 py-1.5 text-sm rounded-md bg-brand-600 hover:bg-brand-500 text-white flex items-center gap-1.5"
      >
        <Plus className="w-4 h-4" />
        添加示例
      </button>

      <div className="space-y-3">
        {examples.length === 0 && (
          <div className="text-center py-12 text-zinc-500 text-sm">
            暂无示例，点击上方按钮添加
          </div>
        )}
        {examples.map((ex, idx) => (
          <div
            key={ex.id}
            className="p-4 rounded-md bg-zinc-900 border border-zinc-800"
          >
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm font-medium text-zinc-300">示例 {idx + 1}</span>
              <button
                onClick={() => handleRemove(ex.id)}
                className="text-zinc-500 hover:text-red-400"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            </div>
            <div className="space-y-2">
              <div>
                <label className="text-xs text-zinc-500">用户输入</label>
                <textarea
                  value={ex.user}
                  onChange={(e) => handleUpdate(ex.id, { user: e.target.value })}
                  className="w-full mt-1 p-2 text-sm bg-zinc-800 border border-zinc-700 rounded text-zinc-200 resize-none"
                  rows={2}
                  placeholder="如：我的订单什么时候发货？"
                />
              </div>
              <div>
                <label className="text-xs text-zinc-500">期望回复</label>
                <textarea
                  value={ex.assistant}
                  onChange={(e) => handleUpdate(ex.id, { assistant: e.target.value })}
                  className="w-full mt-1 p-2 text-sm bg-zinc-800 border border-zinc-700 rounded text-zinc-200 resize-none"
                  rows={3}
                  placeholder="如：亲，您的订单我们会在 24 小时内为您发货哦~"
                />
              </div>
              <div>
                <label className="text-xs text-zinc-500">标签（可选）</label>
                <input
                  type="text"
                  value={ex.tag || ""}
                  onChange={(e) => handleUpdate(ex.id, { tag: e.target.value })}
                  className="w-full mt-1 p-2 text-sm bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                  placeholder="如：售前 / 售后 / 投诉"
                />
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ============================================================================
// Step 4: 配置规则
// ============================================================================

function Step4ConfigureRules({
  rules,
  onUpdate,
}: {
  rules: RuleConfig[];
  onUpdate: (rules: RuleConfig[]) => void;
}) {
  const handleAdd = () => {
    const newRule: RuleConfig = {
      id: `rule_${Date.now()}`,
      keywords: [],
      action: "append",
      message: "",
      enabled: true,
    };
    onUpdate([...rules, newRule]);
  };

  const handleUpdate = (id: string, patch: Partial<RuleConfig>) => {
    onUpdate(rules.map((r) => (r.id === id ? { ...r, ...patch } : r)));
  };

  const handleRemove = (id: string) => {
    onUpdate(rules.filter((r) => r.id !== id));
  };

  return (
    <div>
      <h2 className="text-lg font-semibold text-zinc-100 mb-4">配置业务规则</h2>
      <p className="text-sm text-zinc-400 mb-4">
        规则用于约束 AI 员工的行为。如"涉及金额 &gt; 500 转人工"、"包含'保证收益'拒绝回复"。
      </p>

      <button
        onClick={handleAdd}
        className="mb-4 px-3 py-1.5 text-sm rounded-md bg-brand-600 hover:bg-brand-500 text-white flex items-center gap-1.5"
      >
        <Plus className="w-4 h-4" />
        添加规则
      </button>

      <div className="space-y-3">
        {rules.length === 0 && (
          <div className="text-center py-12 text-zinc-500 text-sm">
            暂无规则（可选步骤，跳过即可）
          </div>
        )}
        {rules.map((rule, idx) => (
          <div
            key={rule.id}
            className={clsx(
              "p-4 rounded-md bg-zinc-900 border",
              rule.enabled ? "border-zinc-800" : "border-zinc-800 opacity-60"
            )}
          >
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm font-medium text-zinc-300">规则 {idx + 1}</span>
              <div className="flex items-center gap-2">
                <label className="flex items-center gap-1.5 text-xs text-zinc-400">
                  <input
                    type="checkbox"
                    checked={rule.enabled}
                    onChange={(e) => handleUpdate(rule.id, { enabled: e.target.checked })}
                    className="rounded"
                  />
                  启用
                </label>
                <button
                  onClick={() => handleRemove(rule.id)}
                  className="text-zinc-500 hover:text-red-400"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="text-xs text-zinc-500">关键词（逗号分隔）</label>
                <input
                  type="text"
                  value={rule.keywords.join(", ")}
                  onChange={(e) =>
                    handleUpdate(rule.id, {
                      keywords: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
                    })
                  }
                  className="w-full mt-1 p-2 text-sm bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                  placeholder="如：保证收益, 稳赚, 一定涨"
                />
              </div>
              <div>
                <label className="text-xs text-zinc-500">动作</label>
                <select
                  value={rule.action}
                  onChange={(e) =>
                    handleUpdate(rule.id, { action: e.target.value as RuleConfig["action"] })
                  }
                  className="w-full mt-1 p-2 text-sm bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                >
                  <option value="append">追加提示</option>
                  <option value="prepend">前置提示</option>
                  <option value="refuse">拒绝回复</option>
                  <option value="replace">替换词汇</option>
                </select>
              </div>
              <div className="col-span-2">
                <label className="text-xs text-zinc-500">
                  {rule.action === "refuse" ? "拒绝消息" : "提示内容 / 替换词"}
                </label>
                <input
                  type="text"
                  value={rule.message}
                  onChange={(e) => handleUpdate(rule.id, { message: e.target.value })}
                  className="w-full mt-1 p-2 text-sm bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                  placeholder={
                    rule.action === "refuse"
                      ? "如：抱歉，我无法提供投资建议，请咨询持牌投顾。"
                      : "如：仅供参考"
                  }
                />
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ============================================================================
// Step 5: 跑评估
// ============================================================================

function Step5RunEvaluation({
  state,
  onUpdate,
  processing,
  setProcessing,
  setMessage,
}: {
  state: WizardState;
  onUpdate: (patch: Partial<WizardState>) => void;
  processing: boolean;
  setProcessing: (v: boolean) => void;
  setMessage: (m: { type: "success" | "error" | "info"; text: string } | null) => void;
}) {
  const handleRunEval = async () => {
    setProcessing(true);
    setMessage(null);
    try {
      // 模拟评估过程
      await new Promise((resolve) => setTimeout(resolve, 2000));

      // 生成模拟评估结果
      const mockResults: EvalResult[] = [
        {
          caseId: "case_001",
          input: "我的订单什么时候发货？",
          aiResponse: "亲，您的订单我们会在 24 小时内为您发货哦~",
          score: 8.5,
          passed: true,
          feedback: "回复友好且包含必要信息",
        },
        {
          caseId: "case_002",
          input: "退款多久到账？",
          aiResponse: "退款一般会在 1-3 个工作日内原路退回。",
          score: 7.0,
          passed: true,
          feedback: "回复正确，但缺少'亲'称呼",
        },
        {
          caseId: "case_003",
          input: "你们家这个质量怎么样？",
          aiResponse: "我们的产品质量非常好，绝对值得购买！",
          score: 4.0,
          passed: false,
          feedback: "使用了禁用词'绝对'，且未客观介绍",
        },
      ];

      onUpdate({ evalResults: mockResults });
      setMessage({ type: "success", text: `评估完成，通过率 ${Math.round((mockResults.filter((r) => r.passed).length / mockResults.length) * 100)}%` });
    } catch (e) {
      setMessage({ type: "error", text: `评估失败: ${e}` });
    } finally {
      setProcessing(false);
    }
  };

  const results = state.evalResults;
  const passedCount = results?.filter((r) => r.passed).length || 0;
  const avgScore = results && results.length > 0
    ? results.reduce((sum, r) => sum + r.score, 0) / results.length
    : 0;

  return (
    <div>
      <h2 className="text-lg font-semibold text-zinc-100 mb-4">运行评估</h2>
      <p className="text-sm text-zinc-400 mb-4">
        点击下方按钮运行评估。系统会用 LLM-as-Judge 评估 AI 员工的回复质量。
      </p>

      {/* 培训摘要 */}
      <div className="mb-6 p-4 rounded-md bg-zinc-900 border border-zinc-800">
        <h3 className="text-sm font-medium text-zinc-300 mb-3">培训摘要</h3>
        <div className="grid grid-cols-2 gap-3 text-sm">
          <div>
            <span className="text-zinc-500">员工：</span>
            <span className="text-zinc-200">
              {BUILTIN_EMPLOYEES.find((e) => e.id === state.selectedEmployee)?.name || "未选择"}
            </span>
          </div>
          <div>
            <span className="text-zinc-500">文档数：</span>
            <span className="text-zinc-200">{state.uploadedDocs.length}</span>
          </div>
          <div>
            <span className="text-zinc-500">话术示例：</span>
            <span className="text-zinc-200">{state.speechExamples.length}</span>
          </div>
          <div>
            <span className="text-zinc-500">规则数：</span>
            <span className="text-zinc-200">{state.rules.filter((r) => r.enabled).length}</span>
          </div>
        </div>
      </div>

      <button
        onClick={handleRunEval}
        disabled={processing}
        className="mb-6 px-4 py-2 rounded-md bg-brand-600 hover:bg-brand-500 text-white flex items-center gap-2 disabled:opacity-50"
      >
        {processing ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
        {processing ? "评估中..." : "运行评估"}
      </button>

      {/* 评估结果 */}
      {results && results.length > 0 && (
        <div>
          <div className="grid grid-cols-3 gap-3 mb-4">
            <div className="p-3 rounded-md bg-zinc-900 border border-zinc-800">
              <div className="text-xs text-zinc-500">通过率</div>
              <div className="text-xl font-bold text-green-400">
                {Math.round((passedCount / results.length) * 100)}%
              </div>
            </div>
            <div className="p-3 rounded-md bg-zinc-900 border border-zinc-800">
              <div className="text-xs text-zinc-500">平均分</div>
              <div className="text-xl font-bold text-brand-400">{avgScore.toFixed(1)}</div>
            </div>
            <div className="p-3 rounded-md bg-zinc-900 border border-zinc-800">
              <div className="text-xs text-zinc-500">用例数</div>
              <div className="text-xl font-bold text-zinc-200">{results.length}</div>
            </div>
          </div>

          <div className="space-y-2">
            {results.map((r) => (
              <div
                key={r.caseId}
                className={clsx(
                  "p-3 rounded-md border",
                  r.passed
                    ? "bg-green-900/20 border-green-800"
                    : "bg-red-900/20 border-red-800"
                )}
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm font-medium text-zinc-200">{r.caseId}</span>
                  <div className="flex items-center gap-2">
                    <span
                      className={clsx(
                        "text-sm font-bold",
                        r.passed ? "text-green-400" : "text-red-400"
                      )}
                    >
                      {r.score.toFixed(1)}
                    </span>
                    {r.passed ? (
                      <CheckCircle2 className="w-4 h-4 text-green-400" />
                    ) : (
                      <X className="w-4 h-4 text-red-400" />
                    )}
                  </div>
                </div>
                <div className="text-xs text-zinc-400">
                  <div><span className="text-zinc-500">输入：</span>{r.input}</div>
                  <div className="mt-1"><span className="text-zinc-500">回复：</span>{r.aiResponse}</div>
                  <div className="mt-1"><span className="text-zinc-500">意见：</span>{r.feedback}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// 工具函数
// ============================================================================

function createInitialState(): WizardState {
  return {
    currentStep: 1,
    selectedEmployee: null,
    uploadedDocs: [],
    speechExamples: [],
    rules: [],
    evalResults: null,
    startedAt: new Date().toISOString(),
  };
}

function loadState(): WizardState {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const parsed = JSON.parse(saved);
      return { ...createInitialState(), ...parsed };
    }
  } catch (e) {
    console.warn("加载向导状态失败:", e);
  }
  return createInitialState();
}

function saveState(state: WizardState) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch (e) {
    console.warn("保存向导状态失败:", e);
  }
}

function canProceed(state: WizardState): boolean {
  switch (state.currentStep) {
    case 1:
      return state.selectedEmployee !== null;
    case 2:
      return true; // 文档可选
    case 3:
      return true; // 话术可选（但有更好）
    case 4:
      return true; // 规则可选
    case 5:
      return true;
    default:
      return false;
  }
}
