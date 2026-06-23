// 无代码工具构建器（M6.2）
//
// 允许用户通过表单快速创建 AI 可调用的工具：
//   - 工具名、描述、参数定义
//   - Prompt 模板 / 调用端点
//   - 本地测试预览
//
// MVP：前端本地生成 prompt，后端 execute_tool_preview 模拟返回结果

import { useState } from "react";
import {
  Wrench,
  Play,
  Plus,
  Trash2,
  RefreshCw,
  Save,
  CheckCircle2,
  AlertCircle,
  Code2,
  FileJson,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { previewToolPrompt, executeToolPreview, type ToolPreviewResult } from "@/lib/tauri";

/** 参数定义 */
interface ToolParameter {
  id: string;
  name: string;
  type: "string" | "number" | "boolean";
  description: string;
  required: boolean;
}

/** 工具定义 */
interface ToolDefinition {
  name: string;
  description: string;
  parameters: ToolParameter[];
  template: string;
}

const EMPTY_PARAM: ToolParameter = {
  id: "",
  name: "",
  type: "string",
  description: "",
  required: true,
};

const DEFAULT_TEMPLATE = `你是一个 {{tool.name}} 工具。
任务：{{tool.description}}

用户输入参数：
{{#each params}}
- {{name}}: {{value}}
{{/each}}

请按以下格式返回结果：
{
  "success": true,
  "result": "..."
}`;

export default function ToolBuilder() {
  const [tool, setTool] = useState<ToolDefinition>({
    name: "",
    description: "",
    parameters: [],
    template: DEFAULT_TEMPLATE,
  });

  const [testValues, setTestValues] = useState<Record<string, string>>({});
  const [preview, setPreview] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  const [result, setResult] = useState<ToolPreviewResult | null>(null);
  const [executeLoading, setExecuteLoading] = useState(false);

  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  function addParameter() {
    setTool((t) => ({
      ...t,
      parameters: [
        ...t.parameters,
        { ...EMPTY_PARAM, id: crypto.randomUUID() },
      ],
    }));
  }

  function updateParameter(id: string, patch: Partial<ToolParameter>) {
    setTool((t) => ({
      ...t,
      parameters: t.parameters.map((p) => (p.id === id ? { ...p, ...patch } : p)),
    }));
  }

  function removeParameter(id: string) {
    setTool((t) => ({
      ...t,
      parameters: t.parameters.filter((p) => p.id !== id),
    }));
    setTestValues((v) => {
      const next = { ...v };
      const param = tool.parameters.find((p) => p.id === id);
      if (param) delete next[param.name];
      return next;
    });
  }

  async function handlePreview() {
    if (!tool.name.trim() || !tool.description.trim()) {
      setMessage({ type: "error", text: "请填写工具名和描述" });
      return;
    }
    setPreviewLoading(true);
    setMessage(null);
    try {
      const prompt = await previewToolPrompt(tool);
      setPreview(prompt);
    } catch (err) {
      setMessage({ type: "error", text: String(err) });
    } finally {
      setPreviewLoading(false);
    }
  }

  async function handleExecute() {
    if (!tool.name.trim() || !tool.description.trim()) {
      setMessage({ type: "error", text: "请填写工具名和描述" });
      return;
    }
    setExecuteLoading(true);
    setMessage(null);
    try {
      const res = await executeToolPreview(tool, testValues);
      setResult(res);
      setMessage({ type: "success", text: "测试执行完成" });
    } catch (err) {
      setMessage({ type: "error", text: String(err) });
    } finally {
      setExecuteLoading(false);
    }
  }

  function handleSave() {
    // MVP：本地导出为 JSON，后续接入后端持久化
    const blob = new Blob([JSON.stringify(tool, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${tool.name || "tool"}.json`;
    a.click();
    URL.revokeObjectURL(url);
    setMessage({ type: "success", text: "工具定义已导出为 JSON" });
  }

  return (
    <div className="flex flex-col h-full">
      <header className="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <h1 className="text-lg font-semibold flex items-center gap-2">
          <Wrench className="w-5 h-5 text-brand-400" />
          工具构建器
        </h1>
        <div className="flex items-center gap-2">
          <button
            onClick={handlePreview}
            disabled={previewLoading}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg bg-zinc-800 hover:bg-zinc-700 disabled:opacity-50 transition-colors"
          >
            {previewLoading ? (
              <RefreshCw className="w-3.5 h-3.5 animate-spin" />
            ) : (
              <Code2 className="w-3.5 h-3.5" />
            )}
            生成 Prompt
          </button>
          <button
            onClick={handleSave}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg bg-zinc-800 hover:bg-zinc-700 transition-colors"
          >
            <Save className="w-3.5 h-3.5" />
            导出 JSON
          </button>
        </div>
      </header>

      <div className="flex-1 overflow-auto p-6">
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
          {/* 左侧：表单 */}
          <div className="space-y-6">
            {message && (
              <div
                className={cn(
                  "flex items-start gap-2 text-sm rounded-lg p-3",
                  message.type === "success"
                    ? "text-green-400 bg-green-950/30"
                    : "text-red-400 bg-red-950/30"
                )}
              >
                {message.type === "success" ? (
                  <CheckCircle2 className="w-4 h-4 mt-0.5 flex-shrink-0" />
                ) : (
                  <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                )}
                <span className="break-all">{message.text}</span>
              </div>
            )}

            {/* 基础信息 */}
            <section className="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 space-y-4">
              <h2 className="text-sm font-medium text-zinc-300 flex items-center gap-2">
                <FileJson className="w-4 h-4 text-brand-400" />
                基础信息
              </h2>
              <div className="grid grid-cols-1 gap-4">
                <div>
                  <label className="text-xs text-zinc-500">工具名称</label>
                  <input
                    type="text"
                    value={tool.name}
                    onChange={(e) => setTool((t) => ({ ...t, name: e.target.value }))}
                    placeholder="例如：sales_quote_generator"
                    className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                  />
                </div>
                <div>
                  <label className="text-xs text-zinc-500">工具描述</label>
                  <textarea
                    value={tool.description}
                    onChange={(e) => setTool((t) => ({ ...t, description: e.target.value }))}
                    placeholder="描述工具用途，例如：根据客户需求生成销售报价单"
                    rows={3}
                    className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none resize-none"
                  />
                </div>
              </div>
            </section>

            {/* 参数定义 */}
            <section className="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 space-y-4">
              <div className="flex items-center justify-between">
                <h2 className="text-sm font-medium text-zinc-300 flex items-center gap-2">
                  <Code2 className="w-4 h-4 text-brand-400" />
                  参数定义
                </h2>
                <button
                  onClick={addParameter}
                  className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-brand-600 hover:bg-brand-700 transition-colors"
                >
                  <Plus className="w-3 h-3" />
                  添加参数
                </button>
              </div>

              {tool.parameters.length === 0 && (
                <p className="text-sm text-zinc-500 text-center py-4">
                  暂无参数，点击上方按钮添加
                </p>
              )}

              <div className="space-y-3">
                {tool.parameters.map((param) => (
                  <div
                    key={param.id}
                    className="rounded-lg border border-zinc-700 bg-zinc-800/50 p-3 space-y-3"
                  >
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                      <input
                        type="text"
                        value={param.name}
                        onChange={(e) => updateParameter(param.id, { name: e.target.value })}
                        placeholder="参数名"
                        className="bg-zinc-900 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                      />
                      <select
                        value={param.type}
                        onChange={(e) =>
                          updateParameter(param.id, { type: e.target.value as ToolParameter["type"] })
                        }
                        className="bg-zinc-900 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                      >
                        <option value="string">字符串 string</option>
                        <option value="number">数字 number</option>
                        <option value="boolean">布尔 boolean</option>
                      </select>
                    </div>
                    <input
                      type="text"
                      value={param.description}
                      onChange={(e) => updateParameter(param.id, { description: e.target.value })}
                      placeholder="参数描述"
                      className="w-full bg-zinc-900 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                    />
                    <div className="flex items-center justify-between">
                      <label className="flex items-center gap-2 text-xs text-zinc-400 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={param.required}
                          onChange={(e) =>
                            updateParameter(param.id, { required: e.target.checked })
                          }
                          className="rounded border-zinc-600 bg-zinc-900 text-brand-500 focus:ring-0"
                        />
                        必填
                      </label>
                      <button
                        onClick={() => removeParameter(param.id)}
                        className="flex items-center gap-1 text-xs text-red-400 hover:text-red-300 transition-colors"
                      >
                        <Trash2 className="w-3 h-3" />
                        删除
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </section>

            {/* Prompt 模板 */}
            <section className="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 space-y-4">
              <h2 className="text-sm font-medium text-zinc-300 flex items-center gap-2">
                <Code2 className="w-4 h-4 text-brand-400" />
                Prompt 模板
              </h2>
              <textarea
                value={tool.template}
                onChange={(e) => setTool((t) => ({ ...t, template: e.target.value }))}
                rows={10}
                className="w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none font-mono"
              />
              <p className="text-xs text-zinc-500">
                可用占位符：{"{{tool.name}}"}, {"{{tool.description}}"}, {"{{#each params}}"} ... {"{{/each}}"}
              </p>
            </section>
          </div>

          {/* 右侧：测试区 */}
          <div className="space-y-6">
            {/* 测试参数 */}
            <section className="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 space-y-4">
              <h2 className="text-sm font-medium text-zinc-300 flex items-center gap-2">
                <Play className="w-4 h-4 text-brand-400" />
                测试参数
              </h2>
              {tool.parameters.length === 0 ? (
                <p className="text-sm text-zinc-500 text-center py-4">
                  左侧添加参数后，可在这里输入测试值
                </p>
              ) : (
                <div className="space-y-3">
                  {tool.parameters.map((param) => (
                    <div key={param.id}>
                      <label className="text-xs text-zinc-500">
                        {param.name}
                        {param.required && <span className="text-red-400 ml-1">*</span>}
                      </label>
                      {param.type === "boolean" ? (
                        <select
                          value={testValues[param.name] || "false"}
                          onChange={(e) =>
                            setTestValues((v) => ({ ...v, [param.name]: e.target.value }))
                          }
                          className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                        >
                          <option value="true">true</option>
                          <option value="false">false</option>
                        </select>
                      ) : (
                        <input
                          type={param.type === "number" ? "number" : "text"}
                          value={testValues[param.name] || ""}
                          onChange={(e) =>
                            setTestValues((v) => ({ ...v, [param.name]: e.target.value }))
                          }
                          placeholder={param.description}
                          className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                        />
                      )}
                    </div>
                  ))}
                </div>
              )}
              <button
                onClick={handleExecute}
                disabled={executeLoading}
                className="w-full flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-sm"
              >
                {executeLoading ? (
                  <RefreshCw className="w-4 h-4 animate-spin" />
                ) : (
                  <Play className="w-4 h-4" />
                )}
                {executeLoading ? "执行中..." : "运行测试"}
              </button>
            </section>

            {/* Prompt 预览 */}
            {preview && (
              <section className="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4">
                <h2 className="mb-2 text-sm font-medium text-zinc-300 flex items-center gap-2">
                  <Code2 className="w-4 h-4 text-brand-400" />
                  Prompt 预览
                </h2>
                <pre className="max-h-64 overflow-auto rounded-lg bg-zinc-950 p-3 text-xs text-zinc-300 font-mono whitespace-pre-wrap">
                  {preview}
                </pre>
              </section>
            )}

            {/* 执行结果 */}
            {result && (
              <section className="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4">
                <h2 className="mb-2 text-sm font-medium text-zinc-300 flex items-center gap-2">
                  <CheckCircle2
                    className={cn(
                      "w-4 h-4",
                      result.success ? "text-green-400" : "text-red-400"
                    )}
                  />
                  执行结果
                </h2>
                <div
                  className={cn(
                    "rounded-lg p-3 text-sm font-mono whitespace-pre-wrap",
                    result.success
                      ? "bg-green-950/20 text-green-300"
                      : "bg-red-950/20 text-red-300"
                  )}
                >
                  {result.output}
                </div>
              </section>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
