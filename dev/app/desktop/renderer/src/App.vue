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
  ZHIPU_API_KEY?: string;
  ZHIPU_MODEL?: string;
  ZHIPU_BASE_URL?: string;
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
const noticeType = ref<"ok" | "warn" | "info">("ok");
const messages = ref<ChatMessage[]>([]);
const currentAssistantId = ref("");

// runMode: 当前活跃的模型来源（决定 sendMessage 用哪个 provider）
const runMode = ref<"cloud" | "ollama" | "api">("ollama");
// apiSource: API 模式下的子来源（qwen / zhipu）
const apiSource = ref<"qwen" | "zhipu">("qwen");
// 新增：用户自定义快捷模型列表（在发送按钮左侧展示）
const quickModels = ref<{ id: string; name: string; provider: "cloud" | "ollama" | "api"; apiSource?: "qwen" | "zhipu"; modelId: string }[]>([]);
const activeQuickModel = ref<string>("");
const showQuickModelDropdown = ref(false);
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
const zhipuApiKey = ref("");
const zhipuModel = ref("glm-4.7-flash");
const zhipuBaseUrl = ref("https://open.bigmodel.cn/api/paas/v4");
const zhipuModels = ref<ModelInfo[]>([]);
const zhipuApiChecked = ref(false);
const zhipuApiChecking = ref(false);
const zhipuApiHost = ref("127.0.0.1");
const zhipuApiPort = ref(7780);
const zhipuStepProgress = ref(0);
const zhipuStepBusy = ref(false);
const zhipuStepMessage = ref("点击「一键启动」自动配置智谱 API 服务");
const zhipuAccounts = ref<any[]>([]);
const zhipuAccountCount = ref(-1);
const zhipuValidCount = ref(0);
const zhipuLocalKeys = ref<any[]>([]);
const zhipuNewKey = ref("");
const zhipuNewLabel = ref("");
const zhipuRegisterBusy = ref(false);
const zhipuRegisterLogs = ref<string[]>([]);
const zhipuLoginEmail = ref("");
const zhipuLoginPassword = ref("");
const zhipuLoginBusy = ref(false);
const zhipuLoginError = ref("");
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
const allServicesBusy = ref(false);

async function startAllServices() {
  if (allServicesBusy.value) return;
  allServicesBusy.value = true;
  try {
    const r = await callBackend("startAllApiServices");
    if (r && r.ok) {
      showNotice("所有 API 服务已启动", "ok");
      apiStepProgress.value = apiSteps.length;
      zhipuStepProgress.value = zhipuSteps.length;
      apiServiceRunning.value = true;
    } else {
      const errors = r?.services ? Object.entries(r.services).filter(([, v]: [string, any]) => !v.ok).map(([k, v]: [string, any]) => `${k}: ${v.error}`).join("; ") : "启动失败";
      showNotice("部分服务启动失败: " + errors, "warn");
    }
  } catch (e) {
    showNotice("启动异常: " + String(e), "warn");
  }
  allServicesBusy.value = false;
}

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
  ZHIPU_API_KEY: "",
  ZHIPU_MODEL: "",
  ZHIPU_BASE_URL: "",
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
  if (runMode.value === "ollama") return getModelConfig(ollamaModel.value || "__default__");
  if (runMode.value === "api") return getModelConfig(apiSource.value === "zhipu" ? zhipuModel.value : apiModel.value || "__default__");
  return getModelConfig(settings.ANTHROPIC_MODEL || "__default__");
}

const activeModelId = computed(() => {
  if (runMode.value === "ollama") return ollamaModel.value;
  if (runMode.value === "api") return apiSource.value === "zhipu" ? zhipuModel.value : apiModel.value;
  return "openrouter/auto";
});

const selectedOllamaModelToolSupport = computed<boolean | undefined>(() => {
  if (!ollamaModel.value) return undefined;
  const found = cloudModels.value.find((m) => m.id === ollamaModel.value);
  return found?.toolSupport;
});

// 自动检测激活的模型，生成快捷模型列表
const autoQuickModels = computed(() => {
  const models: { id: string; name: string; provider: "cloud" | "ollama" | "api"; apiSource?: "qwen" | "zhipu"; modelId: string; auto: true }[] = [];

  // 千问 API：服务已启动且有选择的模型
  if (apiServiceRunning.value && apiModel.value) {
    models.push({
      id: "auto-qwen",
      name: apiModel.value,
      provider: "api",
      apiSource: "qwen",
      modelId: apiModel.value,
      auto: true,
    });
  }

  // 智谱 API：服务已启动且有选择的模型
  if (zhipuStepProgress.value >= zhipuSteps.length && zhipuModel.value) {
    models.push({
      id: "auto-zhipu",
      name: zhipuModel.value,
      provider: "api",
      apiSource: "zhipu",
      modelId: zhipuModel.value,
      auto: true,
    });
  }

  // Ollama：本地模型
  if (ollamaModel.value) {
    models.push({
      id: "auto-ollama",
      name: ollamaModel.value,
      provider: "ollama",
      modelId: ollamaModel.value,
      auto: true,
    });
  }

  return models;
});

// 判断快捷模型是否可用（服务已启动）
function isQuickModelAvailable(m: typeof quickModels.value[0] | typeof autoQuickModels.value[0]): boolean {
  if (m.provider === "cloud") return true;
  if (m.provider === "ollama") return true;
  if (m.provider === "api") {
    if (m.apiSource === "qwen") return apiServiceRunning.value;
    if (m.apiSource === "zhipu") return zhipuStepProgress.value >= zhipuSteps.length;
  }
  return false;
}

// 合并自动模型和用户自定义模型（去重）
const allQuickModels = computed(() => {
  const autoIds = new Set(autoQuickModels.value.map(m => m.modelId + ":" + (m.apiSource || m.provider)));
  const userModels = quickModels.value.filter(m => {
    const key = m.modelId + ":" + (m.apiSource || m.provider);
    return !autoIds.has(key);
  });
  return [...autoQuickModels.value, ...userModels];
});

const availableQuickModels = computed(() => allQuickModels.value.filter(m => isQuickModelAvailable(m)));
const unavailableQuickModels = computed(() => allQuickModels.value.filter(m => !isQuickModelAvailable(m)));

watch(runMode, (newMode) => {
  if (newMode === "ollama") {
    cloudModels.value = [];
    loadCloudModels("ollama");
  } else if (newMode === "api") {
    if (apiSource.value === "zhipu") {
      loadZhipuModels();
    } else {
      checkApiServiceStatus();
    }
  }
});

watch(apiSource, (source) => {
  if (runMode.value === "api") {
    if (source === "zhipu") {
      loadZhipuModels();
    } else {
      checkApiServiceStatus();
    }
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
      // 如果服务在运行，恢复完整状态（获取key + 加载模型）
      if (apiStepProgress.value < apiSteps.length) {
        apiStepProgress.value = apiSteps.length;
        apiStepMessage.value = "✓ 服务已就绪";
        // 自动获取key和加载模型
        await checkQwenAccounts();
        if (!apiKey.value) {
          try {
            const keyResult = await callBackend("fetchApiKey", JSON.stringify({ baseUrl: apiBaseUrl.value.trim(), adminKey: "admin" }));
            if (keyResult && keyResult.ok && keyResult.key) {
              apiKey.value = keyResult.key;
            }
          } catch {}
        }
        if (apiModels.value.length === 0) {
          try {
            const payload = { source: "api", baseUrl: apiBaseUrl.value, apiKey: apiKey.value };
            const modelResult = await callBackend("listModels", JSON.stringify(payload));
            if (modelResult && modelResult.ok) {
              apiModels.value = modelResult.models || [];
              cloudModels.value = modelResult.models || [];
              if (apiModels.value.length > 0 && !apiModel.value) {
                apiModel.value = apiModels.value[0].id;
              }
            }
          } catch {}
        }
      }
    } else {
      apiServiceRunning.value = false;
      apiStepProgress.value = 0;
      apiStepMessage.value = "点击「启动服务」启动千问 API";
    }
  } catch {
    apiServiceRunning.value = false;
    apiStepMessage.value = "点击「启动服务」启动千问 API";
  }
}

