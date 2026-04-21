const { app, BrowserWindow, ipcMain, dialog } = require("electron");
const path = require("node:path");
const { randomUUID } = require("node:crypto");
const { spawn } = require("node:child_process");
const readline = require("node:readline");
const http = require("node:http");
const fs = require("node:fs");

const PROJECT_ROOT = path.resolve(__dirname, "..");
const CLI_ENTRY = path.join(PROJECT_ROOT, "bin", "claude-code-tudou");
const ENV_PATH = path.join(PROJECT_ROOT, ".env");
const BUN_DIR = path.join(PROJECT_ROOT, "bun", "bun-windows-x64");
const BUN_EXE = path.join(BUN_DIR, "bun.exe");

const MODEL_KEYS = [
  "ANTHROPIC_MODEL",
  "ANTHROPIC_DEFAULT_SONNET_MODEL",
  "ANTHROPIC_DEFAULT_HAIKU_MODEL",
  "ANTHROPIC_DEFAULT_OPUS_MODEL",
  "OLLAMA_MODEL",
];

const SETTINGS_KEYS = [
  "MODEL_PROVIDER",
  "ANTHROPIC_BASE_URL",
  "ANTHROPIC_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "ANTHROPIC_MODEL",
  "ANTHROPIC_DEFAULT_SONNET_MODEL",
  "ANTHROPIC_DEFAULT_HAIKU_MODEL",
  "ANTHROPIC_DEFAULT_OPUS_MODEL",
  "OLLAMA_BASE_URL",
  "OLLAMA_MODEL",
  "API_TIMEOUT_MS",
  "DISABLE_TELEMETRY",
  "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
];

const OLLAMA_AGENT_MAX_STEPS = 10;
const TOOL_TEXT_LIMIT = 12000;
const COMMAND_OUTPUT_LIMIT = 8000;

let mainWindow = null;
let activeSessionId = randomUUID();
const startedSessions = new Set();
let isBusy = false;
let currentWorkspace = PROJECT_ROOT;
let activeRequest = null;
const stoppedRequestIds = new Set();

// ── Ollama 格式转换代理（支持 Tool Calling） ──
let ollamaProxyServer = null;
let ollamaProxyPort = 0;
let ollamaProxyModel = "qwen3:8b";

