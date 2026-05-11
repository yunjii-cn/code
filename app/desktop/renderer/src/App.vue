<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from "vue";

type MessageRole = "user" | "assistant" | "error";

type ChatMessage = {
  id: string;
  role: MessageRole;
  text: string;
  time: string;
  model?: string;
  toolStatus?: string;
  completedAt?: string;
  durationMs?: number;
  tokens?: number;
  rating?: number;
  fileChanges?: FileChange[];
};

type FileChange = {
  tool: string;
  path: string;
  action: "create" | "modify" | "delete";
  time: string;
};

type DesktopSettings = {
  MODEL_PROVIDER?: string;
  ANTHROPIC_BASE_URL?: string;
  ANTHROPIC_API_KEY?: string;
  ANTHROPIC_AUTH_TOKEN?: string;
  ANTHROPIC_MODEL?: string;
  ANTHROPIC_DEFAULT_SONNET_MODEL?: string;
  ANTHROPIC_DEFAULT_HAIKU_MODEL?: string;
  ANTHROPIC_DEFAULT_OPUS_MODEL?: string;
  OLLAMA_BASE_URL?: string;
  OLLAMA_MODEL?: string;
  API_BASE_URL?: string;
  API_MODEL?: string;
  API_KEY?: string;
  API_TIMEOUT_MS?: string;
  AI_LANGUAGE?: string;
  AI_TEMPERATURE?: string;
  AI_MAX_TOKENS?: string;
  SYSTEM_PROMPT?: string;
};

type ModelInfo = {
  id: string;
  name: string;
  provider: string;
  toolSupport?: boolean;
  size?: string;
  family?: string;
  paramCount?: string;
  loadable?: boolean;
  healthError?: string;
};

type ModelConfig = {
  language: string;
  temperature: string;
  maxTokens: string;
  systemPrompt: string;
};

let _backend: any = null;

async function getBackend(): Promise<any> {
  if (_backend) return _backend;
  return new Promise((resolve) => {
    if (typeof (window as any).QWebChannel === "undefined") {
      console.warn("QWebChannel not available, running in browser mode");
      resolve(null);
      return;
    }
    new (window as any).QWebChannel(
      (window as any).qt.webChannelTransport,
      (channel: any) => {
        _backend = channel.objects.backend;
        resolve(_backend);
      }
    );
  });
}

async function callBackend(method: string, ...args: any[]): Promise<any> {
  const backend = await getBackend();
  if (!backend) {
    console.warn("[callBackend] backend not available, method:", method);
    return null;
  }
  try {
    const result = await backend[method](...args);
    if (typeof result === "string") {
      try { return JSON.parse(result); } catch { return result; }
    }
    return result;
  } catch (e) {
    console.error("[callBackend]", method, "error:", e);
    return null;
  }
}

const isBusy = ref(false);
let busyTimeoutId: ReturnType<typeof setTimeout> | null = null;
const sessionId = ref("");
const workspacePath = ref("");
const inputText = ref("");
const showPanel = ref(true);
const noticeText = ref("");
const noticeType = ref<"ok" | "warn">("ok");
const messages = ref<ChatMessage[]>([]);
const currentAssistantId = ref("");

const runMode = ref<"cloud" | "ollama" | "api">("ollama");
const apiKey = ref("");
const ollamaBaseUrl = ref("http://127.0.0.1:11434");
const ollamaModel = ref("");
const apiHost = ref("127.0.0.1");
const apiPort = ref("7777");
const apiBaseUrl = computed(() => {
  const host = apiHost.value.trim() || "127.0.0.1";
  const port = apiPort.value.replace(/\D/g, "") || "7777";
  return `http://${host}:${port}`;
});
const apiModel = ref("qwen3.6-plus");
const apiModels = ref<ModelInfo[]>([]);
const cloudModels = ref<ModelInfo[]>([]);
const loadingModels = ref(false);
const expandedModelId = ref("");
const modelDropdownOpen = ref(false);
const messageStartTime = ref(0);
const hardwareInfo = ref<any>(null);
const apiServiceRunning = ref(false);
const qwenToken = ref("");
const qwenAccountCount = ref(-1);
const qwenRegisterBusy = ref(false);
const qwenAccounts = ref<any[]>([]);
const stickyEmail = ref<string>("");

const qwenValidCount = computed(() => qwenAccounts.value.filter(a => a.valid).length);

const regEmail = ref("");
const regPassword = ref("");
const regUsername = ref("");

const loginEmail = ref("");
const loginPassword = ref("");
const qwenLoginBusy = ref(false);
const qwenLoginError = ref("");

const apiSteps = [
  { label: "检测服务", action: "check" },
  { label: "启动服务", action: "start" },
  { label: "获取 Key", action: "key" },
  { label: "加载模型", action: "models" },
];
const apiStepProgress = ref(0);
const apiStepBusy = ref(false);
const apiStepMessage = ref("点击「一键启动」自动配置 API 服务");

const apiProgressPercent = computed(() => {
  if (apiStepProgress.value >= apiSteps.length) return 100;
  return Math.round((apiStepProgress.value / apiSteps.length) * 100);
});

const modelConfigs = reactive<Record<string, ModelConfig>>({});

const settings = reactive<Required<DesktopSettings>>({
  MODEL_PROVIDER: "anthropic",
  ANTHROPIC_BASE_URL: "",
  ANTHROPIC_API_KEY: "",
  ANTHROPIC_AUTH_TOKEN: "",
  ANTHROPIC_MODEL: "",
  ANTHROPIC_DEFAULT_SONNET_MODEL: "",
  ANTHROPIC_DEFAULT_HAIKU_MODEL: "",
  ANTHROPIC_DEFAULT_OPUS_MODEL: "",
  OLLAMA_BASE_URL: "",
  OLLAMA_MODEL: "",
  API_BASE_URL: "",
  API_MODEL: "",
  API_KEY: "",
  API_TIMEOUT_MS: "3000000",
  AI_LANGUAGE: "zh",
  AI_TEMPERATURE: "",
  AI_MAX_TOKENS: "",
  SYSTEM_PROMPT: "",
});

function getModelConfig(modelId: string): ModelConfig {
  if (!modelConfigs[modelId]) {
    modelConfigs[modelId] = {
      language: settings.AI_LANGUAGE || "zh",
      temperature: settings.AI_TEMPERATURE || "",
      maxTokens: settings.AI_MAX_TOKENS || "",
      systemPrompt: settings.SYSTEM_PROMPT || "",
    };
  }
  return modelConfigs[modelId];
}

function getActiveModelConfig(): ModelConfig {
  const activeModel = runMode.value === "ollama" ? ollamaModel.value : runMode.value === "api" ? apiModel.value : settings.ANTHROPIC_MODEL;
  return getModelConfig(activeModel || "__default__");
}

const selectedOllamaModelToolSupport = computed<boolean | undefined>(() => {
  if (!ollamaModel.value) return undefined;
  const found = cloudModels.value.find((m) => m.id === ollamaModel.value);
  return found?.toolSupport;
});

watch(runMode, (newMode) => {
  if (newMode === "ollama") {
    cloudModels.value = [];
    loadCloudModels("ollama");
  } else if (newMode === "api") {
    checkApiServiceStatus();
  }
});

async function checkApiServiceStatus() {
  if (!apiBaseUrl.value.trim()) {
    apiServiceRunning.value = false;
    apiStepMessage.value = "请输入 API 服务地址";
    return;
  }
  try {
    const result = await callBackend("checkApiService", JSON.stringify({ baseUrl: apiBaseUrl.value.trim() }));
    if (result && result.running) {
      apiServiceRunning.value = true;
      apiStepProgress.value = 2;
      apiStepMessage.value = "服务已运行，点击「一键就绪」继续配置";
    } else {
      apiServiceRunning.value = false;
      apiStepProgress.value = 0;
      apiStepMessage.value = "点击「一键就绪」自动配置 API 服务";
    }
  } catch {
    apiServiceRunning.value = false;
    apiStepMessage.value = "点击「一键就绪」自动配置 API 服务";
  }
}

async function autoActivateApiService() {
  try {
    if (apiBaseUrl.value.trim()) {
      await checkQwenAccounts();
    }

    if (stickyEmail.value && qwenAccounts.value.length > 0) {
      const isStickyValid = qwenAccounts.value.find((a: any) => a.email === stickyEmail.value && a.valid);
      if (isStickyValid) {
        try {
          await callBackend("setStickyAccount", JSON.stringify({
            baseUrl: apiBaseUrl.value.trim(),
            adminKey: "admin",
            email: stickyEmail.value,
          }));
        } catch { /* ignore */ }
      }
    } else if (qwenAccounts.value.length > 0) {
      const firstValid = qwenAccounts.value.find((a: any) => a.valid);
      if (firstValid) {
        await setStickyAccount(firstValid.email);
      }
    }

    const checkResult = await callBackend("checkApiService", JSON.stringify({ baseUrl: apiBaseUrl.value.trim() }));
    const serviceRunning = checkResult && checkResult.running;

    if (serviceRunning) {
      apiServiceRunning.value = true;
      apiStepProgress.value = 2;
    }

    if (apiModels.value.length === 0 && apiBaseUrl.value.trim()) {
      await loadApiModels();
    }

    if (!serviceRunning) {
      return;
    }

    if (!apiKey.value.trim()) {
      const keyResult = await callBackend("fetchApiKey", JSON.stringify({ baseUrl: apiBaseUrl.value.trim(), adminKey: "admin" }));
      if (keyResult && keyResult.ok && keyResult.key) {
        apiKey.value = keyResult.key;
      }
    }

    if (apiStepProgress.value < apiSteps.length) {
      apiStepProgress.value = apiSteps.length;
      apiStepMessage.value = "✓ API 服务已就绪，可以开始编程";
    }

    await saveSettings();
  } catch (e) {
    console.warn("[autoActivateApiService] error:", e);
  }
}

function makeId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

const userName = ref("你");
const assistantName = ref("助手");
const autoApprove = ref(true);
const toolApprovalMode = ref<"auto" | "smart" | "manual">("auto");
watch(toolApprovalMode, (mode) => {
  autoApprove.value = mode === "auto";
});
const pendingQueue = ref<string[]>([]);

const projects = ref<any[]>([]);
const activeProject = ref<any>(null);
const showProjectPanel = ref(false);
const newProjectName = ref("");
const newProjectCustomPath = ref(false);
const newProjectPath = ref("");
const newProjectDefaultPath = ref("");
const projectConversations = ref<any[]>([]);
const editingProjectId = ref<string | null>(null);
const editProjectName = ref("");
const editProjectCustomPath = ref(false);
const editProjectPath = ref("");
const convSearchQuery = ref("");
const showMemoryPanel = ref(false);
const memoryContent = ref("");
const globalMemoryContent = ref("");
const memoryTab = ref<"claude-md" | "smart">("claude-md");
const smartMemories = ref<any[]>([]);
const newMemoryType = ref("project");
const newMemoryTitle = ref("");
const newMemoryContent = ref("");
const memorySearchQuery = ref("");
const showTemplatePanel = ref(false);
const templateCategory = ref("all");
const projectTemplates = ref<any[]>([]);
const showAddTemplate = ref(false);
const newTplName = ref("");
const newTplCategory = ref("frontend");
const newTplDesc = ref("");
const newTplPrompt = ref("");
const showPreviewPanel = ref(false);
const previewUrl = ref("");
const previewPath = ref("");
const showTaskPanel = ref(false);
const taskList = ref<any[]>([]);
const newTaskName = ref("");
const newTaskDesc = ref("");
const collaborationMode = ref<"none" | "plan-code-review" | "pair" | "review-only">("none");
const collabPhase = ref<"planning" | "coding" | "reviewing">("planning");
const collabHistory = ref<any[]>([]);
const showSettings = ref(false);
const settingsTab = ref<"general" | "model" | "account" | "memory" | "project" | "advanced">("general");
const APP_VERSION = "2026.05.11";
const activeNav = ref<"chat" | "project" | "version" | "settings">("chat");
const versionHistory = ref<any[]>([]);
const editingConvId = ref<string | null>(null);
const editingConvTitle = ref("");
const showSlashMenu = ref(false);
const slashMenuFilter = ref("");
const showFileChanges = ref(false);
const sessionFileChanges = ref<FileChange[]>([]);

const SLASH_COMMANDS: Record<string, { name: string; desc: string; action: string }> = {
  compact: { name: "/compact", desc: "压缩上下文，节省 token", action: "compact" },
  clear: { name: "/clear", desc: "清空当前对话", action: "clear" },
  save: { name: "/save", desc: "保存当前对话", action: "save" },
  role: { name: "/role", desc: "切换开发角色", action: "role" },
  memory: { name: "/memory", desc: "打开记忆管理", action: "memory" },
  export: { name: "/export", desc: "导出对话", action: "export" },
  files: { name: "/files", desc: "查看文件变更", action: "files" },
  extract: { name: "/extract", desc: "从对话提取记忆", action: "extract" },
  template: { name: "/template", desc: "项目模板", action: "template" },
  preview: { name: "/preview", desc: "预览 Web 项目", action: "preview" },
  task: { name: "/task", desc: "任务编排", action: "task" },
  collab: { name: "/collab", desc: "AI 协作模式", action: "collab" },
  settings: { name: "/settings", desc: "系统设置", action: "settings" },
};

const WORKFLOW_PRESETS: Record<string, { name: string; prompt: string; steps: string[] }> = {
  "new-project": {
    name: "新项目",
    prompt: "请按照以下步骤创建新项目：1. 分析需求并确定技术栈 2. 初始化项目结构 3. 创建核心文件 4. 实现基础功能 5. 测试运行。每完成一步向我确认后再继续。",
    steps: ["需求分析", "技术选型", "项目初始化", "核心编码", "测试验证"],
  },
  bugfix: {
    name: "Bug修复",
    prompt: "请按照以下步骤修复Bug：1. 复现问题 2. 定位根因 3. 制定修复方案 4. 实施修复 5. 验证修复。每步确认后继续。",
    steps: ["复现问题", "定位根因", "修复方案", "实施修复", "验证修复"],
  },
  refactor: {
    name: "重构",
    prompt: "请按照以下步骤进行代码重构：1. 分析现有代码结构 2. 识别重构目标 3. 制定重构计划 4. 逐步实施 5. 回归测试。每步确认后继续，确保不破坏现有功能。",
    steps: ["分析现状", "识别目标", "制定计划", "逐步重构", "回归测试"],
  },
  docs: {
    name: "文档化",
    prompt: "请按照以下步骤生成文档：1. 扫描代码结构 2. 提取关键接口和函数 3. 生成 API 文档 4. 编写使用说明 5. 审查完善。",
    steps: ["扫描代码", "提取接口", "生成文档", "编写说明", "审查完善"],
  },
  deploy: {
    name: "部署",
    prompt: "请按照以下步骤进行部署：1. 检查环境配置 2. 构建项目 3. 配置部署环境 4. 执行部署 5. 验证部署结果。",
    steps: ["环境检查", "项目构建", "配置部署", "执行部署", "验证结果"],
  },
};

const totalTokens = computed(() => {
  return messages.value.reduce((sum, m) => sum + (m.tokens || 0), 0);
});

const contextWindowMax = computed(() => {
  const config = getModelConfig(
    runMode.value === "ollama" ? ollamaModel.value : runMode.value === "api" ? apiModel.value : "openrouter/auto"
  );
  return parseInt(config.maxTokens || "128000") * 4;
});

const contextPercent = computed(() => {
  if (contextWindowMax.value <= 0) return 0;
  return Math.min(100, Math.round((totalTokens.value / contextWindowMax.value) * 100));
});

const contextBarColor = computed(() => {
  if (contextPercent.value > 90) return "#f44";
  if (contextPercent.value > 70) return "#fa0";
  return "#4af";
});

const ROLE_PRESETS: Record<string, { icon: string; name: string; prompt: string; temp: string; desc: string }> = {
  fullstack: { icon: "💻", name: "全栈开发", prompt: "你是一个专业的全栈开发工程师，精通前端（HTML/CSS/JS/TypeScript/Vue/React）和后端（Python/Node.js/Java/Go）技术。请始终使用中文回答。对于编程任务，直接编写高质量代码，遵循最佳实践。", temp: "0.3", desc: "前后端全能" },
  frontend: { icon: "🎨", name: "前端开发", prompt: "你是一个前端开发专家，精通 HTML/CSS/JavaScript/TypeScript/Vue/React 和现代 UI 框架。请始终使用中文回答。专注于创建美观、响应式、高性能的用户界面。", temp: "0.5", desc: "专注 UI/UX" },
  backend: { icon: "⚙️", name: "后端开发", prompt: "你是一个后端架构师，精通 Python/Node.js/Java/Go、数据库设计、API 开发和系统架构。请始终使用中文回答。专注于构建高性能、可扩展、安全的后端服务。", temp: "0.3", desc: "专注 API/DB" },
  review: { icon: "🔍", name: "代码审查", prompt: "你是一个严格的代码审查专家。请始终使用中文回答。只审查代码，不直接修改文件。指出潜在问题、安全漏洞、性能瓶颈和改进建议，给出具体的修改方案。", temp: "0.2", desc: "只读审查" },
  learner: { icon: "📖", name: "学习助手", prompt: "你是一个耐心的编程学习助手。请始终使用中文回答。用通俗易懂的语言解释概念，提供示例代码，循序渐进地引导学习。鼓励提问，不厌其烦地解答。", temp: "0.7", desc: "耐心讲解" },
  devops: { icon: "🛠️", name: "DevOps", prompt: "你是一个 DevOps 和运维专家，精通 Docker/K8s/CI-CD/云服务/监控告警。请始终使用中文回答。专注于部署自动化、基础设施管理和系统可靠性。", temp: "0.2", desc: "部署运维" },
};
const activeRole = ref("");

function roleLabel(role: MessageRole) {
  if (role === "user") return userName.value;
  if (role === "assistant") return assistantName.value;
  return "错误";
}