async function restoreServiceStatus() {
  // 恢复千问服务状态
  if (apiServiceRunning.value) {
    try {
      const result = await callBackend("checkApiService", JSON.stringify({ baseUrl: apiBaseUrl.value.trim() }));
      if (result && result.running && result.serviceType === "qwen") {
        apiStepProgress.value = apiSteps.length;
        apiStepMessage.value = "✓ 服务已就绪";
        await checkQwenAccounts();
      } else {
        apiServiceRunning.value = false;
        apiStepProgress.value = 0;
        apiStepMessage.value = "点击「启动服务」启动千问 API";
      }
    } catch {
      apiServiceRunning.value = false;
      apiStepProgress.value = 0;
      apiStepMessage.value = "点击「启动服务」启动千问 API";
    }
  }
  // 恢复智谱服务状态
  if (zhipuStepProgress.value >= zhipuSteps.length) {
    try {
      const baseUrl = `http://${zhipuApiHost.value || "127.0.0.1"}:${zhipuApiPort.value || 7780}`;
      const result = await callBackend("checkApiService", JSON.stringify({ baseUrl }));
      if (result && result.running && result.serviceType === "zhipu") {
        zhipuStepProgress.value = zhipuSteps.length;
        zhipuStepMessage.value = "✓ 服务已就绪";
        await checkZhipuAccounts();
      } else {
        zhipuStepProgress.value = 0;
        zhipuStepMessage.value = "点击「启动服务」启动智谱 API";
      }
    } catch {
      zhipuStepProgress.value = 0;
      zhipuStepMessage.value = "点击「启动服务」启动智谱 API";
    }
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
const settingsTab = ref<"general" | "model" | "account" | "memory" | "project" | "advanced" | "plugins" | "voice" | "offline">("general");
const newQuickModelName = ref("");
const newQuickModelProvider = ref<"cloud" | "ollama" | "api">("api");
const newQuickModelApiSource = ref<"qwen" | "zhipu">("qwen");
const newQuickModelId = ref("");
const APP_VERSION = "2026.05.11";
const activeNav = ref<"chat" | "project" | "version" | "settings">("chat");
const versionHistory = ref<any[]>([]);
const showTerminal = ref(false);
const terminalInput = ref("");
const terminalHistory = ref<{cmd: string; output: string; ts: string}[]>([]);
const terminalCwd = ref("");
const terminalExpanded = ref(false);
const terminalDebug = ref(false);
const terminalOutputRef = ref<HTMLElement | null>(null);
const gitStatus = ref<any>(null);
const gitLog = ref<any[]>([]);
const gitCommitMsg = ref("");
const showGitPanel = ref(false);
const snippetList = ref<any[]>([]);
const snippetSearch = ref("");
const newSnippetName = ref("");
const newSnippetLang = ref("javascript");
const newSnippetCode = ref("");
const showSnippetPanel = ref(false);
const currentTheme = ref<"dark" | "light" | "blue" | "green">("dark");
const searchQuery = ref("");
const searchResults = ref<{msgId: string; text: string}[]>([]);
const searchIndex = ref(0);
const fileTree = ref<any[]>([]);
const showFileTree = ref(false);
const pluginList = ref<any[]>([]);
const showPluginPanel = ref(false);
const newPluginId = ref("");
const newPluginName = ref("");
const newPluginDesc = ref("");
const newPluginCode = ref("");
const isVoiceActive = ref(false);
const offlineModels = ref<any[]>([]);
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
  terminal: { name: "/terminal", desc: "打开终端", action: "terminal" },
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

function showNotice(text: string, type: "ok" | "warn" | "info" = "ok") {
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
  settings.ZHIPU_API_KEY = data.ZHIPU_API_KEY ?? "";
  settings.ZHIPU_MODEL = data.ZHIPU_MODEL ?? "";
  settings.ZHIPU_BASE_URL = data.ZHIPU_BASE_URL ?? "";
  settings.AI_LANGUAGE = data.AI_LANGUAGE ?? "zh";
  settings.AI_TEMPERATURE = data.AI_TEMPERATURE ?? "";
  settings.AI_MAX_TOKENS = data.AI_MAX_TOKENS ?? "";
  settings.SYSTEM_PROMPT = data.SYSTEM_PROMPT ?? "";

  runMode.value = settings.MODEL_PROVIDER === "ollama" ? "ollama" : settings.MODEL_PROVIDER === "api" ? "api" : "cloud";
  if (settings.MODEL_PROVIDER === "api" && settings.ZHIPU_API_KEY && settings.ZHIPU_MODEL) {
    apiSource.value = "zhipu";
  } else {
    apiSource.value = "qwen";
  }
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

  zhipuApiKey.value = settings.ZHIPU_API_KEY || "";
  zhipuModel.value = settings.ZHIPU_MODEL || "glm-5.1";
  zhipuBaseUrl.value = settings.ZHIPU_BASE_URL || "https://open.bigmodel.cn/api/paas/v4";

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

function selectQuickModel(modelId: string) {
  const model = allQuickModels.value.find((m) => m.id === modelId);
  if (!model) return;
  activeQuickModel.value = modelId;
  showQuickModelDropdown.value = false;
  // 切换对应的运行模式和模型
  runMode.value = model.provider;
  if (model.provider === "api" && model.apiSource) {
    apiSource.value = model.apiSource;
    if (model.apiSource === "qwen") {
      apiModel.value = model.modelId;
    } else if (model.apiSource === "zhipu") {
      zhipuModel.value = model.modelId;
    }
  } else if (model.provider === "ollama") {
    ollamaModel.value = model.modelId;
  } else if (model.provider === "cloud") {
    settings.ANTHROPIC_MODEL = model.modelId;
  }
  showNotice(`已切换至 ${model.name}`, "ok");
}

function addQuickModel() {
  const name = newQuickModelName.value.trim();
  const modelId = newQuickModelId.value.trim();
  if (!name) {
    showNotice("请输入显示名称", "warn");
    return;
  }
  const id = "quick-" + Date.now();
  quickModels.value.push({
    id,
    name,
    provider: newQuickModelProvider.value,
    apiSource: newQuickModelProvider.value === "api" ? newQuickModelApiSource.value : undefined,
    modelId,
  });
  newQuickModelName.value = "";
  newQuickModelId.value = "";
  showNotice("快捷模型已添加", "ok");
}

async function sendMessage() {
  const text = inputText.value.trim();
  if (!text) return;
  inputText.value = "";

  await saveSettingsQuiet();

  if (isBusy.value) {
    pendingQueue.value.push(text);
    addMessage("user", text);
    showNotice(`已加入排队（第${pendingQueue.value.length}条）`, "warn");
    return;
  }

  await doSend(text, true);
}

async function doSend(text: string, addUserMsg: boolean = false) {
  const currentModel = runMode.value === "ollama" ? ollamaModel.value.trim() : runMode.value === "api" ? (apiSource.value === "zhipu" ? zhipuModel.value : apiModel.value).trim() : (settings.ANTHROPIC_MODEL || "").trim();
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
    try { callBackend("showDesktopNotification", "云集智能编程工作站", "AI 回复完成"); } catch {}
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
    if (activeProject.value) {
      await callBackend("updateProject", activeProject.value.id, "", state.workspacePath);
      await loadProjects();
    } else {
      const dirName = state.workspacePath.replace(/\\/g, "/").split("/").pop() || "项目";
      const proj = await callBackend("createProject", dirName, state.workspacePath);
      if (proj) {
        activeProject.value = proj;
        await loadProjects();
      }
    }
    showNotice("工作区已切换。", "ok");
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
      workspacePath.value = active.workspace_path || "";
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
      workspacePath.value = proj.workspace_path || "";
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
      workspacePath.value = proj.workspace_path || "";
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
  editProjectPath.value = proj.workspace_path || "";
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
        workspacePath.value = proj.workspace_path || "";
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

async function loadProjectConversations() {
  if (!activeProject.value) return;
  try {
    const convs = await callBackend("listConversations", activeProject.value.id);
    projectConversations.value = convs || [];
  } catch (e) {
    console.warn("loadProjectConversations failed:", e);
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
  const activeConfig = getActiveModelConfig();
  activeConfig.systemPrompt = preset.prompt;
  activeConfig.temperature = preset.temp;
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
    const content = await callBackend("getClaudeMd", activeProject.value.id);
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
    await callBackend("saveClaudeMd", activeProject.value.id, memoryContent.value);
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
    const result = await callBackend("listMemories", activeProject.value.id);
    smartMemories.value = Array.isArray(result) ? result : [];
  } catch (e) {
    console.warn("loadSmartMemories failed:", e);
  }
}

async function searchSmartMemories() {
  if (!activeProject.value) return;
  try {
    const result = await callBackend("searchMemories", activeProject.value.id, memorySearchQuery.value);
    smartMemories.value = Array.isArray(result) ? result : [];
  } catch (e) {
    console.warn("searchSmartMemories failed:", e);
  }
}

async function addSmartMemory() {
  if (!activeProject.value || !newMemoryTitle.value.trim() || !newMemoryContent.value.trim()) return;
  try {
    const filename = newMemoryTitle.value.trim().replace(/\s+/g, "-").toLowerCase();
    await callBackend("saveMemory", activeProject.value.id, filename, newMemoryContent.value.trim(), newMemoryType.value);
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
    await callBackend("deleteMemory", activeProject.value.id, filename);
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
    const result = await callBackend("autoExtractMemories", activeProject.value.id, JSON.stringify(msgs));
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
    const result = await callBackend("createProjectFromTemplate", activeProject.value.id, template.id);
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
  const projPath = activeProject.value.workspace_path || workspacePath.value;
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
      activeNav.value = "settings";
      break;
    case "terminal":
      showTerminal.value = !showTerminal.value;
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
    if (apiSource.value === "zhipu") {
      if (!zhipuModel.value.trim()) {
        showNotice("请选择模型名称", "warn");
        return;
      }
      const effectiveApiKey = zhipuApiKey.value.trim() || (zhipuLocalKeys.value.length > 0 ? zhipuLocalKeys.value[0].key : "");
      if (!effectiveApiKey) {
        showNotice("请先添加智谱 API Key", "warn");
        return;
      }
      const proxyBaseUrl = `http://${zhipuApiHost.value || "127.0.0.1"}:${zhipuApiPort.value || 7780}`;
      const payload: Record<string, string> = {
        MODEL_PROVIDER: "api",
        API_BASE_URL: proxyBaseUrl,
        API_MODEL: zhipuModel.value.trim(),
        API_KEY: zhipuApiKey.value.trim() || effectiveApiKey,
        ZHIPU_API_KEY: effectiveApiKey,
        ZHIPU_MODEL: zhipuModel.value.trim(),
        ZHIPU_BASE_URL: proxyBaseUrl,
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
      showNotice("配置已保存。智谱 API 已启用。", "ok");
    } else {
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
      showNotice("配置已保存。Qwen API 模式已启用。", "ok");
    }
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

async function saveSettingsQuiet() {
  const activeConfig = getActiveModelConfig();
  let payload: Record<string, string> | null = null;

  if (runMode.value === "ollama") {
    const localBase = ollamaBaseUrl.value.trim() || "http://127.0.0.1:11434";
    payload = {
      MODEL_PROVIDER: "ollama",
      OLLAMA_BASE_URL: localBase,
      OLLAMA_MODEL: ollamaModel.value.trim() || "qwen3:8b",
      API_TIMEOUT_MS: settings.API_TIMEOUT_MS || "3000000",
      DISABLE_TELEMETRY: "1",
      CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
      AI_LANGUAGE: activeConfig.language || "zh",
      AI_TEMPERATURE: activeConfig.temperature || "",
      AI_MAX_TOKENS: activeConfig.maxTokens || "",
      SYSTEM_PROMPT: activeConfig.systemPrompt || "",
    };
  } else if (runMode.value === "api") {
    if (apiSource.value === "zhipu") {
      const effectiveApiKey = zhipuApiKey.value.trim() || (zhipuLocalKeys.value.length > 0 ? zhipuLocalKeys.value[0].key : "");
      const proxyBaseUrl = `http://${zhipuApiHost.value || "127.0.0.1"}:${zhipuApiPort.value || 7780}`;
      payload = {
        MODEL_PROVIDER: "api",
        API_BASE_URL: proxyBaseUrl,
        API_MODEL: zhipuModel.value.trim() || "glm-4.7-flash",
        API_KEY: zhipuApiKey.value.trim() || effectiveApiKey,
        ZHIPU_API_KEY: effectiveApiKey,
        ZHIPU_MODEL: zhipuModel.value.trim() || "glm-4.7-flash",
        ZHIPU_BASE_URL: proxyBaseUrl,
        API_TIMEOUT_MS: settings.API_TIMEOUT_MS || "3000000",
        DISABLE_TELEMETRY: "1",
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
        AI_LANGUAGE: activeConfig.language || "zh",
        AI_TEMPERATURE: activeConfig.temperature || "",
        AI_MAX_TOKENS: activeConfig.maxTokens || "",
        SYSTEM_PROMPT: activeConfig.systemPrompt || "",
      };
    } else {
      const qwenBase = apiBaseUrl.value.trim() || `http://${apiHost.value || "127.0.0.1"}:${apiPort.value || "7777"}`;
      payload = {
        MODEL_PROVIDER: "api",
        API_BASE_URL: qwenBase,
        API_MODEL: apiModel.value.trim() || "qwen3.6-plus",
        API_KEY: apiKey.value.trim(),
        API_TIMEOUT_MS: settings.API_TIMEOUT_MS || "3000000",
        DISABLE_TELEMETRY: "1",
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
        AI_LANGUAGE: activeConfig.language || "zh",
        AI_TEMPERATURE: activeConfig.temperature || "",
        AI_MAX_TOKENS: activeConfig.maxTokens || "",
        SYSTEM_PROMPT: activeConfig.systemPrompt || "",
      };
    }
  } else {
    payload = {
      MODEL_PROVIDER: "anthropic",
      ANTHROPIC_BASE_URL: cloudBaseUrlByProvider("openrouter"),
      ANTHROPIC_API_KEY: apiKey.value.trim(),
      ANTHROPIC_AUTH_TOKEN: apiKey.value.trim(),
      ANTHROPIC_MODEL: "openrouter/auto",
      API_TIMEOUT_MS: settings.API_TIMEOUT_MS || "3000000",
      DISABLE_TELEMETRY: "1",
      CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
      AI_LANGUAGE: activeConfig.language || "zh",
      AI_TEMPERATURE: activeConfig.temperature || "",
      AI_MAX_TOKENS: activeConfig.maxTokens || "",
      SYSTEM_PROMPT: activeConfig.systemPrompt || "",
    };
  }

  if (payload) {
    try {
      const saved = await callBackend("saveSettings", JSON.stringify(payload));
      applySettings(saved);
    } catch {}
  }
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
        callBackend("openExternalUrl", result.adminUrl);
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

async function loadZhipuModels() {
  loadingModels.value = true;
  try {
    const payload = { source: "zhipu", apiKey: zhipuApiKey.value, baseUrl: zhipuBaseUrl.value };
    const result = await callBackend("listModels", JSON.stringify(payload));
    if (result && result.loading) {
      return;
    }
    if (!result || !result.ok) {
      showNotice(result?.error || "智谱模型列表加载失败", "warn");
      loadingModels.value = false;
      return;
    }
    zhipuModels.value = result.models || [];
    loadingModels.value = false;
    showNotice(`已加载 ${zhipuModels.value.length} 个智谱模型`, "ok");
  } catch (e) {
    console.error("[loadZhipuModels] error:", e);
    showNotice("智谱模型列表加载异常", "warn");
    loadingModels.value = false;
  }
}

async function checkZhipuApiConnect() {
  if (!zhipuApiKey.value.trim()) {
    showNotice("请先填写智谱 API Key", "warn");
    return;
  }
  zhipuApiChecking.value = true;
  try {
    const result = await callBackend("checkZhipuApi", JSON.stringify({
      apiKey: zhipuApiKey.value.trim(),
      baseUrl: zhipuBaseUrl.value.trim(),
    }));
    zhipuApiChecking.value = false;
    if (result && result.ok) {
      zhipuApiChecked.value = true;
      showNotice("智谱 API 连接成功！", "ok");
      await loadZhipuModels();
    } else {
      zhipuApiChecked.value = false;
      showNotice(result?.error || "智谱 API 连接失败", "warn");
    }
  } catch (e) {
    zhipuApiChecking.value = false;
    zhipuApiChecked.value = false;
    showNotice("智谱 API 连接异常", "warn");
  }
}

const zhipuSteps = [
  { label: "检测服务", action: "check" },
  { label: "启动服务", action: "start" },
  { label: "获取 Key", action: "key" },
  { label: "加载模型", action: "models" },
];
const zhipuProgressPercent = computed(() => {
  if (zhipuStepProgress.value >= zhipuSteps.length) return 100;
  return Math.round((zhipuStepProgress.value / zhipuSteps.length) * 100);
});

async function zhipuStepAutoRun() {
  if (zhipuStepBusy.value) return;
  zhipuStepBusy.value = true;

  const baseUrl = `http://${zhipuApiHost.value || "127.0.0.1"}:${zhipuApiPort.value || 7780}`;

  try {
    while (zhipuStepProgress.value < zhipuSteps.length) {
      const step = zhipuSteps[zhipuStepProgress.value];
      zhipuStepMessage.value = `${step.label}中...`;

      if (step.action === "check") {
        const r = await callBackend("checkApiService", JSON.stringify({ baseUrl }));
        if (r && r.running && r.serviceType === "zhipu") { zhipuStepProgress.value = zhipuSteps.length; break; }
        if (r && r.running && r.serviceType !== "zhipu") {
          zhipuStepMessage.value = "端口被其他服务占用，正在切换...";
          await callBackend("stopZhipu2Api", JSON.stringify({ baseUrl }));
          await new Promise(ok => setTimeout(ok, 1000));
        }
        zhipuStepProgress.value = 1;
      } else if (step.action === "start") {
        const r = await callBackend("startZhipu2Api", JSON.stringify({ port: zhipuApiPort.value || 7780, adminKey: "admin" }));
        if (!r || !r.ok) { zhipuStepMessage.value = r?.error || "启动失败"; zhipuStepBusy.value = false; return; }
        for (let i = 0; i < 15; i++) {
          await new Promise(ok => setTimeout(ok, 1000));
          const chk = await callBackend("checkApiService", JSON.stringify({ baseUrl }));
          if (chk && chk.running && chk.serviceType === "zhipu") break;
        }
        zhipuStepProgress.value = 2;
      } else if (step.action === "key") {
        let r = await callBackend("fetchZhipuApiKey", JSON.stringify({ baseUrl, adminKey: "admin" }));
        if (r && r.keys && r.keys.length > 0) {
          zhipuApiKey.value = r.keys[r.keys.length - 1].key;
        } else {
          r = await callBackend("createZhipuApiKey", JSON.stringify({ baseUrl, adminKey: "admin" }));
          if (r && r.key) { zhipuApiKey.value = r.key; }
          else { zhipuStepMessage.value = "获取 API Key 失败"; zhipuStepBusy.value = false; return; }
        }
        zhipuStepProgress.value = 3;
      } else if (step.action === "models") {
        const r = await callBackend("listModels", JSON.stringify({ source: "zhipu", baseUrl, apiKey: zhipuApiKey.value }));
        if (r && r.ok) { zhipuModels.value = r.models || []; }
        zhipuStepProgress.value = 4;
      }
    }
    zhipuStepMessage.value = "智谱 API 服务已就绪";
    await checkZhipuAccounts();
  } catch (e) {
    zhipuStepMessage.value = "启动异常: " + String(e);
  }
  zhipuStepBusy.value = false;
}

async function stopZhipuService() {
  const baseUrl = `http://${zhipuApiHost.value || "127.0.0.1"}:${zhipuApiPort.value || 7780}`;
  await callBackend("stopZhipu2Api", JSON.stringify({ baseUrl }));
  zhipuStepProgress.value = 0;
  zhipuStepMessage.value = "智谱 API 服务已停止";
}

async function addZhipuAccount() {
  if (!zhipuNewKey.value.trim()) return;
  const key = zhipuNewKey.value.trim();
  const label = zhipuNewLabel.value.trim() || key.slice(0, 8) + "...";

  if (zhipuLocalKeys.value.some(k => k.key === key)) {
    showNotice("该 Key 已存在", "warn");
    return;
  }

  zhipuLocalKeys.value.push({ key, label, valid: true });
  zhipuNewKey.value = "";
  zhipuNewLabel.value = "";
  saveZhipuLocalKeys();

  if (!zhipuApiKey.value) {
    zhipuApiKey.value = key;
  }

  const baseUrl = `http://${zhipuApiHost.value || "127.0.0.1"}:${zhipuApiPort.value || 7780}`;
  const checkR = await callBackend("checkApiService", JSON.stringify({ baseUrl }));
  if (checkR && checkR.running) {
    await callBackend("addZhipuAccount", JSON.stringify({ baseUrl, apiKey: key, label, adminKey: "admin" }));
  }

  showNotice("API Key 已添加", "ok");
  refreshZhipuAccountDisplay();
}

async function deleteZhipuAccount(apiKey: string) {
  zhipuLocalKeys.value = zhipuLocalKeys.value.filter(k => k.key !== apiKey);
  saveZhipuLocalKeys();

  if (zhipuApiKey.value === apiKey) {
    zhipuApiKey.value = zhipuLocalKeys.value.length > 0 ? zhipuLocalKeys.value[0].key : "";
  }

  const baseUrl = `http://${zhipuApiHost.value || "127.0.0.1"}:${zhipuApiPort.value || 7780}`;
  const checkR = await callBackend("checkApiService", JSON.stringify({ baseUrl }));
  if (checkR && checkR.running) {
    await callBackend("deleteZhipuAccount", JSON.stringify({ baseUrl, apiKey, adminKey: "admin" }));
  }

  showNotice("已删除", "ok");
  refreshZhipuAccountDisplay();
}

async function checkZhipuAccounts() {
  const baseUrl = `http://${zhipuApiHost.value || "127.0.0.1"}:${zhipuApiPort.value || 7780}`;
  const checkR = await callBackend("checkApiService", JSON.stringify({ baseUrl }));
  if (checkR && checkR.running) {
    try {
      const r = await callBackend("listZhipuAccounts", JSON.stringify({ baseUrl, adminKey: "admin" }));
      if (r && r.accounts) {
        for (const acc of r.accounts) {
          const existing = zhipuLocalKeys.value.find(k => k.key === acc.full_key);
          if (existing) {
            existing.valid = acc.valid;
          }
        }
        saveZhipuLocalKeys();
      }
    } catch {}
  }
  refreshZhipuAccountDisplay();
}

function refreshZhipuAccountDisplay() {
  zhipuAccountCount.value = zhipuLocalKeys.value.length;
  zhipuValidCount.value = zhipuLocalKeys.value.filter(k => k.valid).length;
  zhipuAccounts.value = zhipuLocalKeys.value.map(k => ({
    full_key: k.key,
    api_key: k.key.slice(0, 6) + "..." + k.key.slice(-4),
    label: k.label,
    valid: k.valid,
  }));
}

function saveZhipuLocalKeys() {
  try {
    localStorage.setItem("zhipu_local_keys", JSON.stringify(zhipuLocalKeys.value));
  } catch {}
  try {
    callBackend("saveZhipuKeys", JSON.stringify(zhipuLocalKeys.value));
  } catch {}
}

async function loadZhipuLocalKeys() {
  try {
    const backendData = await callBackend("loadZhipuKeys");
    if (backendData) {
      const parsed = typeof backendData === "string" ? JSON.parse(backendData) : backendData;
      if (Array.isArray(parsed) && parsed.length > 0) {
        zhipuLocalKeys.value = parsed;
        refreshZhipuAccountDisplay();
        return;
      }
    }
  } catch {}
  try {
    const saved = localStorage.getItem("zhipu_local_keys");
    if (saved) {
      zhipuLocalKeys.value = JSON.parse(saved);
      refreshZhipuAccountDisplay();
    }
  } catch {}
}

async function loginZhipuAccount() {
  if (zhipuLoginBusy.value || !zhipuLoginEmail.value.trim() || !zhipuLoginPassword.value.trim()) return;
  const baseUrl = `http://${zhipuApiHost.value || "127.0.0.1"}:${zhipuApiPort.value || 7780}`;
  zhipuLoginBusy.value = true;
  zhipuLoginError.value = "";

  const checkR = await callBackend("checkApiService", JSON.stringify({ baseUrl }));
  if (!checkR || !checkR.running) {
    zhipuLoginError.value = "智谱 API 服务未启动，请先点击一键启动";
    zhipuLoginBusy.value = false;
    return;
  }

  try {
    const r = await callBackend("loginZhipuAccount", JSON.stringify({
      baseUrl,
      email: zhipuLoginEmail.value.trim(),
      password: zhipuLoginPassword.value.trim(),
      region: "international",
    }));
    if (r && r.ok && r.api_key) {
      const newKey = r.api_key;
      const existing = zhipuLocalKeys.value.find(k => k.key === newKey);
      if (!existing) {
        zhipuLocalKeys.value.push({ key: newKey, label: r.email || newKey.slice(0, 8) + "...", valid: true });
        saveZhipuLocalKeys();
      }
      if (!zhipuApiKey.value) {
        zhipuApiKey.value = newKey;
      }
      showNotice("登录成功！API Key 已自动添加", "ok");
      zhipuLoginEmail.value = "";
      zhipuLoginPassword.value = "";
      await checkZhipuAccounts();
    } else if (r && r.ok && r.manual_key_needed) {
      showNotice("登录成功但需手动创建 API Key，请在 Z.ai 网站创建后粘贴添加", "warn");
    } else {
      zhipuLoginError.value = r?.error || "登录失败";
    }
  } catch (e) {
    zhipuLoginError.value = "登录异常: " + String(e);
  }
  zhipuLoginBusy.value = false;
}

async function autoRegisterZhipuAccount() {
  if (zhipuRegisterBusy.value) return;
  const baseUrl = `http://${zhipuApiHost.value || "127.0.0.1"}:${zhipuApiPort.value || 7780}`;
  zhipuRegisterBusy.value = true;
  zhipuRegisterLogs.value = ["🚀 启动自动注册..."];

  try {
    const checkR = await callBackend("checkApiService", JSON.stringify({ baseUrl }));
    if (!checkR || !checkR.running) {
      zhipuRegisterLogs.value.push("⚠ 智谱 API 服务未启动，正在自动启动...");
      const startR = await callBackend("startZhipu2Api", JSON.stringify({ port: zhipuApiPort.value || 7780, adminKey: "admin" }));
      if (!startR || !startR.ok) {
        zhipuRegisterLogs.value.push("❌ 自动启动服务失败: " + (startR?.error || "未知错误"));
        zhipuRegisterBusy.value = false;
        return;
      }
      for (let i = 0; i < 15; i++) {
        await new Promise(ok => setTimeout(ok, 1000));
        const chk = await callBackend("checkApiService", JSON.stringify({ baseUrl }));
        if (chk && chk.running) break;
      }
      const chk2 = await callBackend("checkApiService", JSON.stringify({ baseUrl }));
      if (!chk2 || !chk2.running) {
        zhipuRegisterLogs.value.push("❌ 服务启动超时，请手动点击「一键启动」");
        zhipuRegisterBusy.value = false;
        return;
      }
      zhipuRegisterLogs.value.push("✓ 服务已启动");
      zhipuStepProgress.value = zhipuSteps.length;
      const keyR = await callBackend("fetchZhipuApiKey", JSON.stringify({ baseUrl, adminKey: "admin" }));
      if (keyR && keyR.keys && keyR.keys.length > 0) {
        zhipuApiKey.value = keyR.keys[keyR.keys.length - 1].key;
      } else {
        const newKeyR = await callBackend("createZhipuApiKey", JSON.stringify({ baseUrl, adminKey: "admin" }));
        if (newKeyR && newKeyR.key) zhipuApiKey.value = newKeyR.key;
      }
    }

    const startR = await callBackend("startZhipuRegister", JSON.stringify({ baseUrl, adminKey: "admin" }));
    if (!startR || !startR.ok) {
      zhipuRegisterLogs.value.push("❌ 启动注册失败: " + (startR?.error || "服务连接失败，请确认智谱 API 服务已启动"));
      zhipuRegisterBusy.value = false;
      return;
    }

    for (let i = 0; i < 60; i++) {
      await new Promise(ok => setTimeout(ok, 2000));
      const pollR = await callBackend("pollZhipuRegister", JSON.stringify({ baseUrl, adminKey: "admin" }));
      if (pollR && pollR.logs) {
        zhipuRegisterLogs.value = pollR.logs;
      }
      if (pollR && !pollR.busy) {
        if (pollR.result && pollR.result.ok && pollR.result.api_key) {
          showNotice("智谱自动注册成功！", "ok");
          await checkZhipuAccounts();
        } else if (pollR.result && pollR.result.manual_key_needed) {
          showNotice("注册成功但需手动获取 Key，请登录 Z.ai", "warn");
        } else if (pollR.status === "failed" || pollR.status === "error") {
          showNotice("自动注册失败，请手动添加 API Key", "warn");
        }
        break;
      }
    }
  } catch (e) {
    zhipuRegisterLogs.value.push("❌ 注册异常: " + String(e));
  }
  zhipuRegisterBusy.value = false;
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
        if (result && result.running && result.serviceType === "qwen") {
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
        if (result && result.running && result.serviceType !== "qwen") {
          apiStepMessage.value = "端口被其他服务占用，准备切换...";
        } else {
          apiStepMessage.value = "服务未启动，准备启动...";
        }
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

function waitForApiService(maxRetries = 30, expectedType = "qwen"): Promise<boolean> {
  return new Promise((resolve) => {
    let count = 0;
    const timer = setInterval(async () => {
      count++;
      try {
        const result = await callBackend("checkApiService", JSON.stringify({ baseUrl: apiBaseUrl.value.trim() }));
        if (result && result.running && result.serviceType === expectedType) {
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

// 从 localStorage 加载设置（在 applySettings 之后调用，localStorage 优先）
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
      if (typeof data.apiSource === "string") {
        apiSource.value = data.apiSource;
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
      if (typeof data.zhipuApiKey === "string") {
        zhipuApiKey.value = data.zhipuApiKey;
      }
      if (typeof data.zhipuModel === "string") {
        zhipuModel.value = data.zhipuModel;
      }
      if (typeof data.zhipuApiHost === "string") {
        zhipuApiHost.value = data.zhipuApiHost;
      }
      if (typeof data.zhipuApiPort === "number") {
        zhipuApiPort.value = data.zhipuApiPort;
      }
      if (typeof data.zhipuApiChecked === "boolean") {
        zhipuApiChecked.value = data.zhipuApiChecked;
      }
      if (typeof data.stickyEmail === "string") {
        stickyEmail.value = data.stickyEmail;
      }
      if (typeof data.activeRole === "string") {
        activeRole.value = data.activeRole;
      }
      if (Array.isArray(data.quickModels)) {
        quickModels.value = data.quickModels;
      }
      if (typeof data.activeQuickModel === "string") {
        activeQuickModel.value = data.activeQuickModel;
      }
      if (typeof data.workspacePath === "string") {
        workspacePath.value = data.workspacePath;
      }
      if (typeof data.showPanel === "boolean") {
        showPanel.value = data.showPanel;
      }
      if (typeof data.showTerminal === "boolean") {
        showTerminal.value = data.showTerminal;
      }
      if (typeof data.inputText === "string") {
        inputText.value = data.inputText;
      }
      if (typeof data.terminalInput === "string") {
        terminalInput.value = data.terminalInput;
      }
      // 恢复 settings（AI_LANGUAGE, AI_TEMPERATURE, AI_MAX_TOKENS, SYSTEM_PROMPT 等）
      if (data.settings && typeof data.settings === "object") {
        for (const [key, val] of Object.entries(data.settings)) {
          if (key in settings && val !== undefined && val !== null) {
            (settings as any)[key] = val;
          }
        }
      }
      // 恢复 modelConfigs（每个模型的 per-model 参数）
      if (data.modelConfigs && typeof data.modelConfigs === "object") {
        for (const [modelId, cfg] of Object.entries(data.modelConfigs)) {
          if (typeof cfg === "object" && cfg !== null) {
            modelConfigs[modelId] = { ...(cfg as ModelConfig) };
          }
        }
      }
      // 恢复 qwenAccounts
      if (Array.isArray(data.qwenAccounts)) {
        qwenAccounts.value = data.qwenAccounts;
        qwenAccountCount.value = data.qwenAccounts.length;
      }
      // 恢复终端设置
      if (typeof data.terminalExpanded === "boolean") {
        terminalExpanded.value = data.terminalExpanded;
      }
      if (typeof data.terminalDebug === "boolean") {
        terminalDebug.value = data.terminalDebug;
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
      apiSource: apiSource.value,
      apiHost: apiHost.value,
      apiPort: apiPort.value,
      apiModel: apiModel.value,
      apiKey: apiKey.value,
      ollamaBaseUrl: ollamaBaseUrl.value,
      ollamaModel: ollamaModel.value,
      zhipuApiKey: zhipuApiKey.value,
      zhipuModel: zhipuModel.value,
      zhipuApiHost: zhipuApiHost.value,
      zhipuApiPort: zhipuApiPort.value,
      zhipuApiChecked: zhipuApiChecked.value,
      qwenAccounts: qwenAccounts.value,
      stickyEmail: stickyEmail.value,
      activeRole: activeRole.value,
      quickModels: quickModels.value,
      activeQuickModel: activeQuickModel.value,
      workspacePath: workspacePath.value,
      showPanel: showPanel.value,
      showTerminal: showTerminal.value,
      inputText: inputText.value,
      terminalInput: terminalInput.value,
      terminalExpanded: terminalExpanded.value,
      terminalDebug: terminalDebug.value,
      settings: { ...settings },
      modelConfigs: { ...modelConfigs },
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
    apiSource,
    apiHost,
    apiPort,
    apiModel,
    apiKey,
    ollamaBaseUrl,
    ollamaModel,
    zhipuApiKey,
    zhipuModel,
    zhipuApiHost,
    zhipuApiPort,
    zhipuApiChecked,
    activeRole,
    workspacePath,
    showPanel,
    showTerminal,
    inputText,
    terminalInput,
    terminalExpanded,
    terminalDebug,
    () => ({ ...settings }),
    () => ({ ...modelConfigs }),
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

  loadTheme();
  await loadZhipuLocalKeys();

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

  // localStorage 优先级最高，在 applySettings 之后加载，确保用户设置不被后端覆盖
  loadLocalSettings();

  detectHardware();

  if (runMode.value === "ollama") {
    loadCloudModels("ollama");
  } else if (runMode.value === "api") {
    await checkApiServiceStatus();
    await autoActivateApiService();
  }

  // 启动时自动检测并恢复服务状态（支持两边同时运行）
  await restoreServiceStatus();

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
        } else if (source === "zhipu") {
          zhipuModels.value = payload.models || [];
        }
        const label = source === "openrouter" ? " OpenRouter" : source === "anthropic" ? " Anthropic" : source === "api" ? " API" : source === "zhipu" ? " 智谱" : " Ollama";
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

(window as any).setVoiceResult = (text: string) => {
  inputText.value = text;
  isVoiceActive.value = false;
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

async function executeTerminalCommand() {
  const cmd = terminalInput.value.trim();
  if (!cmd) return;
  terminalInput.value = "";
  const cwd = terminalCwd.value || activeProject.value?.workspace_path || workspacePath.value || "";
  const entry = { cmd, output: "", ts: new Date().toLocaleTimeString() };
  terminalHistory.value.push(entry);
  try {
    const result = await callBackend("runTerminalCommand", cmd, cwd);
    const data = typeof result === "string" ? JSON.parse(result) : result;
    entry.output = data.output || data.error || "(无输出)";
    if (data.cwd) terminalCwd.value = data.cwd;
  } catch (e: any) {
    entry.output = `错误: ${e.message || e}`;
  }
}

function clearTerminal() {
  terminalHistory.value = [];
}

function toggleTerminal() {
  showTerminal.value = !showTerminal.value;
}

function scrollTerminalToBottom() {
  nextTick(() => {
    if (terminalOutputRef.value) {
      terminalOutputRef.value.scrollTop = terminalOutputRef.value.scrollHeight;
    }
  });
}

function copyTerminalOutput() {
  const text = terminalHistory.value.map(e => `[${e.ts}] $ ${e.cmd}\n${e.output}`).join("\n\n");
  if (!text) { showNotice("没有日志可复制", "warn"); return; }
  navigator.clipboard.writeText(text).then(() => {
    showNotice("日志已复制到剪贴板", "ok");
  }).catch(() => {
    showNotice("复制失败", "warn");
  });
}

function saveTerminalOutput() {
  const text = terminalHistory.value.map(e => `[${e.ts}] $ ${e.cmd}\n${e.output}`).join("\n\n");
  if (!text) { showNotice("没有日志可保存", "warn"); return; }
  const blob = new Blob([text], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `terminal-log-${new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19)}.txt`;
  a.click();
  URL.revokeObjectURL(url);
  showNotice("日志已保存", "ok");
}

async function loadGitStatus() {
  if (!activeProject.value) return;
  try {
    const data = await callBackend("getGitStatus", activeProject.value.workspace_path || workspacePath.value);
    gitStatus.value = typeof data === "string" ? JSON.parse(data) : data;
  } catch (e) {
    console.warn("loadGitStatus failed:", e);
  }
}

async function loadGitLog() {
  if (!activeProject.value) return;
  try {
    const data = await callBackend("getGitLog", activeProject.value.workspace_path || workspacePath.value);
    const parsed = typeof data === "string" ? JSON.parse(data) : data;
    gitLog.value = parsed.commits || [];
  } catch (e) {
    console.warn("loadGitLog failed:", e);
  }
}

async function doGitCommit() {
  if (!activeProject.value || !gitCommitMsg.value.trim()) return;
  try {
    await callBackend("gitCommit", activeProject.value.workspace_path || workspacePath.value, gitCommitMsg.value.trim());
    gitCommitMsg.value = "";
    loadGitStatus();
    loadGitLog();
    showNotice("提交成功", "ok");
  } catch (e) {
    showNotice("提交失败: " + e, "warn");
  }
}

function toggleGitPanel() {
  showGitPanel.value = !showGitPanel.value;
  if (showGitPanel.value) {
    loadGitStatus();
    loadGitLog();
  }
}

function loadSnippets() {
  try {
    const saved = localStorage.getItem("yunji_snippets");
    snippetList.value = saved ? JSON.parse(saved) : [];
  } catch {
    snippetList.value = [];
  }
}

function saveSnippet() {
  if (!newSnippetName.value.trim() || !newSnippetCode.value.trim()) return;
  const snippet = {
    id: makeId(),
    name: newSnippetName.value.trim(),
    lang: newSnippetLang.value,
    code: newSnippetCode.value,
    created: new Date().toISOString(),
  };
  snippetList.value.push(snippet);
  localStorage.setItem("yunji_snippets", JSON.stringify(snippetList.value));
  newSnippetName.value = "";
  newSnippetCode.value = "";
}

function deleteSnippet(id: string) {
  snippetList.value = snippetList.value.filter(s => s.id !== id);
  localStorage.setItem("yunji_snippets", JSON.stringify(snippetList.value));
}

function insertSnippet(snippet: any) {
  inputText.value += "\n```\n" + snippet.code + "\n```\n";
}

function applyTheme(theme: string) {
  currentTheme.value = theme as any;
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("yunji_theme", theme);
}

function loadTheme() {
  const saved = localStorage.getItem("yunji_theme");
  if (saved && ["dark", "light", "blue", "green"].includes(saved)) {
    applyTheme(saved);
  }
}

function searchInConversation() {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) { searchResults.value = []; return; }
  searchResults.value = messages.value
    .filter(m => m.text && m.text.toLowerCase().includes(q))
    .map(m => ({ msgId: m.id, text: m.text.slice(0, 100) }));
  searchIndex.value = 0;
}

function jumpToSearchResult(idx: number) {
  searchIndex.value = idx;
  const el = document.getElementById("msg-" + searchResults.value[idx]?.msgId);
  if (el) el.scrollIntoView({ behavior: "smooth", block: "center" });
}

async function loadFileTree() {
  if (!activeProject.value) return;
  try {
    const data = await callBackend("getFileTree", activeProject.value.workspace_path || workspacePath.value);
    fileTree.value = typeof data === "string" ? JSON.parse(data) : data;
  } catch (e) {
    console.warn("loadFileTree failed:", e);
  }
}

function toggleFileTree() {
  showFileTree.value = !showFileTree.value;
  if (showFileTree.value) loadFileTree();
}

async function loadPlugins() {
  try {
    const data = await callBackend("listPlugins");
    pluginList.value = typeof data === "string" ? JSON.parse(data) : data;
  } catch (e) {
    console.warn("loadPlugins failed:", e);
  }
}

async function installNewPlugin() {
  if (!newPluginId.value.trim() || !newPluginName.value.trim()) return;
  const meta = {
    id: newPluginId.value.trim(),
    name: newPluginName.value.trim(),
    desc: newPluginDesc.value.trim(),
    version: "1.0.0",
    code: newPluginCode.value,
  };
  try {
    await callBackend("installPlugin", JSON.stringify(meta));
    newPluginId.value = "";
    newPluginName.value = "";
    newPluginDesc.value = "";
    newPluginCode.value = "";
    loadPlugins();
    showNotice("插件安装成功", "ok");
  } catch (e) {
    showNotice("安装失败: " + e, "warn");
  }
}

async function uninstallPluginById(id: string) {
  try {
    await callBackend("uninstallPlugin", id);
    loadPlugins();
    showNotice("插件已卸载", "ok");
  } catch (e) {
    showNotice("卸载失败: " + e, "warn");
  }
}

async function runPlugin(pluginId: string) {
  try {
    const result = await callBackend("executePlugin", pluginId, inputText.value);
    const data = typeof result === "string" ? JSON.parse(result) : result;
    if (data.error) {
      showNotice("插件错误: " + data.error, "warn");
    } else if (data.output) {
      inputText.value = data.output;
    }
  } catch (e) {
    showNotice("执行失败: " + e, "warn");
  }
}

async function startVoice() {
  isVoiceActive.value = true;
  try {
    await callBackend("startVoiceInput", "zh-CN");
  } catch (e) {
    isVoiceActive.value = false;
    showNotice("语音识别启动失败", "warn");
  }
}

function speakMessage(text: string) {
  try {
    callBackend("speakText", text);
  } catch (e) {
    console.warn("speakText failed:", e);
  }
}

async function loadOfflineModels() {
  try {
    const data = await callBackend("getOfflineModels");
    offlineModels.value = typeof data === "string" ? JSON.parse(data) : data;
  } catch (e) {
    console.warn("loadOfflineModels failed:", e);
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
            <div v-if="searchResults.length > 0" style="display: flex; align-items: center; gap: 4px; margin-right: 8px;">
              <span style="font-size: 11px; color: #4af;">{{ searchIndex + 1 }}/{{ searchResults.length }}</span>
              <button class="btn-icon-sm" @click="jumpToSearchResult(Math.max(0, searchIndex - 1))" style="font-size: 10px;">▲</button>
              <button class="btn-icon-sm" @click="jumpToSearchResult(Math.min(searchResults.length - 1, searchIndex + 1))" style="font-size: 10px;">▼</button>
              <button class="btn-icon-sm" @click="searchResults = []; searchQuery = ''" style="font-size: 10px;">✕</button>
            </div>
            <div style="display: flex; align-items: center; gap: 4px; margin-right: 4px;">
              <input v-model="searchQuery" placeholder="搜索对话..." style="font-size: 11px; padding: 2px 8px; border-radius: 4px; border: 1px solid #333; background: #1a1a1a; color: #ddd; width: 120px;" @keydown.enter="searchInConversation" />
              <button class="btn-icon-sm" @click="searchInConversation" style="font-size: 10px;">🔍</button>
            </div>
            <button v-if="showPreviewPanel" class="btn-blue" @click="closePreview">关闭预览</button>
            <button v-else class="btn-blue" @click="openPreview">预览</button>
            <button :class="['btn-blue', { 'btn-active': showTerminal }]" @click="toggleTerminal">⌨ 终端</button>
            <button class="btn-blue" @click="chooseWorkspace">打开工作区</button>
            <button class="btn-blue" @click="showPanel = !showPanel">{{ showPanel ? "隐藏面板" : "显示面板" }}</button>
          </div>
        </div>

        <div class="messages">
          <article v-for="m in messages" :key="m.id" :id="'msg-' + m.id" class="msg" :class="m.role">
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

        <div v-if="showTerminal" class="terminal-panel" :class="{ 'terminal-expanded': terminalExpanded }">
          <div class="terminal-toolbar">
            <span style="font-size: 11px; color: #4af;">⌨ 终端</span>
            <span style="font-size: 10px; color: #555; margin-left: 8px;">{{ terminalCwd || activeProject?.workspace_path || workspacePath || '~' }}</span>
            <div style="display: flex; gap: 4px; margin-left: auto;">
              <button class="btn-icon-sm" @click="scrollTerminalToBottom" style="font-size: 10px;" title="滚动到底部">⬇</button>
              <button class="btn-icon-sm" @click="terminalExpanded = !terminalExpanded" style="font-size: 10px;" :title="terminalExpanded ? '折叠' : '展开'">{{ terminalExpanded ? '🔽' : '🔼' }}</button>
              <button class="btn-icon-sm" @click="terminalDebug = !terminalDebug" :style="{ fontSize: '10px', color: terminalDebug ? '#4af' : '#666' }" title="调试模式">🐛</button>
              <button class="btn-icon-sm" @click="copyTerminalOutput" style="font-size: 10px;" title="复制日志">📋</button>
              <button class="btn-icon-sm" @click="saveTerminalOutput" style="font-size: 10px;" title="保存日志">💾</button>
              <button class="btn-icon-sm" @click="clearTerminal" style="font-size: 10px;" title="清空">🗑️</button>
              <button class="btn-icon-sm" @click="showTerminal = false" style="font-size: 10px;">✕</button>
            </div>
          </div>
          <div class="terminal-output" ref="terminalOutputRef">
            <div v-for="(entry, idx) in terminalHistory" :key="idx" class="terminal-entry">
              <div class="terminal-cmd"><span style="color: #4af;">❯</span> {{ entry.cmd }} <span v-if="terminalDebug" style="color: #555; font-size: 9px;">[{{ entry.ts }}]</span></div>
              <pre class="terminal-result">{{ entry.output }}</pre>
            </div>
            <div v-if="terminalHistory.length === 0" class="terminal-empty">输入命令开始执行（如 ls, dir, npm run dev）</div>
          </div>
          <div class="terminal-input-row">
            <span style="color: #4af; font-size: 12px;">❯</span>
            <input v-model="terminalInput" placeholder="输入命令..." @keydown.enter.exact="executeTerminalCommand" class="terminal-input" />
          </div>
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
            <div style="display: flex; align-items: center; gap: 8px;">
              <!-- 快捷模型选择 -->
              <div class="quick-model-select" style="position: relative;">
                <button class="composer-action-btn" @click="showQuickModelDropdown = !showQuickModelDropdown" title="快捷切换模型">
                  {{ allQuickModels.find(m => m.id === activeQuickModel)?.name || '⚡ 模型' }}
                </button>
                <div v-if="showQuickModelDropdown" class="quick-model-dropdown" style="position: absolute; bottom: 100%; left: 0; margin-bottom: 4px; background: #1a1a1a; border: 1px solid #333; border-radius: 6px; padding: 4px; min-width: 200px; z-index: 100; max-height: 300px; overflow-y: auto;">
                  <!-- 可用模型 -->
                  <div v-if="availableQuickModels.length > 0" style="margin-bottom: 4px;">
                    <div style="font-size: 10px; color: #4af; padding: 2px 8px; font-weight: 600;">可用</div>
                    <div v-for="m in availableQuickModels" :key="m.id" :class="['quick-model-item', { active: activeQuickModel === m.id }]" @click="selectQuickModel(m.id)" style="padding: 4px 8px; cursor: pointer; border-radius: 4px; font-size: 12px; white-space: nowrap; display: flex; align-items: center; gap: 4px;">
                      <span style="color: #4CAF50;">●</span> {{ m.name }} <span v-if="(m as any).auto" style="font-size: 9px; color: #4af; background: #1a2a3a; padding: 0 4px; border-radius: 3px;">自动</span>
                    </div>
                  </div>
                  <!-- 不可用模型 -->
                  <div v-if="unavailableQuickModels.length > 0" style="margin-bottom: 4px;">
                    <div style="font-size: 10px; color: #666; padding: 2px 8px; font-weight: 600;">未就绪</div>
                    <div v-for="m in unavailableQuickModels" :key="m.id" class="quick-model-item disabled" style="padding: 4px 8px; border-radius: 4px; font-size: 12px; white-space: nowrap; display: flex; align-items: center; gap: 4px; color: #555; cursor: not-allowed;">
                      <span style="color: #555;">●</span> {{ m.name }}
                    </div>
                  </div>
                  <div v-if="allQuickModels.length === 0" style="padding: 4px 8px; color: #666; font-size: 11px;">
                    启动服务后自动显示模型
                  </div>
                </div>
              </div>
              <button class="composer-action-btn" @click="exportConversation('markdown')" :disabled="messages.length === 0" title="导出对话">导出</button>
              <button class="composer-action-btn" @click="createSession" :disabled="isBusy" title="新建会话">新建</button>
              <div class="composer-action-divider"></div>
              <button v-if="isBusy" class="composer-stop-btn" @click="stopMessage" title="停止当前任务">■ 停止</button>
              <button v-else class="composer-send-btn" @click="sendMessage">▶ 发送</button>
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
            <span class="help-bubble" style="margin-left: 4px;">?<span class="help-bubble-content">☁️ <b>云端模式</b>：使用 OpenRouter 云端服务，无需本地配置，适合新手<br/><br/>🔗 <b>API 模式</b>：使用千问或智谱的 API 服务，需要启动本地代理或填写 API Key<br/><br/>🦙 <b>Ollama 模式</b>：使用本地安装的模型，数据完全本地处理，隐私最安全</span></span>
          </div>
        </div>

        <div v-if="runMode === 'api'" class="api-source-bar">
          <button :class="['api-source-btn', { active: apiSource === 'qwen' }]" @click="apiSource = 'qwen'">🔗 千问</button>
          <button :class="['api-source-btn', { active: apiSource === 'zhipu' }]" @click="apiSource = 'zhipu'">🧠 智谱</button>
        </div>

        <div v-if="runMode === 'cloud'" class="sidebar-section">
          <div class="sidebar-section-title">☁️ 云端配置
            <span class="help-bubble">?<span class="help-bubble-content">使用 OpenRouter 云端服务<br/><br/>1. 前往 openrouter.ai 注册获取 API Key<br/>2. 粘贴 Key 后即可使用<br/>3. 模型固定为 openrouter/auto（自动选择最优模型）</span></span>
          </div>
          <div class="sidebar-field">
            <label>API Key</label>
            <input v-model="apiKey" type="text" placeholder="OpenRouter API Key" class="setting-input" style="width: 100%;" />
          </div>
          <div class="sidebar-field">
            <label>模型</label>
            <input value="openrouter/auto" readonly class="setting-input" style="width: 100%; opacity: 0.6;" />
          </div>
        </div>

        <div v-if="runMode === 'api' && apiSource === 'qwen'" class="sidebar-section">
          <div class="sidebar-section-title">🔗 千问 API
            <span class="help-bubble">?<span class="help-bubble-content">本地代理服务，将千问网页版转为 OpenAI 兼容 API<br/><br/>1. 点击「启动服务」启动本地代理<br/>2. 启动后自动获取 API Key 和模型列表<br/>3. 或手动添加 Key 直连官方 API</span></span>
          </div>
          <div class="sidebar-field">
            <label>服务地址</label>
            <div style="display: flex; align-items: center; gap: 0;">
              <span style="padding: 0 6px; font-size: 12px; color: #888; background: #1a1a1a; border: 1px solid #333; border-right: none; border-radius: 4px 0 0 4px; height: 32px; line-height: 32px;">http://</span>
              <input v-model="apiHost" placeholder="127.0.0.1" style="width: 90px; border-radius: 0; height: 32px; font-size: 12px;" />
              <span style="padding: 0 4px; font-size: 13px; color: #888; background: #1a1a1a; border: 1px solid #333; border-left: none; border-right: none; height: 32px; line-height: 32px;">:</span>
              <input v-model="apiPort" type="number" min="1" max="65535" placeholder="7777" style="width: 60px; text-align: center; border-radius: 0 4px 4px 0; height: 32px; font-size: 12px;" />
            </div>
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
            <div style="display: flex; gap: 4px; margin-top: 4px;">
              <button class="btn-blue" style="flex: 1; font-size: 12px; padding: 6px 10px;" @click="apiStepAutoRun" :disabled="apiStepBusy || apiStepProgress >= apiSteps.length">{{ apiStepBusy ? apiStepMessage : (apiStepProgress >= apiSteps.length ? '✓ 已就绪' : '▶ 启动服务') }}</button>
              <button class="btn-red" style="flex: 0 0 auto; min-width: 60px; font-size: 12px; padding: 6px 10px;" @click="stopApiService" :disabled="apiStepBusy || apiStepProgress < apiSteps.length" v-if="apiStepProgress >= apiSteps.length">■ 停止</button>
            </div>
          </div>
          <div class="sidebar-field" style="margin-top: 8px;">
            <label>模型选择</label>
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
          </div>
          <details style="margin-top: 8px;">
            <summary style="font-size: 12px; color: #888; cursor: pointer;">➕ 添加 API Key</summary>
            <div class="reg-form" style="margin-top: 4px;">
              <input v-model="apiKey" type="text" placeholder="粘贴千问 API Key" style="font-size: 12px;" />
              <button class="btn-blue btn-sm" @click="showNotice('API Key 已保存', 'ok')" :disabled="!apiKey.trim()" style="width: 100%; font-size: 12px;">添加</button>
            </div>
          </details>
        </div>

        <div v-if="runMode === 'api' && apiSource === 'zhipu'" class="sidebar-section">
          <div class="sidebar-section-title">🧠 智谱 API
            <span class="help-bubble">?<span class="help-bubble-content">本地代理服务，支持多 Key 轮换<br/><br/>1. 点击「启动服务」启动代理<br/>2. 或不启动代理，直接添加 Key 直连官方 API<br/><br/>💡 启动代理后可多 Key 轮换 = 无限算力</span></span>
          </div>
          <div class="sidebar-field">
            <label>服务地址</label>
            <div style="display: flex; align-items: center; gap: 0;">
              <span style="padding: 0 6px; font-size: 12px; color: #888; background: #1a1a1a; border: 1px solid #333; border-right: none; border-radius: 4px 0 0 4px; height: 32px; line-height: 32px;">http://</span>
              <input v-model="zhipuApiHost" placeholder="127.0.0.1" style="width: 90px; border-radius: 0; height: 32px; font-size: 12px;" />
              <span style="padding: 0 4px; font-size: 13px; color: #888; background: #1a1a1a; border: 1px solid #333; border-left: none; border-right: none; height: 32px; line-height: 32px;">:</span>
              <input v-model="zhipuApiPort" type="number" min="1" max="65535" placeholder="7780" style="width: 60px; text-align: center; border-radius: 0 4px 4px 0; height: 32px; font-size: 12px;" />
            </div>
          </div>
          <div class="api-progress-section" style="margin-top: 8px;">
            <div class="api-progress-bar">
              <div v-for="(step, idx) in zhipuSteps" :key="idx" :class="['api-step', { done: zhipuStepProgress > idx, active: zhipuStepProgress === idx, pending: zhipuStepProgress < idx }]">
                <div class="step-dot"><span v-if="zhipuStepProgress > idx">✓</span><span v-else>{{ idx + 1 }}</span></div>
                <div class="step-label">{{ step.label }}</div>
              </div>
            </div>
            <div class="api-progress-track"><div class="api-progress-fill" :style="{ width: zhipuProgressPercent + '%' }"></div></div>
            <p class="api-progress-msg">{{ zhipuStepMessage }}</p>
            <div style="display: flex; gap: 4px; margin-top: 4px;">
              <button class="btn-blue" style="flex: 1; font-size: 12px; padding: 6px 10px;" @click="zhipuStepAutoRun" :disabled="zhipuStepBusy || zhipuStepProgress >= zhipuSteps.length">{{ zhipuStepBusy ? zhipuStepMessage : (zhipuStepProgress >= zhipuSteps.length ? '✓ 已就绪' : '▶ 启动服务') }}</button>
              <button class="btn-red" style="flex: 0 0 auto; min-width: 60px; font-size: 12px; padding: 6px 10px;" @click="stopZhipuService" :disabled="zhipuStepBusy || zhipuStepProgress < zhipuSteps.length" v-if="zhipuStepProgress >= zhipuSteps.length">■ 停止</button>
            </div>
          </div>
          <div class="sidebar-field" style="margin-top: 8px;">
            <label>模型选择</label>
            <div v-if="zhipuModels.length > 0" class="model-quick-select">
              <div class="custom-select" :class="{ open: modelDropdownOpen }" @click.stop="modelDropdownOpen = !modelDropdownOpen">
                <div class="custom-select-value">
                  <span>{{ zhipuModel ? displayName(zhipuModels.find(m => m.id === zhipuModel)?.name || zhipuModel) : '选择模型...' }}</span>
                  <span class="custom-select-arrow">▼</span>
                </div>
                <div v-if="modelDropdownOpen" class="custom-select-options">
                  <div v-for="m in zhipuModels" :key="m.id" :class="['custom-select-option', { selected: zhipuModel === m.id }]" @click.stop="zhipuModel = m.id; modelDropdownOpen = false">
                    <span>{{ displayName(m.name || m.id) }}</span>
                    <span v-if="m.toolSupport === true" class="tool-badge ok">★</span>
                    <span v-else-if="m.toolSupport === false" class="tool-badge no">-</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div v-if="runMode === 'api' && apiSource === 'zhipu'" class="sidebar-section">
          <div class="sidebar-section-title">🔑 智谱账户
            <span class="help-bubble">?<span class="help-bubble-content">1. 点击「Z.ai 注册」用 Email 注册（无需手机号）<br/>2. 登录后进入 Profile → API Keys → Create Key<br/>3. 将 Key 粘贴到「添加 API Key」中<br/><br/>💡 启动代理服务后可添加多个 Key 实现轮换，等同无限算力<br/>💡 未启动代理时，Key 将直连智谱官方 API</span></span>
          </div>
          <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 4px;">
            <span v-if="zhipuAccountCount >= 0" :class="['account-badge', zhipuAccountCount > 0 ? 'ok' : 'warn']">{{ zhipuAccountCount }} 个</span>
            <span v-if="zhipuValidCount > 0" style="color: #4CAF50; font-size: 12px;">{{ zhipuValidCount }} 可用</span>
            <button class="btn-blue btn-sm" @click="checkZhipuAccounts" style="margin-left: auto; font-size: 12px;">刷新</button>
          </div>
          <p v-if="zhipuAccountCount === 0" class="hint warn" style="margin: 2px 0; font-size: 12px;">未添加 API Key，请先添加</p>
          <div v-if="zhipuAccounts.length > 0" class="account-list" style="max-height: 120px; overflow-y: auto;">
            <div v-for="acc in zhipuAccounts" :key="acc.full_key || acc.api_key" :class="['account-row']">
              <span :class="['account-status', acc.valid ? 'valid' : 'invalid']">●</span>
              <span class="account-email">{{ acc.label || acc.api_key }}</span>
              <button class="btn-icon btn-sticky" @click="checkZhipuAccounts()" title="验证全部" style="color: #4af;">✓</button>
              <button class="btn-icon btn-del" @click="deleteZhipuAccount(acc.full_key || acc.api_key)" title="删除">✕</button>
            </div>
          </div>
          <details style="margin-top: 8px;">
            <summary style="font-size: 12px; color: #888; cursor: pointer;">🔑 登录已有账户</summary>
            <div class="reg-form" style="margin-top: 4px;">
              <input v-model="zhipuLoginEmail" type="text" placeholder="邮箱" style="font-size: 12px;" />
              <input v-model="zhipuLoginPassword" type="password" placeholder="密码" style="font-size: 12px;" />
              <button class="btn-blue btn-sm" @click="loginZhipuAccount" :disabled="zhipuLoginBusy || !zhipuLoginEmail.trim() || !zhipuLoginPassword.trim()" style="width: 100%; font-size: 12px;">{{ zhipuLoginBusy ? '⏳ 登录中...' : '登录' }}</button>
            </div>
            <p v-if="zhipuLoginError" class="hint warn" style="margin: 2px 0; font-size: 12px;">{{ zhipuLoginError }}</p>
          </details>
          <details style="margin-top: 8px;">
            <summary style="font-size: 12px; color: #888; cursor: pointer;">➕ 添加 API Key</summary>
            <div class="reg-form" style="margin-top: 4px;">
              <input v-model="zhipuNewLabel" type="text" placeholder="标签（可选）" style="font-size: 12px;" />
              <input v-model="zhipuNewKey" type="text" placeholder="粘贴智谱 API Key" style="font-size: 12px;" />
              <button class="btn-blue btn-sm" @click="addZhipuAccount" :disabled="!zhipuNewKey.trim()" style="width: 100%; font-size: 12px;">添加</button>
            </div>
          </details>
          <div style="display: flex; gap: 4px; margin-top: 8px;">
            <button class="btn-blue" @click="callBackend('openExternalUrl', 'https://z.ai/chat')" style="flex: 1; font-size: 12px; padding: 6px 10px;">🌐 Z.ai 注册(海外)</button>
            <button class="btn-blue" @click="callBackend('openExternalUrl', 'https://open.bigmodel.cn/user/login')" style="flex: 1; font-size: 12px; padding: 6px 10px;">🇨🇳 国内版注册</button>
          </div>
        </div>

        <div v-if="runMode === 'ollama'" class="sidebar-section">
          <div class="sidebar-section-title">🦙 Ollama
            <span class="help-bubble">?<span class="help-bubble-content">本地模型推理，无需联网<br/><br/>1. 安装 Ollama: ollama.com<br/>2. 启动 Ollama 服务<br/>3. 点击「检测」自动发现可用模型<br/><br/>💡 数据完全本地处理，隐私安全</span></span>
          </div>
          <div class="sidebar-field">
            <label>服务地址</label>
            <div style="display: flex; gap: 4px; align-items: center;">
              <input v-model="ollamaBaseUrl" placeholder="http://127.0.0.1:11434" class="setting-input" style="flex: 1;" />
              <button class="btn-blue btn-sm" @click="detectModels" :disabled="loadingModels" style="white-space: nowrap; font-size: 10px;">{{ loadingModels ? "检测中" : "🔍 检测" }}</button>
            </div>
          </div>
        </div>

        <div v-if="runMode === 'api' && apiSource === 'qwen'" class="sidebar-section">
          <div class="sidebar-section-title">🔑 千问账户
            <span class="help-bubble">?<span class="help-bubble-content">1. 点击「🤖 自动注册」一键生成账号<br/>2. 或手动登录已有账户 / 添加 Token<br/><br/>💡 多账户自动轮换，等同无限算力<br/>💡 被限流的账户会自动冷却恢复</span></span>
          </div>
          <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 4px;">
            <span v-if="qwenAccountCount >= 0" :class="['account-badge', qwenAccountCount > 0 ? 'ok' : 'warn']">{{ qwenAccountCount }} 个</span>
            <span v-if="qwenValidCount > 0" style="color: #4CAF50; font-size: 12px;">{{ qwenValidCount }} 可用</span>
            <button class="btn-blue btn-sm" @click="checkQwenAccounts" style="margin-left: auto; font-size: 12px;">刷新</button>
          </div>
          <p v-if="qwenAccountCount === 0" class="hint warn" style="margin: 2px 0; font-size: 12px;">未添加账户，对话将返回 500</p>
          <div v-if="qwenAccounts.length > 0" class="account-list" style="max-height: 120px; overflow-y: auto;">
            <div v-for="acc in qwenAccounts" :key="acc.email" :class="['account-row', { sticky: stickyEmail === acc.email }]">
              <span :class="['account-status', acc.valid ? 'valid' : 'invalid']">●</span>
              <span class="account-email">{{ acc.email }}</span>
              <span v-if="stickyEmail === acc.email" class="sticky-badge">★</span>
              <button v-if="stickyEmail !== acc.email && acc.valid" class="btn-icon btn-sticky" @click="setStickyAccount(acc.email)" title="设为优先">★</button>
              <button class="btn-icon btn-del" @click="deleteQwenAccount(acc.email)" title="删除">✕</button>
            </div>
          </div>
          <div style="display: flex; gap: 4px; margin-top: 8px;">
            <button class="btn-blue" @click="autoRegisterQwenAccount" :disabled="qwenRegisterBusy" style="flex: 1; font-size: 12px; padding: 6px 10px;">{{ qwenRegisterBusy ? '⏳ 注册中...' : '🤖 自动注册' }}</button>
          </div>
          <details style="margin-top: 8px;">
            <summary style="font-size: 12px; color: #888; cursor: pointer;">🔑 登录已有账户</summary>
            <div class="reg-form" style="margin-top: 4px;">
              <input v-model="loginEmail" type="text" placeholder="邮箱" style="font-size: 12px;" />
              <input v-model="loginPassword" type="password" placeholder="密码" style="font-size: 12px;" />
              <button class="btn-blue btn-sm" @click="loginQwenAccount" :disabled="qwenLoginBusy || !loginEmail.trim() || !loginPassword.trim()" style="width: 100%; font-size: 12px;">{{ qwenLoginBusy ? '⏳ 登录中...' : '登录' }}</button>
            </div>
            <p v-if="qwenLoginError" class="hint warn" style="margin: 2px 0; font-size: 12px;">{{ qwenLoginError }}</p>
          </details>
          <details style="margin-top: 8px;">
            <summary style="font-size: 12px; color: #888; cursor: pointer;">➕ 添加 API Key</summary>
            <div class="reg-form" style="margin-top: 4px;">
              <input v-model="qwenToken" type="text" placeholder="粘贴 Token / API Key" style="font-size: 12px;" />
              <button class="btn-blue btn-sm" @click="addQwenAccount" :disabled="!qwenToken.trim()" style="width: 100%; font-size: 12px;">添加</button>
            </div>
          </details>
          <div v-if="qwenRegisterLogs.length > 0" class="register-log-box" style="margin-top: 4px; max-height: 80px; overflow-y: auto;">
            <div style="display: flex; justify-content: flex-end; gap: 4px; margin-bottom: 2px;">
              <button class="btn-icon-sm" @click="navigator.clipboard.writeText(qwenRegisterLogs.join('\n')).then(()=>showNotice('已复制','ok'))" style="font-size: 9px;" title="复制">📋</button>
              <button class="btn-icon-sm" @click="qwenRegisterLogs = []" style="font-size: 9px;" title="清空">🗑️</button>
            </div>
            <div v-for="(log, idx) in qwenRegisterLogs" :key="idx" class="register-log-line">{{ log }}</div>
          </div>
        </div>

        <div class="sidebar-section">
          <div class="sidebar-section-title">🎯 快速角色</div>
          <div class="role-presets">
            <button v-for="(preset, key) in ROLE_PRESETS" :key="key" :class="['role-btn', { active: activeRole === key }]" @click="applyRole(key)" :title="preset.desc">
              {{ preset.icon }} {{ preset.name }}
            </button>
          </div>
        </div>
      </aside>

      <template v-else>
        <div class="panel card" style="padding: 12px;">
          <button class="btn-blue" style="width: 100%;" @click="showPanel = true">📂 展开面板</button>
        </div>
      </template>
    </main>

    <main v-if="activeNav === 'project'" class="page-view">
      <div class="page-view-inner">
        <div class="page-view-header">
          <h2>📁 项目管理</h2>
          <div style="display: flex; gap: 8px; align-items: center;">
            <input v-model="newProjectName" placeholder="新项目名称" class="setting-input" style="width: 200px;" @input="updateDefaultPath" />
            <button class="btn-blue" @click="createProject" :disabled="!newProjectName.trim()">创建项目</button>
            <button class="btn-blue" @click="chooseWorkspace">打开工作区</button>
          </div>
        </div>
        <div class="project-grid">
          <div v-for="p in projects" :key="p.id" :class="['project-card', { active: p.id === activeProject?.id }]" @click="switchProject(p.id)">
            <div class="project-card-header">
              <span class="project-card-name">{{ p.name }}</span>
              <span v-if="p.id === activeProject?.id" class="project-active-badge">当前</span>
            </div>
            <div class="project-card-path">{{ p.workspace_path || '纯对话模式' }}</div>
            <div class="project-card-actions">
              <button v-if="p.workspace_path" class="btn-sm" @click.stop="openInExplorer(p.workspace_path)" style="font-size: 10px;">📁 打开目录</button>
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
              <span style="font-size: 12px; color: #ddd;">{{ runMode === 'api' ? (apiSource === 'zhipu' ? zhipuModel : apiModel) : runMode === 'ollama' ? ollamaModel : 'openrouter/auto' }}</span>
            </div>
            <div class="setting-row">
              <div class="setting-info"><div class="setting-name">工作区路径</div></div>
              <span style="font-size: 11px; color: #888;">{{ activeProject?.workspace_path || workspacePath || '未选择' }}</span>
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
              <span class="nav-icon">🤖</span><span class="nav-label">模型设置</span>
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
            <button :class="['settings-nav-item', { active: settingsTab === 'plugins' }]" @click="settingsTab = 'plugins'; loadPlugins()">
              <span class="nav-icon">🧩</span><span class="nav-label">插件扩展</span>
            </button>
            <button :class="['settings-nav-item', { active: settingsTab === 'voice' }]" @click="settingsTab = 'voice'">
              <span class="nav-icon">🎤</span><span class="nav-label">语音交互</span>
            </button>
            <button :class="['settings-nav-item', { active: settingsTab === 'offline' }]" @click="settingsTab = 'offline'; loadOfflineModels()">
              <span class="nav-icon">📴</span><span class="nav-label">离线模式</span>
            </button>
          </nav>
        </div>
        <div class="settings-content">
          <div v-if="settingsTab === 'general'" class="settings-section">
            <h3 class="section-title">通用设置</h3>
            <div class="settings-card">
              <div class="card-title">界面</div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">主题</div><div class="setting-desc">切换界面配色方案</div></div>
                <div style="display: flex; gap: 4px;">
                  <button :class="['theme-btn', { active: currentTheme === 'dark' }]" @click="applyTheme('dark')" style="--tc: #1a1a1a; --tc2: #0d0d0d;">🌙</button>
                  <button :class="['theme-btn', { active: currentTheme === 'light' }]" @click="applyTheme('light')" style="--tc: #f0f0f0; --tc2: #fff;">☀️</button>
                  <button :class="['theme-btn', { active: currentTheme === 'blue' }]" @click="applyTheme('blue')" style="--tc: #0a1628; --tc2: #0d1f3c;">🔵</button>
                  <button :class="['theme-btn', { active: currentTheme === 'green' }]" @click="applyTheme('green')" style="--tc: #0a1a0a; --tc2: #0d200d;">🟢</button>
                </div>
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">AI 语言</div><div class="setting-desc">AI 回复使用的语言</div></div>
                <select v-model="getModelConfig(activeModelId).language" class="setting-select">
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
            <h3 class="section-title">🤖 模型设置</h3>
            <div class="settings-card">
              <div class="card-title">快捷模型列表</div>
              <div class="setting-desc" style="margin-bottom: 8px;">配置发送按钮左侧的快捷模型切换选项</div>
              <div v-for="m in autoQuickModels" :key="m.id" class="setting-row" style="align-items: center; padding: 6px 0; border-bottom: 1px solid #222;">
                <div style="flex: 1; min-width: 0;">
                  <div style="font-size: 13px; color: #ddd;">{{ m.name }} <span style="font-size: 9px; color: #4af; background: #1a2a3a; padding: 0 4px; border-radius: 3px;">自动</span></div>
                  <div style="font-size: 10px; color: #666;">{{ m.provider }}{{ m.apiSource ? ' / ' + m.apiSource : '' }}</div>
                </div>
                <div style="display: flex; gap: 4px; align-items: center;">
                  <span v-if="activeQuickModel === m.id" style="font-size: 11px; color: #4af;">✓ 当前</span>
                  <button class="btn-sm" @click="selectQuickModel(m.id)" :disabled="activeQuickModel === m.id">切换</button>
                </div>
              </div>
              <div v-for="(m, idx) in quickModels" :key="m.id" class="setting-row" style="align-items: center; padding: 6px 0; border-bottom: 1px solid #222;">
                <div style="flex: 1; min-width: 0;">
                  <div style="font-size: 13px; color: #ddd;">{{ m.name }}</div>
                  <div style="font-size: 10px; color: #666;">{{ m.provider }}{{ m.apiSource ? ' / ' + m.apiSource : '' }} — {{ m.modelId || '默认' }}</div>
                </div>
                <div style="display: flex; gap: 4px; align-items: center;">
                  <span v-if="activeQuickModel === m.id" style="font-size: 11px; color: #4af;">✓ 当前</span>
                  <button class="btn-sm" @click="selectQuickModel(m.id)" :disabled="activeQuickModel === m.id">切换</button>
                  <button class="btn-sm" style="background: #3a1a1a; color: #f55;" @click="quickModels.splice(idx, 1); if (activeQuickModel === m.id) activeQuickModel = ''">删除</button>
                </div>
              </div>
              <div v-if="allQuickModels.length === 0" style="padding: 12px; color: #666; font-size: 12px; text-align: center;">启动服务后自动显示模型</div>
            </div>
            <div class="settings-card">
              <div class="card-title">添加快捷模型</div>
              <div style="display: flex; flex-direction: column; gap: 8px;">
                <input v-model="newQuickModelName" placeholder="显示名称（如：🦙 本地模型）" class="setting-input" />
                <select v-model="newQuickModelProvider" class="setting-select">
                  <option value="ollama">🦙 Ollama</option>
                  <option value="api">🔗 API</option>
                  <option value="cloud">☁️ 云端</option>
                </select>
                <select v-if="newQuickModelProvider === 'api'" v-model="newQuickModelApiSource" class="setting-select">
                  <option value="qwen">🔗 千问</option>
                  <option value="zhipu">🧠 智谱</option>
                </select>
                <input v-model="newQuickModelId" :placeholder="newQuickModelProvider === 'ollama' ? '模型ID（如：qwen2.5:14b）' : newQuickModelProvider === 'api' ? '模型ID（如：qwen3.6-plus）' : '模型ID（如：openrouter/auto）'" class="setting-input" />
                <button class="btn-blue" @click="addQuickModel" style="width: 100%;">添加</button>
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
              <div v-if="activeProject" style="font-size: 11px; color: #42A5F5; margin-bottom: 8px;">当前项目：{{ activeProject.name }} <span style="color: #666;">{{ activeProject.workspace_path || '纯对话' }}</span></div>
              <div style="margin-bottom: 8px;">
                <div style="font-size: 11px; color: #888; margin-bottom: 4px;">新建项目</div>
                <div style="display: flex; gap: 4px; margin-bottom: 4px;">
                  <input v-model="newProjectName" placeholder="项目名称" class="setting-input" style="flex: 1;" @input="updateDefaultPath" />
                  <button class="btn-blue btn-sm" @click="createProject" :disabled="!newProjectName.trim()">创建</button>
                </div>
                <div style="display: flex; align-items: center; gap: 4px; margin-bottom: 4px;">
                  <label style="font-size: 10px; color: #888; cursor: pointer;">
                    <input type="checkbox" v-model="newProjectCustomPath" style="margin-right: 4px;" />指定工作区
                  </label>
                  <button v-if="newProjectCustomPath" class="btn-sm" @click="selectWorkspaceDir" style="font-size: 10px;">📂 选择目录</button>
                </div>
                <div v-if="newProjectCustomPath && newProjectPath" style="font-size: 10px; color: #4af;">工作区: {{ newProjectPath }}</div>
                <div v-if="!newProjectCustomPath" style="font-size: 10px; color: #666;">不指定工作区则为纯对话模式</div>
              </div>
              <div v-for="p in projects" :key="p.id" :class="['project-item', { active: p.id === activeProject?.id }]" style="margin-bottom: 4px;">
                <div style="display: flex; align-items: center; justify-content: space-between;" @click="switchProject(p.id)">
                  <span style="font-size: 12px;">{{ p.name }}</span>
                  <div style="display: flex; gap: 2px;">
                    <button v-if="p.workspace_path" class="btn-icon-sm" @click.stop="openInExplorer(p.workspace_path)" title="打开目录">📁</button>
                    <button class="btn-icon-sm" @click.stop="deleteProject(p.id)" title="删除">🗑️</button>
                  </div>
                </div>
                <div style="font-size: 10px; color: #666;">{{ p.workspace_path || '纯对话模式' }}</div>
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

          <div v-if="settingsTab === 'plugins'" class="settings-section">
            <h3 class="section-title">🧩 插件扩展</h3>
            <div class="settings-card">
              <div class="card-title">已安装插件</div>
              <div v-if="pluginList.length === 0" style="font-size: 11px; color: #555; text-align: center; padding: 12px;">暂无已安装插件</div>
              <div v-for="p in pluginList" :key="p.id" style="display: flex; align-items: center; gap: 8px; padding: 8px 0; border-bottom: 1px solid #222;">
                <span style="font-size: 13px; color: #ddd; flex: 1;">{{ p.name }}</span>
                <span style="font-size: 10px; color: #888;">{{ p.desc }}</span>
                <button class="btn-sm" @click="runPlugin(p.id)" style="font-size: 10px; background: #2a3a2a;">▶ 执行</button>
                <button class="btn-icon-sm" @click="uninstallPluginById(p.id)" style="font-size: 9px;">🗑️</button>
              </div>
            </div>
            <div class="settings-card">
              <div class="card-title">安装新插件</div>
              <div style="display: flex; gap: 4px; margin-bottom: 4px;">
                <input v-model="newPluginId" placeholder="插件ID（英文）" class="setting-input" style="flex: 1;" />
                <input v-model="newPluginName" placeholder="插件名称" class="setting-input" style="flex: 1;" />
              </div>
              <input v-model="newPluginDesc" placeholder="插件描述" class="setting-input" style="width: 100%; margin-bottom: 4px;" />
              <textarea v-model="newPluginCode" rows="5" placeholder="插件代码（Python，使用 input 获取输入，设置 result 返回输出）&#10;&#10;示例：&#10;result = input.upper()" class="setting-textarea" style="font-family: Consolas, monospace;"></textarea>
              <button class="btn-blue" @click="installNewPlugin" style="margin-top: 4px; width: 100%;">安装插件</button>
            </div>
            <div class="settings-card">
              <div class="card-title">插件开发指南</div>
              <div style="font-size: 11px; color: #888; line-height: 1.6;">
                <p>插件使用 Python 编写，运行在安全沙箱中。</p>
                <p><b>可用变量：</b></p>
                <ul style="padding-left: 16px;">
                  <li><code>input</code> - 用户输入的文本</li>
                  <li><code>json</code> - JSON 模块</li>
                  <li><code>os</code> - 操作系统模块</li>
                  <li><code>result</code> - 设置此变量返回输出</li>
                </ul>
                <p style="margin-top: 8px;"><b>示例：</b>文本转大写</p>
                <pre style="background: #111; padding: 8px; border-radius: 4px; font-size: 11px;">result = input.upper()</pre>
              </div>
            </div>
          </div>

          <div v-if="settingsTab === 'voice'" class="settings-section">
            <h3 class="section-title">🎤 语音交互</h3>
            <div class="settings-card">
              <div class="card-title">语音输入</div>
              <div class="setting-desc" style="margin-bottom: 8px;">点击麦克风按钮开始语音识别，识别结果将自动填入输入框</div>
              <button :class="['btn-blue', { 'btn-active': isVoiceActive }]" @click="startVoice" style="width: 100%; font-size: 14px; padding: 12px;">
                {{ isVoiceActive ? '🎤 正在录音...' : '🎤 开始语音输入' }}
              </button>
              <div style="font-size: 10px; color: #555; margin-top: 6px;">需要安装 speech_recognition 库（pip install SpeechRecognition）</div>
            </div>
            <div class="settings-card">
              <div class="card-title">语音朗读</div>
              <div class="setting-desc" style="margin-bottom: 8px;">点击消息旁的🔊按钮朗读AI回复</div>
              <div v-if="messages.length > 0" style="display: flex; gap: 4px; flex-wrap: wrap;">
                <button v-for="m in messages.filter(m => m.role === 'assistant').slice(-5)" :key="m.id" class="btn-sm" @click="speakMessage(m.text?.slice(0, 500) || '')" style="font-size: 10px; background: #2a3a2a;">
                  🔊 {{ (m.text || '').slice(0, 30) }}...
                </button>
              </div>
              <div style="font-size: 10px; color: #555; margin-top: 6px;">需要安装 pyttsx3 库（pip install pyttsx3）</div>
            </div>
          </div>

          <div v-if="settingsTab === 'offline'" class="settings-section">
            <h3 class="section-title">📴 离线模式</h3>
            <div class="settings-card">
              <div class="card-title">本地模型缓存</div>
              <div class="setting-desc" style="margin-bottom: 8px;">下载模型到本地，断网时仍可使用</div>
              <div v-if="offlineModels.length === 0" style="font-size: 11px; color: #555; text-align: center; padding: 12px;">暂无本地模型</div>
              <div v-for="m in offlineModels" :key="m.name" style="display: flex; align-items: center; gap: 8px; padding: 6px 0; border-bottom: 1px solid #222;">
                <span style="font-size: 12px; color: #ddd; flex: 1;">{{ m.name }}</span>
                <span style="font-size: 10px; color: #888;">{{ m.size_mb }} MB</span>
              </div>
            </div>
            <div class="settings-card">
              <div class="card-title">下载模型</div>
              <div class="setting-desc" style="margin-bottom: 8px;">输入模型下载链接（支持 .gguf 格式）</div>
              <div style="display: flex; gap: 4px;">
                <input placeholder="https://huggingface.co/...model.gguf" class="setting-input" style="flex: 1;" />
                <button class="btn-blue btn-sm" @click="showNotice('开始下载（后台）', 'ok')">下载</button>
              </div>
              <div style="font-size: 10px; color: #555; margin-top: 6px;">推荐使用 Ollama 模式配合本地模型</div>
            </div>
            <div class="settings-card">
              <div class="card-title">离线功能</div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">对话缓存</div><div class="setting-desc">对话历史保存在本地，断网可查看</div></div>
                <span style="color: #4CAF50; font-size: 12px;">✓ 已启用</span>
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">记忆缓存</div><div class="setting-desc">项目记忆本地存储，断网可编辑</div></div>
                <span style="color: #4CAF50; font-size: 12px;">✓ 已启用</span>
              </div>
              <div class="setting-row">
                <div class="setting-info"><div class="setting-name">模板缓存</div><div class="setting-desc">项目模板本地存储，断网可使用</div></div>
                <span style="color: #4CAF50; font-size: 12px;">✓ 已启用</span>
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
:root, [data-theme="dark"] {
  --bg-primary: #0d0d0d;
  --bg-secondary: #111;
  --bg-card: #141414;
  --bg-input: #1a1a1a;
  --border-color: #222;
  --border-light: #333;
  --text-primary: #eee;
  --text-secondary: #aaa;
  --text-muted: #666;
  --accent: #4af;
  --accent-bg: rgba(68, 170, 255, 0.08);
  --danger: #f44;
  --success: #4CAF50;
  --warning: #FF9800;
}
[data-theme="light"] {
  --bg-primary: #f5f5f5;
  --bg-secondary: #fff;
  --bg-card: #fff;
  --bg-input: #f0f0f0;
  --border-color: #ddd;
  --border-light: #ccc;
  --text-primary: #222;
  --text-secondary: #555;
  --text-muted: #999;
  --accent: #1976D2;
  --accent-bg: rgba(25, 118, 210, 0.08);
  --danger: #D32F2F;
  --success: #388E3C;
  --warning: #F57C00;
}
[data-theme="blue"] {
  --bg-primary: #0a1628;
  --bg-secondary: #0d1f3c;
  --bg-card: #0f2444;
  --bg-input: #132d52;
  --border-color: #1a3a5c;
  --border-light: #2a4a6c;
  --text-primary: #d0e0f0;
  --text-secondary: #8ab4d8;
  --text-muted: #4a7a9a;
  --accent: #64b5f6;
  --accent-bg: rgba(100, 181, 246, 0.1);
  --danger: #ef5350;
  --success: #66bb6a;
  --warning: #ffa726;
}
[data-theme="green"] {
  --bg-primary: #0a1a0a;
  --bg-secondary: #0d200d;
  --bg-card: #0f280f;
  --bg-input: #133013;
  --border-color: #1a3a1a;
  --border-light: #2a4a2a;
  --text-primary: #d0f0d0;
  --text-secondary: #8ad88a;
  --text-muted: #4a8a4a;
  --accent: #66bb6a;
  --accent-bg: rgba(102, 187, 106, 0.1);
  --danger: #ef5350;
  --success: #81c784;
  --warning: #ffa726;
}
.page {
  height: 100%;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: var(--bg-primary);
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

.composer-action-btn {
  padding: 4px 12px;
  font-size: 12px;
  border: 1px solid #333;
  background: #1a1a1a;
  color: #999;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s;
  line-height: 1.4;
}

.composer-action-btn:hover {
  background: #252525;
  color: #ddd;
  border-color: #555;
}

.composer-action-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.composer-send-btn {
  padding: 4px 20px;
  font-size: 12px;
  border: none;
  background: #ef4444;
  color: #fff;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s;
  line-height: 1.4;
}

.composer-send-btn:hover {
  background: #dc2626;
}

.composer-stop-btn {
  padding: 4px 20px;
  font-size: 12px;
  border: none;
  background: #3b82f6;
  color: #fff;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s;
  line-height: 1.4;
}

.composer-stop-btn:hover {
  background: #2563eb;
}

.composer-action-divider {
  width: 1px;
  height: 18px;
  background: #333;
  margin: 0 2px;
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

.api-source-bar {
  display: flex;
  gap: 0;
  background: #111;
  border-radius: 6px;
  padding: 2px;
  align-items: center;
}
.api-source-btn {
  flex: 1;
  padding: 6px 8px;
  font-size: 11px;
  font-weight: 500;
  border: none;
  background: transparent;
  color: #777;
  cursor: pointer;
  border-radius: 4px;
  transition: all 0.2s;
}
.api-source-btn:hover {
  color: #ccc;
  background: #1a1a2a;
}
.api-source-btn.active {
  background: #1a3a5a;
  color: #4af;
  font-weight: 600;
}
.api-source-hint {
  font-size: 9px;
  color: #555;
  padding: 0 6px;
  white-space: nowrap;
}

.sidebar-section {
  background: #0d0d0d;
  border: 1px solid #222;
  border-radius: 8px;
  padding: 8px 10px;
}
.sidebar-section-title {
  font-size: 12px;
  font-weight: 600;
  color: #ccc;
  margin-bottom: 6px;
  padding-bottom: 4px;
  border-bottom: 1px solid #1a1a1a;
  display: flex;
  align-items: center;
  gap: 4px;
}
.help-bubble {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #2a2a3a;
  color: #888;
  font-size: 9px;
  font-weight: 700;
  cursor: help;
  flex-shrink: 0;
  line-height: 1;
}
.help-bubble:hover .help-bubble-content {
  display: block;
}
.help-bubble-content {
  display: none;
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
  bottom: calc(100% + 6px);
  background: #1a1a2a;
  border: 1px solid #3a3a5a;
  border-radius: 6px;
  padding: 8px 10px;
  font-size: 10px;
  font-weight: 400;
  color: #bbb;
  line-height: 1.5;
  white-space: normal;
  width: 220px;
  z-index: 1000;
  box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  pointer-events: none;
}
.help-bubble-content::after {
  content: "";
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  border: 5px solid transparent;
  border-top-color: #3a3a5a;
}
.sidebar-field {
  margin-bottom: 6px;
}
.sidebar-field:last-child {
  margin-bottom: 0;
}
.sidebar-field label {
  display: block;
  font-size: 10px;
  color: #888;
  margin-bottom: 2px;
}

.api-source-tabs {
  display: flex;
  gap: 0;
  margin-bottom: 8px;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid #333;
}
.api-tab {
  flex: 1;
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 500;
  border: none;
  background: #1a1a1a;
  color: #888;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  transition: all 0.2s;
}
.api-tab:hover {
  background: #2a2a2a;
  color: #ccc;
}
.api-tab.active {
  background: #1a3a5a;
  color: #4af;
  font-weight: 600;
}
.tab-icon {
  font-size: 14px;
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

.current-model-info {
  background: #111;
  border-radius: 6px;
  padding: 8px 10px;
  margin-top: 8px;
}
.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 3px 0;
}
.info-label {
  font-size: 11px;
  color: #666;
}
.info-value {
  font-size: 11px;
  color: #ccc;
}
.model-name-tag {
  font-weight: 600;
  color: #4af;
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

.terminal-panel {
  height: 200px;
  display: flex;
  flex-direction: column;
  border-top: 1px solid #2a2a2a;
  background: #0a0a0a;
  transition: height 0.2s ease;
}

.terminal-panel.terminal-expanded {
  height: 400px;
}

.terminal-toolbar {
  display: flex;
  align-items: center;
  padding: 4px 8px;
  border-bottom: 1px solid #222;
  background: #111;
}

.terminal-output {
  flex: 1;
  overflow-y: auto;
  padding: 6px 8px;
  font-family: Consolas, 'Courier New', monospace;
  font-size: 11px;
}

.terminal-entry {
  margin-bottom: 4px;
}

.terminal-cmd {
  color: #ddd;
  font-size: 11px;
}

.terminal-result {
  color: #888;
  font-size: 11px;
  margin: 0;
  padding: 0;
  background: none;
  border: none;
  white-space: pre-wrap;
  word-break: break-all;
  font-family: Consolas, 'Courier New', monospace;
}

.terminal-empty {
  color: #444;
  font-size: 11px;
  text-align: center;
  padding: 16px;
}

.terminal-input-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-top: 1px solid #222;
  background: #0d0d0d;
}

.terminal-input {
  flex: 1;
  background: none;
  border: none;
  color: #eee;
  font-size: 12px;
  font-family: Consolas, 'Courier New', monospace;
  outline: none;
}

.btn-active {
  background: #1a3a5a !important;
  border-color: #4af !important;
}

.theme-btn {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  border: 2px solid #333;
  background: var(--tc, #1a1a1a);
  cursor: pointer;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.theme-btn:hover {
  border-color: #888;
}

.theme-btn.active {
  border-color: var(--accent);
  box-shadow: 0 0 8px rgba(68, 170, 255, 0.3);
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