function startOllamaProxy(targetBaseUrl, model) {
  if (ollamaProxyServer) return Promise.resolve(ollamaProxyPort);
  const targetUrl = new URL(targetBaseUrl);
  ollamaProxyModel = model || "qwen3:8b";

  ollamaProxyServer = http.createServer((req, res) => {
    // 只处理 POST /v1/messages
    if (req.method === "POST" && req.url?.startsWith("/v1/messages")) {
      const chunks = [];
      req.on("data", (chunk) => chunks.push(chunk));
      req.on("end", () => {
        let bodyStr = Buffer.concat(chunks).toString("utf-8");
        let anthropicBody;

        try {
          anthropicBody = JSON.parse(bodyStr);
        } catch {
          res.writeHead(400);
          res.end(JSON.stringify({ error: "Invalid JSON" }));
          return;
        }

        // ── 转换 Anthropic Messages → Ollama Chat ──
        const ollamaMessages = [];
        let systemPrompt = "";
        if (typeof anthropicBody.system === "string") {
          systemPrompt = anthropicBody.system;
        } else if (Array.isArray(anthropicBody.system)) {
          systemPrompt = anthropicBody.system.map((b) => (b.type === "text" ? b.text : "")).filter(Boolean).join("\n");
        }

        if (Array.isArray(anthropicBody.messages)) {
          for (const msg of anthropicBody.messages) {
            if (msg.role === "user") {
              const toolResults = [];
              const textParts = [];
              if (Array.isArray(msg.content)) {
                for (const block of msg.content) {
                  if (block.type === "tool_result") toolResults.push(block);
                  else if (block.type === "text") textParts.push(block.text);
                }
              } else if (typeof msg.content === "string") {
                textParts.push(msg.content);
              }
              for (const tr of toolResults) {
                const content = typeof tr.content === "string" ? tr.content
                  : Array.isArray(tr.content) ? tr.content.map(b => b.type === "text" ? b.text : "").filter(Boolean).join("\n")
                  : "";
                ollamaMessages.push({ role: "tool", name: tr.tool_use_id || "unknown", content: content || "(no output)" });
              }
              const text = textParts.filter(Boolean).join("\n");
              if (text) ollamaMessages.push({ role: "user", content: text });
            } else if (msg.role === "assistant") {
              const toolCalls = [];
              const textParts = [];
              if (Array.isArray(msg.content)) {
                for (const block of msg.content) {
                  if (block.type === "tool_use") toolCalls.push(block);
                  else if (block.type === "text") textParts.push(block.text);
                }
              } else if (typeof msg.content === "string") {
                textParts.push(msg.content);
              }
              const assistantMsg = { role: "assistant", content: "" };
              const text = textParts.filter(Boolean).join("\n");
              if (text) assistantMsg.content = text;
              const ollamaTC = [];
              for (const tc of toolCalls) {
                let args = {};
                if (tc.input) args = typeof tc.input === "string" ? JSON.parse(tc.input) : tc.input;
                ollamaTC.push({ function: { name: tc.name, arguments: args } });
              }
              if (ollamaTC.length > 0) assistantMsg.tool_calls = ollamaTC;
              ollamaMessages.push(assistantMsg);
            }
          }
        }

        // 转换 tools
        const ollamaTools = [];
        if (Array.isArray(anthropicBody.tools)) {
          for (const tool of anthropicBody.tools) {
            if (tool.type === "custom") continue;
            ollamaTools.push({
              type: "function",
              function: {
                name: tool.name,
                description: tool.description || "",
                parameters: tool.input_schema || { type: "object", properties: {} },
              },
            });
          }
        }

        const stream = anthropicBody.stream === true;
        const ollamaBody = {
          model: ollamaProxyModel,
          messages: ollamaMessages,
          stream: stream,
          options: { num_ctx: 32768 },
        };
        if (systemPrompt) ollamaBody.system = systemPrompt;
        if (ollamaTools.length > 0) ollamaBody.tools = ollamaTools;

        // ── 发送请求到 Ollama（支持自动降级重试） ──
        function sendToOllama(bodyObj, isRetry) {
          const bodyStr = JSON.stringify(bodyObj);
          const opts = {
            hostname: targetUrl.hostname,
            port: targetUrl.port || 11434,
            path: "/api/chat",
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              "Content-Length": Buffer.byteLength(bodyStr),
            },
          };

          const proxyReq = http.request(opts, (proxyRes) => {
            if (proxyRes.statusCode !== 200) {
              const errChunks = [];
              proxyRes.on("data", (c) => errChunks.push(c));
              proxyRes.on("end", () => {
                const errBody = Buffer.concat(errChunks).toString("utf-8");

                // 检测 "does not support tools" 错误，自动去掉 tools 重试
                if (!isRetry && errBody.includes("does not support tools")) {
                  const retryBody = { ...bodyObj };
                  delete retryBody.tools;
                  // 将工具描述注入 system prompt，让模型通过提示模拟工具调用
                  if (ollamaTools.length > 0) {
                    const toolDescs = ollamaTools.map((t) => {
                      const p = JSON.stringify(t.function.parameters || {});
                      return "- " + t.function.name + ": " + (t.function.description || "") + " Params: " + p;
                    }).join("\n");
                    const BT = String.fromCharCode(96); // backtick
                    const toolPrompt = "\n\nYou have access to the following tools. To call a tool, output a JSON block enclosed in " + BT + BT + BT + " tags like this:\n" + BT + BT + BT + "\n{\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}\n" + BT + BT + BT + "\nYou can call multiple tools. After each tool call, wait for the result in the next user message enclosed in <tool_result> tags.\nAvailable tools:\n" + toolDescs;
                    retryBody.system = (retryBody.system || "") + toolPrompt;
                  }
                  sendToOllama(retryBody, true);
                  return;
                }

                const anthropicError = {
                  type: "error",
                  error: {
                    type: "invalid_request_error",
                    message: "Ollama error (" + proxyRes.statusCode + "): " + errBody,
                  },
                };
                res.writeHead(proxyRes.statusCode, { "Content-Type": "application/json" });
                res.end(JSON.stringify(anthropicError));
              });
              return;
            }

            if (stream) {
              handleStreamResponse(proxyRes, res, isRetry);
            } else {
              handleNonStreamResponse(proxyRes, res, isRetry);
            }
          });

          proxyReq.on("error", (err) => {
            const anthropicError = {
              type: "error",
              error: { type: "api_error", message: "Proxy connection error: " + err.message },
            };
            if (!res.headersSent) res.writeHead(502, { "Content-Type": "application/json" });
            res.end(JSON.stringify(anthropicError));
          });

          proxyReq.write(bodyStr);
          proxyReq.end();
        }

        sendToOllama(ollamaBody, false);
      });
    } else {
      res.writeHead(404);
      res.end(JSON.stringify({ error: "Not found" }));
    }
  });

  // 异步启动代理，等待端口分配
  return new Promise((resolve) => {
    ollamaProxyServer.listen(0, "127.0.0.1", () => {
      ollamaProxyPort = ollamaProxyServer.address().port;
      resolve(ollamaProxyPort);
    });
  });

  function handleStreamResponse(proxyRes, res, isRetry) {
    const msgId = "msg_" + Date.now().toString(36);
    let inputTokens = 0;
    let outputTokens = 0;

    if (isRetry) {
      // ── 降级重试模式：先收集完整响应，解析文本中的工具调用 JSON，再发送 ──
      const respChunks = [];
      proxyRes.setEncoding("utf-8");
      proxyRes.on("data", (chunk) => {
        respChunks.push(chunk);
        // 解析 token 统计
        const lines = chunk.split("\n");
        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed) continue;
          try {
            const parsed = JSON.parse(trimmed);
            if (parsed.prompt_eval_count) inputTokens = parsed.prompt_eval_count;
            if (parsed.eval_count) outputTokens = parsed.eval_count;
          } catch {}
        }
      });
      proxyRes.on("end", () => {
        let fullContent = "";
        for (const chunk of respChunks) {
          const lines = chunk.split("\n");
          for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed) continue;
            try {
              const parsed = JSON.parse(trimmed);
              if (parsed.message?.content) fullContent += parsed.message.content;
            } catch {}
          }
        }

        // 解析文本中的工具调用 JSON（模型在提示指导下输出的格式）
        const toolCalls = parseToolCallsFromText(fullContent);
        // 从文本中移除工具调用 JSON 块
        let cleanContent = removeToolCallBlocksFromText(fullContent);

        // 发送流式响应
        res.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-cache", Connection: "keep-alive" });
        res.write("event: message_start\ndata: " + JSON.stringify({
          type: "message_start",
          message: { id: msgId, type: "message", role: "assistant", content: [], model: ollamaProxyModel, stop_reason: null, stop_sequence: null, usage: { input_tokens: inputTokens, output_tokens: 0 } },
        }) + "\n\n");

        let contentBlockIndex = 0;
        // 文本 block
        if (cleanContent.trim()) {
          res.write("event: content_block_start\ndata: " + JSON.stringify({ type: "content_block_start", index: 0, content_block: { type: "text", text: "" } }) + "\n\n");
          res.write("event: content_block_delta\ndata: " + JSON.stringify({ type: "content_block_delta", index: 0, delta: { type: "text_delta", text: cleanContent } }) + "\n\n");
          res.write("event: content_block_stop\ndata: " + JSON.stringify({ type: "content_block_stop", index: 0 }) + "\n\n");
          contentBlockIndex = 1;
        }

        // 工具调用 blocks
        for (let i = 0; i < toolCalls.length; i++) {
          const tc = toolCalls[i];
          const toolUseId = "toolu_" + Date.now().toString(36) + "_" + i;
          res.write("event: content_block_start\ndata: " + JSON.stringify({ type: "content_block_start", index: contentBlockIndex, content_block: { type: "tool_use", id: toolUseId, name: tc.name, input: {} } }) + "\n\n");
          res.write("event: content_block_delta\ndata: " + JSON.stringify({ type: "content_block_delta", index: contentBlockIndex, delta: { type: "input_json_delta", partial_json: JSON.stringify(tc.arguments) } }) + "\n\n");
          res.write("event: content_block_stop\ndata: " + JSON.stringify({ type: "content_block_stop", index: contentBlockIndex }) + "\n\n");
          contentBlockIndex++;
        }

        if (contentBlockIndex === 0) {
          res.write("event: content_block_start\ndata: " + JSON.stringify({ type: "content_block_start", index: 0, content_block: { type: "text", text: "" } }) + "\n\n");
          res.write("event: content_block_stop\ndata: " + JSON.stringify({ type: "content_block_stop", index: 0 }) + "\n\n");
        }

        const stopReason = toolCalls.length > 0 ? "tool_use" : "end_turn";
        res.write("event: message_delta\ndata: " + JSON.stringify({ type: "message_delta", delta: { stop_reason, stop_sequence: null }, usage: { output_tokens: outputTokens || fullContent.length } }) + "\n\n");
        res.write("event: message_stop\ndata: " + JSON.stringify({ type: "message_stop" }) + "\n\n");
        res.end();
      });
      return;
    }

    // ── 正常流式模式 ──
    res.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-cache", Connection: "keep-alive" });
    let sentMessageStart = false;
    let contentBlockIndex = 0;
    let currentToolUseIndex = 0;
    let hasToolCalls = false;
    let fullContent = "";

    proxyRes.setEncoding("utf-8");
    let buffer = "";

    proxyRes.on("data", (chunk) => {
      buffer += chunk;
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        let ollamaChunk;
        try { ollamaChunk = JSON.parse(trimmed); } catch { continue; }

        if (ollamaChunk.prompt_eval_count) inputTokens = ollamaChunk.prompt_eval_count;
        if (ollamaChunk.eval_count) outputTokens = ollamaChunk.eval_count;

        if (!sentMessageStart) {
          res.write("event: message_start\ndata: " + JSON.stringify({
            type: "message_start",
            message: { id: msgId, type: "message", role: "assistant", content: [], model: ollamaProxyModel, stop_reason: null, stop_sequence: null, usage: { input_tokens: inputTokens, output_tokens: 0 } },
          }) + "\n\n");
          sentMessageStart = true;
        }

        if (ollamaChunk.message?.content) {
          if (!hasToolCalls) {
            fullContent += ollamaChunk.message.content;
            if (contentBlockIndex === 0 && !ollamaChunk.message.tool_calls) {
              res.write("event: content_block_start\ndata: " + JSON.stringify({ type: "content_block_start", index: 0, content_block: { type: "text", text: "" } }) + "\n\n");
            }
            res.write("event: content_block_delta\ndata: " + JSON.stringify({ type: "content_block_delta", index: 0, delta: { type: "text_delta", text: ollamaChunk.message.content } }) + "\n\n");
          }
        }

        if (ollamaChunk.message?.tool_calls && ollamaChunk.message.tool_calls.length > 0) {
          if (contentBlockIndex === 0 && fullContent && !hasToolCalls) {
            res.write("event: content_block_stop\ndata: " + JSON.stringify({ type: "content_block_stop", index: 0 }) + "\n\n");
            contentBlockIndex = 1;
          }
          hasToolCalls = true;
          for (const tc of ollamaChunk.message.tool_calls) {
            const toolUseId = "toolu_" + Date.now().toString(36) + "_" + currentToolUseIndex;
            res.write("event: content_block_start\ndata: " + JSON.stringify({ type: "content_block_start", index: contentBlockIndex, content_block: { type: "tool_use", id: toolUseId, name: tc.function?.name || "unknown", input: {} } }) + "\n\n");
            res.write("event: content_block_delta\ndata: " + JSON.stringify({ type: "content_block_delta", index: contentBlockIndex, delta: { type: "input_json_delta", partial_json: JSON.stringify(tc.function?.arguments || {}) } }) + "\n\n");
            res.write("event: content_block_stop\ndata: " + JSON.stringify({ type: "content_block_stop", index: contentBlockIndex }) + "\n\n");
            contentBlockIndex++;
            currentToolUseIndex++;
          }
        }

        if (ollamaChunk.done) {
          if (!hasToolCalls && contentBlockIndex === 0) {
            res.write("event: content_block_stop\ndata: " + JSON.stringify({ type: "content_block_stop", index: 0 }) + "\n\n");
          }
          const stopReason = hasToolCalls ? "tool_use" : (ollamaChunk.done_reason === "length" ? "max_tokens" : "end_turn");
          res.write("event: message_delta\ndata: " + JSON.stringify({ type: "message_delta", delta: { stop_reason, stop_sequence: null }, usage: { output_tokens: outputTokens || fullContent.length } }) + "\n\n");
          res.write("event: message_stop\ndata: " + JSON.stringify({ type: "message_stop" }) + "\n\n");
        }
      }
    });

    proxyRes.on("end", () => {
      if (!sentMessageStart) {
        res.write("event: message_start\ndata: " + JSON.stringify({ type: "message_start", message: { id: msgId, type: "message", role: "assistant", content: [{ type: "text", text: "" }], model: ollamaProxyModel, stop_reason: "end_turn", stop_sequence: null, usage: { input_tokens: 0, output_tokens: 0 } } }) + "\n\n");
      }
      res.end();
    });
  }

  // ── 从文本中解析工具调用 JSON（降级重试模式使用） ──
  function parseToolCallsFromText(text) {
    const calls = [];
    // 匹配多种格式：代码块中的JSON、独立的JSON对象（含name+arguments字段）
    const patterns = [
      /\`\`\`(?:json)?\s*\n?([\s\S]*?)\n?\`\`\`/g,
      /\{\s*"name"\s*:\s*"([^"]+)"\s*,\s*"arguments"\s*:\s*(\{[^}]*\})\s*\}/g,
    ];
    const seen = new Set();
    for (const pattern of patterns) {
      let match;
      while ((match = pattern.exec(text)) !== null) {
        let jsonStr = match[1] || match[0];
        try {
          const parsed = JSON.parse(jsonStr);
          if (parsed.name && typeof parsed.name === "string") {
            const key = parsed.name + ":" + JSON.stringify(parsed.arguments || {});
            if (!seen.has(key)) {
              seen.add(key);
              calls.push({ name: parsed.name, arguments: parsed.arguments || {} });
            }
          }
        } catch {
          // 如果整体解析失败，尝试提取 name 和 arguments
          try {
            if (match[1]) {
              const inner = JSON.parse(match[1]);
              if (inner.name && typeof inner.name === "string") {
                const key = inner.name + ":" + JSON.stringify(inner.arguments || {});
                if (!seen.has(key)) {
                  seen.add(key);
                  calls.push({ name: inner.name, arguments: inner.arguments || {} });
                }
              }
            }
          } catch {}
        }
      }
    }
    return calls;
  }

  // ── 从文本中移除工具调用 JSON 块 ──
  function removeToolCallBlocksFromText(text) {
    // 移除包含工具调用的代码块
    let result = text.replace(/\`\`\`(?:json)?\s*\n?[\s\S]*?\n?\`\`\`/g, (match) => {
      const inner = match.replace(/^\`\`\`(?:json)?\s*\n?/, "").replace(/\n?\`\`\`$/, "");
      try {
        const parsed = JSON.parse(inner);
        if (parsed.name && parsed.arguments) return "";
      } catch {}
      return match; // 不是工具调用，保留
    });
    // 移除独立的工具调用 JSON
    result = result.replace(/\{\s*"name"\s*:\s*"[^"]+"\s*,\s*"arguments"\s*:\s*\{[^}]*\}\s*\}/g, "");
    // 清理多余空行
    result = result.replace(/\n{3,}/g, "\n\n").trim();
    return result;
  }


  function handleNonStreamResponse(proxyRes, res, isRetry) {
    const respChunks = [];
    proxyRes.on("data", (c) => respChunks.push(c));
    proxyRes.on("end", () => {
      let fullContent = "";
      let inputTokens = 0;
      let outputTokens = 0;
      let toolCalls = [];
      const respText = Buffer.concat(respChunks).toString("utf-8");
      for (const line of respText.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        try {
          const chunk = JSON.parse(trimmed);
          if (chunk.message?.content) fullContent += chunk.message.content;
          if (chunk.message?.tool_calls) toolCalls.push(...chunk.message.tool_calls);
          if (chunk.prompt_eval_count) inputTokens = chunk.prompt_eval_count;
          if (chunk.eval_count) outputTokens = chunk.eval_count;
        } catch {}
      }
      // 降级重试模式：从文本中解析工具调用 JSON
      if (isRetry && toolCalls.length === 0 && fullContent) {
        const parsedCalls = parseToolCallsFromText(fullContent);
        for (const pc of parsedCalls) {
          toolCalls.push({ function: { name: pc.name, arguments: pc.arguments } });
        }
        if (toolCalls.length > 0) {
          fullContent = removeToolCallBlocksFromText(fullContent);
        }
      }
      const content = [];
      if (fullContent) content.push({ type: "text", text: fullContent });
      let toolUseIndex = 0;
      for (const tc of toolCalls) {
        content.push({ type: "tool_use", id: `toolu_${Date.now().toString(36)}_${toolUseIndex}`, name: tc.function?.name || "unknown", input: tc.function?.arguments || {} });
        toolUseIndex++;
      }
      if (content.length === 0) content.push({ type: "text", text: "" });
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({
        id: `msg_${Date.now().toString(36)}`, type: "message", role: "assistant", content,
        model: ollamaProxyModel, stop_reason: toolCalls.length > 0 ? "tool_use" : "end_turn",
        stop_sequence: null, usage: { input_tokens: inputTokens, output_tokens: outputTokens },
      }));
    });
  }
}

function stopOllamaProxy() {
  if (ollamaProxyServer) {
    ollamaProxyServer.close();
    ollamaProxyServer = null;
    ollamaProxyPort = 0;
  }
}

function isSessionInUseError(errorText) {
  return /Session ID .* is already in use/i.test(`${errorText || ""}`);
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1260,
    height: 860,
    minWidth: 980,
    minHeight: 680,
    backgroundColor: "#140f12",
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  const devServerUrl = process.env.VITE_DEV_SERVER_URL;
  if (devServerUrl) {
    mainWindow.loadURL(devServerUrl);
    return;
  }
  mainWindow.loadFile(path.join(__dirname, "dist", "index.html"));
}

function sendEvent(channel, payload) {
  if (!mainWindow || mainWindow.isDestroyed()) return;
  mainWindow.webContents.send(channel, payload);
}

function parseEnvLines(raw) {
  const lines = raw.split(/\r?\n/);
  const map = new Map();
  for (const line of lines) {
    if (!line || line.trim().startsWith("#")) continue;
    const idx = line.indexOf("=");
    if (idx <= 0) continue;
    map.set(line.slice(0, idx).trim(), line.slice(idx + 1));
  }
  return { lines, map };
}

function readEnvSettings() {
  if (!fs.existsSync(ENV_PATH)) return {};
  const raw = fs.readFileSync(ENV_PATH, "utf8");
  const { map } = parseEnvLines(raw);
  const settings = {};
  for (const key of SETTINGS_KEYS) {
    settings[key] = map.get(key) ?? "";
  }
  return settings;
}

function writeEnvSettings(partial) {
  const safePartial = partial || {};
  const existing = fs.existsSync(ENV_PATH) ? fs.readFileSync(ENV_PATH, "utf8") : "";
  const { lines, map } = parseEnvLines(existing);

  for (const key of SETTINGS_KEYS) {
    if (Object.prototype.hasOwnProperty.call(safePartial, key)) {
      map.set(key, `${safePartial[key] ?? ""}`);
    }
  }

  const used = new Set();
  const next = lines.map((line) => {
    const idx = line.indexOf("=");
    if (idx <= 0) return line;
    const key = line.slice(0, idx).trim();
    if (!SETTINGS_KEYS.includes(key)) return line;
    used.add(key);
    return `${key}=${map.get(key) ?? ""}`;
  });

  for (const key of SETTINGS_KEYS) {
    if (!used.has(key) && map.has(key)) {
      next.push(`${key}=${map.get(key)}`);
    }
  }

  fs.writeFileSync(ENV_PATH, `${next.join("\n").replace(/\n{3,}/g, "\n\n").trim()}\n`, "utf8");
  return readEnvSettings();
}

function clearModelSettings() {
  const reset = {};
  for (const key of MODEL_KEYS) {
    reset[key] = "";
  }
  return writeEnvSettings(reset);
}

function setWorkspacePath(nextPath) {
  if (!nextPath || typeof nextPath !== "string") return false;
  try {
    const stat = fs.statSync(nextPath);
    if (!stat.isDirectory()) return false;
    currentWorkspace = nextPath;
    return true;
  } catch {
    return false;
  }
}

function buildCliArgs(sessionId, model, isResuming) {
  const args = [
    "--env-file=.env",
    CLI_ENTRY,
    "-p",
    "--output-format",
    "stream-json",
    "--include-partial-messages",
    "--verbose",
  ];
  if (isResuming) {
    args.push("--resume", sessionId);
  } else {
    args.push("--session-id", sessionId);
  }
  if (model && model.trim()) {
    args.push("--model", model.trim());
  }
  return args;
}

function extractTextFromAssistant(message) {
  if (!message || !Array.isArray(message.content)) return "";
  return message.content
    .filter((block) => block && block.type === "text" && typeof block.text === "string")
    .map((block) => block.text)
    .join("");
}

function truncateText(input, maxLength) {
  if (typeof input !== "string") return "";
  if (input.length <= maxLength) return input;
  return `${input.slice(0, maxLength)}\n...[truncated]`;
}

async function fetchJsonWithTimeout(url, { headers = {}, timeoutMs = 15000 } = {}) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(url, {
      method: "GET",
      headers,
      signal: controller.signal,
    });
    const text = await response.text();
    let data = null;
    try {
      data = JSON.parse(text);
    } catch {}
    return {
      ok: response.ok,
      status: response.status,
      data,
      text,
    };
  } finally {
    clearTimeout(timer);
  }
}

function normalizeModelEntries(list, provider) {
  if (!Array.isArray(list)) return [];
  return list
    .map((item) => {
      const id = `${item?.id || ""}`.trim();
      if (!id) return null;
      const name = `${item?.name || item?.display_name || id}`.trim();
      return { id, name, provider };
    })
    .filter(Boolean)
    .slice(0, 120);
}

async function listOpenRouterModels(timeoutMs) {
  const result = await fetchJsonWithTimeout("https://openrouter.ai/api/v1/models", { timeoutMs });
  if (!result.ok) {
    return {
      ok: false,
      error: `OpenRouter models API failed (${result.status})`,
    };
  }
  const models = normalizeModelEntries(result?.data?.data, "openrouter");
  return { ok: true, models };
}

async function listAnthropicModels(apiKey, timeoutMs) {
  if (!apiKey || !apiKey.trim()) {
    return { ok: false, error: "Anthropic API key is required." };
  }
  const result = await fetchJsonWithTimeout("https://api.anthropic.com/v1/models", {
    timeoutMs,
    headers: {
      "x-api-key": apiKey.trim(),
      "anthropic-version": "2023-06-01",
    },
  });
  if (!result.ok) {
    const errorText = (result?.data?.error?.message || result.text || "").slice(0, 200);
    return {
      ok: false,
      error: `Anthropic models API failed (${result.status}) ${errorText}`.trim(),
    };
  }
  const models = normalizeModelEntries(result?.data?.data, "anthropic");
  return { ok: true, models };
}

// 已知支持工具调用的 Ollama 模型名称前缀/关键词（fallback 判断）
const TOOL_CALLING_MODEL_PATTERNS = [
  "qwen3", "qwen2.5", "qwen2-",
  "llama3.1", "llama3.2", "llama3.3", "llama4",
  "mistral", "mixtral",
  "command-r",
  "gemma2", "gemma3",
  "phi3", "phi4",
  "deepseek-r1", "deepseek-coder-v2", "deepseek-v3",
  "snowflake-arctic",
  "cogito",
  "devstral",
];

function guessToolSupportByModelName(name) {
  const lower = (name || "").toLowerCase();
  // :cloud 后缀的是云端模型，不支持本地工具调用
  if (lower.endsWith(":cloud")) return false;
  return TOOL_CALLING_MODEL_PATTERNS.some((p) => lower.includes(p));
}

async function fetchOllamaModelCapabilities(baseUrl, modelName, timeoutMs) {
  try {
    const url = `${baseUrl?.trim() || "http://127.0.0.1:11434"}/api/show`;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), Math.min(timeoutMs, 5000));
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: modelName }),
      signal: controller.signal,
    });
    clearTimeout(timer);
    if (!response.ok) return null;
    const data = await response.json();
    // Ollama 新版本在 /api/show 返回 capabilities 字段
    if (data?.capabilities && typeof data.capabilities === "object") {
      return data.capabilities;
    }
    // 有些版本放在 model_info.capabilities 里
    if (data?.model_info?.capabilities && typeof data.model_info.capabilities === "object") {
      return data.model_info.capabilities;
    }
    return null;
  } catch {
    return null;
  }
}

async function listOllamaModels(baseUrl, timeoutMs) {
  const base = baseUrl?.trim() || "http://127.0.0.1:11434";
  const url = `${base}/api/tags`;
  const result = await fetchJsonWithTimeout(url, { timeoutMs });
  if (!result.ok) {
    const errorText = (result.text || "").slice(0, 200);
    return {
      ok: false,
      error: `Ollama models API failed (${result.status}) ${errorText}`.trim(),
    };
  }
  const rawModels = (result?.data?.models || []).slice(0, 120);

  // 并行查询每个模型的 capabilities（使用 allSettled 避免单个失败阻塞全部）
  const capResults = await Promise.allSettled(
    rawModels.map((m) => fetchOllamaModelCapabilities(base, m.name, timeoutMs))
  );

  const models = rawModels.map((m, i) => {
    let toolSupport = guessToolSupportByModelName(m.name);
    let capResolved = false;
    const capResult = capResults[i];
    if (capResult?.status === "fulfilled" && capResult.value) {
      const caps = capResult.value;
      // 如果 /api/show 返回了 capabilities 对象，以它为准
      if (typeof caps.tool_calling === "boolean") {
        toolSupport = caps.tool_calling;
        capResolved = true;
      } else if (typeof caps.input_tool === "boolean") {
        toolSupport = caps.input_tool;
        capResolved = true;
      }
      // capabilities 存在但没有 tool_calling 字段 → 不支持
      if (!capResolved && Object.keys(caps).length > 0) {
        toolSupport = false;
      }
    }
    return {
      id: m.name,
      name: m.name,
      provider: "ollama",
      toolSupport,
    };
  });
  return { ok: true, models };
}



async function sendViaClaudeCli({ prompt, model, requestId, sessionId, isResuming, workspacePath, envOverrides }) {
  let lastAssistantText = "";
  let stderrLog = "";
  let lastResultText = "";
  let stdoutLog = "";

  // 查找 node 可执行文件：优先用便携版 Node
  let nodePath = "node";
  const nodeDir = path.join(PROJECT_ROOT, "nodejs", "node-v24.11.1-win-x64");
  const localNodePath = path.join(nodeDir, "node.exe");
  if (fs.existsSync(localNodePath)) {
    nodePath = localNodePath;
  }

  const child = spawn(nodePath, buildCliArgs(sessionId, model, isResuming), {
    cwd: workspacePath || PROJECT_ROOT,
    windowsHide: true,
    stdio: ["pipe", "pipe", "pipe"],
    env: {
      ...process.env,
      ...(envOverrides || {}),
      PATH: `${nodeDir};${BUN_DIR};${process.env.PATH}`,
    },
  });

  child.stdin.write(prompt);
  child.stdin.end();

  activeRequest = {
    provider: "cli",
    requestId,
    stop: () => {
      try {
        child.kill();
      } catch {}
    },
  };

  const rl = readline.createInterface({ input: child.stdout });
  rl.on("line", (line) => {
    const trimmed = line.trim();
    if (!trimmed) return;
    stdoutLog += `${trimmed}\n`;
    let parsed;
    try {
      parsed = JSON.parse(trimmed);
    } catch {
      return;
    }

    if (
      parsed?.type === "stream_event" &&
      parsed?.event?.type === "content_block_delta" &&
      parsed?.event?.delta?.type === "text_delta" &&
      typeof parsed?.event?.delta?.text === "string"
    ) {
      sendEvent("chat:delta", { requestId, text: parsed.event.delta.text });
    }

    if (parsed?.type === "assistant") {
      const fullText = extractTextFromAssistant(parsed.message);
      if (fullText) lastAssistantText = fullText;
    }

    if (parsed?.type === "result" && typeof parsed?.result === "string") {
      lastResultText = parsed.result;
    }
  });

  child.stderr.on("data", (chunk) => {
    stderrLog += chunk.toString();
  });

  return await new Promise((resolve) => {
    child.on("close", (code) => {
      if (activeRequest?.requestId === requestId) activeRequest = null;

      if (stoppedRequestIds.has(requestId)) {
        stoppedRequestIds.delete(requestId);
        resolve({ ok: false, requestId, error: "Task stopped.", sessionId, stopped: true });
        return;
      }

      if (code === 0) {
        resolve({ ok: true, requestId, text: lastAssistantText.trim(), sessionId });
        return;
      }

      const errorText = stderrLog.trim() || "Unknown CLI error.";
      const fallbackText =
        errorText === "Unknown CLI error."
          ? (lastResultText || lastAssistantText || truncateText(stdoutLog, 1200))
          : "";
      resolve({
        ok: false,
        requestId,
        error: fallbackText || errorText,
        sessionId,
      });
    });
  });
}

ipcMain.handle("chat:getState", async () => {
  const envSettings = readEnvSettings();
  return {
    sessionId: activeSessionId,
    model: envSettings.ANTHROPIC_MODEL || envSettings.OLLAMA_MODEL || "",
    busy: isBusy,
    settings: envSettings,
    workspacePath: currentWorkspace,
  };
});

ipcMain.handle("workspace:get", async () => ({ path: currentWorkspace }));

ipcMain.handle("workspace:choose", async () => {
  if (!mainWindow || mainWindow.isDestroyed()) {
    return { ok: false, error: "Window not ready." };
  }
  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ["openDirectory"],
    title: "选择项目目录",
    defaultPath: currentWorkspace,
  });
  if (result.canceled || !result.filePaths?.length) {
    return { ok: false, canceled: true };
  }
  const picked = result.filePaths[0];
  if (!setWorkspacePath(picked)) {
    return { ok: false, error: "所选目录无效。" };
  }
  return { ok: true, path: currentWorkspace };
});

ipcMain.handle("settings:get", async () => readEnvSettings());
ipcMain.handle("settings:save", async (_event, payload) => writeEnvSettings(payload));
ipcMain.handle("settings:clearModel", async () => clearModelSettings());
ipcMain.handle("models:list", async (_event, payload) => {
  const source = `${payload?.source || ""}`.toLowerCase();
  const envSettings = readEnvSettings();
  const timeoutMs = Number.parseInt(envSettings.API_TIMEOUT_MS || "15000", 10) || 15000;

  if (source === "openrouter") {
    return listOpenRouterModels(timeoutMs);
  }

  if (source === "anthropic") {
    const apiKey = `${payload?.apiKey || envSettings.ANTHROPIC_API_KEY || ""}`;
    return listAnthropicModels(apiKey, timeoutMs);
  }

  if (source === "ollama") {
    const baseUrl = `${payload?.baseUrl || envSettings.OLLAMA_BASE_URL || "http://127.0.0.1:11434"}`;
    return listOllamaModels(baseUrl, timeoutMs);
  }

  return { ok: false, error: "Unsupported model source." };
});

ipcMain.handle("chat:newSession", async () => {
  activeSessionId = randomUUID();
  return { sessionId: activeSessionId };
});

ipcMain.handle("chat:stop", async () => {
  if (!isBusy || !activeRequest?.stop) {
    return { ok: false, error: "当前没有运行中的任务。" };
  }
  try {
    stoppedRequestIds.add(activeRequest.requestId);
    activeRequest.stop();
    sendEvent("chat:status", { busy: false, requestId: activeRequest.requestId, state: "stopped" });
    isBusy = false;
    activeRequest = null;
    // Avoid immediate session-id reuse after forced stop.
    activeSessionId = randomUUID();
    return { ok: true, sessionId: activeSessionId };
  } catch (error) {
    return { ok: false, error: `停止失败: ${error?.message || "unknown"}` };
  }
});

ipcMain.handle("chat:send", async (_event, payload) => {
  if (isBusy) {
    return { ok: false, error: "A request is already running. Please wait for it to complete." };
  }

  const prompt = typeof payload?.prompt === "string" ? payload.prompt.trim() : "";
  const envSettings = readEnvSettings();
  const provider = (payload?.provider || envSettings.MODEL_PROVIDER || "anthropic").toLowerCase();
  const model =
    typeof payload?.model === "string" && payload.model.trim()
      ? payload.model.trim()
      : provider === "ollama"
        ? envSettings.OLLAMA_MODEL || ""
        : envSettings.ANTHROPIC_MODEL || "";

  const timeoutMs = Number.parseInt(envSettings.API_TIMEOUT_MS || "3000000", 10) || 3000000;
  if (provider !== "ollama") {
    const cloudKey = `${envSettings.ANTHROPIC_API_KEY || envSettings.ANTHROPIC_AUTH_TOKEN || ""}`.trim();
    if (!cloudKey || cloudKey === "ollama-local") {
      return { ok: false, error: "请先在云端模式配置有效的 API Key。当前 key 为空或为本地占位值。" };
    }
  }
  let envOverrides;
  if (provider === "ollama") {
    const ollamaTarget = (envSettings.OLLAMA_BASE_URL || "http://127.0.0.1:11434").trim();
    const ollamaModel = (envSettings.OLLAMA_MODEL || "qwen3:8b").trim();
    const proxyPort = await startOllamaProxy(ollamaTarget, ollamaModel);
    envOverrides = {
      ANTHROPIC_BASE_URL: `http://127.0.0.1:${proxyPort}`,
      ANTHROPIC_API_KEY: "ollama-local",
      ANTHROPIC_AUTH_TOKEN: "ollama-local",
      ANTHROPIC_MODEL: ollamaModel,
    };
  }
  if (!prompt) {
    return { ok: false, error: "Prompt cannot be empty." };
  }

  isBusy = true;
  const requestId = randomUUID();
  sendEvent("chat:status", { busy: true, requestId });

  const isResuming = startedSessions.has(activeSessionId);

  let result = await sendViaClaudeCli({
    prompt,
    model,
    requestId,
    sessionId: activeSessionId,
    isResuming,
    workspacePath: currentWorkspace,
    envOverrides,
  });

  if (!result?.ok && isSessionInUseError(result?.error)) {
    // Auto-recover once by rotating session id and retrying.
    activeSessionId = randomUUID();
    result = await sendViaClaudeCli({
      prompt,
      model,
      requestId,
      sessionId: activeSessionId,
      isResuming: false,
      workspacePath: currentWorkspace,
      envOverrides,
    });
  }

  if (result?.ok) {
    startedSessions.add(activeSessionId);
  }

  isBusy = false;
  sendEvent("chat:status", { busy: false, requestId });
  return result;
});

app.whenReady().then(() => {
  createWindow();
  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on("window-all-closed", () => {
  stopOllamaProxy();
  if (process.platform !== "darwin") app.quit();
});
