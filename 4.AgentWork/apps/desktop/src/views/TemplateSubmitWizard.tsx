// 模板提交向导（M4.2 D4）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.2 D4
//
// 4 步流程：
//   Step 1: 上传 manifest.yaml（模板清单）
//   Step 2: 自动校验（员工 / 知识库 / 规则 / 话术 完整性）
//   Step 3: 设置定价 + 标签
//   Step 4: 提交审核（自动 LLM 评估 + 人工审核）
//
// 特性：
//   - 进度可视化
//   - YAML 解析 + 字段校验
//   - 模拟自动 LLM 评估（accuracy / safety / usefulness 三维度）
//   - 提交后显示审核状态

import { useEffect, useState, useCallback } from "react";
import {
  FileText,
  CheckCircle2,
  Loader2,
  ChevronRight,
  ChevronLeft,
  X,
  Save,
  RotateCcw,
  Upload,
  AlertCircle,
  Shield,
  Tag,
  DollarSign,
  Send,
  Sparkles,
} from "lucide-react";
import { clsx } from "clsx";

// ============================================================================
// 类型定义（与 Rust 后端 template_market.rs 对应）
// ============================================================================

interface TemplateManifestFile {
  id: string;
  name: string;
  version: string;
  industry: string;
  sub_industry: string;
  description: string;
  employees: string[];
  files: {
    employees: string[];
    knowledge: string[];
    rules: string[];
    speeches: string[];
    evals: string[];
  };
  dependencies: Array<{ id: string; version: string; required: boolean }>;
}

interface ValidationResult {
  passed: boolean;
  errors: string[];
  warnings: string[];
  stats: {
    employee_count: number;
    knowledge_count: number;
    rule_count: number;
    speech_count: number;
    eval_count: number;
  };
}

interface AutoReviewDimension {
  name: string;
  score: number;
  max_score: number;
  notes: string;
}

interface AutoReviewResult {
  dimensions: AutoReviewDimension[];
  total_score: number;
  passed: boolean;
  summary: string;
}

interface SubmissionState {
  currentStep: number;
  manifest: TemplateManifestFile | null;
  manifestRaw: string;
  validation: ValidationResult | null;
  pricing: "free" | "paid";
  priceCents: number;
  tags: string[];
  autoReview: AutoReviewResult | null;
  submittedAt: string | null;
  submissionId: string | null;
}

// ============================================================================
// 常量
// ============================================================================

const STORAGE_KEY = "yunji_template_submit_wizard_state";
const TOTAL_STEPS = 4;

const STEPS = [
  { id: 1, label: "上传清单", icon: FileText },
  { id: 2, label: "自动校验", icon: Shield },
  { id: 3, label: "定价标签", icon: Tag },
  { id: 4, label: "提交审核", icon: Send },
];

// ============================================================================
// 工具函数
// ============================================================================

function createInitialState(): SubmissionState {
  return {
    currentStep: 1,
    manifest: null,
    manifestRaw: "",
    validation: null,
    pricing: "free",
    priceCents: 0,
    tags: [],
    autoReview: null,
    submittedAt: null,
    submissionId: null,
  };
}

function loadState(): SubmissionState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return createInitialState();
    const parsed = JSON.parse(raw) as SubmissionState;
    return { ...createInitialState(), ...parsed };
  } catch {
    return createInitialState();
  }
}

function saveState(state: SubmissionState) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // 忽略存储失败
  }
}