function nowStr() {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function addMessage(role: MessageRole, text: string, model?: string) {
  const item: ChatMessage = { id: makeId(), role, text, time: nowStr(), model };
  messages.value.push(item);
  return item.id;
}

function showNotice(text: string, type: "ok" | "warn" = "ok") {
  noticeText.value = text;
  noticeType.value = type;
}

function cloudBaseUrlByProvider(provider: "openrouter" | "anthropic") {
  return provider === "openrouter" ? "https://openrouter.ai/api" : "https://api.anthropic.com";
}

function applySettings(data?: DesktopSettings) {
  if (!data) return;
  settings.MODEL_PROVIDER = data.MODEL_PROVIDER || "anthropic";
  settings.ANTHROPIC_BASE_URL = data.ANTHROPIC_BASE_URL ?? "";
  settings.ANTHROPIC_API_KEY = data.ANTHROPIC_API_KEY ?? "";
  settings.ANTHROPIC_AUTH_TOKEN = data.ANTHROPIC_AUTH_TOKEN ?? "";
  settings.ANTHROPIC_MODEL = data.ANTHROPIC_MODEL ?? "";
  settings.ANTHROPIC_DEFAULT_SONNET_MODEL = data.ANTHROPIC_DEFAULT_SONNET_MODEL ?? "";
  settings.ANTHROPIC_DEFAULT_HAIKU_MODEL = data.ANTHROPIC_DEFAULT_HAIKU_MODEL ?? "";
  settings.ANTHROPIC_DEFAULT_OPUS_MODEL = data.ANTHROPIC_DEFAULT_OPUS_MODEL ?? "";
  settings.OLLAMA_BASE_URL = data.OLLAMA_BASE_URL ?? "";
  settings.OLLAMA_MODEL = data.OLLAMA_MODEL ?? "";
  settings.API_BASE_URL = data.API_BASE_URL ?? "";
  settings.API_MODEL = data.API_MODEL ?? "";
  settings.API_KEY = data.API_KEY ?? "";
  settings.API_TIMEOUT_MS = data.API_TIMEOUT_MS ?? "3000000";
  settings.AI_LANGUAGE = data.AI_LANGUAGE ?? "zh";
  settings.AI_TEMPERATURE = data.AI_TEMPERATURE ?? "";
  settings.AI_MAX_TOKENS = data.AI_MAX_TOKENS ?? "";
  settings.SYSTEM_PROMPT = data.SYSTEM_PROMPT ?? "";

  runMode.value = settings.MODEL_PROVIDER === "ollama" ? "ollama" : settings.MODEL_PROVIDER === "api" ? "api" : "cloud";
  const cloudKey = settings.ANTHROPIC_API_KEY || settings.ANTHROPIC_AUTH_TOKEN || "";
  apiKey.value = cloudKey === "ollama-local" ? "" : cloudKey;
  ollamaBaseUrl.value = settings.OLLAMA_BASE_URL || "http://127.0.0.1:11434";
  ollamaModel.value = settings.OLLAMA_MODEL || "";
  const _apiBase = settings.API_BASE_URL || "http://127.0.0.1:7777";
  const _apiMatch = _apiBase.match(/^https?:\/\/([^:/]+)(?::(\d+))?/);
  apiHost.value = _apiMatch ? _apiMatch[1] : "127.0.0.1";
  apiPort.value = _apiMatch && _apiMatch[2] ? _apiMatch[2] : "7777";
  apiModel.value = settings.API_MODEL || "qwen3.6-plus";
  if (settings.API_KEY) {
    const ak = settings.API_KEY;
    if (ak !== "ollama-local" && ak !== cloudKey) {
      apiKey.value = ak;
    }
  }

  if (settings.OLLAMA_MODEL || settings.ANTHROPIC_MODEL) {
    const modelId = settings.OLLAMA_MODEL || settings.ANTHROPIC_MODEL;
    if (!modelConfigs[modelId]) {
      modelConfigs[modelId] = {
        language: settings.AI_LANGUAGE || "zh",
        temperature: settings.AI_TEMPERATURE || "",
        maxTokens: settings.AI_MAX_TOKENS || "",
        systemPrompt: settings.SYSTEM_PROMPT || "",
      };
    }
  }
}

function displayName(name: string): string {
  const idx = name.indexOf("/");
  return idx >= 0 ? name.slice(idx + 1) : name;
}

function toggleModelSettings(modelId: string) {
  expandedModelId.value = expandedModelId.value === modelId ? "" : modelId;
}

function selectModel(modelId: string) {
  ollamaModel.value = modelId;
}

function parseModelSize(model: ModelInfo): number {
  const sizeStr = model.size || "";
  const match = sizeStr.match(/([\d.]+)\s*(GB|MB|TB)/i);
  if (!match) return 0;
  const val = parseFloat(match[1]);
  const unit = match[2].toUpperCase();
  if (unit === "TB") return val * 1024;
  if (unit === "GB") return val;
  if (unit === "MB") return val / 1024;
  return 0;
}

function autoConfigure(modelId: string) {
  const model = cloudModels.value.find((m) => m.id === modelId);
  const config = getModelConfig(modelId);
  const hw = hardwareInfo.value;

  let paramB = 0;
  const pcStr = model?.paramCount || "";
  const pcMatch = pcStr.match(/([\d.]+)\s*[Bb]/);
  if (pcMatch) paramB = parseFloat(pcMatch[1]);

  const modelSizeGB = model ? parseModelSize(model) : 0;
  const hasToolSupport = model?.toolSupport === true;

  if (hasToolSupport) {
    config.temperature = "0.3";
  } else {
    config.temperature = "0.7";
  }

  if (paramB > 0) {
    if (paramB <= 4) config.maxTokens = "4096";
    else if (paramB <= 8) config.maxTokens = "4096";
    else if (paramB <= 14) config.maxTokens = "8192";
    else if (paramB <= 33) config.maxTokens = "8192";
    else config.maxTokens = "16384";
  } else if (modelSizeGB > 0) {
    if (modelSizeGB <= 5) config.maxTokens = "4096";
    else if (modelSizeGB <= 12) config.maxTokens = "8192";
    else config.maxTokens = "16384";
  } else {
    config.maxTokens = "4096";
  }

  if (hw) {
    const totalRamGB = (hw.total_ram || 0) / (1024 * 1024 * 1024);
    if (totalRamGB > 0 && totalRamGB < 16) {
      if (parseInt(config.maxTokens) > 4096) config.maxTokens = "4096";
    }
    if (hw.gpu_vram_gb && hw.gpu_vram_gb > 0) {
      if (hw.gpu_vram_gb < 8 && parseInt(config.maxTokens) > 4096) {
        config.maxTokens = "2048";
      }
    }
  }

  config.language = "zh";
  config.systemPrompt = "";

  showNotice(`已为 ${modelId} 自动配置参数`, "ok");
}

function getAutoConfigHint(modelId: string): string {
  const model = cloudModels.value.find((m) => m.id === modelId);
  const modelSizeGB = model ? parseModelSize(model) : 0;
  const hasToolSupport = model?.toolSupport === true;
  const hints: string[] = [];

  if (hasToolSupport) {
    hints.push("✅ 支持工具调用，推荐 Temperature 0.2~0.4（精确编程）");
  } else {
    hints.push("⚠ 不支持工具调用，推荐 Temperature 0.6~0.8（对话辅助）");
  }

  const pcStr = model?.paramCount || "";
  if (pcStr) {
    hints.push(`参数量: ${pcStr}`);
  }

  if (modelSizeGB > 0 && modelSizeGB <= 5) {
    hints.push("小模型，建议 MaxTokens 2048~4096");
  } else if (modelSizeGB > 5 && modelSizeGB <= 12) {
    hints.push("中等模型，建议 MaxTokens 4096~8192");
  } else if (modelSizeGB > 12) {
    hints.push("大模型，建议 MaxTokens 8192~16384");
  }

  const hw = hardwareInfo.value;
  if (hw) {
    const totalRamGB = (hw.total_ram || 0) / (1024 * 1024 * 1024);
    if (totalRamGB > 0 && totalRamGB < 8) {
      hints.push("⚠ 内存 <8GB，建议降低 MaxTokens 至 2048");
    }
    if (hw.gpu_name) {
      hints.push(`GPU: ${hw.gpu_name}${hw.gpu_vram_gb ? ` (${hw.gpu_vram_gb}GB)` : ""}`);
    }
  }

  return hints.join(" | ");
}

async function sendMessage() {
  const text = inputText.value.trim();
  if (!text) return;
  inputText.value = "";

  if (isBusy.value) {
    pendingQueue.value.push(text);
    addMessage("user", text);
    showNotice(`已加入排队（第${pendingQueue.value.length}条）`, "warn");
    return;
  }

  await doSend(text, true);
}

async function doSend(text: string, addUserMsg: boolean = false) {
  const currentModel = runMode.value === "ollama" ? ollamaModel.value.trim() : runMode.value === "api" ? apiModel.value.trim() : (settings.ANTHROPIC_MODEL || "").trim();
  if (addUserMsg) {
    addMessage("user", text);
  }
  currentAssistantId.value = addMessage("assistant", "", displayName(currentModel));
  messageStartTime.value = Date.now();
  isBusy.value = true;

  if (busyTimeoutId) clearTimeout(busyTimeoutId);
  busyTimeoutId = setTimeout(async () => {
    if (isBusy.value) {
      isBusy.value = false;
      await callBackend("stopMessage");
      showNotice("任务超时，已自动恢复输入", "warn");
      processQueue();
    }
  }, 120000);

  const activeConfig = getActiveModelConfig();

  const result = await callBackend("sendMessage", JSON.stringify({
    prompt: text,
    provider: runMode.value === "ollama" ? "ollama" : runMode.value === "api" ? "api" : "anthropic",
    model: currentModel,
    ai_language: activeConfig.language,
    ai_temperature: activeConfig.temperature,
    ai_max_tokens: activeConfig.maxTokens,
    system_prompt: activeConfig.systemPrompt,
    auto_approve: autoApprove.value,
    workspace_path: workspacePath.value.trim(),
  }));

  if (!result?.ok) {
    if (result?.sessionId) sessionId.value = result.sessionId;
    messages.value = messages.value.filter((m) => m.id !== currentAssistantId.value);
    if (result?.error === "A request is already running.") {
      await callBackend("stopMessage");
      isBusy.value = false;
      if (busyTimeoutId) { clearTimeout(busyTimeoutId); busyTimeoutId = null; }
      showNotice("已自动恢复，请重新发送", "warn");
    } else if (result?.stopped) {
      addMessage("assistant", "任务已停止。", displayName(currentModel));
    } else {
      addMessage("error", result?.error || "请求失败");
    }
    isBusy.value = false;
    if (busyTimeoutId) { clearTimeout(busyTimeoutId); busyTimeoutId = null; }
    processQueue();
    return;
  }

  if (result?.sessionId) sessionId.value = result.sessionId;
}

function processQueue() {
  if (pendingQueue.value.length > 0 && !isBusy.value) {
    const next = pendingQueue.value.shift()!;
    doSend(next, false);
  }
}

async function stopMessage() {
  const result = await callBackend("stopMessage");
  if (result?.sessionId) sessionId.value = result.sessionId;
  if (!result.ok && result.error) showNotice(result.error, "warn");
  if (result?.ok) {
    isBusy.value = false;
    if (busyTimeoutId) { clearTimeout(busyTimeoutId); busyTimeoutId = null; }
  }
}

async function chooseWorkspace() {
  const result = await callBackend("chooseWorkspace");
  if (!result?.ok) {
    if (result?.error) showNotice(result.error, "warn");
    return;
  }
  const state = await callBackend("getState");
  if (state?.workspacePath) {
    workspacePath.value = state.workspacePath;
    showNotice("项目目录已切换。", "ok");
  }
}

async function createSession() {
  if (isBusy.value) return;
  const result = await callBackend("newSession");
  if (result?.sessionId) {
    sessionId.value = result.sessionId;
  }
}

async function selectWorkspaceDir() {
  try {
    const result = await callBackend("selectDirectory", "");
    if (result && result.path) {
      if (editingProjectId.value) {
        editProjectPath.value = result.path;
        editProjectCustomPath.value = true;
      } else if (showProjectPanel.value) {
        newProjectPath.value = result.path;
        newProjectCustomPath.value = true;
      } else {
        workspacePath.value = result.path;
      }
    }
  } catch (e) {
    console.warn("selectDirectory failed:", e);
  }
}

async function openInExplorer(path: string) {
  try {
    await callBackend("openInExplorer", path);
  } catch (e) {
    console.warn("openInExplorer failed:", e);
  }
}

async function loadProjects() {
  try {
    const list = await callBackend("listProjects");
    projects.value = list || [];
    const active = await callBackend("getActiveProject");
    activeProject.value = active || null;
    if (active) {
      workspacePath.value = active.path || "";
      const convs = await callBackend("listConversations", active.id);
      projectConversations.value = convs || [];
    }
  } catch (e) {
    console.warn("loadProjects failed:", e);
  }
}

async function updateDefaultPath() {
  if (!newProjectName.value.trim()) {
    newProjectDefaultPath.value = "";
    return;
  }
  try {
    const path = await callBackend("getDefaultProjectPath", newProjectName.value.trim());
    newProjectDefaultPath.value = path || "";
  } catch (e) {
    console.warn("getDefaultProjectPath failed:", e);
  }
}

async function createProject() {
  if (!newProjectName.value.trim()) return;
  try {
    const path = newProjectCustomPath.value ? newProjectPath.value.trim() : "";
    const proj = await callBackend("createProject", newProjectName.value.trim(), path);
    if (proj) {
      activeProject.value = proj;
      workspacePath.value = proj.path || "";
      newProjectName.value = "";
      newProjectPath.value = "";
      newProjectCustomPath.value = false;
      newProjectDefaultPath.value = "";
      await loadProjects();
    }
  } catch (e) {
    console.warn("createProject failed:", e);
  }
}

async function switchProject(projectId: string) {
  try {
    const proj = await callBackend("switchProject", projectId);
    if (proj) {
      activeProject.value = proj;
      workspacePath.value = proj.path || "";
      messages.value = [];
      await createSession();
      await loadProjects();
    }
  } catch (e) {
    console.warn("switchProject failed:", e);
  }
}

function startEditProject(projectId: string) {
  const proj = projects.value.find((p: any) => p.id === projectId);
  if (!proj) return;
  editingProjectId.value = projectId;
  editProjectName.value = proj.name || "";
  editProjectPath.value = proj.path || "";
  editProjectCustomPath.value = false;
}

function cancelEditProject() {
  editingProjectId.value = null;
  editProjectName.value = "";
  editProjectPath.value = "";
  editProjectCustomPath.value = false;
}

async function saveEditProject() {
  if (!editingProjectId.value) return;
  try {
    const newName = editProjectName.value.trim();
    const newPath = editProjectCustomPath.value ? editProjectPath.value.trim() : "";
    const proj = await callBackend("updateProject", editingProjectId.value, newName, newPath);
    if (proj && !proj.error) {
      if (activeProject.value?.id === editingProjectId.value) {
        activeProject.value = proj;
        workspacePath.value = proj.path || "";
      }
      cancelEditProject();
      await loadProjects();
    }
  } catch (e) {
    console.warn("saveEditProject failed:", e);
  }
}

async function deleteProject(projectId: string) {
  if (!confirm("确定删除此项目？对话历史也将被删除。")) return;
  try {
    await callBackend("deleteProject", projectId);
    if (activeProject.value?.id === projectId) {
      activeProject.value = null;
      workspacePath.value = "";
    }
    await loadProjects();
  } catch (e) {
    console.warn("deleteProject failed:", e);
  }
}

async function loadConversationHistory(sessionId: string) {
  if (!activeProject.value) return;
  try {
    const msgs = await callBackend("loadConversation", activeProject.value.id, sessionId);
    if (msgs && Array.isArray(msgs)) {
      messages.value = msgs.map((m: any) => ({
        id: m.id || String(Math.random()),
        role: m.role || "assistant",
        text: m.text || "",
        time: m.time || new Date().toLocaleTimeString(),
        model: m.model,
        completedAt: m.completedAt,
        durationMs: m.durationMs,
        tokens: m.tokens,
        rating: m.rating,
        fileChanges: m.fileChanges || [],
      }));
      sessionFileChanges.value = messages.value.flatMap((m: any) => m.fileChanges || []);
    }
  } catch (e) {
    console.warn("loadConversationHistory failed:", e);
  }
}

async function copyConversationToProject(sessionId: string, targetProjectId: string) {
  if (!activeProject.value) return;
  try {
    await callBackend("copyConversation", activeProject.value.id, sessionId, targetProjectId);
    showNotice("对话已复制到目标项目", "ok");
  } catch (e) {
    console.warn("copyConversationToProject failed:", e);
  }
}

async function saveCurrentConversation() {
  if (!activeProject.value || messages.value.length === 0) return;
  try {
    const msgs = messages.value.map(m => ({
      id: m.id,
      role: m.role,
      text: m.text,
      time: m.time,
      model: m.model,
      completedAt: m.completedAt,
      durationMs: m.durationMs,
      tokens: m.tokens,
      rating: m.rating,
      fileChanges: m.fileChanges,
    }));
    await callBackend("saveConversation", activeProject.value.id, sessionId.value, JSON.stringify(msgs));
  } catch (e) {
    console.warn("saveCurrentConversation failed:", e);
  }
}

async function clearMessages() {
  messages.value = [];
  sessionFileChanges.value = [];
  await createSession();
}

function rateMessage(id: string, rating: number) {
  const target = messages.value.find((m) => m.id === id);
  if (target) {
    target.rating = target.rating === rating ? 0 : rating;
    saveCurrentConversation();
  }
}

function deleteMessage(id: string) {
  messages.value = messages.value.filter((m) => m.id !== id);
  saveCurrentConversation();
}

async function searchConversations() {
  if (!activeProject.value) return;
  try {
    const result = await callBackend("searchConversations", activeProject.value.id, convSearchQuery.value);
    if (Array.isArray(result)) {
      projectConversations.value = result;
    }
  } catch (e) {
    console.warn("searchConversations failed:", e);
  }
}

function clearConvSearch() {
  convSearchQuery.value = "";
  loadProjectConversations();
}

async function deleteConversation(sessionId: string) {
  if (!activeProject.value) return;
  try {
    await callBackend("deleteConversation", activeProject.value.id, sessionId);
    showNotice("对话已删除", "ok");
    loadProjectConversations();
  } catch (e) {
    console.warn("deleteConversation failed:", e);
  }
}

function startRenameConversation(sessionId: string, currentTitle: string) {
  editingConvId.value = sessionId;
  editingConvTitle.value = currentTitle;
}

async function saveRenameConversation(sessionId: string) {
  if (!activeProject.value || !editingConvTitle.value.trim()) return;
  try {
    await callBackend("renameConversation", activeProject.value.id, sessionId, editingConvTitle.value.trim());
    editingConvId.value = null;
    editingConvTitle.value = "";
    loadProjectConversations();
  } catch (e) {
    console.warn("renameConversation failed:", e);
  }
}

function cancelRenameConversation() {
  editingConvId.value = null;
  editingConvTitle.value = "";
}

function resendMessage(id: string) {
  const target = messages.value.find((m) => m.id === id);
  if (target && target.role === "user") {
    inputText.value = target.text;
    sendMessage();
  }
}

function editMessage(id: string) {
  const target = messages.value.find((m) => m.id === id);
  if (target && target.role === "user") {
    inputText.value = target.text;
  }
}

function branchFromMessage(id: string) {
  const idx = messages.value.findIndex((m) => m.id === id);
  if (idx < 0) return;
  const branchMsg = messages.value[idx];
  messages.value = messages.value.slice(0, idx + 1);
  sessionFileChanges.value = messages.value.flatMap((m: any) => m.fileChanges || []);
  saveCurrentConversation();
  inputText.value = "";
  showNotice(`已从"${branchMsg.text.slice(0, 20)}..."处分支，后续消息已移除`, "ok");
}

function applyRole(roleKey: string) {
  const preset = ROLE_PRESETS[roleKey];
  if (!preset) return;
  if (activeRole.value === roleKey) {
    activeRole.value = "";
    return;
  }
  activeRole.value = roleKey;
  systemPrompt.value = preset.prompt;
  temperature.value = preset.temp;
  saveLocalSettings();
}

function applyWorkflow(workflowKey: string) {
  const workflow = WORKFLOW_PRESETS[workflowKey];
  if (!workflow) return;
  inputText.value = workflow.prompt;
  showNotice(`已加载"${workflow.name}"工作流：${workflow.steps.join(" → ")}`, "ok");
}

async function loadMemoryContent() {
  if (!activeProject.value) return;
  try {
    const content = await callBackend("getClaudeMd", activeProject.value.path);
    memoryContent.value = content || "";
    const global = await callBackend("getGlobalClaudeMd");
    globalMemoryContent.value = global || "";
  } catch (e) {
    console.warn("loadMemoryContent failed:", e);
  }
}

async function saveMemoryContent() {
  if (!activeProject.value) return;
  try {
    await callBackend("saveClaudeMd", activeProject.value.path, memoryContent.value);
    showNotice("项目记忆已保存", "ok");
  } catch (e) {
    console.warn("saveMemoryContent failed:", e);
  }
}

async function saveGlobalMemory() {
  try {
    await callBackend("saveGlobalClaudeMd", globalMemoryContent.value);
    showNotice("全局记忆已保存", "ok");
  } catch (e) {
    console.warn("saveGlobalMemory failed:", e);
  }
}

function appendMemory(text: string) {
  if (!text.trim()) return;
  if (memoryContent.value && !memoryContent.value.endsWith("\n")) {
    memoryContent.value += "\n";
  }
  memoryContent.value += "- " + text.trim() + "\n";
  saveMemoryContent();
}

async function loadSmartMemories() {
  if (!activeProject.value) return;
  try {
    const result = await callBackend("listMemories", activeProject.value.path);
    smartMemories.value = Array.isArray(result) ? result : [];
  } catch (e) {
    console.warn("loadSmartMemories failed:", e);
  }
}

async function searchSmartMemories() {
  if (!activeProject.value) return;
  try {
    const result = await callBackend("searchMemories", activeProject.value.path, memorySearchQuery.value);
    smartMemories.value = Array.isArray(result) ? result : [];
  } catch (e) {
    console.warn("searchSmartMemories failed:", e);
  }
}

async function addSmartMemory() {
  if (!activeProject.value || !newMemoryTitle.value.trim() || !newMemoryContent.value.trim()) return;
  try {
    const filename = newMemoryTitle.value.trim().replace(/\s+/g, "-").toLowerCase();
    await callBackend("saveMemory", activeProject.value.path, filename, newMemoryContent.value.trim(), newMemoryType.value);
    newMemoryTitle.value = "";
    newMemoryContent.value = "";
    loadSmartMemories();
    showNotice("记忆已添加", "ok");
  } catch (e) {
    console.warn("addSmartMemory failed:", e);
  }
}

async function deleteSmartMemory(filename: string) {
  if (!activeProject.value) return;
  try {
    await callBackend("deleteMemory", activeProject.value.path, filename);
    loadSmartMemories();
    showNotice("记忆已删除", "ok");
  } catch (e) {
    console.warn("deleteSmartMemory failed:", e);
  }
}

async function extractMemoriesFromConversation() {
  if (!activeProject.value || messages.value.length === 0) return;
  try {
    const msgs = messages.value.map(m => ({ role: m.role, text: m.text }));
    const result = await callBackend("autoExtractMemories", activeProject.value.path, JSON.stringify(msgs));
    if (Array.isArray(result) && result.length > 0) {
      const summary = result.map((r: any) => `${r.type}: ${r.count}条`).join(", ");
      showNotice(`已提取记忆: ${summary}`, "ok");
      loadSmartMemories();
    } else {
      showNotice("未发现可提取的记忆", "warn");
    }
  } catch (e) {
    console.warn("extractMemories failed:", e);
  }
}

async function loadProjectTemplates() {
  try {
    const result = await callBackend("listProjectTemplates", templateCategory.value);
    projectTemplates.value = Array.isArray(result) ? result : [];
  } catch (e) {
    console.warn("loadProjectTemplates failed:", e);
  }
}

function useTemplate(template: any) {
  if (!template) return;
  inputText.value = template.prompt;
  showTemplatePanel.value = false;
  showNotice(`已加载"${template.name}"模板`, "ok");
}

async function scaffoldFromTemplate(template: any) {
  if (!template || !activeProject.value) return;
  try {
    const result = await callBackend("createProjectFromTemplate", activeProject.value.path, template.id);
    const data = typeof result === "string" ? JSON.parse(result) : result;
    if (data.success) {
      showNotice(`已创建项目脚手架：${data.files?.length || 0} 个文件`, "ok");
      if (data.init_cmd) {
        showNotice(`请运行: ${data.init_cmd}`, "warn");
      }
    } else {
      showNotice(data.error || "创建脚手架失败", "warn");
    }
  } catch (e) {
    console.warn("scaffoldFromTemplate failed:", e);
  }
}

async function addCustomTemplate() {
  if (!newTplName.value.trim() || !newTplPrompt.value.trim()) return;
  try {
    await callBackend("saveCustomTemplate", newTplName.value.trim(), newTplCategory.value, newTplDesc.value.trim(), newTplPrompt.value.trim(), "");
    newTplName.value = "";
    newTplDesc.value = "";
    newTplPrompt.value = "";
    showAddTemplate.value = false;
    loadProjectTemplates();
    showNotice("自定义模板已保存", "ok");
  } catch (e) {
    console.warn("addCustomTemplate failed:", e);
  }
}

async function deleteCustomTemplate(tplId: string) {
  try {
    await callBackend("deleteCustomTemplate", tplId);
    loadProjectTemplates();
    showNotice("模板已删除", "ok");
  } catch (e) {
    console.warn("deleteCustomTemplate failed:", e);
  }
}

function openPreview() {
  if (!activeProject.value) {
    showNotice("请先选择项目", "warn");
    return;
  }
  const projPath = activeProject.value.path;
  previewPath.value = projPath;
  const indexPath = projPath.replace(/\\/g, "/") + "/index.html";
  previewUrl.value = `file:///${indexPath}`;
  showPreviewPanel.value = true;
}

function refreshPreview() {
  if (previewUrl.value) {
    const url = previewUrl.value;
    previewUrl.value = "";
    nextTick(() => { previewUrl.value = url; });
  }
}

function closePreview() {
  showPreviewPanel.value = false;
  previewUrl.value = "";
}

function addTask() {
  if (!newTaskName.value.trim()) return;
  taskList.value.push({
    id: makeId(),
    name: newTaskName.value.trim(),
    desc: newTaskDesc.value.trim(),
    status: "pending",
    createdAt: new Date().toISOString(),
  });
  newTaskName.value = "";
  newTaskDesc.value = "";
}

function updateTaskStatus(taskId: string, status: "pending" | "in_progress" | "done") {
  const task = taskList.value.find(t => t.id === taskId);
  if (task) task.status = status;
}

function removeTask(taskId: string) {
  taskList.value = taskList.value.filter(t => t.id !== taskId);
}

function executeTaskAsPrompt(task: any) {
  inputText.value = task.desc || task.name;
  task.status = "in_progress";
  showTaskPanel.value = false;
  sendMessage();
}

function setCollaborationMode(mode: "none" | "plan-code-review" | "pair" | "review-only") {
  collaborationMode.value = mode;
  if (mode === "none") {
    collabPhase.value = "planning";
    return;
  }
  collabPhase.value = "planning";
  let prompt = "";
  if (mode === "plan-code-review") {
    prompt = "我们采用规划-编码-审查三阶段协作模式。当前是【规划阶段】，请先分析需求，制定详细的开发计划，列出所有需要完成的任务。不要直接编码，只做规划。";
  } else if (mode === "pair") {
    prompt = "我们采用结对编程模式。你和我交替工作：你写一段代码，我审查并提出修改意见，你再修改。请先从第一个功能开始编码。";
  } else if (mode === "review-only") {
    prompt = "我们采用纯审查模式。你只负责审查我提供的代码，指出问题、安全漏洞、性能瓶颈和改进建议，不直接修改代码。请等待我提供代码。";
  }
  if (prompt) {
    inputText.value = prompt;
    sendMessage();
  }
}

function advanceCollabPhase() {
  if (collaborationMode.value === "plan-code-review") {
    if (collabPhase.value === "planning") {
      collabPhase.value = "coding";
      inputText.value = "规划阶段完成，现在进入【编码阶段】。请按照规划逐步实现功能，每完成一个功能点向我确认。";
    } else if (collabPhase.value === "coding") {
      collabPhase.value = "reviewing";
      inputText.value = "编码阶段完成，现在进入【审查阶段】。请全面审查已编写的代码，检查：1.功能完整性 2.代码质量 3.安全漏洞 4.性能问题 5.最佳实践遵循情况。";
    } else {
      collabPhase.value = "planning";
      collaborationMode.value = "none";
      showNotice("协作流程已完成", "ok");
      return;
    }
    sendMessage();
  }
}

function handleInputChange() {
  if (inputText.value.startsWith("/")) {
    slashMenuFilter.value = inputText.value.slice(1).toLowerCase();
    showSlashMenu.value = true;
  } else {
    showSlashMenu.value = false;
  }
}

function executeSlashCommand(cmdKey: string) {
  showSlashMenu.value = false;
  inputText.value = "";
  const cmd = SLASH_COMMANDS[cmdKey];
  if (!cmd) return;
  switch (cmd.action) {
    case "clear":
      clearMessages();
      break;
    case "save":
      saveCurrentConversation();
      showNotice("对话已保存", "ok");
      break;
    case "role":
      showPanel.value = true;
      break;
    case "memory":
      showPanel.value = true;
      showMemoryPanel.value = true;
      loadMemoryContent();
      break;
    case "export":
      exportConversation("markdown");
      break;
    case "files":
      showFileChanges.value = !showFileChanges.value;
      break;
    case "compact":
      inputText.value = "请总结我们目前的对话要点，然后我们继续。保持简洁。";
      sendMessage();
      break;
    case "extract":
      extractMemoriesFromConversation();
      break;
    case "template":
      showTemplatePanel.value = true;
      loadProjectTemplates();
      break;
    case "preview":
      openPreview();
      break;
    case "task":
      showTaskPanel.value = !showTaskPanel.value;
      break;
    case "collab":
      showPanel.value = true;
      break;
    case "settings":
      showSettings.value = true;
      break;
  }
}

function exportConversation(format: string) {
  if (messages.value.length === 0) return;
  let content = "";
  const title = messages.value.find(m => m.role === "user")?.text?.slice(0, 50) || "对话";
  if (format === "markdown") {
    content += `# ${title}\n\n`;
    content += `导出时间: ${new Date().toLocaleString("zh-CN")}\n\n---\n\n`;
    for (const m of messages.value) {
      const label = m.role === "user" ? userName.value : assistantName.value;
      const time = m.time || "";
      content += `### ${label} ${time}\n\n${m.text}\n\n`;
      if (m.toolStatus) {
        content += `> ${m.toolStatus}\n\n`;
      }
      if (m.rating) {
        content += `> 评分: ${"★".repeat(m.rating)}${"☆".repeat(5 - m.rating)}\n\n`;
      }
    }
  } else {
    content = JSON.stringify({
      title,
      exported_at: new Date().toISOString(),
      messages: messages.value.map(m => ({
        role: m.role,
        text: m.text,
        time: m.time,
        model: m.model,
        tokens: m.tokens,
        rating: m.rating,
        fileChanges: m.fileChanges,
      })),
    }, null, 2);
  }
  const blob = new Blob([content], { type: format === "markdown" ? "text/markdown" : "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${title.slice(0, 20)}.${format === "markdown" ? "md" : "json"}`;
  a.click();
  URL.revokeObjectURL(url);
  showNotice(`对话已导出为 ${format === "markdown" ? "Markdown" : "JSON"}`, "ok");
}

function trackFileChange(tool: string, filePath: string, action: "create" | "modify" | "delete") {
  const change: FileChange = { tool, path: filePath, action, time: new Date().toLocaleTimeString() };
  sessionFileChanges.value.push(change);
  const lastMsg = messages.value[messages.value.length - 1];
  if (lastMsg) {
    if (!lastMsg.fileChanges) lastMsg.fileChanges = [];
    lastMsg.fileChanges.push(change);
  }
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  const min = Math.floor(ms / 60000);
  const sec = Math.floor((ms % 60000) / 1000);
  return `${min}m${sec}s`;
}

async function saveSettings() {
  if (isBusy.value) return;

  const activeConfig = getActiveModelConfig();

  if (runMode.value === "ollama") {
    if (!ollamaModel.value.trim()) {
      showNotice("请先选择一个模型", "warn");
      return;
    }

    const localBase = ollamaBaseUrl.value.trim() || "http://127.0.0.1:11434";
    const payload: Record<string, string> = {
      MODEL_PROVIDER: "ollama",
      OLLAMA_BASE_URL: localBase,
      OLLAMA_MODEL: ollamaModel.value.trim(),
      API_TIMEOUT_MS: settings.API_TIMEOUT_MS || "3000000",
      DISABLE_TELEMETRY: "1",
      CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
      AI_LANGUAGE: activeConfig.language || "zh",
      AI_TEMPERATURE: activeConfig.temperature || "",
      AI_MAX_TOKENS: activeConfig.maxTokens || "",
      SYSTEM_PROMPT: activeConfig.systemPrompt || "",
    };

    const saved = await callBackend("saveSettings", JSON.stringify(payload));
    applySettings(saved);

    const selectedModel = cloudModels.value.find((m) => m.id === ollamaModel.value.trim());
    if (selectedModel && selectedModel.toolSupport === false) {
      showNotice("配置已保存。⚠ 该模型不支持工具调用，编程功能将受限。建议选择带 ★ 标记的模型。", "warn");
    } else {
      showNotice("配置已保存。", "ok");
    }
    return;
  }

  if (runMode.value === "api") {
    if (!apiBaseUrl.value.trim()) {
      showNotice("请填写 API 服务地址", "warn");
      return;
    }
    if (!apiModel.value.trim()) {
      showNotice("请填写或选择模型名称", "warn");
      return;
    }

    const payload: Record<string, string> = {
      MODEL_PROVIDER: "api",
      API_BASE_URL: apiBaseUrl.value.trim(),
      API_MODEL: apiModel.value.trim(),
      API_KEY: apiKey.value.trim(),
      API_TIMEOUT_MS: settings.API_TIMEOUT_MS || "3000000",
      DISABLE_TELEMETRY: "1",
      CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
      AI_LANGUAGE: activeConfig.language || "zh",
      AI_TEMPERATURE: activeConfig.temperature || "",
      AI_MAX_TOKENS: activeConfig.maxTokens || "",
      SYSTEM_PROMPT: activeConfig.systemPrompt || "",
    };

    const saved = await callBackend("saveSettings", JSON.stringify(payload));
    applySettings(saved);
    showNotice("配置已保存。API 模式已启用。", "ok");
    return;
  }

  if (!apiKey.value.trim()) {
    showNotice("请先填写 API Key", "warn");
    return;
  }
  const cloudModel = "openrouter/auto";
  const cloudBaseUrl = cloudBaseUrlByProvider("openrouter");

  const payload: Record<string, string> = {
    MODEL_PROVIDER: "anthropic",
    ANTHROPIC_BASE_URL: cloudBaseUrl,
    ANTHROPIC_API_KEY: apiKey.value.trim(),
    ANTHROPIC_AUTH_TOKEN: apiKey.value.trim(),
    ANTHROPIC_MODEL: cloudModel,
    ANTHROPIC_DEFAULT_SONNET_MODEL: "",
    ANTHROPIC_DEFAULT_HAIKU_MODEL: "",
    ANTHROPIC_DEFAULT_OPUS_MODEL: "",
    API_TIMEOUT_MS: settings.API_TIMEOUT_MS || "3000000",
    DISABLE_TELEMETRY: "1",
    CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
    AI_LANGUAGE: activeConfig.language || "zh",
    AI_TEMPERATURE: activeConfig.temperature || "",
    AI_MAX_TOKENS: activeConfig.maxTokens || "",
    SYSTEM_PROMPT: activeConfig.systemPrompt || "",
  };

  const saved = await callBackend("saveSettings", JSON.stringify(payload));
  applySettings(saved);
  showNotice("已保存。云端模型已固定为 openrouter/auto。", "ok");
}

async function clearModelFields() {
  if (isBusy.value) return;
  const saved = await callBackend("clearModelSettings");
  applySettings(saved);
  showNotice("模型字段已清空。", "warn");
}

async function loadApiModels() {
  loadingModels.value = true;
  try {
    const payload = { source: "api", baseUrl: apiBaseUrl.value, apiKey: apiKey.value };
    const result = await callBackend("listModels", JSON.stringify(payload));
    if (result && result.loading) {
      return;
    }
    if (!result || !result.ok) {
      showNotice(result?.error || "API 模型列表加载失败", "warn");
      loadingModels.value = false;
      return;
    }
    apiModels.value = result.models || [];
    cloudModels.value = result.models || [];
    loadingModels.value = false;
    showNotice(`已加载 ${apiModels.value.length} 个 API 模型`, "ok");
  } catch (e) {
    console.error("[loadApiModels] error:", e);
    showNotice("API 模型列表加载异常", "warn");
    loadingModels.value = false;
  }
}

async function fetchApiKey() {
  if (!apiBaseUrl.value.trim()) {
    showNotice("请先填写 API 服务地址", "warn");
    return;
  }
  try {
    const result = await callBackend("fetchApiKey", JSON.stringify({ baseUrl: apiBaseUrl.value.trim(), adminKey: "admin" }));
    if (result && result.ok && result.key) {
      apiKey.value = result.key;
      showNotice("API Key 已自动获取", "ok");
    } else {
      if (result?.adminUrl) {
        window.open(result.adminUrl, "_blank");
        showNotice("已打开 API 服务管理界面，请手动创建 API Key", "warn");
      } else {
        showNotice(result?.error || "未能自动获取 API Key", "warn");
      }
    }
  } catch (e) {
    console.error("[fetchApiKey] error:", e);
    showNotice("获取 API Key 异常", "warn");
  }
}

async function addQwenAccount() {
  if (!qwenToken.value.trim()) {
    showNotice("请输入 chat.qwen.ai 的 Token", "warn");
    return;
  }
  try {
    const result = await callBackend("addQwenAccount", JSON.stringify({
      baseUrl: apiBaseUrl.value.trim(),
      token: qwenToken.value.trim(),
      adminKey: "admin",
    }));
    if (result && result.ok) {
      qwenToken.value = "";
      qwenAccountCount.value = -1;
      showNotice("上游账户添加成功", "ok");
      await checkQwenAccounts();
    } else {
      showNotice(result?.error || "添加账户失败", "warn");
    }
  } catch (e) {
    console.error("[addQwenAccount] error:", e);
    showNotice("添加账户异常", "warn");
  }
}

async function deleteQwenAccount(email: string) {
  if (!email) return;
  try {
    const result = await callBackend("deleteQwenAccount", JSON.stringify({
      baseUrl: apiBaseUrl.value.trim(),
      email: email,
      adminKey: "admin",
    }));
    if (result && result.ok) {
      showNotice(`已删除账户: ${email}`, "ok");
      await checkQwenAccounts();
    } else {
      showNotice(result?.error || "删除账户失败", "warn");
    }
  } catch (e) {
    console.error("[deleteQwenAccount] error:", e);
    showNotice("删除账户异常", "warn");
  }
}

async function setStickyAccount(email: string) {
  if (!email) return;
  try {
    const result = await callBackend("setStickyAccount", JSON.stringify({
      baseUrl: apiBaseUrl.value.trim(),
      email: email,
      adminKey: "admin",
    }));
    if (result && result.ok) {
      stickyEmail.value = email;
      showNotice(result.message || `已设置 ${email} 为优先账户`, "ok");
    } else {
      showNotice(result?.error || "设置优先账户失败", "warn");
    }
  } catch (e) {
    showNotice("设置优先账户异常", "warn");
  }
}

async function clearStickyAccount() {
  try {
    const result = await callBackend("clearStickyAccount", JSON.stringify({
      baseUrl: apiBaseUrl.value.trim(),
      adminKey: "admin",
    }));
    if (result && result.ok) {
      stickyEmail.value = "";
      showNotice(result.message || "已恢复自动轮换", "ok");
    } else {
      showNotice(result?.error || "清除优先账户失败", "warn");
    }
  } catch (e) {
    showNotice("清除优先账户异常", "warn");
  }
}

let _loginPollTimer: ReturnType<typeof setInterval> | null = null;

async function loginQwenAccount() {
  if (qwenLoginBusy.value) return;
  if (!loginEmail.value.trim() || !loginPassword.value.trim()) {
    showNotice("请输入邮箱和密码", "warn");
    return;
  }
  qwenLoginBusy.value = true;
  qwenLoginError.value = "";

  try {
    const startResult = await callBackend("startQwenLogin", JSON.stringify({
      baseUrl: apiBaseUrl.value.trim(),
      email: loginEmail.value.trim(),
      password: loginPassword.value.trim(),
      adminKey: "admin",
    }));
    if (!startResult || !startResult.ok) {
      qwenLoginBusy.value = false;
      qwenLoginError.value = startResult?.error || "启动登录失败";
      return;
    }

    _loginPollTimer = setInterval(async () => {
      try {
        const result = await callBackend("pollQwenLogin");
        if (!result) return;
        if (result.done) {
          qwenLoginBusy.value = false;
          if (_loginPollTimer) { clearInterval(_loginPollTimer); _loginPollTimer = null; }
          if (result.success) {
            loginEmail.value = "";
            loginPassword.value = "";
            showNotice(`登录成功: ${result.email}`, "ok");
            await checkQwenAccounts();
          } else {
            qwenLoginError.value = result.error || "登录失败";
          }
        }
      } catch { /* ignore */ }
    }, 2000);
  } catch (e) {
    console.error("[loginQwenAccount] error:", e);
    qwenLoginBusy.value = false;
    qwenLoginError.value = "登录异常";
  }
}

async function checkQwenAccounts() {
  try {
    const result = await callBackend("listQwenAccounts", JSON.stringify({
      baseUrl: apiBaseUrl.value.trim(),
      adminKey: "admin",
    }));
    if (result && result.ok) {
      qwenAccountCount.value = result.count || 0;
      qwenAccounts.value = (result.accounts || []).map((a: any) => ({
        email: a.email || "",
        valid: !!a.valid,
        status_code: a.status_code || "",
        activation_pending: !!a.activation_pending,
      }));
      stickyEmail.value = result.sticky_email || "";
      saveLocalSettings();
    }
  } catch { /* ignore */ }
}

const qwenRegisterLogs = ref<string[]>([]);
let _registerPollTimer: ReturnType<typeof setInterval> | null = null;

watch(qwenRegisterLogs, () => {
  nextTick(() => {
    const box = document.querySelector(".register-log-box");
    if (box) box.scrollTop = box.scrollHeight;
  });
}, { deep: true });

async function autoRegisterQwenAccount() {
  if (qwenRegisterBusy.value) return;
  qwenRegisterBusy.value = true;
  qwenRegisterLogs.value = ["[启动] 正在发起自动注册..."];

  try {
    const startResult = await callBackend("startQwenRegister", JSON.stringify({
      baseUrl: apiBaseUrl.value.trim(),
      adminKey: "admin",
      email: regEmail.value.trim(),
      password: regPassword.value.trim(),
      username: regUsername.value.trim(),
    }));
    if (!startResult || !startResult.ok) {
      qwenRegisterBusy.value = false;
      showNotice(startResult?.error || "启动注册失败", "warn");
      return;
    }

    qwenRegisterLogs.value.push("[启动] 注册请求已发送，等待服务端处理...");
    _registerPollTimer = setInterval(pollRegisterProgress, 2000);
  } catch (e) {
    console.error("[autoRegisterQwenAccount] error:", e);
    qwenRegisterBusy.value = false;
    showNotice("启动注册异常", "warn");
  }
}

async function pollRegisterProgress() {
  try {
    const result = await callBackend("pollQwenRegister");
    if (!result) return;

    if (result.newLines && result.newLines.length > 0) {
      for (const line of result.newLines) {
        const clean = line.replace(/^[\d\-:\s]+/, "").trim();
        if (clean) qwenRegisterLogs.value.push(clean);
      }
    }

    if (result.done) {
      if (_registerPollTimer) {
        clearInterval(_registerPollTimer);
        _registerPollTimer = null;
      }
      qwenRegisterBusy.value = false;
      if (result.success) {
        qwenRegisterLogs.value.push(`[完成] ✓ 注册成功！邮箱: ${result.email || ""}`);
        qwenAccountCount.value = -1;
        showNotice(`自动注册成功！邮箱: ${result.email || ""}`, "ok");
        await checkQwenAccounts();
      } else {
        qwenRegisterLogs.value.push(`[失败] ✗ ${result.error || "注册失败"}`);
        showNotice(result.error || "自动注册失败", "warn");
      }
    }
  } catch (e) {
    console.error("[pollRegisterProgress] error:", e);
  }
}

async function apiStepAutoRun() {
  if (apiStepBusy.value) return;
  apiStepBusy.value = true;

  try {
    while (apiStepProgress.value < apiSteps.length) {
      const step = apiSteps[apiStepProgress.value];
      apiStepMessage.value = `${step.label}中...`;

      if (step.action === "check") {
        const result = await callBackend("checkApiService", JSON.stringify({ baseUrl: apiBaseUrl.value.trim() }));
        if (result && result.running) {
          apiServiceRunning.value = true;
          if (result.warning) {
            showNotice(result.warning, "warn");
          }
          if (result.accountCount !== undefined) {
            qwenAccountCount.value = result.accountCount;
          } else {
            await checkQwenAccounts();
          }
          apiStepMessage.value = "服务已运行";
          apiStepProgress.value++;
          continue;
        }
        apiStepMessage.value = "服务未启动，准备启动...";
        apiStepProgress.value++;
      }

      else if (step.action === "start") {
        const port = parseInt(apiPort.value.replace(/\D/g, "")) || 7777;
        const result = await callBackend("startQwen2Api", JSON.stringify({ port }));
        if (result && result.ok) {
          if (result.baseUrl) {
            const m = result.baseUrl.match(/^https?:\/\/([^:/]+)(?::(\d+))?/);
            if (m) { apiHost.value = m[1]; if (m[2]) apiPort.value = m[2]; }
          }
          if (result.message && result.message.includes("已在运行")) {
            apiServiceRunning.value = true;
            apiStepMessage.value = "服务已运行";
            apiStepProgress.value++;
            continue;
          }
          apiStepMessage.value = "等待服务就绪...";
          const ready = await waitForApiService();
          if (ready) {
            apiServiceRunning.value = true;
            apiStepMessage.value = "服务已启动";
            await checkQwenAccounts();
            apiStepProgress.value++;
          } else {
            apiStepMessage.value = "服务启动超时";
            break;
          }
        } else {
          apiStepMessage.value = result?.error || "启动失败";
          showNotice(result?.error || "API 服务启动失败", "warn");
          break;
        }
      }

      else if (step.action === "key") {
        const result = await callBackend("fetchApiKey", JSON.stringify({ baseUrl: apiBaseUrl.value.trim(), adminKey: "admin" }));
        if (result && result.ok && result.key) {
          apiKey.value = result.key;
          apiStepMessage.value = "Key 已获取";
          apiStepProgress.value++;
        } else {
          apiStepMessage.value = "Key 获取失败，可手动输入";
          apiStepProgress.value++;
        }
      }

      else if (step.action === "models") {
        loadingModels.value = true;
        const payload = { source: "api", baseUrl: apiBaseUrl.value, apiKey: apiKey.value };
        const result = await callBackend("listModels", JSON.stringify(payload));
        loadingModels.value = false;
        if (result && result.ok) {
          apiModels.value = result.models || [];
          cloudModels.value = result.models || [];
          if (apiModels.value.length > 0 && !apiModel.value) {
            apiModel.value = apiModels.value[0].id;
          }
          apiStepMessage.value = `已加载 ${apiModels.value.length} 个模型`;
          apiStepProgress.value++;
        } else {
          apiStepMessage.value = result?.error || "模型加载失败";
          apiStepProgress.value++;
        }
      }
    }

    if (apiStepProgress.value >= apiSteps.length) {
      apiStepMessage.value = "✓ API 服务已就绪，可以开始编程";
      showNotice("API 服务已就绪", "ok");
    }
  } catch (e) {
    console.error("[apiStepAutoRun] error:", e);
    apiStepMessage.value = "配置异常: " + (e as Error).message;
  } finally {
    apiStepBusy.value = false;
  }
}

async function stopApiService() {
  try {
    const result = await callBackend("stopQwen2Api", JSON.stringify({ baseUrl: apiBaseUrl.value.trim() }));
    if (result && result.ok) {
      apiStepProgress.value = 0;
      apiServiceRunning.value = false;
      apiStepMessage.value = "API 服务已停止，点击「一键启动」重新启动";
      apiModels.value = [];
      showNotice(result.message || "API 服务已停止", "ok");
    } else {
      showNotice(result?.error || "停止服务失败", "warn");
    }
  } catch (e) {
    showNotice("停止服务异常: " + (e as Error).message, "warn");
  }
}

function waitForApiService(maxRetries = 30): Promise<boolean> {
  return new Promise((resolve) => {
    let count = 0;
    const timer = setInterval(async () => {
      count++;
      try {
        const result = await callBackend("checkApiService", JSON.stringify({ baseUrl: apiBaseUrl.value.trim() }));
        if (result && result.running) {
          clearInterval(timer);
          resolve(true);
          return;
        }
      } catch { /* ignore */ }
      if (count >= maxRetries) {
        clearInterval(timer);
        resolve(false);
      }
    }, 2000);
  });
}

async function loadCloudModels(source: "openrouter" | "anthropic" | "ollama") {
  loadingModels.value = true;
  try {
    let payload: any;
    if (source === "ollama") {
      payload = { source, baseUrl: ollamaBaseUrl.value };
    } else {
      payload = { source, apiKey: apiKey.value };
    }
    const result = await callBackend("listModels", JSON.stringify(payload));
    if (result && result.loading) {
      return;
    }
    if (!result || !result.ok) {
      showNotice(result?.error || "模型列表加载失败", "warn");
      return;
    }
    cloudModels.value = result.models || [];
    showNotice(`已加载 ${cloudModels.value.length} 个模型`, "ok");
  } catch (e) {
    console.error("[loadCloudModels] error:", e);
    showNotice("模型列表加载异常", "warn");
  } finally {
    loadingModels.value = false;
  }
}

async function detectModels() {
  loadingModels.value = true;
  try {
    const payload = { source: "ollama", baseUrl: ollamaBaseUrl.value, checkHealth: true };
    const result = await callBackend("listModels", JSON.stringify(payload));
    if (result && result.loading) {
      return;
    }
    if (!result || !result.ok) {
      showNotice(result?.error || "模型检测失败", "warn");
      return;
    }
    cloudModels.value = result.models || [];
    const broken = cloudModels.value.filter((m) => m.loadable === false);
    const noTool = cloudModels.value.filter((m) => m.toolSupport === false);
    const okCount = cloudModels.value.filter((m) => m.loadable !== false && m.toolSupport === true).length;
    let msg = `检测完成: ${okCount}个可用`;
    if (broken.length > 0) msg += `，${broken.length}个损坏`;
    if (noTool.length > 0) msg += `，${noTool.length}个无工具`;
    showNotice(msg, broken.length > 0 ? "warn" : "ok");
  } catch (e) {
    console.error("[detectModels] error:", e);
    showNotice("模型检测异常", "warn");
  } finally {
    loadingModels.value = false;
  }
}

async function deleteModel(modelName: string) {
  const display = displayName(modelName);
  if (!confirm(`确定要卸载模型「${display}」吗？\n此操作不可撤销，模型文件将被删除。`)) return;
  const result = await callBackend("deleteModel", JSON.stringify({ name: modelName }));
  if (result?.ok) {
    cloudModels.value = cloudModels.value.filter((m) => m.id !== modelName);
    if (ollamaModel.value === modelName) ollamaModel.value = "";
    showNotice(`已卸载 ${display}`, "ok");
  } else {
    showNotice(result?.error || "卸载失败", "warn");
  }
}

const recommendedModels = ref<any[]>([]);
const showRecommendations = ref(false);
const librarySearchQuery = ref("");
const libraryResults = ref<any[]>([]);
const librarySearching = ref(false);
const pullingModel = ref("");
const showModelManager = ref(false);

async function loadRecommendations() {
  const result = await callBackend("recommendModels");
  if (result?.ok) {
    recommendedModels.value = result.models || [];
    showRecommendations.value = true;
  }
}

async function detectHardware() {
  const result = await callBackend("detectHardware");
  if (result) {
    hardwareInfo.value = result;
  }
}

async function searchOllamaLibrary() {
  if (!librarySearchQuery.value.trim()) return;
  librarySearching.value = true;
  try {
    const result = await callBackend("searchOllamaLibrary", librarySearchQuery.value.trim());
    if (result?.ok) {
      libraryResults.value = result.models || [];
    } else {
      showNotice(result?.error || "搜索失败", "warn");
    }
  } catch (e) {
    console.warn("searchOllamaLibrary failed:", e);
  } finally {
    librarySearching.value = false;
  }
}

async function pullModel(modelName: string) {
  pullingModel.value = modelName;
  showNotice(`正在下载 ${displayName(modelName)}...`, "ok");
  try {
    const result = await callBackend("pullModel", JSON.stringify({ name: modelName }));
    if (result?.loading) {
      return;
    }
    if (result?.ok) {
      showNotice(`${displayName(modelName)} 下载完成`, "ok");
      await detectModels();
    } else {
      showNotice(result?.error || "下载失败", "warn");
    }
  } catch (e) {
    showNotice("下载失败", "warn");
  } finally {
    pullingModel.value = "";
  }
}

const categorizedModels = computed(() => {
  const models = cloudModels.value;
  const local: any[] = [];
  const cloud: any[] = [];
  for (const m of models) {
    if (m.provider === "ollama") {
      local.push(m);
    } else {
      cloud.push(m);
    }
  }
  return { local, cloud };
});

// 从 localStorage 加载设置
function loadLocalSettings() {
  try {
    const saved = localStorage.getItem("claude-desktop-settings");
    if (saved) {
      const data = JSON.parse(saved);
      if (typeof data.autoApprove === "boolean") {
        autoApprove.value = data.autoApprove;
        if (!data.autoApprove) toolApprovalMode.value = "manual";
      }
      if (typeof data.toolApprovalMode === "string") {
        toolApprovalMode.value = data.toolApprovalMode;
        autoApprove.value = data.toolApprovalMode === "auto";
      }
      if (typeof data.runMode === "string") {
        runMode.value = data.runMode;
      }
      if (typeof data.apiHost === "string") {
        apiHost.value = data.apiHost;
      }
      if (typeof data.apiPort === "string") {
        apiPort.value = data.apiPort;
      }
      if (typeof data.apiModel === "string") {
        apiModel.value = data.apiModel;
      }
      if (typeof data.apiKey === "string") {
        apiKey.value = data.apiKey;
      }
      if (typeof data.ollamaBaseUrl === "string") {
        ollamaBaseUrl.value = data.ollamaBaseUrl;
      }
      if (typeof data.ollamaModel === "string") {
        ollamaModel.value = data.ollamaModel;
      }
      if (typeof data.apiStepProgress === "number") {
        apiStepProgress.value = data.apiStepProgress;
      }
      if (Array.isArray(data.qwenAccounts)) {
        qwenAccounts.value = data.qwenAccounts;
        qwenAccountCount.value = data.qwenAccounts.length;
      }
      if (typeof data.stickyEmail === "string") {
        stickyEmail.value = data.stickyEmail;
      }
      if (typeof data.activeRole === "string") {
        activeRole.value = data.activeRole;
      }
    }
  } catch (e) {
    console.error("Failed to load local settings:", e);
  }
}

function saveLocalSettings() {
  try {
    const data = {
      autoApprove: autoApprove.value,
      toolApprovalMode: toolApprovalMode.value,
      runMode: runMode.value,
      apiHost: apiHost.value,
      apiPort: apiPort.value,
      apiModel: apiModel.value,
      apiKey: apiKey.value,
      ollamaBaseUrl: ollamaBaseUrl.value,
      ollamaModel: ollamaModel.value,
      apiStepProgress: apiStepProgress.value,
      qwenAccounts: qwenAccounts.value,
      stickyEmail: stickyEmail.value,
      activeRole: activeRole.value,
    };
    localStorage.setItem("claude-desktop-settings", JSON.stringify(data));
  } catch (e) {
    console.error("Failed to save local settings:", e);
  }
}

// 监听设置变化，实时保存
watch(
  [
    autoApprove,
    toolApprovalMode,
    runMode,
    apiHost,
    apiPort,
    apiModel,
    apiKey,
    ollamaBaseUrl,
    ollamaModel,
    apiStepProgress,
  ],
  () => {
    saveLocalSettings();
  },
  { deep: true }
);

// 同时自动调用 saveSettings 保存到后端
watch(
  [
    runMode,
    apiHost,
    apiPort,
    apiModel,
    apiKey,
    ollamaBaseUrl,
    ollamaModel,
  ],
  () => {
    // 避免在加载时触发，设置一个小延迟
    setTimeout(() => {
      // 这里可以直接调用 saveSettings，但需要检查是否已经完成初始化
      // 暂时只使用 localStorage，用户可以手动点击保存按钮
    }, 100);
  },
  { deep: true }
);

onMounted(async () => {
  document.addEventListener("click", () => { modelDropdownOpen.value = false; });

  loadLocalSettings();

  await loadProjects();
  const appState = await callBackend("getState");
  if (appState) {
    sessionId.value = appState.sessionId || "";
    if (!activeProject.value) {
      workspacePath.value = appState.workspacePath || "";
    }
    isBusy.value = Boolean(appState.busy);
    applySettings(appState.settings || {});
  }

  detectHardware();

  if (runMode.value === "ollama") {
    loadCloudModels("ollama");
  } else if (runMode.value === "api") {
    await checkApiServiceStatus();
    await autoActivateApiService();
  }

  addMessage("assistant", "选择模型并配置参数，打开项目目录后即可下达编码任务。");

  const backend = await getBackend();
  if (backend) {
    backend.deltaReceived.connect((jsonStr: string) => {
      try {
        const payload = JSON.parse(jsonStr);
        if (!payload?.text) return;
        const target = messages.value.find((m) => m.id === currentAssistantId.value);
        if (target) {
          let newText = payload.text;
          // 过滤 "Not logged in" 提示
          const isLoginPrompt = 
            newText.trim() === "Not logged in · Please run /login" ||
            newText.trim() === "Not logged in · Run /login";
          if (isLoginPrompt) {
            return;
          }
          // 清理登录提示文本
          if (newText.includes("Not logged in")) {
            newText = newText
              .replace(/Not logged in · Please run \/login/g, "")
              .replace(/Not logged in · Run \/login/g, "")
              .trim();
            if (!newText) {
              return;
            }
          }
          if (newText.startsWith("\x00TOOL\x00")) {
            const statusLine = newText.slice(5);
            target.toolStatus = (target.toolStatus || "") + statusLine + "\n";
            const writeMatch = statusLine.match(/写入文件|Write.*?✅\s*(.+)/);
            const editMatch = statusLine.match(/编辑文件|Edit.*?✅\s*(.+)/);
            const bashMatch = statusLine.match(/执行命令|Bash.*?✅/);
            if (writeMatch) {
              trackFileChange("Write", writeMatch[1].trim(), "create");
            } else if (editMatch) {
              trackFileChange("Edit", editMatch[1].trim(), "modify");
            }
          } else {
            if (target.text === "" && newText.trim() === "") return;
            if (target.toolStatus) target.toolStatus = "";
            target.text += newText;
          }
        }
      } catch {}
    });

    backend.statusReceived.connect((jsonStr: string) => {
      try {
        const payload = JSON.parse(jsonStr);
        if (payload && typeof payload.busy === "boolean") {
          isBusy.value = payload.busy;
          if (!payload.busy) {
            if (busyTimeoutId) { clearTimeout(busyTimeoutId); busyTimeoutId = null; }
            if (currentAssistantId.value) {
              const checkId = currentAssistantId.value;
              setTimeout(() => {
                const target = messages.value.find((m) => m.id === checkId);
                if (target && !target.text.trim() && !target.toolStatus?.trim()) {
                  target.text = "[模型未返回文本]";
                }
              }, 500);
              const target = messages.value.find((m) => m.id === currentAssistantId.value);
              if (target && messageStartTime.value > 0) {
                target.completedAt = new Date().toLocaleString("zh-CN");
                target.durationMs = Date.now() - messageStartTime.value;
                const textLen = target.text.length;
                target.tokens = Math.max(1, Math.round(textLen / 2));
              }
              messageStartTime.value = 0;
            }
            saveCurrentConversation();
            processQueue();
          }
        }
      } catch {}
    });

    backend.modelsLoaded.connect((jsonStr: string) => {
      try {
        const payload = JSON.parse(jsonStr);
        loadingModels.value = false;
        if (payload?.action === "pull_complete") {
          pullingModel.value = "";
          showNotice(`${displayName(payload.model)} 下载完成`, "ok");
          detectModels();
          return;
        }
        if (payload?.action === "pull_failed") {
          pullingModel.value = "";
          showNotice(`${displayName(payload.model)} 下载失败: ${payload.error}`, "warn");
          return;
        }
        if (!payload || !payload.ok) {
          showNotice(payload?.error || "模型列表加载失败", "warn");
          return;
        }
        cloudModels.value = payload.models || [];
        const broken = cloudModels.value.filter((m: ModelInfo) => m.loadable === false);
        const source = (payload.models?.[0]?.provider) || "ollama";
        if (source === "api") {
          apiModels.value = payload.models || [];
        }
        const label = source === "openrouter" ? " OpenRouter" : source === "anthropic" ? " Anthropic" : source === "api" ? " API" : " Ollama";
        let msg = `已加载 ${cloudModels.value.length} 个${label}模型`;
        if (broken.length > 0) msg += `，${broken.length}个损坏`;
        showNotice(msg, broken.length > 0 ? "warn" : "ok");
      } catch (e) {
        console.error("[modelsLoaded] parse error:", e);
        loadingModels.value = false;
      }
    });
  }

  await nextTick();
  try {
    const b = await getBackend();
    if (b && b.frontendReady) {
      b.frontendReady();
    }
  } catch {}
});

(window as any).switchNav = (nav: string) => {
  activeNav.value = nav as any;
  if (nav === "version") loadVersionHistory();
};

async function loadVersionHistory() {
  try {
    const data = await callBackend("getVersionHistory");
    if (data) {
      versionHistory.value = typeof data === "string" ? JSON.parse(data) : data;
    }
  } catch (e) {
    console.warn("loadVersionHistory failed:", e);
  }
}
</script>

<template>
  <div class="page">
    <main v-if="activeNav === 'chat'" class="workbench" :class="{ single: !showPanel }">
      <section class="chat card">
        <div class="toolbar">
          <div class="session">会话：{{ sessionId || "未创建" }}</div>
          <div class="actions">
            <button v-if="showPreviewPanel" class="btn-blue" @click="closePreview">关闭预览</button>
            <button v-else class="btn-blue" @click="openPreview">预览</button>
            <button class="btn-blue" @click="chooseWorkspace">打开项目</button>
            <button class="btn-blue" @click="createSession" :disabled="isBusy">新会话</button>
            <button class="btn-red" @click="stopMessage" :disabled="!isBusy">停止</button>
            <button class="btn-blue" @click="showPanel = !showPanel">{{ showPanel ? "隐藏面板" : "显示面板" }}</button>
          </div>
        </div>

        <div class="messages">
          <article v-for="m in messages" :key="m.id" class="msg" :class="m.role">
            <div class="msg-header">
              <label>{{ roleLabel(m.role) }}</label>
              <span class="msg-time">{{ m.time }}</span>
              <span v-if="m.model" class="msg-model">{{ m.model }}</span>
              <div class="msg-actions">
                <button v-if="m.role === 'user'" class="btn-msg-action" @click="resendMessage(m.id)" title="重发">↻</button>
                <button v-if="m.role === 'user'" class="btn-msg-action" @click="editMessage(m.id)" title="编辑">✎</button>
                <button v-if="m.role === 'user'" class="btn-msg-action" @click="branchFromMessage(m.id)" title="从此处分支">⎇</button>
                <button class="btn-msg-del" @click="deleteMessage(m.id)" title="删除">✕</button>
              </div>
            </div>
            <pre v-if="m.text">{{ m.text }}</pre>
            <pre v-else-if="m.toolStatus?.trim()" class="tool-status">{{ m.toolStatus }}</pre>
            <pre v-else class="thinking">思考中<span class="dots">...</span></pre>
            <div v-if="m.role === 'assistant' && m.completedAt" class="msg-footer">
              <div class="msg-meta">
                <span>{{ m.completedAt }}</span>
                <span v-if="m.durationMs">· {{ formatDuration(m.durationMs) }}</span>
                <span v-if="m.tokens">· ~{{ m.tokens }} tokens</span>
              </div>
              <div class="msg-rating">
                <span
                  v-for="s in 5" :key="s"
                  :class="['star', { active: m.rating && m.rating >= s }]"
                  @click="rateMessage(m.id, s)"
                  title="评分"
                >★</span>
              </div>
            </div>
          </article>
        </div>

        <div v-if="showPreviewPanel && previewUrl" class="preview-panel">
          <div class="preview-toolbar">
            <span style="font-size: 11px; color: #888;">🖥️ 实时预览</span>
            <div style="display: flex; gap: 4px;">
              <button class="btn-icon-sm" @click="refreshPreview" style="font-size: 10px;">🔄</button>
              <button class="btn-icon-sm" @click="closePreview" style="font-size: 10px;">✕</button>
            </div>
          </div>
          <iframe :src="previewUrl" style="flex: 1; width: 100%; border: none; background: #fff;"></iframe>
        </div>

        <div class="composer">
          <div style="position: relative;">
            <textarea
              v-model="inputText"
              placeholder="输入编码任务（Enter 发送，Shift+Enter 换行，/ 快捷指令）"
              @keydown.enter.exact.prevent="sendMessage"
              @input="handleInputChange"
            />
            <div v-if="showSlashMenu" class="slash-menu">
              <div v-for="(cmd, key) in SLASH_COMMANDS" :key="key" v-show="!slashMenuFilter || key.startsWith(slashMenuFilter)" class="slash-item" @click="executeSlashCommand(key)">
                <span class="slash-name">{{ cmd.name }}</span>
                <span class="slash-desc">{{ cmd.desc }}</span>
              </div>
            </div>
          </div>
          <div class="composer-foot">
            <div style="display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0;">
              <span>{{ isBusy ? (pendingQueue.length > 0 ? `执行中... 排队${pendingQueue.length}条` : "执行中...") : "就绪" }}</span>
              <div class="context-bar-wrap" :title="`上下文: ~${totalTokens} / ${contextWindowMax} tokens (${contextPercent}%)`">
                <div class="context-bar" :style="{ width: contextPercent + '%', background: contextBarColor }"></div>
              </div>
              <span style="font-size: 10px; color: #666;">~{{ totalTokens }}tk</span>
              <button v-if="sessionFileChanges.length > 0" class="btn-icon-sm" @click="showFileChanges = !showFileChanges" :title="`${sessionFileChanges.length} 个文件变更`" style="font-size: 10px;">📁{{ sessionFileChanges.length }}</button>
            </div>
            <div>
              <button class="btn-sm" @click="exportConversation('markdown')" :disabled="messages.length === 0" title="导出 Markdown" style="background: #2a3a2a;">📤</button>
              <button class="btn-blue" @click="clearMessages" :disabled="isBusy">新会话</button>
              <button class="btn-red" @click="sendMessage">发送任务</button>
            </div>
          </div>
          <div v-if="showFileChanges && sessionFileChanges.length > 0" class="file-changes-panel">
            <div style="font-size: 11px; color: #888; margin-bottom: 4px;">本次会话文件变更</div>
            <div v-for="(fc, i) in sessionFileChanges" :key="i" class="file-change-item">
              <span :class="['fc-action', fc.action]">{{ fc.action === 'create' ? '+' : fc.action === 'delete' ? '-' : '~' }}</span>
              <span class="fc-path">{{ fc.path }}</span>
              <span class="fc-time">{{ fc.time }}</span>
            </div>
          </div>
        </div>
      </section>

      <aside v-if="showPanel" class="panel card">
        <div class="panel-header">
          <h2>模型与配置</h2>
          <div class="mode-switch">
            <button :class="['mode-btn', { active: runMode === 'cloud' }]" @click="runMode = 'cloud'">☁️ 云端</button>
            <button :class="['mode-btn', { active: runMode === 'api' }]" @click="runMode = 'api'">🔗 API</button>
            <button :class="['mode-btn', { active: runMode === 'ollama' }]" @click="runMode = 'ollama'">🦙 Ollama</button>
          </div>
        </div>

        <template v-if="runMode === 'cloud'">
          <label class="field">
            <span>API Key</span>
            <input v-model="apiKey" type="text" placeholder="输入你的 API Key" />
          </label>
          <label class="field">
            <span>云端模型（固定）</span>
            <input value="openrouter/auto" readonly />
          </label>

          <div class="cloud-model-item model-row selected">
            <div class="model-row-info">
              <span class="model-name">openrouter/auto</span>
              <span class="tool-badge ok">★ 工具</span>
            </div>
            <button class="btn-icon" :class="{ active: expandedModelId === 'openrouter/auto' }" @click="toggleModelSettings('openrouter/auto')">⚙</button>
          </div>

          <div v-if="expandedModelId === 'openrouter/auto'" class="model-settings">
            <ModelSettingsPanel :config="getModelConfig('openrouter/auto')" :hint="getAutoConfigHint('openrouter/auto')" @auto-configure="autoConfigure('openrouter/auto')" />
          </div>
          <div class="role-presets">
            <span style="font-size: 11px; color: #888; margin-right: 4px;">角色</span>
            <button v-for="(preset, key) in ROLE_PRESETS" :key="key" :class="['role-btn', { active: activeRole === key }]" @click="applyRole(key)" :title="preset.desc">
              {{ preset.icon }} {{ preset.name }}
            </button>
          </div>
        </template>

        <template v-else-if="runMode === 'api'">
          <label class="field">
            <span>API 服务地址</span>
            <div style="display: flex; align-items: center; gap: 0;">
              <span style="padding: 0 6px; font-size: 12px; color: #888; white-space: nowrap; background: #1a1a1a; border: 1px solid #333; border-right: none; border-radius: 4px 0 0 4px; height: 28px; line-height: 28px;">http://</span>
              <input v-model="apiHost" placeholder="127.0.0.1" style="flex: 1; border-radius: 0;" />
              <span style="padding: 0 6px; font-size: 14px; color: #888; background: #1a1a1a; border: 1px solid #333; border-left: none; border-right: none; height: 28px; line-height: 28px;">:</span>
              <input v-model="apiPort" type="number" min="1" max="65535" placeholder="7777" style="width: 80px; text-align: center; border-radius: 0 4px 4px 0;" />
            </div>
          </label>

          <div class="api-progress-section">
            <div class="api-progress-bar">
              <div
                v-for="(step, idx) in apiSteps"
                :key="idx"
                :class="['api-step', { done: apiStepProgress > idx, active: apiStepProgress === idx, pending: apiStepProgress < idx }]"
              >
                <div class="step-dot">
                  <span v-if="apiStepProgress > idx">✓</span>
                  <span v-else-if="apiStepProgress === idx && apiStepBusy">{{ idx + 1 }}</span>
                  <span v-else>{{ idx + 1 }}</span>
                </div>
                <div class="step-label">{{ step.label }}</div>
              </div>
            </div>
            <div class="api-progress-track">
              <div class="api-progress-fill" :style="{ width: apiProgressPercent + '%' }"></div>
            </div>
            <p class="api-progress-msg">{{ apiStepMessage }}</p>
            <div style="display: flex; gap: 6px; margin-top: 6px;">
              <button
                class="btn-blue"
                style="flex: 1;"
                @click="apiStepAutoRun"
                :disabled="apiStepBusy || apiStepProgress >= apiSteps.length"
              >
                {{ apiStepBusy ? apiStepMessage : (apiStepProgress >= apiSteps.length ? '✓ API 服务已就绪' : '▶ 一键启动') }}
              </button>
              <button
                class="btn-red"
                style="flex: 0 0 auto; min-width: 80px;"
                @click="stopApiService"
                :disabled="apiStepBusy || apiStepProgress < apiSteps.length"
                v-if="apiStepProgress >= apiSteps.length"
              >
                ■ 停止服务
              </button>
            </div>
          </div>

          <div class="field-group-title">模型选择</div>
          <div v-if="apiModels.length > 0" class="model-quick-select">
            <div class="custom-select" :class="{ open: modelDropdownOpen }" @click.stop="modelDropdownOpen = !modelDropdownOpen">
              <div class="custom-select-value">
                <span>{{ apiModel ? displayName(apiModels.find(m => m.id === apiModel)?.name || apiModel) : '选择模型...' }}</span>
                <span class="custom-select-arrow">▼</span>
              </div>
              <div v-if="modelDropdownOpen" class="custom-select-options">
                <div
                  v-for="m in apiModels" :key="m.id"
                  :class="['custom-select-option', { selected: apiModel === m.id }]"
                  @click.stop="apiModel = m.id; modelDropdownOpen = false"
                >
                  <span>{{ displayName(m.name || m.id) }}</span>
                  <span v-if="m.toolSupport === true" class="tool-badge ok">★</span>
                  <span v-else-if="m.toolSupport === false" class="tool-badge no">-</span>
                </div>
              </div>
            </div>
            <button class="btn-icon" :class="{ active: expandedModelId === apiModel }" @click="toggleModelSettings(apiModel)" title="模型参数">⚙</button>
          </div>
          <label class="field">
            <span>模型名称</span>
            <input v-model="apiModel" placeholder="qwen3.6-plus" />
          </label>
          <div class="role-presets">
            <span style="font-size: 11px; color: #888; margin-right: 4px;">角色</span>
            <button v-for="(preset, key) in ROLE_PRESETS" :key="key" :class="['role-btn', { active: activeRole === key }]" @click="applyRole(key)" :title="preset.desc">
              {{ preset.icon }} {{ preset.name }}
            </button>
          </div>
          <div v-if="expandedModelId && apiModels.find(m => m.id === expandedModelId)" class="model-settings">
            <ModelSettingsPanel :config="getModelConfig(expandedModelId)" :hint="getAutoConfigHint(expandedModelId)" @auto-configure="autoConfigure(expandedModelId)" />
          </div>

          <label class="field">
            <span>API Key</span>
            <input v-model="apiKey" type="text" placeholder="自动获取或手动输入" />
          </label>

          <div class="qwen-account-section">
            <div class="qwen-account-header">
              <span>🔑 上游账户</span>
              <span v-if="qwenAccountCount >= 0" :class="['account-badge', qwenAccountCount > 0 ? 'ok' : 'warn']">
                {{ qwenAccountCount }} 个
              </span>
              <span v-if="qwenValidCount > 0" style="color: #4CAF50; font-size: 10px; margin-left: 4px;">{{ qwenValidCount }} 可用</span>
              <button class="btn-blue btn-sm" @click="checkQwenAccounts" style="margin-left: auto;">刷新</button>
            </div>
            <p v-if="qwenAccountCount === 0" class="hint warn" style="margin: 4px 0;">
              未添加上游账户，AI 对话将返回 500 错误。请点击下方按钮自动注册。
            </p>
            <div v-if="qwenAccounts.length > 0" class="account-list">
              <div v-for="acc in qwenAccounts" :key="acc.email" :class="['account-row', { sticky: stickyEmail === acc.email }]">
                <span :class="['account-status', acc.valid ? 'valid' : 'invalid']">●</span>
                <span class="account-email">{{ acc.email }}</span>
                <span v-if="stickyEmail === acc.email" class="sticky-badge">★ 优先</span>
                <span v-if="!acc.valid" class="account-err">{{ acc.status_code || '不可用' }}</span>
                <button v-if="stickyEmail !== acc.email && acc.valid" class="btn-icon btn-sticky" @click="setStickyAccount(acc.email)" title="设为优先使用账户">★</button>
                <button class="btn-icon btn-del" @click="deleteQwenAccount(acc.email)" title="删除此账户">✕</button>
              </div>
              <button v-if="stickyEmail" class="btn-blue btn-sm" @click="clearStickyAccount" style="margin-top: 4px; width: 100%;">
                取消优先，恢复自动轮换
              </button>
            </div>
            <p v-if="qwenAccountCount > 0 && qwenValidCount === 0" class="hint warn" style="margin: 4px 0;">
              所有账户均不可用，请注册新账户或手动添加有效 Token。
            </p>
            <p v-if="qwenValidCount > 0" class="hint ok" style="margin: 4px 0;">
              {{ qwenValidCount }} 个账户可用，可进行 AI 对话。
            </p>
            <details style="margin-top: 6px;">
              <summary style="font-size: 11px; color: #888; cursor: pointer;">🔑 登录已有账户</summary>
              <div class="reg-form" style="margin-top: 4px;">
                <input v-model="loginEmail" type="text" placeholder="邮箱" />
                <input v-model="loginPassword" type="password" placeholder="密码" />
                <button class="btn-blue btn-sm" @click="loginQwenAccount" :disabled="qwenLoginBusy || !loginEmail.trim() || !loginPassword.trim()" style="width: 100%;">
                  {{ qwenLoginBusy ? '⏳ 登录中...' : '登录' }}
                </button>
              </div>
              <p v-if="qwenLoginError" class="hint warn" style="margin: 4px 0;">{{ qwenLoginError }}</p>
              <p class="hint" style="margin: 4px 0; font-size: 10px;">
                使用已注册的 chat.qwen.ai 账户邮箱和密码登录
              </p>
            </details>
            <details style="margin-top: 6px;">
              <summary style="font-size: 11px; color: #888; cursor: pointer;">自定义注册信息（可选）</summary>
              <div class="reg-form" style="margin-top: 4px;">
                <input v-model="regEmail" type="text" placeholder="邮箱（留空自动生成）" />
                <input v-model="regPassword" type="password" placeholder="密码（留空自动生成）" />
                <input v-model="regUsername" type="text" placeholder="用户名（留空自动生成）" />
              </div>
            </details>
            <button
              class="btn-blue"
              style="width: 100%; margin: 6px 0;"
              @click="autoRegisterQwenAccount"
              :disabled="qwenRegisterBusy"
            >
              {{ qwenRegisterBusy ? '⏳ 自动注册中...' : '🤖 自动注册上游账户' }}
            </button>
            <div v-if="qwenRegisterLogs.length > 0" class="register-log-box">
              <div v-for="(log, idx) in qwenRegisterLogs" :key="idx" class="register-log-line">
                {{ log }}
              </div>
            </div>
            <details style="margin-top: 6px;">
              <summary style="font-size: 11px; color: #888; cursor: pointer;">手动添加 Token</summary>
              <div class="qwen-token-input" style="margin-top: 4px;">
                <input v-model="qwenToken" type="text" placeholder="粘贴 chat.qwen.ai 的 Token" />
                <button class="btn-blue btn-sm" @click="addQwenAccount" :disabled="!qwenToken.trim()">添加</button>
              </div>
              <p class="hint" style="margin: 4px 0; font-size: 10px;">
                获取方式：登录 chat.qwen.ai → F12 开发者工具 → Application → Local Storage → 复制 token 值
              </p>
            </details>
          </div>
        </template>

        <template v-else>
          <label class="field">
            <span>Ollama 地址</span>
            <div class="model-loader">
              <input v-model="ollamaBaseUrl" placeholder="http://127.0.0.1:11434" />
              <button class="btn-blue btn-sm" @click="detectModels" :disabled="loadingModels">
                {{ loadingModels ? "检测中..." : "🔍 检测" }}
              </button>
            </div>
          </label>

          <div class="model-manager-toggle" @click="showModelManager = !showModelManager">
            <span style="font-weight: bold; font-size: 12px;">🧠 模型管理</span>
            <span style="font-size: 11px; color: #888;">{{ showModelManager ? '收起' : '展开' }}</span>
          </div>

          <div v-if="showModelManager" class="model-manager-panel">
            <div class="model-search-bar">
              <input v-model="librarySearchQuery" placeholder="搜索 Ollama 模型库（如 qwen、llama）" @keydown.enter="searchOllamaLibrary" style="flex: 1;" />
              <button class="btn-blue btn-sm" @click="searchOllamaLibrary" :disabled="librarySearching">
                {{ librarySearching ? "搜索中..." : "🔍 搜索" }}
              </button>
              <button class="btn-auto btn-sm" @click="loadRecommendations" :disabled="loadingModels">
                💡 推荐
              </button>
            </div>

            <div v-if="showRecommendations && recommendedModels.length > 0" class="recommend-section">
              <div class="recommend-header">
                <span>根据硬件推荐</span>
                <button class="btn-icon-sm" @click="showRecommendations = false">✕</button>
              </div>
              <div v-for="r in recommendedModels" :key="r.name" class="recommend-item">
                <div class="recommend-info">
                  <span class="recommend-name">{{ displayName(r.name) }}</span>
                  <span v-if="r.toolSupport" class="tool-badge ok">★ 工具</span>
                  <span v-else class="tool-badge no">无工具</span>
                  <span class="model-size">{{ r.size }}</span>
                </div>
                <div class="recommend-reason">{{ r.reason }}</div>
                <button class="btn-blue btn-sm" style="margin-top: 4px;" @click="pullModel(r.name)" :disabled="pullingModel === r.name">
                  {{ pullingModel === r.name ? '⏳ 下载中...' : '⬇ 下载安装' }}
                </button>
              </div>
            </div>

            <div v-if="libraryResults.length > 0" class="recommend-section">
              <div class="recommend-header">
                <span>搜索结果</span>
                <button class="btn-icon-sm" @click="libraryResults = []">✕</button>
              </div>
              <div v-for="r in libraryResults" :key="r.name" class="recommend-item">
                <div class="recommend-info">
                  <span class="recommend-name">{{ r.name }}</span>
                  <span v-if="r.toolSupport" class="tool-badge ok">★ 工具</span>
                  <span v-if="r.sizeStr" class="model-size">{{ r.sizeStr }}</span>
                </div>
                <div v-if="r.description" class="recommend-reason">{{ r.description }}</div>
                <button class="btn-blue btn-sm" style="margin-top: 4px;" @click="pullModel(r.name)" :disabled="pullingModel === r.name">
                  {{ pullingModel === r.name ? '⏳ 下载中...' : '⬇ 下载安装' }}
                </button>
              </div>
            </div>

            <div v-if="pullingModel" class="pulling-indicator">
              ⏳ 正在下载: {{ displayName(pullingModel) }}...（这可能需要几分钟）
            </div>
          </div>

          <div v-if="categorizedModels.local.length > 0" class="model-list">
            <div class="model-category-title">本地模型 (Ollama)</div>
            <div v-for="m in categorizedModels.local" :key="m.id" class="model-row-wrapper">
              <div
                :class="['model-row', { selected: ollamaModel === m.id, 'no-tool': m.toolSupport === false, 'broken': m.loadable === false }]"
                @click="m.loadable !== false && selectModel(m.id)"
              >
                <div class="model-row-info">
                  <span class="model-name">{{ displayName(m.name) }}</span>
                  <span v-if="m.loadable === false" class="tool-badge broken">⚠ 损坏</span>
                  <span v-else-if="m.toolSupport === true" class="tool-badge ok">★ 工具</span>
                  <span v-else-if="m.toolSupport === false" class="tool-badge no">无工具</span>
                  <span v-if="m.size" class="model-size">{{ m.size }}</span>
                </div>
                <div class="model-row-actions">
                  <button class="btn-icon" :class="{ active: expandedModelId === m.id }" @click.stop="toggleModelSettings(m.id)">⚙</button>
                  <button class="btn-icon btn-delete" @click.stop="deleteModel(m.id)" title="卸载模型">🗑</button>
                </div>
              </div>
              <div v-if="m.loadable === false && m.healthError" class="model-error-hint">
                ⚠ {{ m.healthError }}
              </div>
              <div v-if="expandedModelId === m.id" class="model-settings">
                <ModelSettingsPanel :config="getModelConfig(m.id)" :hint="getAutoConfigHint(m.id)" @auto-configure="autoConfigure(m.id)" />
              </div>
            </div>
          </div>

          <div v-if="categorizedModels.cloud.length > 0" class="model-list" style="margin-top: 8px;">
            <div class="model-category-title cloud">云端模型</div>
            <div v-for="m in categorizedModels.cloud" :key="m.id" class="model-row-wrapper">
              <div
                :class="['model-row', { selected: ollamaModel === m.id, 'no-tool': m.toolSupport === false }]"
                @click="selectModel(m.id)"
              >
                <div class="model-row-info">
                  <span class="model-name">{{ displayName(m.name) }}</span>
                  <span v-if="m.toolSupport === true" class="tool-badge ok">★ 工具</span>
                  <span v-else-if="m.toolSupport === false" class="tool-badge no">无工具</span>
                  <span v-if="m.size" class="model-size">{{ m.size }}</span>
                </div>
                <button class="btn-icon" :class="{ active: expandedModelId === m.id }" @click.stop="toggleModelSettings(m.id)">⚙</button>
              </div>
              <div v-if="expandedModelId === m.id" class="model-settings">
                <ModelSettingsPanel :config="getModelConfig(m.id)" :hint="getAutoConfigHint(m.id)" @auto-configure="autoConfigure(m.id)" />
              </div>
            </div>
          </div>

          <div v-if="cloudModels.length === 0 && !loadingModels" class="empty-hint">
            <p>点击"检测"加载并检测可用模型</p>
          </div>

          <p v-if="ollamaModel && selectedOllamaModelToolSupport === false" class="hint warn">
            ⚠ 该模型不支持工具调用，编程功能将受限。建议选择带 ★ 标记的模型。
          </p>
          <p v-else-if="ollamaModel && selectedOllamaModelToolSupport === true" class="hint ok">
            ★ 该模型支持工具调用，可使用全功能编程。
          </p>
          <div class="role-presets">
            <span style="font-size: 11px; color: #888; margin-right: 4px;">角色</span>
            <button v-for="(preset, key) in ROLE_PRESETS" :key="key" :class="['role-btn', { active: activeRole === key }]" @click="applyRole(key)" :title="preset.desc">
              {{ preset.icon }} {{ preset.name }}
            </button>
          </div>
        </template>

        <div class="field-group-title">对话设置</div>
        <label class="field">
          <span>工具调用审批</span>
          <select v-model="toolApprovalMode" class="select-input" style="width: 100%;">
            <option value="auto">🟢 全部自动 — AI 直接执行所有操作</option>
            <option value="smart">🟡 智能审批 — 安全操作自动，风险操作需确认</option>
            <option value="manual">🔴 全部手动 — 每次操作都需确认</option>
          </select>
        </label>
        <p v-if="toolApprovalMode === 'auto'" style="font-size: 11px; color: #FF9800; margin: -4px 0 4px 0;">⚠ AI 可直接读写文件和执行命令，无需审批</p>
        <p v-else-if="toolApprovalMode === 'smart'" style="font-size: 11px; color: #4af; margin: -4px 0 4px 0;">读取/搜索自动通过，写入/编辑/命令需确认</p>
        <p v-else style="font-size: 11px; color: #888; margin: -4px 0 4px 0;">所有工具调用都需要手动确认</p>

        <div class="field" style="border: 1px solid #2a2a2a; border-radius: 6px; padding: 8px;">
          <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px;">
            <span style="font-weight: bold; font-size: 12px;">📁 项目管理</span>
            <button class="btn-sm" @click="showProjectPanel = !showProjectPanel">{{ showProjectPanel ? '收起' : '展开' }}</button>
          </div>
          <div v-if="activeProject" style="font-size: 11px; color: #42A5F5; margin-bottom: 4px;">
            当前项目：{{ activeProject.name }} <span style="color: #666;">{{ activeProject.path }}</span>
          </div>
          <div v-if="showProjectPanel">
            <div class="proj-section-title">新建项目</div>
            <div style="margin-bottom: 4px;">
              <input v-model="newProjectName" placeholder="项目名称" style="width: 100%; font-size: 11px;" @input="updateDefaultPath" />
            </div>
            <div class="proj-path-row">
              <div class="proj-default-path" style="flex: 1; min-width: 0;">
                {{ newProjectDefaultPath || '（输入项目名称后显示默认目录）' }}
              </div>
              <button v-if="newProjectDefaultPath" class="btn-icon-sm" @click="openInExplorer(newProjectDefaultPath)" title="打开目录">📁</button>
              <label class="proj-checkbox" @click="newProjectCustomPath = !newProjectCustomPath">
                <span :class="['proj-check-box', { checked: newProjectCustomPath }]">
                  <span v-if="newProjectCustomPath" style="font-size: 10px;">✓</span>
                </span>
                <span style="font-size: 11px;">自定义</span>
              </label>
            </div>
            <div v-if="newProjectCustomPath" style="display: flex; gap: 4px; margin: 4px 0;">
              <input v-model="newProjectPath" placeholder="选择或输入自定义路径" style="flex: 1; font-size: 11px;" />
              <button class="btn-icon" @click="selectWorkspaceDir" title="选择目录" style="font-size: 11px;">📂</button>
            </div>
            <button class="btn-sm" style="width: 100%; margin-top: 4px;" @click="createProject" :disabled="!newProjectName.trim()">创建项目</button>

            <div v-if="projects.length === 0" style="font-size: 11px; color: #666; padding: 8px 0;">暂无项目，请创建一个</div>

            <div v-if="projects.length > 0" class="proj-section-title" style="margin-top: 8px;">项目列表</div>
            <div v-for="p in projects" :key="p.id" :class="['project-item', { active: p.id === activeProject?.id }]">
              <template v-if="editingProjectId === p.id">
                <div style="margin-bottom: 4px;">
                  <input v-model="editProjectName" placeholder="项目名称" style="width: 100%; font-size: 11px;" />
                </div>
                <div class="proj-path-row">
                  <div class="proj-default-path" style="flex: 1; min-width: 0;">
                    {{ p.path }}
                  </div>
                  <button class="btn-icon-sm" @click="openInExplorer(p.path)" title="打开目录">📁</button>
                  <label class="proj-checkbox" @click="editProjectCustomPath = !editProjectCustomPath">
                    <span :class="['proj-check-box', { checked: editProjectCustomPath }]">
                      <span v-if="editProjectCustomPath" style="font-size: 10px;">✓</span>
                    </span>
                    <span style="font-size: 11px;">修改</span>
                  </label>
                </div>
                <div v-if="editProjectCustomPath" style="display: flex; gap: 4px; margin: 4px 0;">
                  <input v-model="editProjectPath" placeholder="新的项目目录" style="flex: 1; font-size: 11px;" />
                  <button class="btn-icon" @click="selectWorkspaceDir" title="选择目录" style="font-size: 11px;">📂</button>
                </div>
                <div style="display: flex; gap: 4px; margin-top: 4px;">
                  <button class="btn-sm" style="flex: 1;" @click="saveEditProject">保存</button>
                  <button class="btn-sm" style="flex: 1; background: #333;" @click="cancelEditProject">取消</button>
                </div>
              </template>
              <template v-else>
                <div style="display: flex; align-items: center; justify-content: space-between;" @click="switchProject(p.id)">
                  <span style="font-size: 12px; font-weight: 500;">{{ p.name }}</span>
                  <div style="display: flex; gap: 2px;">
                    <button class="btn-icon-sm" @click.stop="openInExplorer(p.path)" title="打开目录">📁</button>
                    <button class="btn-icon-sm" @click.stop="startEditProject(p.id)" title="编辑">✏️</button>
                    <button class="btn-icon-sm" @click.stop="deleteProject(p.id)" title="删除">🗑️</button>
                  </div>
                </div>
                <div style="font-size: 11px; color: #888; margin-top: 2px;">{{ p.path }}</div>
              </template>
            </div>

            <div v-if="projectConversations.length > 0" style="margin-top: 8px; border-top: 1px solid #2a2a2a; padding-top: 6px;">
              <div style="display: flex; align-items: center; gap: 4px; margin-bottom: 4px;">
                <span style="font-size: 11px; color: #888;">对话历史</span>
                <div style="flex: 1; display: flex; gap: 2px;">
                  <input v-model="convSearchQuery" placeholder="搜索对话..." style="flex: 1; font-size: 10px; padding: 2px 6px; border-radius: 3px; border: 1px solid #333; background: #1a1a1a; color: #ddd;" @keydown.enter="searchConversations" />
                  <button v-if="convSearchQuery" class="btn-icon-sm" @click="clearConvSearch" title="清除搜索" style="font-size: 10px;">✕</button>
                  <button class="btn-icon-sm" @click="searchConversations" title="搜索" style="font-size: 10px;">🔍</button>
                </div>
              </div>
              <div v-for="c in projectConversations" :key="c.session_id" class="conv-item">
                <div style="flex: 1; min-width: 0;" @click="loadConversationHistory(c.session_id)">
                  <template v-if="editingConvId === c.session_id">
                    <div style="display: flex; gap: 2px;">
                      <input v-model="editingConvTitle" style="flex: 1; font-size: 10px; padding: 1px 4px; border-radius: 2px; border: 1px solid #555; background: #1a1a1a; color: #ddd;" @keydown.enter="saveRenameConversation(c.session_id)" @keydown.escape="cancelRenameConversation" />
                      <button class="btn-icon-sm" @click="saveRenameConversation(c.session_id)" style="font-size: 9px;">✓</button>
                      <button class="btn-icon-sm" @click="cancelRenameConversation" style="font-size: 9px;">✕</button>
                    </div>
                  </template>
                  <template v-else>
                    <div style="font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" :title="c.title || c.session_id">{{ c.title || c.session_id.slice(0, 8) + '...' }}</div>
                    <div style="font-size: 10px; color: #666;">{{ c.message_count }}条 · {{ c.updated_at ? c.updated_at.slice(0, 10) : '' }}</div>
                    <div v-if="c.snippet" style="font-size: 10px; color: #888; margin-top: 1px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" v-html="c.snippet"></div>
                  </template>
                </div>
                <div style="display: flex; gap: 1px; flex-shrink: 0;">
                  <button class="btn-icon-sm" @click.stop="startRenameConversation(c.session_id, c.title || '')" title="重命名" style="font-size: 9px;">✏️</button>
                  <button class="btn-icon-sm" @click.stop="deleteConversation(c.session_id)" title="删除" style="font-size: 9px;">🗑️</button>
                  <select v-if="projects.length > 1" class="copy-select" style="font-size: 9px;" @change="(e: any) => { copyConversationToProject(c.session_id, e.target.value); e.target.value = ''; }">
                    <option value="">复制</option>
                    <option v-for="tp in projects.filter((x: any) => x.id !== activeProject?.id)" :key="tp.id" :value="tp.id">{{ tp.name }}</option>
                  </select>
                </div>
              </div>
            </div>
          </div>

          <div v-if="activeProject" style="margin-top: 8px; border-top: 1px solid #2a2a2a; padding-top: 6px;">
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
              <span style="font-size: 11px; color: #888;">开发工作流</span>
            </div>
            <div style="display: flex; flex-wrap: wrap; gap: 3px;">
              <button class="workflow-btn" @click="applyWorkflow('new-project')">🏗️ 新项目</button>
              <button class="workflow-btn" @click="applyWorkflow('bugfix')">🐛 Bug修复</button>
              <button class="workflow-btn" @click="applyWorkflow('refactor')">🔧 重构</button>
              <button class="workflow-btn" @click="applyWorkflow('docs')">📚 文档化</button>
              <button class="workflow-btn" @click="applyWorkflow('deploy')">🚀 部署</button>
            </div>
          </div>

          <div v-if="activeProject" style="margin-top: 8px; border-top: 1px solid #2a2a2a; padding-top: 6px;">
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
              <span style="font-size: 11px; color: #888;">🧠 项目记忆</span>
              <button class="btn-icon-sm" @click="showMemoryPanel = !showMemoryPanel; if (showMemoryPanel) { loadMemoryContent(); loadSmartMemories(); }" :title="showMemoryPanel ? '收起' : '展开'" style="font-size: 10px;">{{ showMemoryPanel ? '▼' : '▶' }}</button>
            </div>
            <div v-if="showMemoryPanel" style="margin-top: 4px;">
              <div style="display: flex; gap: 2px; margin-bottom: 6px;">
                <button :class="['tab-btn', { active: memoryTab === 'claude-md' }]" @click="memoryTab = 'claude-md'">CLAUDE.md</button>
                <button :class="['tab-btn', { active: memoryTab === 'smart' }]" @click="memoryTab = 'smart'; loadSmartMemories()">智能记忆</button>
              </div>
              <template v-if="memoryTab === 'claude-md'">
                <div style="font-size: 10px; color: #666; margin-bottom: 4px;">项目级记忆（当前项目目录下的 CLAUDE.md）</div>
                <textarea v-model="memoryContent" rows="5" placeholder="在此编辑项目记忆，AI 每次对话都会读取此内容..." style="width: 100%; font-size: 11px; padding: 6px; border-radius: 4px; border: 1px solid #333; background: #1a1a1a; color: #ddd; resize: vertical; font-family: monospace;"></textarea>
                <div style="display: flex; gap: 4px; margin-top: 4px;">
                  <button class="btn-sm" @click="saveMemoryContent" style="flex: 1;">保存项目记忆</button>
                  <button class="btn-sm" @click="appendMemory('技术栈: ')" style="background: #2a3a2a;">+ 技术栈</button>
                  <button class="btn-sm" @click="appendMemory('编码规范: ')" style="background: #2a3a2a;">+ 规范</button>
                </div>
                <div style="font-size: 10px; color: #666; margin-top: 6px; margin-bottom: 4px;">全局记忆（~/.claude/CLAUDE.md）</div>
                <textarea v-model="globalMemoryContent" rows="2" placeholder="全局记忆，适用于所有项目..." style="width: 100%; font-size: 11px; padding: 6px; border-radius: 4px; border: 1px solid #333; background: #1a1a1a; color: #ddd; resize: vertical; font-family: monospace;"></textarea>
                <button class="btn-sm" @click="saveGlobalMemory" style="margin-top: 4px;">保存全局记忆</button>
              </template>
              <template v-if="memoryTab === 'smart'">
                <div style="display: flex; gap: 4px; margin-bottom: 6px; align-items: center;">
                  <input v-model="memorySearchQuery" placeholder="搜索记忆..." style="flex: 1; font-size: 10px; padding: 2px 6px; border-radius: 3px; border: 1px solid #333; background: #1a1a1a; color: #ddd;" @keydown.enter="searchSmartMemories" />
                  <button class="btn-sm" @click="searchSmartMemories" style="background: #2a3a2a; font-size: 10px;">🔍</button>
                  <button class="btn-sm" @click="extractMemoriesFromConversation" style="background: #2a3a2a; font-size: 10px;">📥 提取</button>
                </div>
                <div style="font-size: 10px; color: #666; margin-bottom: 4px;">四类记忆：👤 用户偏好 | 🔄 反馈纠正 | 📋 项目上下文 | 🔗 外部引用</div>
                <div v-if="smartMemories.length === 0" style="font-size: 11px; color: #555; text-align: center; padding: 8px;">暂无智能记忆<br>点击"提取"从对话中自动提取</div>
                <div v-for="mem in smartMemories" :key="mem.filename" class="memory-item">
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <span :class="['mem-type-badge', mem.type]">{{ mem.type === 'user' ? '👤' : mem.type === 'feedback' ? '🔄' : mem.type === 'reference' ? '🔗' : '📋' }} {{ mem.title }}</span>
                    <div style="display: flex; gap: 2px;">
                      <button v-if="mem.relevance" style="font-size: 9px; color: #4af; background: none; border: none; cursor: default;">相关度: {{ mem.relevance }}</button>
                      <button class="btn-icon-sm" @click="deleteSmartMemory(mem.filename)" style="font-size: 9px;">🗑️</button>
                    </div>
                  </div>
                  <div v-if="mem.match_snippet" style="font-size: 10px; color: #4af; margin-top: 2px; padding: 2px 4px; background: #1a2a3a; border-radius: 2px;">匹配: {{ mem.match_snippet }}</div>
                  <div style="font-size: 10px; color: #888; margin-top: 2px; white-space: pre-wrap; max-height: 60px; overflow-y: auto;">{{ mem.content.replace(/^---[\s\S]*?---\n*/, '').slice(0, 200) }}</div>
                </div>
                <div style="margin-top: 6px; border-top: 1px solid #222; padding-top: 6px;">
                  <div style="font-size: 10px; color: #666; margin-bottom: 4px;">添加新记忆</div>
                  <div style="display: flex; gap: 4px; margin-bottom: 4px;">
                    <input v-model="newMemoryTitle" placeholder="标题" style="flex: 1; font-size: 10px; padding: 2px 6px; border-radius: 3px; border: 1px solid #333; background: #1a1a1a; color: #ddd;" />
                    <select v-model="newMemoryType" style="font-size: 10px; padding: 2px; border-radius: 3px; border: 1px solid #333; background: #1a1a1a; color: #ddd;">
                      <option value="user">👤 用户</option>
                      <option value="feedback">🔄 反馈</option>
                      <option value="project">📋 项目</option>
                      <option value="reference">🔗 引用</option>
                    </select>
                  </div>
                  <textarea v-model="newMemoryContent" rows="2" placeholder="记忆内容..." style="width: 100%; font-size: 10px; padding: 4px; border-radius: 3px; border: 1px solid #333; background: #1a1a1a; color: #ddd; resize: vertical; font-family: monospace;"></textarea>
                  <button class="btn-sm" @click="addSmartMemory" style="margin-top: 3px; width: 100%;">添加记忆</button>
                </div>
              </template>
            </div>
          </div>

          <div v-if="activeProject" style="margin-top: 8px; border-top: 1px solid #2a2a2a; padding-top: 6px;">
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
              <span style="font-size: 11px; color: #888;">📦 项目模板</span>
              <div style="display: flex; gap: 2px;">
                <button class="btn-icon-sm" @click="showAddTemplate = !showAddTemplate" style="font-size: 10px;" title="添加自定义模板">➕</button>
                <button class="btn-icon-sm" @click="showTemplatePanel = !showTemplatePanel; if (showTemplatePanel) loadProjectTemplates()" style="font-size: 10px;">{{ showTemplatePanel ? '▼' : '▶' }}</button>
              </div>
            </div>
            <div v-if="showAddTemplate" style="margin-bottom: 6px; padding: 6px; border: 1px solid #333; border-radius: 4px; background: #1a1a1a;">
              <div style="font-size: 10px; color: #888; margin-bottom: 4px;">添加自定义模板</div>
              <input v-model="newTplName" placeholder="模板名称" style="width: 100%; font-size: 10px; padding: 2px 6px; border-radius: 3px; border: 1px solid #333; background: #111; color: #ddd; margin-bottom: 3px;" />
              <div style="display: flex; gap: 4px; margin-bottom: 3px;">
                <select v-model="newTplCategory" style="flex: 1; font-size: 10px; padding: 2px; border-radius: 3px; border: 1px solid #333; background: #111; color: #ddd;">
                  <option value="frontend">前端</option>
                  <option value="backend">后端</option>
                  <option value="fullstack">全栈</option>
                  <option value="desktop">桌面</option>
                  <option value="datascience">数据科学</option>
                </select>
                <input v-model="newTplDesc" placeholder="描述" style="flex: 2; font-size: 10px; padding: 2px 6px; border-radius: 3px; border: 1px solid #333; background: #111; color: #ddd;" />
              </div>
              <textarea v-model="newTplPrompt" rows="2" placeholder="模板提示词（发送给AI的指令）" style="width: 100%; font-size: 10px; padding: 4px; border-radius: 3px; border: 1px solid #333; background: #111; color: #ddd; resize: vertical; font-family: monospace;"></textarea>
              <div style="display: flex; gap: 4px; margin-top: 3px;">
                <button class="btn-sm" @click="addCustomTemplate" style="flex: 1;">保存模板</button>
                <button class="btn-sm" @click="showAddTemplate = false" style="flex: 1; background: #333;">取消</button>
              </div>
            </div>
            <div v-if="showTemplatePanel" style="margin-top: 4px;">
              <div style="display: flex; gap: 3px; margin-bottom: 6px; flex-wrap: wrap;">
                <button v-for="cat in ['all', 'frontend', 'backend', 'fullstack', 'desktop', 'datascience']" :key="cat" :class="['tab-btn', { active: templateCategory === cat }]" @click="templateCategory = cat; loadProjectTemplates()" style="font-size: 10px;">{{ cat === 'all' ? '全部' : cat === 'frontend' ? '前端' : cat === 'backend' ? '后端' : cat === 'fullstack' ? '全栈' : cat === 'desktop' ? '桌面' : '数据' }}</button>
              </div>
              <div v-for="t in projectTemplates" :key="t.id" class="template-item" style="cursor: pointer;">
                <div style="display: flex; justify-content: space-between; align-items: center;">
                  <div @click="useTemplate(t)" style="flex: 1; min-width: 0;">
                    <div style="font-size: 11px; color: #ddd;">{{ t.custom ? '⭐ ' : '' }}{{ t.name }}</div>
                    <div style="font-size: 10px; color: #888;">{{ t.desc }}</div>
                  </div>
                  <div style="display: flex; gap: 2px; flex-shrink: 0;">
                    <button v-if="t.scaffold" class="btn-icon-sm" @click.stop="scaffoldFromTemplate(t)" title="创建脚手架" style="font-size: 9px;">🏗️</button>
                    <button v-if="t.custom" class="btn-icon-sm" @click.stop="deleteCustomTemplate(t.id)" title="删除" style="font-size: 9px;">🗑️</button>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div v-if="activeProject" style="margin-top: 8px; border-top: 1px solid #2a2a2a; padding-top: 6px;">
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
              <span style="font-size: 11px; color: #888;">🖥️ 实时预览</span>
              <button class="btn-icon-sm" @click="showPreviewPanel = !showPreviewPanel" style="font-size: 10px;">{{ showPreviewPanel ? '▼' : '▶' }}</button>
            </div>
            <div v-if="showPreviewPanel" style="margin-top: 4px;">
              <div style="display: flex; gap: 4px; margin-bottom: 4px;">
                <button class="btn-sm" @click="openPreview" style="flex: 1; font-size: 10px;">📂 打开预览</button>
                <button class="btn-sm" @click="refreshPreview" style="font-size: 10px;">🔄 刷新</button>
                <button class="btn-sm" @click="closePreview" style="font-size: 10px; background: #333;">✕</button>
              </div>
              <div v-if="previewUrl" style="border: 1px solid #333; border-radius: 4px; overflow: hidden; background: #fff; height: 200px;">
                <iframe :src="previewUrl" style="width: 100%; height: 100%; border: none;"></iframe>
              </div>
              <div v-else style="font-size: 10px; color: #555; text-align: center; padding: 12px;">点击"打开预览"查看项目网页</div>
            </div>
          </div>

          <div v-if="activeProject" style="margin-top: 8px; border-top: 1px solid #2a2a2a; padding-top: 6px;">
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
              <span style="font-size: 11px; color: #888;">📋 任务编排</span>
              <button class="btn-icon-sm" @click="showTaskPanel = !showTaskPanel" style="font-size: 10px;">{{ showTaskPanel ? '▼' : '▶' }}</button>
            </div>
            <div v-if="showTaskPanel" style="margin-top: 4px;">
              <div style="display: flex; gap: 4px; margin-bottom: 4px;">
                <input v-model="newTaskName" placeholder="任务名称" style="flex: 1; font-size: 10px; padding: 2px 6px; border-radius: 3px; border: 1px solid #333; background: #1a1a1a; color: #ddd;" @keydown.enter="addTask" />
                <button class="btn-sm" @click="addTask" style="font-size: 10px;">添加</button>
              </div>
              <textarea v-model="newTaskDesc" rows="1" placeholder="任务描述（可选）" style="width: 100%; font-size: 10px; padding: 2px 4px; border-radius: 3px; border: 1px solid #333; background: #1a1a1a; color: #ddd; resize: vertical; margin-bottom: 4px;"></textarea>
              <div v-if="taskList.length === 0" style="font-size: 10px; color: #555; text-align: center; padding: 6px;">暂无任务，添加任务来编排开发流程</div>
              <div v-for="task in taskList" :key="task.id" style="display: flex; align-items: center; gap: 4px; padding: 3px 0; border-bottom: 1px solid #222;">
                <span :style="{ color: task.status === 'done' ? '#4CAF50' : task.status === 'in_progress' ? '#FF9800' : '#888', fontSize: '10px', cursor: 'pointer' }" @click="updateTaskStatus(task.id, task.status === 'pending' ? 'in_progress' : task.status === 'in_progress' ? 'done' : 'pending')">
                  {{ task.status === 'done' ? '✅' : task.status === 'in_progress' ? '🔄' : '⬜' }}
                </span>
                <span style="flex: 1; font-size: 10px; color: #ddd; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;" :style="{ textDecoration: task.status === 'done' ? 'line-through' : 'none' }">{{ task.name }}</span>
                <button class="btn-icon-sm" @click="executeTaskAsPrompt(task)" title="执行此任务" style="font-size: 9px;">▶</button>
                <button class="btn-icon-sm" @click="removeTask(task.id)" title="删除" style="font-size: 9px;">✕</button>
              </div>
            </div>
          </div>

          <div v-if="activeProject" style="margin-top: 8px; border-top: 1px solid #2a2a2a; padding-top: 6px;">
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
              <span style="font-size: 11px; color: #888;">🤝 AI 协作</span>
            </div>
            <div style="display: flex; flex-wrap: wrap; gap: 3px;">
              <button :class="['workflow-btn', { active: collaborationMode === 'plan-code-review' }]" @click="setCollaborationMode('plan-code-review')">📋 规划→编码→审查</button>
              <button :class="['workflow-btn', { active: collaborationMode === 'pair' }]" @click="setCollaborationMode('pair')">👥 结对编程</button>
              <button :class="['workflow-btn', { active: collaborationMode === 'review-only' }]" @click="setCollaborationMode('review-only')">🔍 纯审查</button>
              <button v-if="collaborationMode !== 'none'" class="workflow-btn" @click="setCollaborationMode('none')" style="background: #3a2222;">✕ 退出协作</button>
            </div>
            <div v-if="collaborationMode !== 'none'" style="margin-top: 4px; padding: 4px; border: 1px solid #333; border-radius: 4px; background: #1a1a1a;">
              <div style="font-size: 10px; color: #888;">
                模式：{{ collaborationMode === 'plan-code-review' ? '规划→编码→审查' : collaborationMode === 'pair' ? '结对编程' : '纯审查' }}
                <span v-if="collaborationMode === 'plan-code-review'" style="color: #4af;"> | 阶段：{{ collabPhase === 'planning' ? '📋 规划中' : collabPhase === 'coding' ? '💻 编码中' : '🔍 审查中' }}</span>
              </div>
              <div v-if="collaborationMode === 'plan-code-review'" style="display: flex; gap: 4px; margin-top: 4px;">
                <button class="btn-sm" @click="advanceCollabPhase" style="flex: 1; font-size: 10px;">⏭ 进入下一阶段</button>
              </div>
            </div>
          </div>
        </div>
        <label class="field">
          <span>你的称谓</span>
          <input v-model="userName" placeholder="你" class="short-input" />
        </label>
        <label class="field">
          <span>AI 称谓</span>
          <input v-model="assistantName" placeholder="助手" class="short-input" />
        </label>

        <div class="setting-actions">
          <button class="btn-blue" @click="clearModelFields" :disabled="isBusy">清空模型</button>
          <button class="btn-red" @click="saveSettings" :disabled="isBusy">保存并启用</button>
        </div>

        <p class="notice" :class="noticeType">{{ noticeText }}</p>
      </aside>
    </main>

    <main v-if="activeNav === 'project'" class="page-view">
      <div class="page-view-inner">
        <div class="page-view-header">
          <h2>📁 项目管理</h2>
          <div style="display: flex; gap: 8px; align-items: center;">
            <input v-model="newProjectName" placeholder="新项目名称" class="setting-input" style="width: 200px;" @input="updateDefaultPath" />
            <button class="btn-blue" @click="createProject" :disabled="!newProjectName.trim()">创建项目</button>
            <button class="btn-blue" @click="chooseWorkspace">打开目录</button>
          </div>
        </div>
        <div v-if="newProjectDefaultPath" style="font-size: 11px; color: #666; padding: 0 24px 8px;">{{ newProjectDefaultPath }}</div>
        <div class="project-grid">
          <div v-for="p in projects" :key="p.id" :class="['project-card', { active: p.id === activeProject?.id }]" @click="switchProject(p.id)">
            <div class="project-card-header">
              <span class="project-card-name">{{ p.name }}</span>
              <span v-if="p.id === activeProject?.id" class="project-active-badge">当前</span>
            </div>
            <div class="project-card-path">{{ p.path }}</div>
            <div class="project-card-actions">
              <button class="btn-sm" @click.stop="openInExplorer(p.path)" style="font-size: 10px;">📁 打开目录</button>
              <button class="btn-icon-sm" @click.stop="deleteProject(p.id)" style="font-size: 10px; color: #f44;">🗑️</button>
            </div>
          </div>
          <div v-if="projects.length === 0" class="empty-hint">暂无项目，创建或打开一个项目开始开发</div>
        </div>
        <div style="padding: 16px 24px;">
          <h3 style="font-size: 14px; color: #ccc; margin-bottom: 8px;">📦 项目模板</h3>
          <div style="display: flex; gap: 3px; margin-bottom: 8px; flex-wrap: wrap;">
            <button v-for="cat in ['all', 'frontend', 'backend', 'fullstack', 'desktop', 'datascience']" :key="cat" :class="['tab-btn', { active: templateCategory === cat }]" @click="templateCategory = cat; loadProjectTemplates()" style="font-size: 10px;">{{ cat === 'all' ? '全部' : cat === 'frontend' ? '前端' : cat === 'backend' ? '后端' : cat === 'fullstack' ? '全栈' : cat === 'desktop' ? '桌面' : '数据' }}</button>
          </div>
          <div class="template-grid">
            <div v-for="t in projectTemplates" :key="t.id" class="template-card" @click="useTemplate(t)">
              <div style="font-size: 12px; font-weight: 500; color: #ddd;">{{ t.custom ? '⭐ ' : '' }}{{ t.name }}</div>
              <div style="font-size: 10px; color: #888; margin-top: 2px;">{{ t.desc }}</div>
              <div style="display: flex; gap: 4px; margin-top: 4px;">
                <button v-if="t.scaffold" class="btn-sm" @click.stop="scaffoldFromTemplate(t)" style="font-size: 9px; background: #2a3a2a;">🏗️ 脚手架</button>
                <button v-if="t.custom" class="btn-icon-sm" @click.stop="deleteCustomTemplate(t.id)" style="font-size: 9px;">🗑️</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>

    <main v-if="activeNav === 'version'" class="page-view">
      <div class="page-view-inner">
        <div class="page-view-header">
          <h2>📦 版本管理</h2>
        </div>
        <div class="version-content">
          <div class="settings-card">
            <div class="card-title">当前版本</div>
            <div style="font-size: 24px; font-weight: 700; color: #4af; margin-bottom: 4px;">v{{ APP_VERSION }}</div>
            <div style="font-size: 11px; color: #888;">云集智能编程工作站</div>
          </div>
          <div class="settings-card">
            <div class="card-title">版本历史</div>
            <div v-if="versionHistory.length === 0" style="font-size: 11px; color: #555; padding: 8px;">暂无版本记录</div>
            <div v-for="(ver, idx) in versionHistory" :key="idx" class="version-item">
              <div class="version-dot"></div>
              <div class="version-info">
                <div class="version-name">v{{ ver.version }}</div>
                <div class="version-date">{{ ver.date || ver.built_at || '' }}</div>
                <div v-if="ver.desc" class="version-desc">{{ ver.desc }}</div>
              </div>
            </div>
          </div>
          <div class="settings-card">
            <div class="card-title">构建信息</div>
            <div class="setting-row">
              <div class="setting-info"><div class="setting-name">运行模式</div></div>
              <span style="font-size: 12px; color: #4af;">{{ runMode === 'cloud' ? '☁️ 云端' : runMode === 'api' ? '🔗 API' : '🦙 Ollama' }}</span>
            </div>
            <div class="setting-row">
              <div class="setting-info"><div class="setting-name">当前模型</div></div>
              <span style="font-size: 12px; color: #ddd;">{{ runMode === 'api' ? apiModel : runMode === 'ollama' ? ollamaModel : 'openrouter/auto' }}</span>
            </div>
            <div class="setting-row">
              <div class="setting-info"><div class="setting-name">项目路径</div></div>
              <span style="font-size: 11px; color: #888;">{{ activeProject?.path || workspacePath || '未选择' }}</span>
            </div>
          </div>
        </div>
      </div>
    </main>

    <main v-if="activeNav === 'settings'" class="page-view">
      <div class="settings-page-inline">
        <div class="settings-sidebar">
          <nav class="settings-nav">
            <button :class="['settings-nav-item', { active: settingsTab === 'general' }]" @click="settingsTab = 'general'">
              <span class="nav-icon">🏠</span><span class="nav-label">通用</span>
            </button>
            <button :class="['settings-nav-item', { active: settingsTab === 'model' }]" @click="settingsTab = 'model'">
              <span class="nav-icon">🤖</span><span class="nav-label">模型配置</span>
            </button>
            <button :class="['settings-nav-item', { active: settingsTab === 'account' }]" @click="settingsTab = 'account'">
              <span class="nav-icon">🔑</span><span class="nav-label">账户管理</span>
            </button>
            <button :class="['settings-nav-item', { active: settingsTab === 'memory' }]" @click="settingsTab = 'memory'">
              <span class="nav-icon">🧠</span><span class="nav-label">记忆与知识</span>
            </button>
            <button :class="['settings-nav-item', { active: settingsTab === 'project' }]" @click="settingsTab = 'project'">
              <span class="nav-icon">📁</span><span class="nav-label">项目与模板</span>
            </button>
            <button :class="['settings-nav-item', { active: settingsTab === 'advanced' }]" @click="settingsTab = 'advanced'">
              <span class="nav-icon">🔧</span><span class="nav-label">高级设置</span>
            </button>
          </nav>
        </div>
        <div class="settings-content">
          <div v-if="settingsTab === 'general'" class="settings-section">
            <h3 class="section-title">通用设置</h3>
            <div class="settings-card">
              <div class="card-title">界面</div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">AI 语言</div><div class="setting-desc">AI 回复使用的语言</div></div>
                <select v-model="getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').language" class="setting-select">
                  <option value="zh">🇨🇳 中文</option>
                  <option value="en">🇺🇸 English</option>
                  <option value="ja">🇯🇵 日本語</option>
                  <option value="ko">🇰🇷 한국어</option>
                </select>
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">你的称谓</div><div class="setting-desc">对话中显示的用户名称</div></div>
                <input v-model="userName" placeholder="你" class="setting-input" style="width: 120px;" />
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">AI 称谓</div><div class="setting-desc">对话中显示的助手名称</div></div>
                <input v-model="assistantName" placeholder="助手" class="setting-input" style="width: 120px;" />
              </div>
            </div>
            <div class="settings-card">
              <div class="card-title">开发角色</div>
              <div class="setting-desc" style="margin-bottom: 8px;">选择预设角色可快速配置系统提示词和温度参数</div>
              <div class="role-grid">
                <button v-for="(preset, key) in ROLE_PRESETS" :key="key" :class="['role-card', { active: activeRole === key }]" @click="applyRole(key)">
                  <span class="role-icon">{{ preset.icon }}</span>
                  <span class="role-name">{{ preset.name }}</span>
                  <span class="role-desc">{{ preset.desc }}</span>
                </button>
              </div>
            </div>
            <div class="settings-card">
              <div class="card-title">快捷键</div>
              <div class="shortcut-list">
                <div class="shortcut-row"><span>发送消息</span><kbd>Enter</kbd></div>
                <div class="shortcut-row"><span>换行</span><kbd>Shift + Enter</kbd></div>
                <div class="shortcut-row"><span>快捷指令</span><kbd>/</kbd></div>
                <div class="shortcut-row"><span>打开设置</span><kbd>/settings</kbd></div>
                <div class="shortcut-row"><span>压缩上下文</span><kbd>/compact</kbd></div>
                <div class="shortcut-row"><span>导出对话</span><kbd>/export</kbd></div>
              </div>
            </div>
          </div>

          <div v-if="settingsTab === 'model'" class="settings-section">
            <h3 class="section-title">模型配置</h3>
            <div class="settings-card">
              <div class="card-title">运行模式</div>
              <div class="mode-switch-large">
                <button :class="['mode-card', { active: runMode === 'cloud' }]" @click="runMode = 'cloud'">
                  <span class="mode-icon">☁️</span><span class="mode-name">云端</span><span class="mode-desc">OpenRouter 云端推理</span>
                </button>
                <button :class="['mode-card', { active: runMode === 'api' }]" @click="runMode = 'api'">
                  <span class="mode-icon">🔗</span><span class="mode-name">API</span><span class="mode-desc">本地 API 代理服务</span>
                </button>
                <button :class="['mode-card', { active: runMode === 'ollama' }]" @click="runMode = 'ollama'">
                  <span class="mode-icon">🦙</span><span class="mode-name">Ollama</span><span class="mode-desc">本地模型推理</span>
                </button>
              </div>
            </div>

            <div v-if="runMode === 'cloud'" class="settings-card">
              <div class="card-title">云端配置</div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">API Key</div><div class="setting-desc">OpenRouter API 密钥</div></div>
                <input v-model="apiKey" type="text" placeholder="输入你的 API Key" class="setting-input" style="width: 240px;" />
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">模型</div><div class="setting-desc">固定使用 openrouter/auto</div></div>
                <input value="openrouter/auto" readonly class="setting-input" style="width: 240px; opacity: 0.6;" />
              </div>
            </div>

            <div v-if="runMode === 'api'" class="settings-card">
              <div class="card-title">API 服务</div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">服务地址</div><div class="setting-desc">API 代理服务地址和端口</div></div>
                <div style="display: flex; align-items: center; gap: 0;">
                  <span style="padding: 0 8px; font-size: 12px; color: #888; background: #1a1a1a; border: 1px solid #333; border-right: none; border-radius: 4px 0 0 4px; height: 32px; line-height: 32px;">http://</span>
                  <input v-model="apiHost" placeholder="127.0.0.1" style="width: 120px; border-radius: 0; height: 32px;" />
                  <span style="padding: 0 6px; font-size: 14px; color: #888; background: #1a1a1a; border: 1px solid #333; border-left: none; border-right: none; height: 32px; line-height: 32px;">:</span>
                  <input v-model="apiPort" type="number" min="1" max="65535" placeholder="7777" style="width: 80px; text-align: center; border-radius: 0 4px 4px 0; height: 32px;" />
                </div>
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">API Key</div><div class="setting-desc">自动获取或手动输入</div></div>
                <input v-model="apiKey" type="text" placeholder="自动获取或手动输入" class="setting-input" style="width: 240px;" />
              </div>
              <div class="api-progress-section" style="margin-top: 8px;">
                <div class="api-progress-bar">
                  <div v-for="(step, idx) in apiSteps" :key="idx" :class="['api-step', { done: apiStepProgress > idx, active: apiStepProgress === idx, pending: apiStepProgress < idx }]">
                    <div class="step-dot"><span v-if="apiStepProgress > idx">✓</span><span v-else>{{ idx + 1 }}</span></div>
                    <div class="step-label">{{ step.label }}</div>
                  </div>
                </div>
                <div class="api-progress-track"><div class="api-progress-fill" :style="{ width: apiProgressPercent + '%' }"></div></div>
                <p class="api-progress-msg">{{ apiStepMessage }}</p>
                <div style="display: flex; gap: 6px; margin-top: 6px;">
                  <button class="btn-blue" style="flex: 1;" @click="apiStepAutoRun" :disabled="apiStepBusy || apiStepProgress >= apiSteps.length">{{ apiStepBusy ? apiStepMessage : (apiStepProgress >= apiSteps.length ? '✓ API 服务已就绪' : '▶ 一键启动') }}</button>
                  <button class="btn-red" style="flex: 0 0 auto; min-width: 80px;" @click="stopApiService" :disabled="apiStepBusy || apiStepProgress < apiSteps.length" v-if="apiStepProgress >= apiSteps.length">■ 停止</button>
                </div>
              </div>
            </div>

            <div v-if="runMode === 'api'" class="settings-card">
              <div class="card-title">模型选择</div>
              <div v-if="apiModels.length > 0" class="model-quick-select">
                <div class="custom-select" :class="{ open: modelDropdownOpen }" @click.stop="modelDropdownOpen = !modelDropdownOpen">
                  <div class="custom-select-value">
                    <span>{{ apiModel ? displayName(apiModels.find(m => m.id === apiModel)?.name || apiModel) : '选择模型...' }}</span>
                    <span class="custom-select-arrow">▼</span>
                  </div>
                  <div v-if="modelDropdownOpen" class="custom-select-options">
                    <div v-for="m in apiModels" :key="m.id" :class="['custom-select-option', { selected: apiModel === m.id }]" @click.stop="apiModel = m.id; modelDropdownOpen = false">
                      <span>{{ displayName(m.name || m.id) }}</span>
                      <span v-if="m.toolSupport === true" class="tool-badge ok">★</span>
                      <span v-else-if="m.toolSupport === false" class="tool-badge no">-</span>
                    </div>
                  </div>
                </div>
              </div>
              <div class="setting-row" style="margin-top: 8px;">
                <div class="setting-info"><div class="setting-name">模型名称</div><div class="setting-desc">手动输入或从列表选择</div></div>
                <input v-model="apiModel" placeholder="qwen3.6-plus" class="setting-input" style="width: 240px;" />
              </div>
            </div>

            <div v-if="runMode === 'ollama'" class="settings-card">
              <div class="card-title">Ollama 配置</div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">Ollama 地址</div><div class="setting-desc">本地 Ollama 服务地址</div></div>
                <div class="model-loader">
                  <input v-model="ollamaBaseUrl" placeholder="http://127.0.0.1:11434" class="setting-input" style="width: 200px;" />
                  <button class="btn-blue btn-sm" @click="detectModels" :disabled="loadingModels">{{ loadingModels ? "检测中..." : "🔍 检测" }}</button>
                </div>
              </div>
            </div>

            <div class="settings-card">
              <div class="card-title">模型参数</div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">Temperature</div><div class="setting-desc">创造性程度，0=精确 2=创造</div></div>
                <div class="range-row"><input type="number" min="0" max="2" step="0.1" placeholder="0.3" class="setting-input" style="width: 80px;" :value="getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').temperature" @input="(e: any) => { getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').temperature = e.target.value; }" /></div>
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">Max Tokens</div><div class="setting-desc">最大输出长度</div></div>
                <input type="number" min="256" max="65536" step="256" placeholder="4096" class="setting-input" style="width: 120px;" :value="getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').maxTokens" @input="(e: any) => { getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').maxTokens = e.target.value; }" />
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">Top P</div><div class="setting-desc">核采样，0.1=聚焦 1.0=开放</div></div>
                <input type="number" min="0" max="1" step="0.05" placeholder="1.0" class="setting-input" style="width: 80px;" :value="getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').topP" @input="(e: any) => { getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').topP = e.target.value; }" />
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">频率惩罚</div><div class="setting-desc">降低重复词</div></div>
                <input type="number" min="-2" max="2" step="0.1" placeholder="0" class="setting-input" style="width: 80px;" :value="getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').frequencyPenalty" @input="(e: any) => { getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').frequencyPenalty = e.target.value; }" />
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">存在惩罚</div><div class="setting-desc">鼓励新话题</div></div>
                <input type="number" min="-2" max="2" step="0.1" placeholder="0" class="setting-input" style="width: 80px;" :value="getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').presencePenalty" @input="(e: any) => { getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').presencePenalty = e.target.value; }" />
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">系统提示词</div><div class="setting-desc">自定义 AI 行为指令</div></div>
              </div>
              <textarea :value="getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').systemPrompt" @input="(e: any) => { getModelConfig(runMode === 'ollama' ? ollamaModel : runMode === 'api' ? apiModel : 'openrouter/auto').systemPrompt = e.target.value; }" rows="3" placeholder="留空则根据语言自动生成" class="setting-textarea"></textarea>
            </div>

            <div style="display: flex; gap: 8px; margin-top: 12px;">
              <button class="btn-blue" @click="saveSettings" :disabled="isBusy" style="flex: 1;">💾 保存配置</button>
              <button class="btn-sm" @click="clearModelFields" :disabled="isBusy" style="background: #333;">清空模型</button>
            </div>
          </div>

          <div v-if="settingsTab === 'account'" class="settings-section">
            <h3 class="section-title">账户管理</h3>
            <div class="settings-card">
              <div class="card-title">上游账户池</div>
              <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 8px;">
                <span v-if="qwenAccountCount >= 0" :class="['account-badge', qwenAccountCount > 0 ? 'ok' : 'warn']">{{ qwenAccountCount }} 个账户</span>
                <span v-if="qwenValidCount > 0" style="color: #4CAF50; font-size: 11px;">{{ qwenValidCount }} 可用</span>
                <button class="btn-blue btn-sm" @click="checkQwenAccounts" style="margin-left: auto;">刷新</button>
              </div>
              <p v-if="qwenAccountCount === 0" class="hint warn" style="margin: 4px 0;">未添加上游账户，AI 对话将返回 500 错误。</p>
              <div v-if="qwenAccounts.length > 0" class="account-list">
                <div v-for="acc in qwenAccounts" :key="acc.email" :class="['account-row', { sticky: stickyEmail === acc.email }]">
                  <span :class="['account-status', acc.valid ? 'valid' : 'invalid']">●</span>
                  <span class="account-email">{{ acc.email }}</span>
                  <span v-if="stickyEmail === acc.email" class="sticky-badge">★ 优先</span>
                  <span v-if="!acc.valid" class="account-err">{{ acc.status_code || '不可用' }}</span>
                  <button v-if="stickyEmail !== acc.email && acc.valid" class="btn-icon btn-sticky" @click="setStickyAccount(acc.email)" title="设为优先">★</button>
                  <button class="btn-icon btn-del" @click="deleteQwenAccount(acc.email)" title="删除">✕</button>
                </div>
                <button v-if="stickyEmail" class="btn-blue btn-sm" @click="clearStickyAccount" style="margin-top: 4px; width: 100%;">取消优先，恢复自动轮换</button>
              </div>
            </div>
            <div class="settings-card">
              <div class="card-title">添加账户</div>
              <div style="display: flex; gap: 8px; margin-bottom: 8px;">
                <button class="btn-blue" @click="autoRegisterQwenAccount" :disabled="qwenRegisterBusy" style="flex: 1;">{{ qwenRegisterBusy ? '⏳ 注册中...' : '🤖 自动注册' }}</button>
              </div>
              <details style="margin-top: 6px;">
                <summary style="font-size: 11px; color: #888; cursor: pointer;">🔑 登录已有账户</summary>
                <div class="reg-form" style="margin-top: 4px;">
                  <input v-model="loginEmail" type="text" placeholder="邮箱" />
                  <input v-model="loginPassword" type="password" placeholder="密码" />
                  <button class="btn-blue btn-sm" @click="loginQwenAccount" :disabled="qwenLoginBusy || !loginEmail.trim() || !loginPassword.trim()" style="width: 100%;">{{ qwenLoginBusy ? '⏳ 登录中...' : '登录' }}</button>
                </div>
                <p v-if="qwenLoginError" class="hint warn" style="margin: 4px 0;">{{ qwenLoginError }}</p>
              </details>
              <details style="margin-top: 6px;">
                <summary style="font-size: 11px; color: #888; cursor: pointer;">手动添加 Token</summary>
                <div class="qwen-token-input" style="margin-top: 4px;">
                  <input v-model="qwenToken" type="text" placeholder="粘贴 Token" />
                  <button class="btn-blue btn-sm" @click="addQwenAccount" :disabled="!qwenToken.trim()">添加</button>
                </div>
              </details>
              <details style="margin-top: 6px;">
                <summary style="font-size: 11px; color: #888; cursor: pointer;">自定义注册信息</summary>
                <div class="reg-form" style="margin-top: 4px;">
                  <input v-model="regEmail" type="text" placeholder="邮箱（留空自动生成）" />
                  <input v-model="regPassword" type="password" placeholder="密码（留空自动生成）" />
                  <input v-model="regUsername" type="text" placeholder="用户名（留空自动生成）" />
                </div>
              </details>
              <div v-if="qwenRegisterLogs.length > 0" class="register-log-box" style="margin-top: 6px;">
                <div v-for="(log, idx) in qwenRegisterLogs" :key="idx" class="register-log-line">{{ log }}</div>
              </div>
            </div>
          </div>

          <div v-if="settingsTab === 'memory'" class="settings-section">
            <h3 class="section-title">记忆与知识</h3>
            <div class="settings-card">
              <div class="card-title">项目记忆 (CLAUDE.md)</div>
              <div class="setting-desc" style="margin-bottom: 8px;">AI 每次对话都会读取此内容，用于存储项目级指令和规范</div>
              <div v-if="activeProject">
                <div style="font-size: 11px; color: #888; margin-bottom: 4px;">项目级记忆</div>
                <textarea v-model="memoryContent" rows="5" placeholder="在此编辑项目记忆..." class="setting-textarea"></textarea>
                <div style="display: flex; gap: 4px; margin-top: 4px;">
                  <button class="btn-sm" @click="saveMemoryContent" style="flex: 1;">保存项目记忆</button>
                  <button class="btn-sm" @click="appendMemory('技术栈: ')" style="background: #2a3a2a;">+ 技术栈</button>
                  <button class="btn-sm" @click="appendMemory('编码规范: ')" style="background: #2a3a2a;">+ 规范</button>
                </div>
                <div style="font-size: 11px; color: #888; margin-top: 8px; margin-bottom: 4px;">全局记忆</div>
                <textarea v-model="globalMemoryContent" rows="3" placeholder="全局记忆，适用于所有项目..." class="setting-textarea"></textarea>
                <button class="btn-sm" @click="saveGlobalMemory" style="margin-top: 4px;">保存全局记忆</button>
              </div>
              <div v-else style="font-size: 11px; color: #555; padding: 8px;">请先选择项目</div>
            </div>
            <div class="settings-card">
              <div class="card-title">智能记忆</div>
              <div style="display: flex; gap: 4px; margin-bottom: 8px; align-items: center;">
                <input v-model="memorySearchQuery" placeholder="搜索记忆..." class="setting-input" style="flex: 1;" @keydown.enter="searchSmartMemories" />
                <button class="btn-sm" @click="searchSmartMemories" style="background: #2a3a2a;">🔍</button>
                <button class="btn-sm" @click="extractMemoriesFromConversation" style="background: #2a3a2a;">📥 从对话提取</button>
              </div>
              <div style="font-size: 10px; color: #666; margin-bottom: 8px;">四类记忆：👤 用户偏好 | 🔄 反馈纠正 | 📋 项目上下文 | 🔗 外部引用</div>
              <div v-if="activeProject">
                <div v-if="smartMemories.length === 0" style="font-size: 11px; color: #555; text-align: center; padding: 12px;">暂无智能记忆，点击"从对话提取"自动生成</div>
                <div v-for="mem in smartMemories" :key="mem.filename" class="memory-item">
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <span :class="['mem-type-badge', mem.type]">{{ mem.type === 'user' ? '👤' : mem.type === 'feedback' ? '🔄' : mem.type === 'reference' ? '🔗' : '📋' }} {{ mem.title }}</span>
                    <button class="btn-icon-sm" @click="deleteSmartMemory(mem.filename)" style="font-size: 9px;">🗑️</button>
                  </div>
                  <div v-if="mem.match_snippet" style="font-size: 10px; color: #4af; margin-top: 2px; padding: 2px 4px; background: #1a2a3a; border-radius: 2px;">匹配: {{ mem.match_snippet }}</div>
                  <div style="font-size: 10px; color: #888; margin-top: 2px; white-space: pre-wrap; max-height: 60px; overflow-y: auto;">{{ mem.content.replace(/^---[\s\S]*?---\n*/, '').slice(0, 200) }}</div>
                </div>
                <div style="margin-top: 8px; border-top: 1px solid #222; padding-top: 8px;">
                  <div style="font-size: 11px; color: #888; margin-bottom: 4px;">添加新记忆</div>
                  <div style="display: flex; gap: 4px; margin-bottom: 4px;">
                    <input v-model="newMemoryTitle" placeholder="标题" class="setting-input" style="flex: 1;" />
                    <select v-model="newMemoryType" class="setting-select" style="width: 100px;">
                      <option value="user">👤 用户</option>
                      <option value="feedback">🔄 反馈</option>
                      <option value="project">📋 项目</option>
                      <option value="reference">🔗 引用</option>
                    </select>
                  </div>
                  <textarea v-model="newMemoryContent" rows="2" placeholder="记忆内容..." class="setting-textarea"></textarea>
                  <button class="btn-sm" @click="addSmartMemory" style="margin-top: 4px; width: 100%;">添加记忆</button>
                </div>
              </div>
              <div v-else style="font-size: 11px; color: #555; padding: 8px;">请先选择项目</div>
            </div>
          </div>

          <div v-if="settingsTab === 'project'" class="settings-section">
            <h3 class="section-title">项目与模板</h3>
            <div class="settings-card">
              <div class="card-title">项目管理</div>
              <div v-if="activeProject" style="font-size: 11px; color: #42A5F5; margin-bottom: 8px;">当前项目：{{ activeProject.name }} <span style="color: #666;">{{ activeProject.path }}</span></div>
              <div style="margin-bottom: 8px;">
                <div style="font-size: 11px; color: #888; margin-bottom: 4px;">新建项目</div>
                <div style="display: flex; gap: 4px; margin-bottom: 4px;">
                  <input v-model="newProjectName" placeholder="项目名称" class="setting-input" style="flex: 1;" @input="updateDefaultPath" />
                  <button class="btn-blue btn-sm" @click="createProject" :disabled="!newProjectName.trim()">创建</button>
                </div>
                <div v-if="newProjectDefaultPath" style="font-size: 10px; color: #666;">{{ newProjectDefaultPath }}</div>
              </div>
              <div v-for="p in projects" :key="p.id" :class="['project-item', { active: p.id === activeProject?.id }]" style="margin-bottom: 4px;">
                <div style="display: flex; align-items: center; justify-content: space-between;" @click="switchProject(p.id)">
                  <span style="font-size: 12px;">{{ p.name }}</span>
                  <div style="display: flex; gap: 2px;">
                    <button class="btn-icon-sm" @click.stop="openInExplorer(p.path)" title="打开目录">📁</button>
                    <button class="btn-icon-sm" @click.stop="deleteProject(p.id)" title="删除">🗑️</button>
                  </div>
                </div>
                <div style="font-size: 10px; color: #666;">{{ p.path }}</div>
              </div>
            </div>
            <div class="settings-card">
              <div class="card-title">项目模板</div>
              <div style="display: flex; gap: 3px; margin-bottom: 8px; flex-wrap: wrap;">
                <button v-for="cat in ['all', 'frontend', 'backend', 'fullstack', 'desktop', 'datascience']" :key="cat" :class="['tab-btn', { active: templateCategory === cat }]" @click="templateCategory = cat; loadProjectTemplates()" style="font-size: 10px;">{{ cat === 'all' ? '全部' : cat === 'frontend' ? '前端' : cat === 'backend' ? '后端' : cat === 'fullstack' ? '全栈' : cat === 'desktop' ? '桌面' : '数据' }}</button>
              </div>
              <div class="template-grid">
                <div v-for="t in projectTemplates" :key="t.id" class="template-card" @click="useTemplate(t)">
                  <div style="font-size: 12px; font-weight: 500; color: #ddd;">{{ t.custom ? '⭐ ' : '' }}{{ t.name }}</div>
                  <div style="font-size: 10px; color: #888; margin-top: 2px;">{{ t.desc }}</div>
                  <div style="display: flex; gap: 4px; margin-top: 4px;">
                    <button v-if="t.scaffold" class="btn-sm" @click.stop="scaffoldFromTemplate(t)" style="font-size: 9px; background: #2a3a2a;">🏗️ 脚手架</button>
                    <button v-if="t.custom" class="btn-icon-sm" @click.stop="deleteCustomTemplate(t.id)" style="font-size: 9px;">🗑️</button>
                  </div>
                </div>
              </div>
              <div style="margin-top: 8px; border-top: 1px solid #222; padding-top: 8px;">
                <button class="btn-sm" @click="showAddTemplate = !showAddTemplate" style="width: 100%;">{{ showAddTemplate ? '取消' : '➕ 添加自定义模板' }}</button>
                <div v-if="showAddTemplate" style="margin-top: 6px; padding: 8px; border: 1px solid #333; border-radius: 4px; background: #1a1a1a;">
                  <input v-model="newTplName" placeholder="模板名称" class="setting-input" style="width: 100%; margin-bottom: 4px;" />
                  <div style="display: flex; gap: 4px; margin-bottom: 4px;">
                    <select v-model="newTplCategory" class="setting-select" style="flex: 1;">
                      <option value="frontend">前端</option><option value="backend">后端</option><option value="fullstack">全栈</option><option value="desktop">桌面</option><option value="datascience">数据科学</option>
                    </select>
                    <input v-model="newTplDesc" placeholder="描述" class="setting-input" style="flex: 2;" />
                  </div>
                  <textarea v-model="newTplPrompt" rows="2" placeholder="模板提示词" class="setting-textarea"></textarea>
                  <button class="btn-sm" @click="addCustomTemplate" style="margin-top: 4px; width: 100%;">保存模板</button>
                </div>
              </div>
            </div>
            <div class="settings-card">
              <div class="card-title">开发工作流</div>
              <div style="display: flex; flex-wrap: wrap; gap: 6px;">
                <button v-for="(wf, key) in WORKFLOW_PRESETS" :key="key" class="workflow-card" @click="applyWorkflow(key)">
                  <span style="font-size: 13px;">{{ wf.name }}</span>
                  <span style="font-size: 10px; color: #888;">{{ wf.steps.join(' → ') }}</span>
                </button>
              </div>
            </div>
          </div>

          <div v-if="settingsTab === 'advanced'" class="settings-section">
            <h3 class="section-title">高级设置</h3>
            <div class="settings-card">
              <div class="card-title">工具调用审批</div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">审批模式</div><div class="setting-desc">控制 AI 执行工具操作的权限</div></div>
                <select v-model="toolApprovalMode" class="setting-select" style="width: 240px;">
                  <option value="auto">🟢 全部自动 — AI 直接执行</option>
                  <option value="smart">🟡 智能审批 — 安全自动，风险确认</option>
                  <option value="manual">🔴 全部手动 — 每次确认</option>
                </select>
              </div>
              <p v-if="toolApprovalMode === 'auto'" style="font-size: 11px; color: #FF9800; margin: 4px 0;">⚠ AI 可直接读写文件和执行命令</p>
              <p v-else-if="toolApprovalMode === 'smart'" style="font-size: 11px; color: #4af; margin: 4px 0;">读取/搜索自动通过，写入/编辑/命令需确认</p>
              <p v-else style="font-size: 11px; color: #888; margin: 4px 0;">所有工具调用都需要手动确认</p>
            </div>
            <div class="settings-card">
              <div class="card-title">AI 协作模式</div>
              <div style="display: flex; flex-wrap: wrap; gap: 6px;">
                <button :class="['workflow-card', { active: collaborationMode === 'plan-code-review' }]" @click="setCollaborationMode('plan-code-review')">
                  <span style="font-size: 13px;">📋 规划→编码→审查</span>
                  <span style="font-size: 10px; color: #888;">三阶段协作流程</span>
                </button>
                <button :class="['workflow-card', { active: collaborationMode === 'pair' }]" @click="setCollaborationMode('pair')">
                  <span style="font-size: 13px;">👥 结对编程</span>
                  <span style="font-size: 10px; color: #888;">交替工作模式</span>
                </button>
                <button :class="['workflow-card', { active: collaborationMode === 'review-only' }]" @click="setCollaborationMode('review-only')">
                  <span style="font-size: 13px;">🔍 纯审查</span>
                  <span style="font-size: 10px; color: #888;">只审查不修改</span>
                </button>
              </div>
              <div v-if="collaborationMode !== 'none'" style="margin-top: 8px; padding: 8px; border: 1px solid #333; border-radius: 4px; background: #1a1a1a;">
                <div style="font-size: 11px; color: #888;">当前模式：{{ collaborationMode === 'plan-code-review' ? '规划→编码→审查' : collaborationMode === 'pair' ? '结对编程' : '纯审查' }}
                  <span v-if="collaborationMode === 'plan-code-review'" style="color: #4af;"> | 阶段：{{ collabPhase === 'planning' ? '📋 规划中' : collabPhase === 'coding' ? '💻 编码中' : '🔍 审查中' }}</span>
                </div>
                <div v-if="collaborationMode === 'plan-code-review'" style="display: flex; gap: 4px; margin-top: 4px;">
                  <button class="btn-sm" @click="advanceCollabPhase" style="flex: 1;">⏭ 进入下一阶段</button>
                  <button class="btn-sm" @click="setCollaborationMode('none')" style="background: #3a2222;">退出协作</button>
                </div>
              </div>
            </div>
            <div class="settings-card">
              <div class="card-title">任务编排</div>
              <div style="display: flex; gap: 4px; margin-bottom: 8px;">
                <input v-model="newTaskName" placeholder="任务名称" class="setting-input" style="flex: 1;" @keydown.enter="addTask" />
                <button class="btn-blue btn-sm" @click="addTask">添加</button>
              </div>
              <textarea v-model="newTaskDesc" rows="1" placeholder="任务描述（可选）" class="setting-textarea" style="margin-bottom: 8px;"></textarea>
              <div v-if="taskList.length === 0" style="font-size: 11px; color: #555; text-align: center; padding: 8px;">暂无任务</div>
              <div v-for="task in taskList" :key="task.id" style="display: flex; align-items: center; gap: 6px; padding: 4px 0; border-bottom: 1px solid #222;">
                <span :style="{ color: task.status === 'done' ? '#4CAF50' : task.status === 'in_progress' ? '#FF9800' : '#888', fontSize: '12px', cursor: 'pointer' }" @click="updateTaskStatus(task.id, task.status === 'pending' ? 'in_progress' : task.status === 'in_progress' ? 'done' : 'pending')">
                  {{ task.status === 'done' ? '✅' : task.status === 'in_progress' ? '🔄' : '⬜' }}
                </span>
                <span style="flex: 1; font-size: 11px; color: #ddd;" :style="{ textDecoration: task.status === 'done' ? 'line-through' : 'none' }">{{ task.name }}</span>
                <button class="btn-icon-sm" @click="executeTaskAsPrompt(task)" title="执行" style="font-size: 9px;">▶</button>
                <button class="btn-icon-sm" @click="removeTask(task.id)" title="删除" style="font-size: 9px;">✕</button>
              </div>
            </div>
            <div class="settings-card">
              <div class="card-title">数据管理</div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">导出对话</div><div class="setting-desc">将当前对话导出为文件</div></div>
                <div style="display: flex; gap: 4px;">
                  <button class="btn-sm" @click="exportConversation('markdown')" :disabled="messages.length === 0" style="background: #2a3a2a;">📄 Markdown</button>
                  <button class="btn-sm" @click="exportConversation('json')" :disabled="messages.length === 0" style="background: #2a3a2a;">📋 JSON</button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script lang="ts">
import { defineComponent, h } from "vue";

const ModelSettingsPanel = defineComponent({
  name: "ModelSettingsPanel",
  props: {
    config: { type: Object, required: true },
    hint: { type: String, default: "" },
  },
  emits: ["auto-configure"],
  setup(props, { emit }) {
    const showAdvanced = ref(false);
    return () => h("div", { class: "model-settings-inner" }, [
      h("div", { class: "settings-header" }, [
        h("span", { class: "settings-title" }, "模型参数"),
        h("button", {
          class: "btn-auto",
          onClick: () => emit("auto-configure"),
        }, "🪄 自动配置"),
      ]),
      props.hint ? h("p", { class: "auto-hint" }, props.hint) : null,
      h("label", { class: "field" }, [
        h("span", "AI 语言"),
        h("select", {
          class: "select-input",
          value: props.config.language,
          onChange: (e: Event) => { props.config.language = (e.target as HTMLSelectElement).value; },
        }, [
          h("option", { value: "zh" }, "🇨🇳 中文"),
          h("option", { value: "en" }, "🇺🇸 English"),
          h("option", { value: "ja" }, "🇯🇵 日本語"),
          h("option", { value: "ko" }, "🇰🇷 한국어"),
        ]),
      ]),
      h("label", { class: "field" }, [
        h("span", "Temperature（创造性）"),
        h("div", { class: "range-row" }, [
          h("input", {
            type: "number", min: "0", max: "2", step: "0.1",
            placeholder: "0.3", class: "short-input",
            value: props.config.temperature,
            onInput: (e: Event) => { props.config.temperature = (e.target as HTMLInputElement).value; },
          }),
          h("span", { class: "range-hint" }, "0=精确 2=创造"),
        ]),
      ]),
      h("label", { class: "field" }, [
        h("span", "Max Tokens（最大输出长度）"),
        h("div", { class: "range-row" }, [
          h("input", {
            type: "number", min: "256", max: "65536", step: "256",
            placeholder: "4096", class: "short-input",
            value: props.config.maxTokens,
            onInput: (e: Event) => { props.config.maxTokens = (e.target as HTMLInputElement).value; },
          }),
          h("span", { class: "range-hint" }, "留空=默认"),
        ]),
      ]),
      h("label", { class: "field" }, [
        h("span", "自定义系统提示词"),
        h("textarea", {
          rows: 2, class: "textarea-input",
          placeholder: "留空则根据语言自动生成",
          value: props.config.systemPrompt,
          onInput: (e: Event) => { props.config.systemPrompt = (e.target as HTMLTextAreaElement).value; },
        }),
      ]),
      h("div", {
        class: "advanced-toggle",
        onClick: () => { showAdvanced.value = !showAdvanced.value; },
      }, [
        h("span", {}, showAdvanced.value ? "▼ 高级参数" : "▶ 高级参数"),
      ]),
      showAdvanced.value ? h("div", { class: "advanced-params" }, [
        h("label", { class: "field" }, [
          h("span", "Top P（核采样）"),
          h("div", { class: "range-row" }, [
            h("input", {
              type: "number", min: "0", max: "1", step: "0.05",
              placeholder: "1.0", class: "short-input",
              value: props.config.topP || "",
              onInput: (e: Event) => { props.config.topP = (e.target as HTMLInputElement).value; },
            }),
            h("span", { class: "range-hint" }, "0.1=聚焦 1.0=开放"),
          ]),
        ]),
        h("label", { class: "field" }, [
          h("span", "频率惩罚"),
          h("div", { class: "range-row" }, [
            h("input", {
              type: "number", min: "-2", max: "2", step: "0.1",
              placeholder: "0", class: "short-input",
              value: props.config.frequencyPenalty || "",
              onInput: (e: Event) => { props.config.frequencyPenalty = (e.target as HTMLInputElement).value; },
            }),
            h("span", { class: "range-hint" }, "降低重复词"),
          ]),
        ]),
        h("label", { class: "field" }, [
          h("span", "存在惩罚"),
          h("div", { class: "range-row" }, [
            h("input", {
              type: "number", min: "-2", max: "2", step: "0.1",
              placeholder: "0", class: "short-input",
              value: props.config.presencePenalty || "",
              onInput: (e: Event) => { props.config.presencePenalty = (e.target as HTMLInputElement).value; },
            }),
            h("span", { class: "range-hint" }, "鼓励新话题"),
          ]),
        ]),
        h("label", { class: "field" }, [
          h("span", "停止词"),
          h("input", {
            type: "text",
            placeholder: "用逗号分隔，如: \\n,###",
            class: "short-input",
            value: props.config.stopSequences || "",
            onInput: (e: Event) => { props.config.stopSequences = (e.target as HTMLInputElement).value; },
          }),
        ]),
      ]) : null,
    ]);
  },
});

export default { name: "App" };
</script>

<style scoped>
.page {
  height: 100%;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: #0d0d0d;
}

.flowbar {
  border-radius: 6px;
  border: 1px solid #2a2a2a;
  background: #1a1a1a;
  color: #888888;
  font-size: 12px;
  padding: 8px 12px;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.workbench {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 1fr 380px;
  gap: 10px;
}

.workbench.single {
  grid-template-columns: 1fr;
}

.card {
  border-radius: 8px;
  border: 1px solid #2a2a2a;
  background: #1a1a1a;
  min-height: 0;
}

.chat {
  display: flex;
  flex-direction: column;
  padding: 10px;
  gap: 10px;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  align-items: center;
  padding-bottom: 8px;
  border-bottom: 1px solid #2a2a2a;
}

.session {
  color: #888888;
  font-size: 12px;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.messages {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 4px;
}

.msg {
  border-radius: 4px;
  border: 1px solid #2a2a2a;
  padding: 12px 14px;
}

.msg.user {
  background: #1e3a8a;
  border-color: #3b82f6;
}

.msg.assistant {
  background: #262626;
}

.msg.error {
  background: #7f1d1d;
  border-color: #ef4444;
}

.msg .msg-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.msg label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #888888;
}

.msg .msg-time {
  font-size: 10px;
  color: #555;
}

.msg .msg-model {
  font-size: 10px;
  color: #666;
  background: #1a1a2e;
  padding: 1px 6px;
  border-radius: 3px;
}

.msg .msg-actions {
  margin-left: auto;
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.15s;
}

.msg:hover .msg-actions {
  opacity: 1;
}

.btn-msg-del {
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  font-size: 12px;
  padding: 0 4px;
  border-radius: 3px;
  line-height: 1;
}

.btn-msg-del:hover {
  color: #f44;
  background: rgba(255, 68, 68, 0.1);
}

.btn-msg-action {
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  font-size: 13px;
  padding: 0 4px;
  border-radius: 3px;
  line-height: 1;
}

.btn-msg-action:hover {
  color: #4af;
  background: rgba(68, 170, 255, 0.1);
}

.role-presets {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  align-items: center;
  margin: 4px 0;
}

.role-btn {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 3px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #aaa;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s;
}

.role-btn:hover {
  border-color: #555;
  color: #ddd;
}

.role-btn.active {
  border-color: #4af;
  color: #4af;
  background: rgba(68, 170, 255, 0.1);
}

.context-bar-wrap {
  flex: 1;
  max-width: 120px;
  height: 4px;
  background: #222;
  border-radius: 2px;
  overflow: hidden;
}

.context-bar {
  height: 100%;
  border-radius: 2px;
  transition: width 0.3s, background 0.3s;
}

.slash-menu {
  position: absolute;
  bottom: 100%;
  left: 0;
  right: 0;
  background: #1a1a1a;
  border: 1px solid #333;
  border-radius: 6px;
  max-height: 200px;
  overflow-y: auto;
  z-index: 100;
  box-shadow: 0 -4px 12px rgba(0,0,0,0.4);
}

.slash-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  cursor: pointer;
  transition: background 0.1s;
}

.slash-item:hover {
  background: #252525;
}

.slash-name {
  font-size: 12px;
  color: #4af;
  font-weight: 500;
  white-space: nowrap;
}

.slash-desc {
  font-size: 11px;
  color: #888;
}

.file-changes-panel {
  margin-top: 4px;
  padding: 6px 8px;
  background: #111;
  border-radius: 4px;
  border: 1px solid #222;
  max-height: 150px;
  overflow-y: auto;
}

.file-change-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
  font-size: 11px;
}

.fc-action {
  font-weight: bold;
  width: 14px;
  text-align: center;
  font-family: monospace;
}

.fc-action.create { color: #4f4; }
.fc-action.modify { color: #fa0; }
.fc-action.delete { color: #f44; }

.fc-path {
  flex: 1;
  min-width: 0;
  color: #aaa;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: monospace;
}

.fc-time {
  color: #555;
  font-size: 10px;
  flex-shrink: 0;
}

.advanced-toggle {
  padding: 4px 0;
  cursor: pointer;
  color: #666;
  font-size: 11px;
  user-select: none;
  transition: color 0.15s;
}

.advanced-toggle:hover {
  color: #aaa;
}

.advanced-params {
  padding-top: 4px;
  border-top: 1px solid #222;
}

.workflow-btn {
  font-size: 10px;
  padding: 3px 8px;
  border-radius: 4px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #aaa;
  cursor: pointer;
  transition: all 0.15s;
}

.workflow-btn:hover {
  border-color: #4af;
  color: #4af;
  background: rgba(68, 170, 255, 0.08);
}

.workflow-btn.active {
  border-color: #4af;
  color: #4af;
  background: rgba(68, 170, 255, 0.12);
}

.tab-btn {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 3px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #888;
  cursor: pointer;
  transition: all 0.15s;
}

.tab-btn:hover {
  color: #bbb;
  border-color: #444;
}

.tab-btn.active {
  color: #4af;
  border-color: #4af;
  background: rgba(68, 170, 255, 0.08);
}

.memory-item {
  padding: 6px;
  margin-bottom: 4px;
  border-radius: 4px;
  border: 1px solid #222;
  background: #111;
}

.mem-type-badge {
  font-size: 11px;
  font-weight: 500;
}

.mem-type-badge.user { color: #4af; }
.mem-type-badge.feedback { color: #fa0; }
.mem-type-badge.project { color: #4f4; }
.mem-type-badge.reference { color: #a8f; }

.template-item {
  padding: 6px 8px;
  margin-bottom: 3px;
  border-radius: 4px;
  border: 1px solid #222;
  background: #111;
  cursor: pointer;
  transition: all 0.15s;
}

.template-item:hover {
  border-color: #4af;
  background: rgba(68, 170, 255, 0.05);
}

.preview-panel {
  height: 250px;
  display: flex;
  flex-direction: column;
  border-top: 1px solid #2a2a2a;
  background: #0d0d0d;
}

.preview-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  border-bottom: 1px solid #222;
  background: #111;
}

.msg .msg-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 6px;
  padding-top: 6px;
  border-top: 1px solid #222;
}

.msg .msg-meta {
  font-size: 10px;
  color: #555;
  display: flex;
  gap: 4px;
}

.msg .msg-rating {
  display: flex;
  gap: 1px;
}

.msg .msg-rating .star {
  font-size: 14px;
  color: #333;
  cursor: pointer;
  transition: color 0.1s;
  line-height: 1;
}

.msg .msg-rating .star:hover {
  color: #888;
}

.msg .msg-rating .star.active {
  color: #f5a623;
}

.msg .thinking {
  color: #666;
  font-style: italic;
}

.msg .tool-status {
  color: #888;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
}

.msg .thinking .dots {
  animation: blink 1.2s infinite;
}

@keyframes blink {
  0%, 20% { opacity: 0; }
  50% { opacity: 1; }
  80%, 100% { opacity: 0; }
}

.msg.user label {
  color: #93c5fd;
}

.msg.error label {
  color: #fca5a5;
}

.msg pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  overflow-wrap: break-word;
  line-height: 1.6;
  color: #e5e5e5;
  font-family: inherit;
  font-size: 14px;
}

.composer {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px solid #2a2a2a;
}

.composer textarea {
  width: 100%;
  min-height: 90px;
  max-height: 200px;
  resize: vertical;
  background: #0d0d0d;
  border: 1px solid #2a2a2a;
  border-radius: 4px;
  padding: 10px 12px;
  color: #e5e5e5;
  font-size: 14px;
  outline: none;
}

.composer textarea:focus {
  border-color: #3b82f6;
}

.composer-foot {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  font-size: 12px;
  color: #888888;
}

.composer-foot > div {
  display: flex;
  gap: 8px;
}

.panel {
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow: auto;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.panel-header h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: #ffffff;
}

.mode-switch {
  display: flex;
  gap: 4px;
}

.mode-btn {
  padding: 5px 10px;
  font-size: 12px;
  border: 1px solid #333;
  border-radius: 4px;
  background: #1a1a1a;
  color: #888;
  cursor: pointer;
  transition: all 0.15s;
  font-weight: 500;
}

.mode-btn:hover {
  background: #252525;
  color: #ccc;
}

.mode-btn.active {
  background: #1565C0;
  border-color: #1976D2;
  color: #fff;
}

.api-progress-section {
  background: #111;
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  padding: 12px;
  margin: 4px 0;
}

.qwen-account-section {
  background: #111;
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  padding: 10px 12px;
  margin: 4px 0;
}

.qwen-account-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #ccc;
  margin-bottom: 6px;
}

.account-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 8px;
  font-weight: bold;
}

.account-badge.ok {
  background: #1b3a1b;
  color: #4CAF50;
}

.account-badge.warn {
  background: #3a2a1b;
  color: #FF9800;
}

.qwen-token-input {
  display: flex;
  gap: 6px;
}

.qwen-token-input input {
  flex: 1;
  background: #1a1a1a;
  border: 1px solid #333;
  border-radius: 4px;
  padding: 4px 8px;
  color: #f0f0f0;
  font-size: 11px;
}

.qwen-token-input input:focus {
  border-color: #3b82f6;
  outline: none;
}

.register-log-box {
  background: #0a0a0a;
  border: 1px solid #222;
  border-radius: 4px;
  padding: 6px 8px;
  margin: 6px 0;
  max-height: 160px;
  overflow-y: auto;
  font-family: "Consolas", "Monaco", monospace;
  font-size: 10px;
  line-height: 1.5;
}

.register-log-line {
  color: #aaa;
  word-break: break-all;
}

.register-log-line:last-child {
  color: #4CAF50;
}

.account-list {
  margin: 4px 0;
}

.account-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  padding: 2px 0;
}

.account-status {
  font-size: 8px;
}

.account-status.valid {
  color: #4CAF50;
}

.account-status.invalid {
  color: #F44336;
}

.account-email {
  color: #ccc;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.account-err {
  color: #FF9800;
  font-size: 9px;
  background: #2a1a00;
  padding: 1px 4px;
  border-radius: 3px;
}

.btn-icon.btn-del {
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  font-size: 10px;
  padding: 0 4px;
  line-height: 1;
}

.btn-icon.btn-del:hover {
  color: #F44336;
}

.btn-icon.btn-sticky {
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  font-size: 10px;
  padding: 0 4px;
  line-height: 1;
}

.btn-icon.btn-sticky:hover {
  color: #fbbf24;
}

.account-row.sticky {
  background: rgba(251, 191, 36, 0.08);
  border-radius: 4px;
  padding: 2px 4px;
}

.sticky-badge {
  color: #fbbf24;
  font-size: 9px;
  background: #2a2200;
  padding: 1px 4px;
  border-radius: 3px;
}

.reg-form {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.reg-form input {
  background: #1a1a2e;
  border: 1px solid #333;
  color: #ddd;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 11px;
  width: 100%;
  box-sizing: border-box;
}

.reg-form input:focus {
  border-color: #4a9eff;
  outline: none;
}

.api-progress-bar {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 8px;
}

.api-step {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 1;
  gap: 4px;
}

.api-step .step-dot {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: bold;
  border: 2px solid #333;
  background: #1a1a1a;
  color: #666;
  transition: all 0.3s;
}

.api-step.done .step-dot {
  background: #2E7D32;
  border-color: #4CAF50;
  color: #fff;
}

.api-step.active .step-dot {
  background: #1565C0;
  border-color: #42A5F5;
  color: #fff;
  animation: stepPulse 1.5s infinite;
}

.api-step .step-label {
  font-size: 10px;
  color: #666;
  text-align: center;
}

.api-step.done .step-label {
  color: #4CAF50;
}

.api-step.active .step-label {
  color: #42A5F5;
}

@keyframes stepPulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(66, 165, 245, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(66, 165, 245, 0); }
}

.api-progress-track {
  height: 4px;
  background: #222;
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: 8px;
}

.api-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #1565C0, #4CAF50);
  border-radius: 2px;
  transition: width 0.5s ease;
}

.api-progress-msg {
  font-size: 11px;
  color: #888;
  text-align: center;
  margin: 0;
  min-height: 16px;
}

.field-group-title {
  font-size: 12px;
  font-weight: 600;
  color: #888;
  margin-top: 12px;
  margin-bottom: 4px;
  padding-bottom: 4px;
  border-bottom: 1px solid #222;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field span {
  font-size: 12px;
  font-weight: 500;
  color: #888888;
}

.model-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 400px;
  overflow-y: auto;
  padding: 2px;
}

.model-quick-select {
  display: flex;
  gap: 6px;
  align-items: center;
}

.custom-select {
  flex: 1;
  position: relative;
  cursor: pointer;
  user-select: none;
}

.custom-select-value {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #f0f0f0;
  font-size: 13px;
  min-height: 28px;
}

.custom-select.open .custom-select-value {
  border-color: #4a90d9;
  border-radius: 6px 6px 0 0;
}

.custom-select-arrow {
  font-size: 10px;
  color: #888;
  transition: transform 0.15s;
}

.custom-select.open .custom-select-arrow {
  transform: rotate(180deg);
}

.custom-select-options {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  max-height: 240px;
  overflow-y: auto;
  background: #1a1a1a;
  border: 1px solid #4a90d9;
  border-top: none;
  border-radius: 0 0 6px 6px;
  z-index: 100;
}

.custom-select-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 10px;
  color: #ccc;
  font-size: 12px;
  cursor: pointer;
}

