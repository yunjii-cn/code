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
};

// ── QWebChannel 桥接层 ──
// 替代 Electron 的 window.desktopApi
// 所有方法返回 JSON 字符串，需要 JSON.parse

let _backend: any = null;

async function getBackend(): Promise<any> {
  if (_backend) return _backend;
  // QWebChannel 初始化
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
  if (!backend) return null;
  const result = await backend[method](...args);
  if (typeof result === "string") {
    try { return JSON.parse(result); } catch { return result; }
  }
  return result;
}

// ── 响应式状态 ──

const isBusy = ref(false);
const sessionId = ref("");
const workspacePath = ref("");
const inputText = ref("");
const showSettings = ref(true);
const noticeText = ref("");
const noticeType = ref<"ok" | "warn">("ok");
const messages = ref<ChatMessage[]>([]);
const currentAssistantId = ref("");

const runMode = ref<"cloud" | "ollama">("cloud");
const apiKey = ref("");
const selectedModelId = ref("");
const selectedModelProvider = ref<"openrouter" | "anthropic" | "">("");
const ollamaBaseUrl = ref("http://127.0.0.1:11434");
const ollamaModel = ref("");
const cloudModels = ref<Array<{ id: string; name: string; provider: string; toolSupport?: boolean }>>([]);
const loadingModels = ref(false);

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

function inferProviderByModel(model: string) {
  if ((model || "").startsWith("openrouter/")) return "openrouter";
  return "anthropic";
}

function inferProviderByKey(key: string) {
  const k = (key || "").trim();
  if (k.startsWith("sk-or-")) return "openrouter";
  return "";
}

function resolveCloudProvider(model: string, key: string, selected: "openrouter" | "anthropic" | "") {
  const byKey = inferProviderByKey(key);
  if (byKey) return byKey as "openrouter" | "anthropic";
  if (selected) return selected;
  return inferProviderByModel(model) as "openrouter" | "anthropic";
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

  runMode.value = settings.MODEL_PROVIDER === "ollama" ? "ollama" : "cloud";
  const cloudKey = settings.ANTHROPIC_API_KEY || settings.ANTHROPIC_AUTH_TOKEN || "";
  apiKey.value = cloudKey === "ollama-local" ? "" : cloudKey;
  selectedModelId.value = settings.ANTHROPIC_MODEL || "openrouter/auto";
  selectedModelProvider.value = "openrouter";
  ollamaBaseUrl.value = settings.OLLAMA_BASE_URL || "http://127.0.0.1:11434";
  ollamaModel.value = settings.OLLAMA_MODEL || "";
}

function onModelPicked(value: string) {
  selectedModelId.value = value;
  const found = cloudModels.value.find((m) => m.id === value);
  selectedModelProvider.value = (found?.provider as "openrouter" | "anthropic") || inferProviderByModel(value);
}

async function sendMessage() {
  if (isBusy.value) return;
  const text = inputText.value.trim();
  if (!text) return;

  addMessage("user", text);
  inputText.value = "";
  currentAssistantId.value = addMessage("assistant", "");
  isBusy.value = true;

  const result = await callBackend("sendMessage", JSON.stringify({
    prompt: text,
    provider: runMode.value === "ollama" ? "ollama" : "anthropic",
    model: runMode.value === "ollama" ? ollamaModel.value.trim() : (settings.ANTHROPIC_MODEL || "").trim(),
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

  const target = messages.value.find((m) => m.id === currentAssistantId.value);
  if (result?.sessionId) sessionId.value = result.sessionId;
  if (target && !target.text.trim()) {
    target.text = result.text || "[模型未返回文本]";
  }
  isBusy.value = false;
}

async function stopMessage() {
  const result = await callBackend("stopMessage");
  if (result?.sessionId) sessionId.value = result.sessionId;
  if (!result.ok && result.error) showNotice(result.error, "warn");
}

async function chooseWorkspace() {
  // Python 端弹出文件夹选择对话框
  const result = await callBackend("chooseWorkspace");
  if (!result?.ok) {
    if (result?.error) showNotice(result.error, "warn");
    return;
  }
  // 等待 Python 端完成选择后刷新路径
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
  if (runMode.value === "ollama") {
    if (!ollamaModel.value.trim()) {
      showNotice("请先填写 Ollama 模型", "warn");
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
    };

    const saved = await callBackend("saveSettings", JSON.stringify(payload));
    applySettings(saved);

    const selectedModel = cloudModels.value.find((m) => m.id === ollamaModel.value.trim());
    if (selectedModel && selectedModel.toolSupport === false) {
      showNotice("Ollama 配置已保存。⚠ 该模型不支持工具调用，编程功能将受限。建议选择带 ★ 标记的模型。", "warn");
    } else {
      showNotice("Ollama 配置已保存。", "ok");
    }
    return;
  }

  if (!apiKey.value.trim()) {
    showNotice("请先填写 API Key", "warn");
    return;
  }
  const cloudModel = "openrouter/auto";
  const provider: "openrouter" = "openrouter";
  const cloudBaseUrl = cloudBaseUrlByProvider(provider);
  selectedModelId.value = cloudModel;
  selectedModelProvider.value = "openrouter";

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
  };

  const saved = await callBackend("saveSettings", JSON.stringify(payload));
  applySettings(saved);
  showNotice("已保存。云端模型已固定为 openrouter/auto。", "ok");
}

async function clearModelFields() {
  if (isBusy.value) return;
  const saved = await callBackend("clearModelSettings");
  applySettings(saved);
  selectedModelId.value = "";
  selectedModelProvider.value = "";
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
    if (!result.ok) {
      showNotice(result.error || "模型列表加载失败", "warn");
      return;
    }
    cloudModels.value = result.models || [];
    showNotice(`已加载 ${cloudModels.value.length} 个${source === "openrouter" ? " OpenRouter" : source === "anthropic" ? " Anthropic" : " Ollama"}模型`, "ok");
  } finally {
    loadingModels.value = false;
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

  if (runMode.value === "ollama") {
    loadCloudModels("ollama");
  }

  addMessage("assistant", "先完成模型配置，再打开项目目录，直接下达编码任务。");

  // 监听 QWebChannel 信号
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
        }
      } catch {}
    });
  }
});
</script>