// 简易 YAML 解析（仅识别本向导所需的扁平 + 嵌套 list 字段）
// 真实生产环境应使用 js-yaml；此处为 MVP 实现，避免引入额外依赖。
function parseManifestYAML(raw: string): { ok: true; manifest: TemplateManifestFile } | { ok: false; error: string } {
  const lines = raw.split(/\r?\n/);
  const root: Record<string, unknown> = {};
  let currentSection: string | null = null;
  let currentList: string[] | null = null;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (!line.trim() || line.trim().startsWith("#")) continue;

    // 顶层 key: value
    const topMatch = line.match(/^([a-z_]+):\s*(.*)$/);
    if (topMatch && !line.startsWith(" ")) {
      const key = topMatch[1];
      const value = topMatch[2].trim();
      currentSection = null;
      currentList = null;

      if (value === "") {
        // 可能是嵌套对象或列表的开始
        currentSection = key;
        root[key] = {};
      } else {
        // 去除引号
        root[key] = value.replace(/^["']|["']$/g, "");
      }
      continue;
    }

    // 嵌套对象下的列表项 "- value"
    const listMatch = line.match(/^\s+-\s+(.*)$/);
    if (listMatch && currentSection) {
      const section = root[currentSection] as Record<string, unknown>;
      if (!section) continue;
      const value = listMatch[1].trim().replace(/^["']|["']$/g, "");

      // 检测这是 employees / files.* / dependencies 的列表
      if (currentSection === "employees") {
        if (!Array.isArray(section)) {
          root[currentSection] = [];
        }
        (root[currentSection] as string[]).push(value);
        currentList = root[currentSection] as string[];
      } else if (currentSection === "dependencies") {
        if (!Array.isArray(section)) {
          root[currentSection] = [];
        }
        (root[currentSection] as Array<Record<string, string>>).push({ id: value });
      } else if (currentSection === "files") {
        // files 下的列表项需要识别子段，此处简化：把所有都收集到对应数组
        // 实际格式：
        //   files:
        //     employees:
        //       - xxx.yml
        //     knowledge:
        //       - xxx.md
        // 简化处理：根据上一个 "  xxx:" 标记归类
        if (currentList) currentList.push(value);
      }
      continue;
    }

    // 嵌套对象下的 "  key: value"
    const nestedMatch = line.match(/^  ([a-z_]+):\s*(.*)$/);
    if (nestedMatch && currentSection) {
      const key = nestedMatch[1];
      const value = nestedMatch[2].trim();
      const section = root[currentSection] as Record<string, unknown>;
      if (!section) continue;

      if (currentSection === "files") {
        if (value === "") {
          section[key] = [];
          currentList = section[key] as string[];
        } else {
          section[key] = value.replace(/^["']|["']$/g, "");
        }
      } else if (currentSection === "dependencies") {
        // dependencies 下的列表项里的 key: value（id: xxx / version: 1.0.0）
        const arr = root[currentSection] as Array<Record<string, string>>;
        if (arr.length > 0) {
          arr[arr.length - 1][key] = value.replace(/^["']|["']$/g, "");
        }
      } else {
        section[key] = value.replace(/^["']|["']$/g, "");
      }
      continue;
    }
  }

  // 校验必填字段
  const required = ["id", "name", "version", "industry", "description"];
  for (const f of required) {
    if (!root[f]) {
      return { ok: false, error: `缺少必填字段: ${f}` };
    }
  }

  const files = (root.files as TemplateManifestFile["files"]) || {
    employees: [],
    knowledge: [],
    rules: [],
    speeches: [],
    evals: [],
  };

  const manifest: TemplateManifestFile = {
    id: String(root.id),
    name: String(root.name),
    version: String(root.version),
    industry: String(root.industry),
    sub_industry: root.sub_industry ? String(root.sub_industry) : "",
    description: String(root.description),
    employees: Array.isArray(root.employees) ? (root.employees as string[]) : [],
    files: {
      employees: Array.isArray(files.employees) ? files.employees : [],
      knowledge: Array.isArray(files.knowledge) ? files.knowledge : [],
      rules: Array.isArray(files.rules) ? files.rules : [],
      speeches: Array.isArray(files.speeches) ? files.speeches : [],
      evals: Array.isArray(files.evals) ? files.evals : [],
    },
    dependencies: Array.isArray(root.dependencies)
      ? (root.dependencies as TemplateManifestFile["dependencies"])
      : [],
  };

  return { ok: true, manifest };
}

function validateManifest(manifest: TemplateManifestFile): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // 必填字段
  if (!manifest.id) errors.push("id 不能为空");
  if (!manifest.name) errors.push("name 不能为空");
  if (!manifest.version) errors.push("version 不能为空");
  if (!manifest.industry) errors.push("industry 不能为空");
  if (!manifest.description) errors.push("description 不能为空");

  // 版本号格式
  const versionRegex = /^\d+\.\d+\.\d+$/;
  if (manifest.version && !versionRegex.test(manifest.version)) {
    errors.push(`version 格式错误（应为 x.y.z，实际: ${manifest.version}）`);
  }

  // 员工完整性
  if (manifest.employees.length === 0) {
    errors.push("至少需要 1 个员工");
  }
  if (manifest.employees.length !== manifest.files.employees.length) {
    errors.push(
      `employees 字段数 (${manifest.employees.length}) 与 files.employees 文件数 (${manifest.files.employees.length}) 不一致`
    );
  }

  // 知识库（警告，非错误）
  if (manifest.files.knowledge.length === 0) {
    warnings.push("未提供知识库文档（建议至少 1 个）");
  }

  // 规则（警告）
  if (manifest.files.rules.length === 0) {
    warnings.push("未提供规则文件（建议至少 1 个）");
  }

  // 话术示例（必须至少 3 条）
  if (manifest.files.speeches.length === 0) {
    errors.push("至少需要 1 个话术示例文件");
  } else if (manifest.files.speeches.length < 3) {
    warnings.push(`话术示例仅 ${manifest.files.speeches.length} 条，建议至少 3 条`);
  }

  // 评估用例（必须至少 5 条）
  if (manifest.files.evals.length === 0) {
    errors.push("至少需要 1 个评估用例文件");
  } else if (manifest.files.evals.length < 5) {
    warnings.push(`评估用例仅 ${manifest.files.evals.length} 条，建议至少 5 条`);
  }

  // 金融行业额外合规检查
  if (manifest.industry === "finance") {
    if (manifest.files.rules.length === 0) {
      errors.push("金融行业必须提供规则文件（合规红线）");
    }
  }

  return {
    passed: errors.length === 0,
    errors,
    warnings,
    stats: {
      employee_count: manifest.employees.length,
      knowledge_count: manifest.files.knowledge.length,
      rule_count: manifest.files.rules.length,
      speech_count: manifest.files.speeches.length,
      eval_count: manifest.files.evals.length,
    },
  };
}

// 模拟 LLM-as-Judge 自动评估
// 真实实现会调用后端 invoke("auto_review_template", { manifest })
function mockAutoReview(manifest: TemplateManifestFile): AutoReviewResult {
  // 基于模板内容生成模拟评分
  const baseScore = 60;
  const employeeBonus = Math.min(manifest.employees.length * 5, 15);
  const knowledgeBonus = Math.min(manifest.files.knowledge.length * 3, 10);
  const speechBonus = Math.min(manifest.files.speeches.length * 2, 10);
  const evalBonus = Math.min(manifest.files.evals.length * 1, 5);

  const accuracyScore = Math.min(baseScore + employeeBonus + knowledgeBonus, 100);
  const safetyScore = manifest.industry === "finance"
    ? Math.min(baseScore + (manifest.files.rules.length > 0 ? 30 : 0), 100)
    : Math.min(baseScore + 20, 100);
  const usefulnessScore = Math.min(baseScore + speechBonus + evalBonus, 100);

  const dimensions: AutoReviewDimension[] = [
    {
      name: "准确性 (Accuracy)",
      score: accuracyScore,
      max_score: 100,
      notes: `员工定义 ${manifest.employees.length} 个，知识库 ${manifest.files.knowledge.length} 个文档`,
    },
    {
      name: "安全性 (Safety)",
      score: safetyScore,
      max_score: 100,
      notes: manifest.industry === "finance"
        ? "金融行业合规检查通过"
        : "通用安全检查通过",
    },
    {
      name: "实用性 (Usefulness)",
      score: usefulnessScore,
      max_score: 100,
      notes: `话术 ${manifest.files.speeches.length} 条，评估用例 ${manifest.files.evals.length} 条`,
    },
  ];

  const totalScore = (accuracyScore + safetyScore + usefulnessScore) / 3;
  const passed = totalScore >= 70 && safetyScore >= 80;

  return {
    dimensions,
    total_score: Math.round(totalScore * 10) / 10,
    passed,
    summary: passed
      ? `自动评估通过（综合得分 ${totalScore.toFixed(1)}），进入人工审核队列`
      : `自动评估未通过（综合得分 ${totalScore.toFixed(1)}），请优化后重新提交`,
  };
}

function canProceed(state: SubmissionState): boolean {
  switch (state.currentStep) {
    case 1:
      return state.manifest !== null;
    case 2:
      return state.validation?.passed === true;
    case 3:
      // 付费模板价格必须 > 0
      if (state.pricing === "paid" && state.priceCents <= 0) return false;
      return state.tags.length > 0;
    case 4:
      return true;
    default:
      return false;
  }
}

// ============================================================================
// 主组件
// ============================================================================

export default function TemplateSubmitWizard() {
  const [state, setState] = useState<SubmissionState>(loadState);
  const [processing, setProcessing] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error" | "info"; text: string } | null>(null);
  const [tagInput, setTagInput] = useState("");

  useEffect(() => {
    saveState(state);
  }, [state]);

  const updateState = useCallback((patch: Partial<SubmissionState>) => {
    setState((prev) => ({ ...prev, ...patch }));
  }, []);

  const handleNext = () => {
    if (state.currentStep < TOTAL_STEPS) {
      // Step 2 → Step 3：触发自动评估
      if (state.currentStep === 2 && state.manifest && !state.autoReview) {
        setProcessing(true);
        // 模拟异步 LLM 评估
        setTimeout(() => {
          const review = mockAutoReview(state.manifest!);
          updateState({ autoReview: review, currentStep: 3 });
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
      setTagInput("");
      setMessage({ type: "info", text: "向导已重置" });
    }
  };

  const handleSave = () => {
    saveState(state);
    setMessage({ type: "success", text: "进度已保存，可稍后继续" });
  };

  const handleSubmit = () => {
    if (!state.manifest || !state.autoReview) return;
    setProcessing(true);
    // 模拟提交到后端
    setTimeout(() => {
      const submissionId = `sub_${Date.now().toString(36)}`;
      updateState({
        submittedAt: new Date().toISOString(),
        submissionId,
        currentStep: TOTAL_STEPS,
      });
      setProcessing(false);
      setMessage({
        type: "success",
        text: `模板已提交审核，提交 ID: ${submissionId}`,
      });
    }, 1000);
  };

  const handleAddTag = () => {
    const tag = tagInput.trim();
    if (!tag) return;
    if (state.tags.includes(tag)) {
      setTagInput("");
      return;
    }
    if (state.tags.length >= 8) {
      setMessage({ type: "error", text: "最多 8 个标签" });
      return;
    }
    updateState({ tags: [...state.tags, tag] });
    setTagInput("");
  };

  const handleRemoveTag = (tag: string) => {
    updateState({ tags: state.tags.filter((t) => t !== tag) });
  };

  return (
    <div className="flex flex-col h-full p-6">
      {/* 头部 */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-zinc-100">模板提交向导</h1>
          <p className="text-sm text-zinc-400 mt-1">
            4 步提交你的模板到云集社区市场：上传清单 → 自动校验 → 定价标签 → 提交审核
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
          <Step1UploadManifest
            manifestRaw={state.manifestRaw}
            manifest={state.manifest}
            onUpdate={(manifestRaw, manifest) => updateState({ manifestRaw, manifest })}
            setMessage={setMessage}
          />
        )}
        {state.currentStep === 2 && (
          <Step2AutoValidation
            manifest={state.manifest}
            validation={state.validation}
            onUpdate={(validation) => updateState({ validation })}
            processing={processing}
            setProcessing={setProcessing}
          />
        )}
        {state.currentStep === 3 && (
          <Step3PricingTags
            pricing={state.pricing}
            priceCents={state.priceCents}
            tags={state.tags}
            tagInput={tagInput}
            autoReview={state.autoReview}
            onUpdate={(patch) => updateState(patch)}
            onTagInputChange={setTagInput}
            onAddTag={handleAddTag}
            onRemoveTag={handleRemoveTag}
          />
        )}
        {state.currentStep === 4 && (
          <Step4SubmitReview
            state={state}
            processing={processing}
            onSubmit={handleSubmit}
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
            disabled={processing}
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
              <span className="hidden sm:inline">{step.label}</span>
              <span className="sm:hidden">{step.id}</span>
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
// Step 1: 上传 manifest.yaml
// ============================================================================

interface Step1Props {
  manifestRaw: string;
  manifest: TemplateManifestFile | null;
  onUpdate: (raw: string, manifest: TemplateManifestFile | null) => void;
  setMessage: (msg: { type: "success" | "error" | "info"; text: string } | null) => void;
}

function Step1UploadManifest({ manifestRaw, manifest, onUpdate, setMessage }: Step1Props) {
  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (!file.name.endsWith(".yaml") && !file.name.endsWith(".yml")) {
      setMessage({ type: "error", text: "请上传 YAML 文件 (.yaml / .yml)" });
      return;
    }
    const reader = new FileReader();
    reader.onload = (ev) => {
      const content = String(ev.target?.result || "");
      const result = parseManifestYAML(content);
      if (result.ok) {
        onUpdate(content, result.manifest);
        setMessage({ type: "success", text: `清单已解析: ${result.manifest.name} v${result.manifest.version}` });
      } else {
        onUpdate(content, null);
        setMessage({ type: "error", text: `YAML 解析失败: ${result.error}` });
      }
    };
    reader.readAsText(file);
  };

  const handlePaste = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const content = e.target.value;
    if (!content.trim()) {
      onUpdate("", null);
      return;
    }
    const result = parseManifestYAML(content);
    if (result.ok) {
      onUpdate(content, result.manifest);
    } else {
      onUpdate(content, null);
    }
  };

  const handleLoadSample = () => {
    const sample = `id: ecommerce-fashion-custom
name: 电商-穿搭-定制版
version: 1.0.0
industry: ecommerce
sub_industry: fashion
description: 基于电商-穿搭模板定制的西装品牌专属模板
employees:
  - customer_service
  - fashion_advisor
  - tailor
files:
  employees:
    - employees/customer_service.yml
    - employees/fashion_advisor.yml
    - employees/tailor.yml
  knowledge:
    - knowledge/fabric_guide.md
    - knowledge/size_chart.md
  rules:
    - rules/rules.yaml
  speeches:
    - speeches/professional_advisor.yml
  evals:
    - evals/eval_suite.yaml
dependencies: []
`;
    const result = parseManifestYAML(sample);
    if (result.ok) {
      onUpdate(sample, result.manifest);
      setMessage({ type: "success", text: "已加载示例清单" });
    }
  };

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold text-zinc-100 mb-2">上传模板清单 (manifest.yaml)</h2>
        <p className="text-sm text-zinc-400 mb-4">
          清单文件描述了模板的元信息、员工列表、文件结构和依赖关系。请上传或粘贴 YAML 内容。
        </p>
      </div>

      {/* 上传区域 */}
      <div className="border-2 border-dashed border-zinc-700 rounded-lg p-6 text-center hover:border-brand-600 transition-colors">
        <Upload className="w-10 h-10 mx-auto text-zinc-500 mb-2" />
        <p className="text-sm text-zinc-400 mb-3">点击或拖拽上传 manifest.yaml</p>
        <input
          type="file"
          accept=".yaml,.yml"
          onChange={handleFileUpload}
          className="hidden"
          id="manifest-upload"
        />
        <label
          htmlFor="manifest-upload"
          className="inline-block px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded text-sm cursor-pointer"
        >
          选择文件
        </label>
        <button
          onClick={handleLoadSample}
          className="ml-2 px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded text-sm"
        >
          加载示例
        </button>
      </div>

      {/* 粘贴区域 */}
      <div>
        <label className="block text-sm text-zinc-400 mb-2">
          或直接粘贴 YAML 内容：
        </label>
        <textarea
          value={manifestRaw}
          onChange={handlePaste}
          placeholder="id: my-template&#10;name: 我的模板&#10;version: 1.0.0&#10;..."
          className="w-full h-64 p-3 bg-zinc-900 border border-zinc-800 rounded text-sm font-mono text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-brand-600"
        />
      </div>

      {/* 解析结果 */}
      {manifest && (
        <div className="bg-green-900/20 border border-green-800 rounded-lg p-4">
          <div className="flex items-center gap-2 text-green-300 mb-2">
            <CheckCircle2 className="w-4 h-4" />
            <span className="font-medium">清单解析成功</span>
          </div>
          <div className="grid grid-cols-2 gap-3 text-sm">
            <div>
              <span className="text-zinc-500">ID: </span>
              <span className="text-zinc-200">{manifest.id}</span>
            </div>
            <div>
              <span className="text-zinc-500">名称: </span>
              <span className="text-zinc-200">{manifest.name}</span>
            </div>
            <div>
              <span className="text-zinc-500">版本: </span>
              <span className="text-zinc-200">{manifest.version}</span>
            </div>
            <div>
              <span className="text-zinc-500">行业: </span>
              <span className="text-zinc-200">{manifest.industry}{manifest.sub_industry ? ` / ${manifest.sub_industry}` : ""}</span>
            </div>
            <div className="col-span-2">
              <span className="text-zinc-500">描述: </span>
              <span className="text-zinc-200">{manifest.description}</span>
            </div>
            <div>
              <span className="text-zinc-500">员工数: </span>
              <span className="text-zinc-200">{manifest.employees.length}</span>
            </div>
            <div>
              <span className="text-zinc-500">依赖: </span>
              <span className="text-zinc-200">{manifest.dependencies.length}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Step 2: 自动校验
// ============================================================================

interface Step2Props {
  manifest: TemplateManifestFile | null;
  validation: ValidationResult | null;
  onUpdate: (validation: ValidationResult) => void;
  processing: boolean;
  setProcessing: (b: boolean) => void;
}

function Step2AutoValidation({ manifest, validation, onUpdate, processing, setProcessing }: Step2Props) {
  const handleValidate = () => {
    if (!manifest) return;
    setProcessing(true);
    setTimeout(() => {
      const result = validateManifest(manifest);
      onUpdate(result);
      setProcessing(false);
    }, 600);
  };

  // 进入页面自动触发一次
  useEffect(() => {
    if (manifest && !validation) {
      handleValidate();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [manifest]);

  if (!manifest) {
    return (
      <div className="text-center text-zinc-500 py-12">
        <AlertCircle className="w-12 h-12 mx-auto mb-3 opacity-50" />
        <p>请先返回 Step 1 上传清单</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold text-zinc-100 mb-2">自动校验模板完整性</h2>
        <p className="text-sm text-zinc-400 mb-4">
          检查清单字段、文件结构、员工一致性、话术/评估用例数量等。
        </p>
      </div>

      {processing && (
        <div className="flex items-center gap-2 text-zinc-400 text-sm">
          <Loader2 className="w-4 h-4 animate-spin" />
          正在校验...
        </div>
      )}

      {validation && !processing && (
        <>
          {/* 校验结果汇总 */}
          <div
            className={clsx(
              "rounded-lg p-4 border",
              validation.passed
                ? "bg-green-900/20 border-green-800"
                : "bg-red-900/20 border-red-800"
            )}
          >
            <div className="flex items-center gap-2 mb-2">
              {validation.passed ? (
                <CheckCircle2 className="w-5 h-5 text-green-400" />
              ) : (
                <AlertCircle className="w-5 h-5 text-red-400" />
              )}
              <span
                className={clsx(
                  "font-medium",
                  validation.passed ? "text-green-300" : "text-red-300"
                )}
              >
                {validation.passed ? "校验通过" : "校验未通过"}
              </span>
            </div>
            <p className="text-sm text-zinc-400">
              {validation.passed
                ? "模板结构完整，可进入下一步定价与标签设置。"
                : `发现 ${validation.errors.length} 个错误，请修正后重新校验。`}
            </p>
          </div>

          {/* 统计信息 */}
          <div className="grid grid-cols-5 gap-3">
            <StatCard label="员工" value={validation.stats.employee_count} />
            <StatCard label="知识库" value={validation.stats.knowledge_count} />
            <StatCard label="规则" value={validation.stats.rule_count} />
            <StatCard label="话术" value={validation.stats.speech_count} />
            <StatCard label="评估用例" value={validation.stats.eval_count} />
          </div>

          {/* 错误列表 */}
          {validation.errors.length > 0 && (
            <div className="bg-red-900/10 border border-red-900 rounded-lg p-4">
              <h3 className="text-sm font-medium text-red-300 mb-2">
                错误 ({validation.errors.length})
              </h3>
              <ul className="space-y-1 text-sm text-red-200">
                {validation.errors.map((err, idx) => (
                  <li key={idx} className="flex items-start gap-2">
                    <X className="w-4 h-4 mt-0.5 flex-shrink-0" />
                    <span>{err}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* 警告列表 */}
          {validation.warnings.length > 0 && (
            <div className="bg-yellow-900/10 border border-yellow-900 rounded-lg p-4">
              <h3 className="text-sm font-medium text-yellow-300 mb-2">
                警告 ({validation.warnings.length})
              </h3>
              <ul className="space-y-1 text-sm text-yellow-200">
                {validation.warnings.map((warn, idx) => (
                  <li key={idx} className="flex items-start gap-2">
                    <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                    <span>{warn}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          <button
            onClick={handleValidate}
            className="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded text-sm flex items-center gap-2"
          >
            <RotateCcw className="w-4 h-4" />
            重新校验
          </button>
        </>
      )}
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: number }) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-3 text-center">
      <div className="text-2xl font-semibold text-brand-400">{value}</div>
      <div className="text-xs text-zinc-500 mt-1">{label}</div>
    </div>
  );
}

// ============================================================================
// Step 3: 定价 + 标签
// ============================================================================

interface Step3Props {
  pricing: "free" | "paid";
  priceCents: number;
  tags: string[];
  tagInput: string;
  autoReview: AutoReviewResult | null;
  onUpdate: (patch: Partial<SubmissionState>) => void;
  onTagInputChange: (v: string) => void;
  onAddTag: () => void;
  onRemoveTag: (tag: string) => void;
}

function Step3PricingTags({
  pricing,
  priceCents,
  tags,
  tagInput,
  autoReview,
  onUpdate,
  onTagInputChange,
  onAddTag,
  onRemoveTag,
}: Step3Props) {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-zinc-100 mb-2">设置定价与标签</h2>
        <p className="text-sm text-zinc-400 mb-4">
          为你的模板设置定价（免费 / 付费）和标签（最多 8 个，便于用户搜索）。
        </p>
      </div>

      {/* 自动评估结果 */}
      {autoReview && (
        <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-3">
            <Sparkles className="w-4 h-4 text-brand-400" />
            <h3 className="text-sm font-medium text-zinc-200">LLM 自动评估结果</h3>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3 mb-3">
            {autoReview.dimensions.map((dim) => (
              <div key={dim.name} className="bg-zinc-950 rounded p-3">
                <div className="text-xs text-zinc-500 mb-1">{dim.name}</div>
                <div className="text-xl font-semibold text-brand-400">
                  {dim.score}<span className="text-sm text-zinc-500">/{dim.max_score}</span>
                </div>
                <div className="text-xs text-zinc-500 mt-1">{dim.notes}</div>
              </div>
            ))}
          </div>
          <div className="flex items-center justify-between pt-3 border-t border-zinc-800">
            <div>
              <span className="text-sm text-zinc-400">综合得分: </span>
              <span className="text-lg font-semibold text-brand-400">{autoReview.total_score}</span>
            </div>
            <div
              className={clsx(
                "px-2 py-0.5 rounded text-xs font-medium",
                autoReview.passed
                  ? "bg-green-900/50 text-green-400"
                  : "bg-red-900/50 text-red-400"
              )}
            >
              {autoReview.passed ? "通过" : "未通过"}
            </div>
          </div>
          <p className="text-xs text-zinc-500 mt-2">{autoReview.summary}</p>
        </div>
      )}

      {/* 定价 */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
        <div className="flex items-center gap-2 mb-3">
          <DollarSign className="w-4 h-4 text-yellow-400" />
          <h3 className="text-sm font-medium text-zinc-200">定价</h3>
        </div>
        <div className="grid grid-cols-2 gap-3 mb-3">
          <button
            onClick={() => onUpdate({ pricing: "free", priceCents: 0 })}
            className={clsx(
              "p-3 rounded border text-left",
              pricing === "free"
                ? "border-brand-600 bg-brand-900/20"
                : "border-zinc-800 hover:border-zinc-700"
            )}
          >
            <div className="font-medium text-zinc-100">免费</div>
            <div className="text-xs text-zinc-500 mt-1">开放共享，积累声誉</div>
          </button>
          <button
            onClick={() => onUpdate({ pricing: "paid", priceCents: priceCents || 9900 })}
            className={clsx(
              "p-3 rounded border text-left",
              pricing === "paid"
                ? "border-brand-600 bg-brand-900/20"
                : "border-zinc-800 hover:border-zinc-700"
            )}
          >
            <div className="font-medium text-zinc-100">付费</div>
            <div className="text-xs text-zinc-500 mt-1">作者得 70% 分成</div>
          </button>
        </div>
        {pricing === "paid" && (
          <div>
            <label className="block text-sm text-zinc-400 mb-1">价格 (元)</label>
            <input
              type="number"
              min={1}
              step={0.01}
              value={priceCents / 100}
              onChange={(e) => {
                const yuan = parseFloat(e.target.value) || 0;
                onUpdate({ priceCents: Math.round(yuan * 100) });
              }}
              className="w-32 px-3 py-2 bg-zinc-950 border border-zinc-800 rounded text-sm text-zinc-100 focus:outline-none focus:border-brand-600"
            />
            <span className="ml-2 text-sm text-zinc-500">
              作者分成: ¥{((priceCents / 100) * 0.7).toFixed(2)}
            </span>
          </div>
        )}
      </div>

      {/* 标签 */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
        <div className="flex items-center gap-2 mb-3">
          <Tag className="w-4 h-4 text-brand-400" />
          <h3 className="text-sm font-medium text-zinc-200">标签 (最多 8 个)</h3>
        </div>
        <div className="flex gap-2 mb-3">
          <input
            type="text"
            value={tagInput}
            onChange={(e) => onTagInputChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                onAddTag();
              }
            }}
            placeholder="输入标签后回车，如: 官方 / 电商 / 穿搭"
            className="flex-1 px-3 py-2 bg-zinc-950 border border-zinc-800 rounded text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-brand-600"
          />
          <button
            onClick={onAddTag}
            className="px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded text-sm"
          >
            添加
          </button>
        </div>
        {tags.length > 0 ? (
          <div className="flex flex-wrap gap-2">
            {tags.map((tag) => (
              <span
                key={tag}
                className="px-2 py-1 bg-zinc-800 text-zinc-300 rounded text-sm flex items-center gap-1"
              >
                {tag}
                <button
                  onClick={() => onRemoveTag(tag)}
                  className="text-zinc-500 hover:text-red-400"
                >
                  <X className="w-3 h-3" />
                </button>
              </span>
            ))}
          </div>
        ) : (
          <p className="text-xs text-zinc-500">尚未添加标签</p>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Step 4: 提交审核
// ============================================================================

interface Step4Props {
  state: SubmissionState;
  processing: boolean;
  onSubmit: () => void;
}

function Step4SubmitReview({ state, processing, onSubmit }: Step4Props) {
  const { manifest, autoReview, pricing, priceCents, tags, submittedAt, submissionId } = state;

  if (!manifest || !autoReview) {
    return (
      <div className="text-center text-zinc-500 py-12">
        <AlertCircle className="w-12 h-12 mx-auto mb-3 opacity-50" />
        <p>请先完成前序步骤</p>
      </div>
    );
  }

  // 已提交
  if (submittedAt && submissionId) {
    return (
      <div className="space-y-4">
        <div className="bg-green-900/20 border border-green-800 rounded-lg p-6 text-center">
          <CheckCircle2 className="w-16 h-16 mx-auto text-green-400 mb-3" />
          <h2 className="text-xl font-semibold text-green-300 mb-2">提交成功</h2>
          <p className="text-sm text-zinc-400 mb-4">
            你的模板已进入审核队列，预计 1-3 个工作日内完成审核。
          </p>
          <div className="inline-block bg-zinc-950 px-4 py-2 rounded text-sm">
            <span className="text-zinc-500">提交 ID: </span>
            <span className="text-zinc-200 font-mono">{submissionId}</span>
          </div>
          <div className="text-xs text-zinc-500 mt-2">
            提交时间: {new Date(submittedAt).toLocaleString("zh-CN")}
          </div>
        </div>

        <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
          <h3 className="text-sm font-medium text-zinc-200 mb-3">审核流程</h3>
          <ol className="space-y-2 text-sm text-zinc-400">
            <li className="flex items-center gap-2">
              <CheckCircle2 className="w-4 h-4 text-green-400" />
              自动 LLM 评估 (已完成，得分 {autoReview.total_score})
            </li>
            <li className="flex items-center gap-2">
              <Loader2 className="w-4 h-4 text-yellow-400 animate-spin" />
              人工审核 (等待中)
            </li>
            <li className="flex items-center gap-2 text-zinc-600">
              <div className="w-4 h-4 rounded-full border border-zinc-700" />
              发布到市场
            </li>
          </ol>
        </div>
      </div>
    );
  }

  // 未提交
  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold text-zinc-100 mb-2">确认提交审核</h2>
        <p className="text-sm text-zinc-400 mb-4">
          请确认以下信息无误后提交。提交后将进入人工审核队列（1-3 个工作日）。
        </p>
      </div>

      {/* 模板信息汇总 */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4 space-y-3">
        <h3 className="text-sm font-medium text-zinc-200 border-b border-zinc-800 pb-2">
          模板信息
        </h3>
        <div className="grid grid-cols-2 gap-3 text-sm">
          <div>
            <span className="text-zinc-500">ID: </span>
            <span className="text-zinc-200">{manifest.id}</span>
          </div>
          <div>
            <span className="text-zinc-500">名称: </span>
            <span className="text-zinc-200">{manifest.name}</span>
          </div>
          <div>
            <span className="text-zinc-500">版本: </span>
            <span className="text-zinc-200">{manifest.version}</span>
          </div>
          <div>
            <span className="text-zinc-500">行业: </span>
            <span className="text-zinc-200">{manifest.industry}{manifest.sub_industry ? ` / ${manifest.sub_industry}` : ""}</span>
          </div>
          <div className="col-span-2">
            <span className="text-zinc-500">描述: </span>
            <span className="text-zinc-200">{manifest.description}</span>
          </div>
        </div>
      </div>

      {/* 定价与标签 */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4 space-y-3">
        <h3 className="text-sm font-medium text-zinc-200 border-b border-zinc-800 pb-2">
          定价与标签
        </h3>
        <div className="grid grid-cols-2 gap-3 text-sm">
          <div>
            <span className="text-zinc-500">定价: </span>
            <span className="text-zinc-200">
              {pricing === "free" ? "免费" : `¥${(priceCents / 100).toFixed(2)}`}
            </span>
          </div>
          <div>
            <span className="text-zinc-500">作者分成: </span>
            <span className="text-zinc-200">
              {pricing === "free" ? "—" : `¥${((priceCents / 100) * 0.7).toFixed(2)} (70%)`}
            </span>
          </div>
          <div className="col-span-2">
            <span className="text-zinc-500">标签: </span>
            <div className="flex flex-wrap gap-1 mt-1">
              {tags.map((tag) => (
                <span key={tag} className="px-2 py-0.5 bg-zinc-800 text-zinc-300 rounded text-xs">
                  {tag}
                </span>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* 自动评估结果 */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4 space-y-3">
        <h3 className="text-sm font-medium text-zinc-200 border-b border-zinc-800 pb-2">
          自动评估结果
        </h3>
        <div className="grid grid-cols-3 gap-3">
          {autoReview.dimensions.map((dim) => (
            <div key={dim.name} className="bg-zinc-950 rounded p-3 text-center">
              <div className="text-xs text-zinc-500 mb-1">{dim.name}</div>
              <div className="text-xl font-semibold text-brand-400">{dim.score}</div>
            </div>
          ))}
        </div>
        <div className="flex items-center justify-between pt-2 border-t border-zinc-800">
          <span className="text-sm text-zinc-400">综合得分</span>
          <span className="text-lg font-semibold text-brand-400">{autoReview.total_score}</span>
        </div>
        <div
          className={clsx(
            "text-sm",
            autoReview.passed ? "text-green-400" : "text-red-400"
          )}
        >
          {autoReview.passed ? "✓ 通过自动评估" : "✗ 未通过自动评估"}
        </div>
      </div>

      {/* 提交按钮 */}
      <div className="flex justify-end">
        <button
          onClick={onSubmit}
          disabled={processing || !autoReview.passed}
          className={clsx(
            "px-6 py-2 rounded-md flex items-center gap-2 text-sm",
            processing || !autoReview.passed
              ? "bg-zinc-900 text-zinc-600 cursor-not-allowed"
              : "bg-brand-600 hover:bg-brand-500 text-white"
          )}
        >
          {processing ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              提交中...
            </>
          ) : (
            <>
              <Send className="w-4 h-4" />
              确认提交审核
            </>
          )}
        </button>
      </div>

      {!autoReview.passed && (
        <p className="text-xs text-red-400 text-right">
          自动评估未通过，无法提交。请返回 Step 2 优化模板内容。
        </p>
      )}
    </div>
  );
}