.custom-select-option:hover {
  background: #2a2a2a;
  color: #fff;
}

.custom-select-option.selected {
  background: #333;
  color: #fff;
}

.account-details-section {
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  padding: 0;
  margin: 4px 0;
}

.account-details-summary {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  cursor: pointer;
  font-size: 12px;
  color: #ccc;
  font-weight: 500;
  list-style: none;
}

.account-details-summary::-webkit-details-marker {
  display: none;
}

.account-details-summary::before {
  content: '▶';
  font-size: 9px;
  color: #666;
  transition: transform 0.15s;
}

.account-details-section[open] .account-details-summary::before {
  transform: rotate(90deg);
}

.model-row-wrapper {
  display: flex;
  flex-direction: column;
}

.model-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 10px;
  border: 1px solid #333;
  border-radius: 4px;
  background: #1a1a1a;
  color: #ccc;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;
}

.model-row:hover {
  background: #252525;
  border-color: #444;
}

.model-row.selected {
  background: #1e3a8a;
  border-color: #3b82f6;
  color: #fff;
}

.model-row.no-tool {
  opacity: 0.7;
}

.model-row.broken {
  border-color: #7f1d1d;
  background: #1a0a0a;
  opacity: 0.6;
}

.model-row.broken.selected {
  background: #7f1d1d;
  border-color: #ef4444;
  opacity: 0.8;
}

