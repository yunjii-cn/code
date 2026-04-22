<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";

type MessageRole = "user" | "assistant" | "error";

type ChatMessage = {
  id: string;
  role: MessageRole;
  text: string;
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
const sessionId = ref("");
const workspacePath = ref("");
const inputText = ref("");
const showPanel = ref(true);
const noticeText = ref("");
const noticeType = ref<"ok" | "warn">("ok");
const messages = ref<ChatMessage[]>([]);
const currentAssistantId = ref("");

const runMode = ref<"cloud" | "ollama">("ollama");
const apiKey = ref("");
const ollamaBaseUrl = ref("http://127.0.0.1:11434");
const ollamaModel = ref("");
const cloudModels = ref<ModelInfo[]>([]);
const loadingModels = ref(false);
const expandedModelId = ref("");
const hardwareInfo = ref<any>(null);

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
  const activeModel = runMode.value === "ollama" ? ollamaModel.value : settings.ANTHROPIC_MODEL;
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
  }
});

function makeId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function roleLabel(role: MessageRole) {
  if (role === "user") return "你";
  if (role === "assistant") return "助手";
  return "错误";
}

function addMessage(role: MessageRole, text: string) {
  const item: ChatMessage = { id: makeId(), role, text };
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
  settings.API_TIMEOUT_MS = data.API_TIMEOUT_MS ?? "3000000";
  settings.AI_LANGUAGE = data.AI_LANGUAGE ?? "zh";
  settings.AI_TEMPERATURE = data.AI_TEMPERATURE ?? "";
  settings.AI_MAX_TOKENS = data.AI_MAX_TOKENS ?? "";
  settings.SYSTEM_PROMPT = data.SYSTEM_PROMPT ?? "";

  runMode.value = settings.MODEL_PROVIDER === "ollama" ? "ollama" : "cloud";
  const cloudKey = settings.ANTHROPIC_API_KEY || settings.ANTHROPIC_AUTH_TOKEN || "";
  apiKey.value = cloudKey === "ollama-local" ? "" : cloudKey;
  ollamaBaseUrl.value = settings.OLLAMA_BASE_URL || "http://127.0.0.1:11434";
  ollamaModel.value = settings.OLLAMA_MODEL || "";

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
  if (isBusy.value) return;
  const text = inputText.value.trim();
  if (!text) return;

  addMessage("user", text);
  inputText.value = "";
  currentAssistantId.value = addMessage("assistant", "");
  isBusy.value = true;

  const activeConfig = getActiveModelConfig();

  const result = await callBackend("sendMessage", JSON.stringify({
    prompt: text,
    provider: runMode.value === "ollama" ? "ollama" : "anthropic",
    model: runMode.value === "ollama" ? ollamaModel.value.trim() : (settings.ANTHROPIC_MODEL || "").trim(),
    ai_language: activeConfig.language,
    ai_temperature: activeConfig.temperature,
    ai_max_tokens: activeConfig.maxTokens,
    system_prompt: activeConfig.systemPrompt,
  }));

  if (!result?.ok) {
    if (result?.sessionId) sessionId.value = result.sessionId;
    messages.value = messages.value.filter((m) => m.id !== currentAssistantId.value);
    if (result?.stopped) {
      addMessage("assistant", "任务已停止。");
    } else {
      addMessage("error", result?.error || "请求失败");
    }
    isBusy.value = false;
    return;
  }

  if (result?.sessionId) sessionId.value = result.sessionId;
}

async function stopMessage() {
  const result = await callBackend("stopMessage");
  if (result?.sessionId) sessionId.value = result.sessionId;
  if (!result.ok && result.error) showNotice(result.error, "warn");
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
  sessionId.value = result.sessionId;
  addMessage("assistant", `新会话已创建：${result.sessionId}`);
}

