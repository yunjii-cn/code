// 模型卡片组件
// 复用 Marketplace.tsx 的 TemplateCard 风格，展示单个模型信息

import { clsx } from "clsx";
import {
  Code,
  Brain,
  Eye,
  MessageCircle,
  Server,
  Cpu,
  Zap,
  CheckCircle2,
  Settings,
  Sparkles,
} from "lucide-react";
import type { ModelCardData } from "../views/ModelService";

interface ModelCardProps {
  model: ModelCardData;
  enabled: boolean;
  onToggle: (model: ModelCardData) => void;
  onConfigure: (model: ModelCardData) => void;
}

/// 能力标签图标映射
const capabilityIcons: Record<string, typeof Code> = {
  code: Code,
  reasoning: Brain,
  vision: Eye,
  chat: MessageCircle,
};

/// 能力标签中文
const capabilityLabels: Record<string, string> = {
  code: "代码",
  reasoning: "推理",
  vision: "视觉",
  chat: "对话",
};

/// Provider 中文
const providerLabels: Record<string, string> = {
  mg: "云集网关",
  openai: "OpenAI",
  ollama: "本地",
  anthropic: "Anthropic",
  deepseek: "DeepSeek",
  zhipu: "智谱",
  qwen: "通义千问",
  siliconflow: "硅基流动",
  mock: "测试",
};

export default function ModelCard({ model, enabled, onToggle, onConfigure }: ModelCardProps) {
  const isFree = model.pricing === "free";
  const isLocal = model.pricing === "local";
  const isMg = model.provider === "mg";

  return (
    <div
      className={clsx(
        "bg-zinc-900 border rounded-lg p-4 hover:border-zinc-700 transition-colors flex flex-col gap-3",
        enabled ? "border-brand-600/50" : "border-zinc-800"
      )}
    >
      {/* 头部：图标 + 名称 + 价格 */}
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-2 flex-1 min-w-0">
          <div
            className={clsx(
              "w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0",
              isMg
                ? "bg-brand-600/20 text-brand-400"
                : isLocal
                  ? "bg-zinc-700/50 text-zinc-300"
                  : "bg-zinc-800 text-zinc-400"
            )}
          >
            {isMg ? <Sparkles className="w-5 h-5" /> : <Cpu className="w-5 h-5" />}
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-medium text-zinc-100 truncate">{model.display_name}</h3>
            <div className="flex items-center gap-1.5 mt-0.5 text-xs text-zinc-500">
              <Server className="w-3 h-3" />
              <span>{providerLabels[model.provider] ?? model.provider}</span>
            </div>
          </div>
        </div>
        {/* 价格标签 */}
        <div
          className={clsx(
            "px-2 py-0.5 rounded text-xs font-medium flex-shrink-0",
            isFree && "bg-green-900/50 text-green-400",
            isLocal && "bg-zinc-700/50 text-zinc-400",
            !isFree && !isLocal && "bg-yellow-900/50 text-yellow-400"
          )}
        >
          {isFree && "免费"}
          {isLocal && "本地"}
          {!isFree && !isLocal && `¥${model.price_per_1k}/1k`}
        </div>
      </div>

      {/* 能力标签 */}
      <div className="flex flex-wrap gap-1">
        {model.capabilities.map((cap: string) => {
          const Icon = capabilityIcons[cap] ?? MessageCircle;
          return (
            <span
              key={cap}
              className="px-1.5 py-0.5 text-xs rounded bg-zinc-800 text-zinc-400 flex items-center gap-1"
            >
              <Icon className="w-3 h-3" />
              {capabilityLabels[cap] ?? cap}
            </span>
          );
        })}
        {model.recommended && (
          <span className="px-1.5 py-0.5 text-xs rounded bg-brand-600/20 text-brand-400 flex items-center gap-1">
            <Zap className="w-3 h-3" />
            推荐
          </span>
        )}
      </div>

      {/* 描述 */}
      <p className="text-sm text-zinc-500 line-clamp-2 min-h-[2.5rem]">
        {model.description}
      </p>

      {/* 操作按钮 */}
      <div className="flex gap-2 pt-2 border-t border-zinc-800">
        <button
          onClick={() => onToggle(model)}
          className={clsx(
            "flex-1 px-3 py-1.5 text-sm rounded flex items-center justify-center gap-1 transition-colors",
            enabled
              ? "bg-green-900/50 text-green-400 cursor-default"
              : "bg-brand-600 hover:bg-brand-500 text-white"
          )}
        >
          {enabled ? (
            <>
              <CheckCircle2 className="w-4 h-4" />
              已启用
            </>
          ) : (
            <>
              <Zap className="w-4 h-4" />
              启用
            </>
          )}
        </button>
        <button
          onClick={() => onConfigure(model)}
          className="px-3 py-1.5 text-sm rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 flex items-center justify-center gap-1"
        >
          <Settings className="w-4 h-4" />
          配置
        </button>
      </div>
    </div>
  );
}