.model-row-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.model-error-hint {
  font-size: 11px;
  color: #f87171;
  padding: 4px 10px 6px;
  background: #1a0a0a;
  border-left: 2px solid #7f1d1d;
  margin: 0 0 2px;
}

.btn-delete {
  color: #666;
  border-color: #333;
}

.btn-delete:hover {
  color: #ef4444;
  border-color: #ef4444;
  background: #2a0a0a;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

.recommend-section {
  background: #0d1a0d;
  border: 1px solid #1e3a1e;
  border-radius: 6px;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.recommend-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
  font-weight: 600;
  color: #60a5fa;
}

.btn-icon-sm {
  width: 20px;
  height: 20px;
  border: none;
  background: transparent;
  color: #666;
  cursor: pointer;
  font-size: 12px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-icon-sm:hover {
  color: #fff;
}

.btn-sm {
  padding: 2px 8px;
  font-size: 11px;
  background: #1565C0;
  color: #fff;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

.btn-sm:hover {
  background: #1976D2;
}

.btn-sm:disabled {
  background: #333;
  color: #666;
  cursor: not-allowed;
}

.project-item {
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  margin-bottom: 4px;
  border: 1px solid transparent;
  transition: all 0.15s;
}

.project-item:hover {
  background: #1a1a1a;
  border-color: #2a2a2a;
}

.project-item.active {
  background: #0d2137;
  border-color: #1565C0;
}

.conv-item {
  padding: 4px 6px;
  border-radius: 3px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 2px;
}

.conv-item:hover {
  background: #1a1a1a;
}

.copy-select {
  background: #111;
  color: #888;
  border: 1px solid #333;
  border-radius: 3px;
  font-size: 10px;
  padding: 1px 4px;
}

.model-manager-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px;
  background: #111;
  border: 1px solid #2a2a2a;
  border-radius: 6px;
  cursor: pointer;
  margin: 4px 0;
}

.model-manager-toggle:hover {
  background: #1a1a1a;
}

.model-manager-panel {
  padding: 8px;
  background: #0d0d0d;
  border: 1px solid #2a2a2a;
  border-radius: 6px;
  margin-bottom: 8px;
}

.model-search-bar {
  display: flex;
  gap: 4px;
  margin-bottom: 6px;
}

.model-category-title {
  font-size: 11px;
  font-weight: bold;
  color: #42A5F5;
  padding: 4px 8px;
  border-bottom: 1px solid #1a3a5c;
  margin-bottom: 4px;
}

.model-category-title.cloud {
  color: #AB47BC;
  border-bottom-color: #3a1a4c;
}

.pulling-indicator {
  font-size: 11px;
  color: #FF9800;
  padding: 6px 8px;
  background: #1a1500;
  border-radius: 4px;
  margin-top: 4px;
}

.proj-section-title {
  font-size: 11px;
  font-weight: bold;
  color: #888;
  padding: 2px 0;
  border-bottom: 1px solid #222;
  margin-bottom: 4px;
}

.proj-path-row {
  display: flex;
  align-items: center;
  gap: 4px;
  margin: 4px 0;
}

.proj-default-path {
  font-size: 11px;
  color: #888;
  padding: 2px 4px;
  background: #0a0a0a;
  border-radius: 3px;
  word-break: break-all;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.proj-checkbox {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 2px 0;
  user-select: none;
}

.proj-check-box {
  width: 14px;
  height: 14px;
  border: 1px solid #444;
  border-radius: 3px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #111;
  transition: all 0.15s;
}

.proj-check-box.checked {
  background: #1565C0;
  border-color: #1976D2;
}

.recommend-item {
  padding: 6px 8px;
  background: #111;
  border-radius: 4px;
  border: 1px solid #222;
}

.recommend-info {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.recommend-name {
  font-size: 12px;
  color: #e5e5e5;
  font-weight: 500;
}

.recommend-reason {
  font-size: 11px;
  color: #888;
  line-height: 1.4;
}

.tool-badge.broken {
  background: #7f1d1d;
  color: #fca5a5;
}

.model-row-info {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
}

.model-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.model-size {
  font-size: 10px;
  color: #888;
  flex-shrink: 0;
}

.tool-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  font-weight: 600;
  flex-shrink: 0;
}

.tool-badge.ok {
  background: #1e3a8a;
  color: #60a5fa;
}

.tool-badge.no {
  background: #7f1d1d;
  color: #fca5a5;
}

.btn-icon {
  width: 28px;
  height: 28px;
  border: 1px solid #444;
  border-radius: 4px;
  background: #222;
  color: #888;
  cursor: pointer;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  flex-shrink: 0;
  padding: 0;
}

.btn-icon:hover {
  background: #333;
  color: #fff;
  border-color: #666;
}

.btn-icon.active {
  background: #1565C0;
  border-color: #1976D2;
  color: #fff;
}

.toggle-btn {
  width: 40px;
  height: 22px;
  border-radius: 11px;
  border: 1px solid #555;
  background: #333;
  cursor: pointer;
  position: relative;
  transition: all 0.2s;
  padding: 0;
  flex-shrink: 0;
}
.toggle-btn.on {
  background: #1976D2;
  border-color: #42A5F5;
}
.toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #ccc;
  transition: all 0.2s;
}
.toggle-btn.on .toggle-knob {
  left: 20px;
  background: #fff;
}

.model-settings {
  background: #111;
  border: 1px solid #2a2a2a;
  border-top: none;
  border-radius: 0 0 4px 4px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.model-settings-inner {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.settings-title {
  font-size: 12px;
  font-weight: 600;
  color: #aaa;
}

.btn-auto {
  padding: 4px 10px;
  font-size: 11px;
  border: 1px solid #4a90d9;
  border-radius: 4px;
  background: transparent;
  color: #4a90d9;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-auto:hover {
  background: #4a90d9;
  color: #fff;
}

.auto-hint {
  margin: 0;
  font-size: 11px;
  line-height: 1.5;
  color: #888;
  padding: 6px 8px;
  background: #0d0d0d;
  border-radius: 4px;
  border: 1px solid #222;
}

.cloud-model-item {
  padding: 8px 10px;
  border: 1px solid #3b82f6;
  border-radius: 4px;
  background: #1e3a8a;
  color: #fff;
  font-size: 12px;
}

.setting-actions,
.model-loader {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.btn-sm {
  padding: 6px 10px;
  font-size: 11px;
}

.empty-hint {
  text-align: center;
  color: #555;
  font-size: 12px;
  padding: 20px 0;
}

.notice {
  margin: 0;
  font-size: 12px;
  min-height: 16px;
}

.notice.ok {
  color: #60a5fa;
}

.notice.warn {
  color: #f87171;
}

.hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  padding: 6px 8px;
  border-radius: 4px;
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
}

.hint.ok {
  color: #60a5fa;
  border-color: #1e3a8a;
}

.hint.warn {
  color: #fbbf24;
  border-color: #92400e;
}

input,
textarea,
button,
select {
  border: 1px solid #2a2a2a;
  border-radius: 4px;
  background: #0d0d0d;
  color: #e5e5e5;
  padding: 8px 12px;
  font-size: 14px;
  outline: none;
}

input:focus,
select:focus {
  border-color: #3b82f6;
}

button {
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s;
}

.btn-blue {
  background: #3b82f6;
  border-color: #3b82f6;
  color: #ffffff;
}

.btn-blue:hover {
  background: #2563eb;
  border-color: #2563eb;
}

.btn-red {
  background: #ef4444;
  border-color: #ef4444;
  color: #ffffff;
}

.btn-red:hover {
  background: #dc2626;
  border-color: #dc2626;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.select-input {
  width: 100%;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #f0f0f0;
  font-size: 13px;
  outline: none;
}

.select-input option {
  background: #1a1a1a;
  color: #f0f0f0;
  padding: 6px;
}

.select-input:focus {
  border-color: #4a90d9;
}

.range-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.short-input {
  width: 100px;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #f0f0f0;
  font-size: 13px;
  outline: none;
}

.short-input:focus {
  border-color: #4a90d9;
}

.range-hint {
  color: #666;
  font-size: 12px;
}

.textarea-input {
  width: 100%;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #f0f0f0;
  font-size: 13px;
  outline: none;
  resize: vertical;
  font-family: inherit;
}

.textarea-input:focus {
  border-color: #4a90d9;
}

@media (max-width: 1024px) {
  .flowbar {
    grid-template-columns: 1fr;
  }

  .workbench {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 760px) {
  .page {
    padding: 16px;
  }

  .toolbar,
  .composer-foot {
    flex-direction: column;
    align-items: flex-start;
  }
}

.page-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: #0d0d0d;
}

.page-view-inner {
  flex: 1;
  overflow-y: auto;
  padding: 0;
}

.page-view-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px;
  border-bottom: 1px solid #222;
  background: #111;
}

.page-view-header h2 {
  font-size: 16px;
  font-weight: 600;
  color: #eee;
  margin: 0;
}

.settings-page-inline {
  display: flex;
  height: 100%;
}

.project-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 10px;
  padding: 16px 24px;
}

.project-card {
  padding: 12px;
  border: 1px solid #222;
  border-radius: 8px;
  background: #141414;
  cursor: pointer;
  transition: all 0.15s;
}

.project-card:hover {
  border-color: #4af;
}

.project-card.active {
  border-color: #4af;
  background: rgba(68, 170, 255, 0.05);
}

.project-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.project-card-name {
  font-size: 13px;
  font-weight: 500;
  color: #ddd;
}

.project-active-badge {
  font-size: 10px;
  color: #4af;
  background: rgba(68, 170, 255, 0.1);
  padding: 1px 6px;
  border-radius: 3px;
}

.project-card-path {
  font-size: 10px;
  color: #666;
  margin-bottom: 6px;
  word-break: break-all;
}

.project-card-actions {
  display: flex;
  gap: 4px;
  align-items: center;
}

.version-content {
  padding: 16px 24px;
}

.version-item {
  display: flex;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid #1a1a1a;
}

.version-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #4af;
  flex-shrink: 0;
  margin-top: 4px;
}

.version-info {
  flex: 1;
}

.version-name {
  font-size: 13px;
  font-weight: 500;
  color: #ddd;
}

.version-date {
  font-size: 10px;
  color: #666;
}

.version-desc {
  font-size: 11px;
  color: #888;
  margin-top: 2px;
}

.empty-hint {
  font-size: 12px;
  color: #555;
  text-align: center;
  padding: 24px;
  grid-column: 1 / -1;
}

.settings-sidebar {
  width: 200px;
  flex-shrink: 0;
  background: #111;
  border-right: 1px solid #222;
  display: flex;
  flex-direction: column;
}

.settings-nav {
  flex: 1;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.settings-nav-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border: none;
  background: none;
  color: #888;
  font-size: 13px;
  cursor: pointer;
  border-radius: 6px;
  transition: all 0.15s;
  text-align: left;
}

.settings-nav-item:hover {
  background: #1a1a1a;
  color: #ccc;
}

.settings-nav-item.active {
  background: rgba(68, 170, 255, 0.1);
  color: #4af;
}

.settings-nav-item .nav-icon {
  font-size: 16px;
  width: 20px;
  text-align: center;
}

.settings-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}