function clearMessages() {
  messages.value = [];
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
  }

  addMessage("assistant", "选择模型并配置参数，打开项目目录后即可下达编码任务。");

  const backend = await getBackend();
  if (backend) {
    backend.deltaReceived.connect((jsonStr: string) => {
      try {
        const payload = JSON.parse(jsonStr);
        if (!payload?.text) return;
        const target = messages.value.find((m) => m.id === currentAssistantId.value);
        if (target) target.text += payload.text;
      } catch {}
    });

    backend.statusReceived.connect((jsonStr: string) => {
      try {
        const payload = JSON.parse(jsonStr);
        if (payload && typeof payload.busy === "boolean") {
          isBusy.value = payload.busy;
          if (!payload.busy && currentAssistantId.value) {
            const checkId = currentAssistantId.value;
            setTimeout(() => {
              const target = messages.value.find((m) => m.id === checkId);
              if (target && !target.text.trim()) {
                target.text = "[模型未返回文本]";
              }
            }, 500);
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
        const source = (payload.models?.[0]?.provider) || "ollama";
        const label = source === "openrouter" ? " OpenRouter" : source === "anthropic" ? " Anthropic" : " Ollama";
        showNotice(`已加载 ${cloudModels.value.length} 个${label}模型`, "ok");
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
            <label>{{ roleLabel(m.role) }}</label>
            <pre>{{ m.text }}</pre>
          </article>
        </div>

        <div class="composer">
          <textarea
            v-model="inputText"
            :disabled="isBusy"
            placeholder="输入编码任务（Enter 发送，Shift+Enter 换行）"
            @keydown.enter.exact.prevent="sendMessage"
          />
          <div class="composer-foot">
            <span>{{ isBusy ? "执行中..." : "就绪" }}</span>
            <div>
              <button class="btn-blue" @click="clearMessages" :disabled="isBusy">清空</button>
              <button class="btn-red" @click="sendMessage" :disabled="isBusy">发送任务</button>
            </div>
          </div>
        </div>
      </section>

      <aside v-if="showPanel" class="panel card">
        <div class="panel-header">
          <h2>模型与配置</h2>
          <div class="mode-switch">
            <button :class="['mode-btn', { active: runMode === 'cloud' }]" @click="runMode = 'cloud'">☁️ 云端</button>
            <button :class="['mode-btn', { active: runMode === 'ollama' }]" @click="runMode = 'ollama'">🦙 Ollama</button>
          </div>
        </div>

        <template v-if="runMode === 'cloud'">
          <label class="field">
            <span>API Key</span>
            <input v-model="apiKey" type="password" placeholder="输入你的 API Key" />
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

        <template v-else>
          <label class="field">
            <span>Ollama 地址</span>
            <div class="model-loader">
              <input v-model="ollamaBaseUrl" placeholder="http://127.0.0.1:11434" />
              <button class="btn-blue btn-sm" @click="loadCloudModels('ollama')" :disabled="loadingModels">
                {{ loadingModels ? "加载中..." : "刷新" }}
              </button>
            </div>
          </label>

          <div v-if="cloudModels.length > 0" class="model-list">
            <div v-for="m in cloudModels" :key="m.id" class="model-row-wrapper">
              <div
                :class="['model-row', { selected: ollamaModel === m.id, 'no-tool': m.toolSupport === false }]"
                @click="selectModel(m.id)"
              >
                <div class="model-row-info">
                  <span class="model-name">{{ m.name }}</span>
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
          <div v-else-if="!loadingModels" class="empty-hint">
            <p>点击"刷新"加载可用模型</p>
          </div>

          <p v-if="ollamaModel && selectedOllamaModelToolSupport === false" class="hint warn">
            ⚠ 该模型不支持工具调用，编程功能将受限。建议选择带 ★ 标记的模型。
          </p>
          <p v-else-if="ollamaModel && selectedOllamaModelToolSupport === true" class="hint ok">
            ★ 该模型支持工具调用，可使用全功能编程。
          </p>
        </template>

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

.msg label {
  display: block;
  margin-bottom: 6px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #888888;
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
