<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from "vue";

type MessageRole = "user" | "assistant" | "error";

type ChatMessage = {
  id: string;
  role: MessageRole;
  text: string;
  time: string;
  model?: string;
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
const apiBaseUrl = ref("http://127.0.0.1:7860");
const apiModel = ref("qwen3.6-plus");
const apiModels = ref<ModelInfo[]>([]);
const cloudModels = ref<ModelInfo[]>([]);
const loadingModels = ref(false);
const expandedModelId = ref("");
const hardwareInfo = ref<any>(null);
const apiServiceRunning = ref(false);
const qwenToken = ref("");
const qwenAccountCount = ref(-1);
const qwenRegisterBusy = ref(false);
const qwenAccounts = ref<any[]>([]);

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

function makeId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

const userName = ref("你");
const assistantName = ref("助手");
const pendingQueue = ref<string[]>([]);

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
  apiBaseUrl.value = settings.API_BASE_URL || "http://127.0.0.1:7860";
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

async function clearMessages() {
  messages.value = [];
  await createSession();
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
        const portMatch = apiBaseUrl.value.match(/:(\d+)/);
        const port = portMatch ? parseInt(portMatch[1]) : 7860;
        const result = await callBackend("startQwen2Api", JSON.stringify({ port }));
        if (result && result.ok) {
          apiBaseUrl.value = result.baseUrl || `http://127.0.0.1:${port}`;
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

onMounted(async () => {
  const appState = await callBackend("getState");
  if (appState) {
    sessionId.value = appState.sessionId || "";
    workspacePath.value = appState.workspacePath || "";
    isBusy.value = Boolean(appState.busy);
    applySettings(appState.settings || {});
  }

  detectHardware();

  if (runMode.value === "ollama") {
    loadCloudModels("ollama");
  } else if (runMode.value === "api") {
    checkApiServiceStatus();
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
          const newText = payload.text;
          if (target.text === "" && newText.trim() === "") return;
          target.text += newText;
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
                if (target && !target.text.trim()) {
                  target.text = "[模型未返回文本]";
                }
              }, 500);
            }
            processQueue();
          }
        }
      } catch {}
    });

    backend.modelsLoaded.connect((jsonStr: string) => {
      try {
        const payload = JSON.parse(jsonStr);
        loadingModels.value = false;
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
});
</script>

<template>
  <div class="page">
    <section class="flowbar">
      <span>1. 选择模型</span>
      <span>2. 配置参数</span>
      <span>3. 打开项目并执行编码任务</span>
    </section>

    <main class="workbench" :class="{ single: !showPanel }">
      <section class="chat card">
        <div class="toolbar">
          <div class="session">会话：{{ sessionId || "未创建" }}</div>
          <div class="actions">
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
            </div>
            <pre v-if="m.text">{{ m.text }}</pre>
            <pre v-else class="thinking">思考中<span class="dots">...</span></pre>
          </article>
        </div>

        <div class="composer">
          <textarea
            v-model="inputText"
            placeholder="输入编码任务（Enter 发送，Shift+Enter 换行）"
            @keydown.enter.exact.prevent="sendMessage"
          />
          <div class="composer-foot">
            <span>{{ isBusy ? (pendingQueue.length > 0 ? `执行中... 排队${pendingQueue.length}条` : "执行中...") : "就绪" }}</span>
            <div>
              <button class="btn-blue" @click="clearMessages" :disabled="isBusy">新会话</button>
              <button class="btn-red" @click="sendMessage">发送任务</button>
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
        </template>

        <template v-else-if="runMode === 'api'">
          <label class="field">
            <span>API 服务地址</span>
            <input v-model="apiBaseUrl" placeholder="http://127.0.0.1:7860" />
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
            <button
              class="btn-blue"
              style="width: 100%; margin-top: 6px;"
              @click="apiStepAutoRun"
              :disabled="apiStepBusy || apiStepProgress >= apiSteps.length"
            >
              {{ apiStepBusy ? apiStepMessage : (apiStepProgress >= apiSteps.length ? '✓ API 服务已就绪' : '▶ 一键启动') }}
            </button>
          </div>

          <label class="field">
            <span>API Key</span>
            <input v-model="apiKey" type="text" placeholder="自动获取或手动输入" />
          </label>

          <div class="qwen-account-section">
            <div class="qwen-account-header">
              <span>上游账户</span>
              <span v-if="qwenAccountCount >= 0" :class="['account-badge', qwenAccountCount > 0 ? 'ok' : 'warn']">
                {{ qwenAccountCount }} 个
              </span>
              <button class="btn-blue btn-sm" @click="checkQwenAccounts" style="margin-left: auto;">刷新</button>
            </div>
            <p v-if="qwenAccountCount === 0" class="hint warn" style="margin: 4px 0;">
              未添加上游账户，AI 对话将返回 500 错误。请点击下方按钮自动注册。
            </p>
            <div v-if="qwenAccounts.length > 0" class="account-list">
              <div v-for="acc in qwenAccounts" :key="acc.email" class="account-row">
                <span :class="['account-status', acc.valid ? 'valid' : 'invalid']">●</span>
                <span class="account-email">{{ acc.email }}</span>
                <span v-if="!acc.valid" class="account-err">{{ acc.status_code || '不可用' }}</span>
                <button class="btn-icon btn-del" @click="deleteQwenAccount(acc.email)" title="删除此账户">✕</button>
              </div>
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

          <div v-if="apiModels.length > 0" class="model-list">
            <div v-for="m in apiModels" :key="m.id" class="model-row-wrapper">
              <div
                :class="['model-row', { selected: apiModel === m.id }]"
                @click="apiModel = m.id"
              >
                <div class="model-row-info">
                  <span class="model-name">{{ displayName(m.name || m.id) }}</span>
                  <span v-if="m.toolSupport === true" class="tool-badge ok">★ 工具</span>
                  <span v-else-if="m.toolSupport === false" class="tool-badge no">无工具</span>
                </div>
                <button class="btn-icon" :class="{ active: expandedModelId === m.id }" @click.stop="toggleModelSettings(m.id)">⚙</button>
              </div>
              <div v-if="expandedModelId === m.id" class="model-settings">
                <ModelSettingsPanel :config="getModelConfig(m.id)" :hint="getAutoConfigHint(m.id)" @auto-configure="autoConfigure(m.id)" />
              </div>
            </div>
          </div>

          <label v-if="apiModels.length > 0" class="field" style="margin-top: 8px;">
            <span>模型名称（可手动修改）</span>
            <input v-model="apiModel" placeholder="qwen3.6-plus" />
          </label>

          <p v-if="apiModel" class="hint ok">
            ★ API 模式支持工具调用，可使用全功能编程。
          </p>
        </template>

        <template v-else>
          <label class="field">
            <span>Ollama 地址</span>
            <div class="model-loader">
              <input v-model="ollamaBaseUrl" placeholder="http://127.0.0.1:11434" />
              <button class="btn-blue btn-sm" @click="detectModels" :disabled="loadingModels">
                {{ loadingModels ? "检测中..." : "🔍 检测" }}
              </button>
              <button class="btn-auto btn-sm" @click="loadRecommendations" :disabled="loadingModels">
                💡 推荐
              </button>
            </div>
          </label>

          <div v-if="showRecommendations && recommendedModels.length > 0" class="recommend-section">
            <div class="recommend-header">
              <span>推荐安装</span>
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
            </div>
          </div>

          <div v-if="cloudModels.length > 0" class="model-list">
            <div v-for="m in cloudModels" :key="m.id" class="model-row-wrapper">
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
          <div v-else-if="!loadingModels" class="empty-hint">
            <p>点击"检测"加载并检测可用模型</p>
          </div>

          <p v-if="ollamaModel && selectedOllamaModelToolSupport === false" class="hint warn">
            ⚠ 该模型不支持工具调用，编程功能将受限。建议选择带 ★ 标记的模型。
          </p>
          <p v-else-if="ollamaModel && selectedOllamaModelToolSupport === true" class="hint ok">
            ★ 该模型支持工具调用，可使用全功能编程。
          </p>
        </template>

        <div class="field-group-title">对话设置</div>
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

.msg .thinking {
  color: #666;
  font-style: italic;
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
</style>