.settings-section {
  max-width: 700px;
}

.section-title {
  font-size: 18px;
  font-weight: 600;
  color: #eee;
  margin: 0 0 16px 0;
  padding-bottom: 8px;
  border-bottom: 1px solid #222;
}

.settings-card {
  background: #141414;
  border: 1px solid #222;
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 12px;
}

.card-title {
  font-size: 13px;
  font-weight: 600;
  color: #ccc;
  margin-bottom: 12px;
  padding-bottom: 6px;
  border-bottom: 1px solid #1a1a1a;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid #1a1a1a;
}

.setting-row:last-child {
  border-bottom: none;
}

.setting-info {
  flex: 1;
  min-width: 0;
}

.setting-name {
  font-size: 12px;
  font-weight: 500;
  color: #ddd;
}

.setting-desc {
  font-size: 10px;
  color: #666;
  margin-top: 2px;
}

.setting-input {
  font-size: 12px;
  padding: 6px 10px;
  border-radius: 4px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #ddd;
  outline: none;
  transition: border-color 0.15s;
}

.setting-input:focus {
  border-color: #4af;
}

.setting-select {
  font-size: 12px;
  padding: 6px 10px;
  border-radius: 4px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #ddd;
  outline: none;
  cursor: pointer;
}

.setting-select option {
  background: #1a1a1a;
  color: #ddd;
  padding: 4px;
}

