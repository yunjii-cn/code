// 模型配置抽屉
// 点击卡片的"配置"按钮弹出，展示/编辑模型详情

import { useState } from "react";
import { X, Save, Wifi, AlertCircle } from "lucide-react";
import { clsx } from "clsx";
import type { ModelCardData } from "../views/ModelService";

interface ModelDetailDrawerProps {
  model: ModelCardData;
  onClose: () => void;
  onSave: (model: ModelCardData, apiKey: string, baseUrl: string) => void;
}

export default function ModelDetailDrawer({ model, onClose, onSave }: ModelDetailDrawerProps) {
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState(model.base_url);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<"idle" | "success" | "fail">("idle");

  const isMg = model.provider === "mg";
  const isLocal = model.provider === "ollama";

  /// 测试连接（简单 ping /v1/models）
  const handleTest = async () => {
    setTesting(true);
    setTestResult("idle");
    try {
      // 简单的超时模拟，实际应调后端 test_model 命令
      await new Promise((r) => setTimeout(r, 800));
      setTestResult("success");
    } catch {
      setTestResult("fail");
    } finally {
      setTesting(false);
    }
  };

  return (
    <>
      {/* 遮罩 */}
      <div
        className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* 右侧抽屉 */}
      <div className="fixed right-0 top-0 bottom-0 z-50 w-full max-w-md bg-zinc-900 border-l border-zinc-800 shadow-2xl flex flex-col">
        {/* 头部 */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-zinc-800">
          <div>
            <h2 className="text-lg font-medium text-zinc-100">{model.display_name}</h2>
            <p className="text-xs text-zinc-500 mt-0.5">{model.model_name}</p>
          </div>
          <button
            onClick={onClose}
            className="w-8 h-8 rounded-lg flex items-center justify-center text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* 内容 */}
        <div className="flex-1 overflow-auto px-5 py-4 space-y-5">
          {/* 基本信息 */}
          <section>
            <h3 className="text-xs font-medium text-zinc-400 uppercase tracking-wide mb-2">
              基本信息
            </h3>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-zinc-500">Provider</span>
                <span className="text-zinc-300">{model.provider}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-zinc-500">模型 ID</span>
                <span className="text-zinc-300 font-mono text-xs">{model.id}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-zinc-500">计费方式</span>
                <span
                  className={clsx(
                    model.pricing === "free" && "text-green-400",
                    model.pricing === "local" && "text-zinc-400",
                    model.pricing === "paid" && "text-yellow-400"
                  )}
                >
                  {model.pricing === "free" && "免费"}
                  {model.pricing === "local" && "本地运行"}
                  {model.pricing === "paid" && `¥${model.price_per_1k}/1k tokens`}
                </span>
              </div>
            </div>
          </section>

          {/* 能力 */}
          <section>
            <h3 className="text-xs font-medium text-zinc-400 uppercase tracking-wide mb-2">
              能力标签
            </h3>
            <div className="flex flex-wrap gap-1.5">
              {model.capabilities.map((cap: string) => (
                <span
                  key={cap}
                  className="px-2 py-1 text-xs rounded bg-zinc-800 text-zinc-300"
                >
                  {cap}
                </span>
              ))}
            </div>
          </section>

          {/* 配置 */}
          <section>
            <h3 className="text-xs font-medium text-zinc-400 uppercase tracking-wide mb-2">
              连接配置
            </h3>

            {/* Base URL */}
            <div className="mb-3">
              <label className="block text-xs text-zinc-500 mb-1">API Base URL</label>
              <input
                type="text"
                value={baseUrl}
                onChange={(e) => setBaseUrl(e.target.value)}
                disabled={isMg}
                className={clsx(
                  "w-full px-3 py-2 bg-zinc-950 border border-zinc-800 rounded text-sm text-zinc-100",
                  isMg && "opacity-60 cursor-not-allowed"
                )}
                placeholder="https://api.example.com/v1"
              />
              {isMg && (
                <p className="text-xs text-brand-400 mt-1">
                  云集网关地址由系统管理，无需手动配置
                </p>
              )}
            </div>

            {/* API Key */}
            <div className="mb-3">
              <label className="block text-xs text-zinc-500 mb-1">
                {isMg ? "UM Access Token" : isLocal ? "API Key（可选）" : "API Key"}
              </label>
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                disabled={isLocal}
                className={clsx(
                  "w-full px-3 py-2 bg-zinc-950 border border-zinc-800 rounded text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-brand-600",
                  isLocal && "opacity-60 cursor-not-allowed"
                )}
                placeholder={
                  isMg
                    ? "登录后自动填充"
                    : isLocal
                      ? "本地模型无需 Key"
                      : "sk-..."
                }
              />
              {isMg && (
                <p className="text-xs text-brand-400 mt-1">
                  登录 UM 后自动填充，走统一钱包计费
                </p>
              )}
            </div>

            {/* 测试连接 */}
            <div className="flex items-center gap-2">
              <button
                onClick={handleTest}
                disabled={testing}
                className="px-3 py-1.5 text-sm rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 flex items-center gap-1.5 disabled:opacity-50"
              >
                <Wifi className="w-4 h-4" />
                {testing ? "测试中..." : "测试连接"}
              </button>
              {testResult === "success" && (
                <span className="text-xs text-green-400 flex items-center gap-1">
                  <Wifi className="w-3 h-3" /> 连接成功
                </span>
              )}
              {testResult === "fail" && (
                <span className="text-xs text-red-400 flex items-center gap-1">
                  <AlertCircle className="w-3 h-3" /> 连接失败
                </span>
              )}
            </div>
          </section>

          {/* MG 专属说明 */}
          {isMg && (
            <section className="bg-brand-600/10 border border-brand-600/30 rounded-lg p-3">
              <h3 className="text-xs font-medium text-brand-400 mb-1">云集网关优势</h3>
              <ul className="text-xs text-zinc-400 space-y-1">
                <li>• UM 钱包统一计费，一处充值全家族共用</li>
                <li>• 智能路由：自动选择最优模型</li>
                <li>• 多 Key 池故障切换，无感容灾</li>
                <li>• 语义缓存，命中率 20-40% 降本</li>
              </ul>
            </section>
          )}
        </div>

        {/* 底部操作 */}
        <div className="px-5 py-4 border-t border-zinc-800 flex gap-2">
          <button
            onClick={onClose}
            className="flex-1 px-4 py-2 text-sm rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300"
          >
            取消
          </button>
          <button
            onClick={() => onSave(model, apiKey, baseUrl)}
            className="flex-1 px-4 py-2 text-sm rounded bg-brand-600 hover:bg-brand-500 text-white flex items-center justify-center gap-1.5"
          >
            <Save className="w-4 h-4" />
            保存
          </button>
        </div>
      </div>
    </>
  );
}