<template>
  <div class="page">
    <header class="hero">
      <div class="badge">YUNJII-CODE</div>
      <h1>云集智能编程工作站</h1>
      <p>高效智能的编程助手，让代码创作更简单。</p>
      <div class="path-chip">当前项目：{{ workspacePath || "未选择" }}</div>
    </header>

    <section class="flowbar">
      <span>1. 选择模式（云端 / Ollama）</span>
      <span>2. 配置模型</span>
      <span>3. 打开项目并执行编码任务</span>
    </section>

    <main class="workbench" :class="{ single: !showSettings }">
      <section class="chat card">
        <div class="toolbar">
          <div class="session">会话：{{ sessionId || "未创建" }}</div>
          <div class="actions">
            <button class="btn-blue" @click="chooseWorkspace">打开项目</button>
            <button class="btn-blue" @click="createSession" :disabled="isBusy">新会话</button>
            <button class="btn-red" @click="stopMessage" :disabled="!isBusy">停止</button>
            <button class="btn-blue" @click="showSettings = !showSettings">{{ showSettings ? "隐藏设置" : "显示设置" }}</button>
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

      <aside v-if="showSettings" class="settings card">
        <h2>快速配置</h2>

        <label class="field">
          <span>运行模式</span>
          <select v-model="runMode">
            <option value="cloud">云端（OpenRouter / Anthropic）</option>
            <option value="ollama">Ollama（本地）</option>
          </select>
        </label>

        <template v-if="runMode === 'cloud'">
          <label class="field">
            <span>API Key</span>
            <input v-model="apiKey" type="password" placeholder="输入你的 API Key" />
          </label>

          <label class="field">
            <span>云端模型（固定）</span>
            <input value="openrouter/auto" readonly />
          </label>
        </template>

        <template v-else>
          <label class="field">
            <span>Ollama 地址</span>
            <input v-model="ollamaBaseUrl" placeholder="http://127.0.0.1:11434" />
          </label>
          <label class="field">
            <span>Ollama 模型</span>
            <div class="model-loader">
              <select v-if="cloudModels.length > 0" v-model="ollamaModel">
                <option value="">选择本地模型...</option>
                <option v-for="m in cloudModels" :key="m.id" :value="m.id">
                  {{ m.name }}{{ m.toolSupport === true ? " ★" : m.toolSupport === false ? " (不支持工具调用)" : "" }}
                </option>
              </select>
              <input v-else v-model="ollamaModel" placeholder="例如 qwen3:4b" />
              <button class="btn-blue" @click="loadCloudModels('ollama')" :disabled="loadingModels">
                {{ loadingModels ? "加载中..." : "刷新" }}
              </button>
            </div>
          </label>
          <p v-if="ollamaModel && selectedOllamaModelToolSupport === false" class="hint warn">
            ⚠ 该模型不支持工具调用（Tool Calling），编程功能将受限。建议选择带 ★ 标记的模型（如 qwen3、llama3.1 等）。
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

<style scoped>
.page {
  height: 100%;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  background: #0d0d0d;
}

.hero {
  border-radius: 8px;
  border: 1px solid #2a2a2a;
  background: #1a1a1a;
  padding: 24px;
}

.badge {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.5px;
  color: #ffffff;
  background: #3b82f6;
}

.hero h1 {
  margin: 16px 0 8px;
  font-size: 28px;
  line-height: 1.2;
  color: #ffffff;
  font-weight: 700;
}

.hero p {
  margin: 0;
  color: #888888;
  line-height: 1.5;
  font-size: 14px;
}

.path-chip {
  margin-top: 16px;
  border-radius: 4px;
  border: 1px solid #2a2a2a;
  background: #262626;
  color: #a3a3a3;
  font-size: 12px;
  padding: 8px 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.flowbar {
  border-radius: 8px;
  border: 1px solid #2a2a2a;
  background: #1a1a1a;
  color: #888888;
  font-size: 13px;
  padding: 12px 16px;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.workbench {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 1fr 360px;
  gap: 16px;
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
  padding: 16px;
  gap: 16px;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
  padding-bottom: 12px;
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
  gap: 12px;
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
  gap: 12px;
  padding-top: 12px;
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

.settings {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow: auto;
}

.settings h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: #ffffff;
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

.setting-actions,
.model-loader {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
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