.setting-textarea {
  width: 100%;
  font-size: 12px;
  padding: 8px;
  border-radius: 6px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #ddd;
  resize: vertical;
  font-family: monospace;
  outline: none;
  transition: border-color 0.15s;
}

.setting-textarea:focus {
  border-color: #4af;
}

.mode-switch-large {
  display: flex;
  gap: 8px;
}

.mode-card {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 16px 12px;
  border: 1px solid #333;
  border-radius: 8px;
  background: #1a1a1a;
  color: #888;
  cursor: pointer;
  transition: all 0.2s;
}

.mode-card:hover {
  border-color: #555;
  color: #ccc;
}

.mode-card.active {
  border-color: #4af;
  background: rgba(68, 170, 255, 0.08);
  color: #4af;
}

.mode-card .mode-icon {
  font-size: 24px;
}

.mode-card .mode-name {
  font-size: 14px;
  font-weight: 600;
}

.mode-card .mode-desc {
  font-size: 10px;
  opacity: 0.7;
}

.role-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
}

.role-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 10px 8px;
  border: 1px solid #333;
  border-radius: 6px;
  background: #1a1a1a;
  color: #888;
  cursor: pointer;
  transition: all 0.15s;
}

.role-card:hover {
  border-color: #555;
  color: #ccc;
}

.role-card.active {
  border-color: #4af;
  background: rgba(68, 170, 255, 0.08);
  color: #4af;
}

.role-card .role-icon {
  font-size: 20px;
}

.role-card .role-name {
  font-size: 11px;
  font-weight: 500;
}

.role-card .role-desc {
  font-size: 9px;
  opacity: 0.7;
}

.shortcut-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.shortcut-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0;
  font-size: 11px;
  color: #aaa;
}

.shortcut-row kbd {
  background: #222;
  border: 1px solid #333;
  border-radius: 4px;
  padding: 2px 8px;
  font-size: 10px;
  color: #ccc;
  font-family: monospace;
}

.template-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 6px;
}

.template-card {
  padding: 10px;
  border: 1px solid #222;
  border-radius: 6px;
  background: #111;
  cursor: pointer;
  transition: all 0.15s;
}

.template-card:hover {
  border-color: #4af;
  background: rgba(68, 170, 255, 0.05);
}

.workflow-card {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 14px;
  border: 1px solid #333;
  border-radius: 6px;
  background: #1a1a1a;
  color: #888;
  cursor: pointer;
  transition: all 0.15s;
}

.workflow-card:hover {
  border-color: #555;
  color: #ccc;
}

.workflow-card.active {
  border-color: #4af;
  background: rgba(68, 170, 255, 0.08);
  color: #4af;
}</style>
