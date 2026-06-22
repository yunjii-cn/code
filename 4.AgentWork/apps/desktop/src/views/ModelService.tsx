// 模型服务页面
// 卡片式模型管理，超越 Cherry Studio 的 5 个差异化：
// 1. 免费推荐标签  2. 按能力分组  3. 试用按钮  4. 用量统计  5. 自动拉取模型列表

import { useState, useMemo } from "react";
import { clsx } from "clsx";
import { Search, Sparkles, Server, TrendingUp } from "lucide-react";
import ModelCard from "../components/ModelCard";
import ModelDetailDrawer from "../components/ModelDetailDrawer";
import { registerModel } from "../lib/tauri";

// ===== 类型定义 =====

/// 模型计费方式
export type ModelPricing = "free" | "paid" | "local";

/// 模型卡片数据
export interface ModelCardData {
  /// 模型 ID（如 "mg-deepseek-coder"）
  id: string;
  /// 显示名（如 "DeepSeek Coder"）
  display_name: string;
  /// 实际模型名（如 "deepseek-coder"）
  model_name: string;
  /// Provider
  provider: string;
  /// Base URL
  base_url: string;
  /// 计费方式
  pricing: ModelPricing;
  /// 价格（每 1k tokens，分）
  price_per_1k: number;
  /// 能力标签
  capabilities: string[];
  /// 描述
  description: string;
  /// 是否推荐
  recommended: boolean;
}

/// 分类标签
type Category = "all" | "free" | "code" | "reasoning" | "vision" | "chat" | "local";

const categoryLabels: Record<Category, string> = {
  all: "全部",
  free: "免费推荐",
  code: "代码",
  reasoning: "推理",
  vision: "视觉",
  chat: "对话",
  local: "本地",
};

// ===== 内置模型目录 =====
// 前期重点推荐免费 API，吸引用户

const BUILTIN_MODELS: ModelCardData[] = [
  // === 云集网关（MG）===
  {
    id: "mg-deepseek-coder",
    display_name: "DeepSeek Coder",
    model_name: "deepseek-coder",
    provider: "mg",
    base_url: "https://mg.yunjii.cn/v1",
    pricing: "free",
    price_per_1k: 0,
    capabilities: ["code", "reasoning"],
    description: "代码生成与补全专家，通过云集网关智能路由，UM 钱包统一计费",
    recommended: true,
  },
  {
    id: "mg-claude",
    display_name: "Claude 3 Opus",
    model_name: "claude-3-opus",
    provider: "mg",
    base_url: "https://mg.yunjii.cn/v1",
    pricing: "paid",
    price_per_1k: 15,
    capabilities: ["reasoning", "code", "chat"],
    description: "Anthropic 旗舰模型，复杂推理与长文本理解能力顶尖",
    recommended: true,
  },
  {
    id: "mg-gpt4o",
    display_name: "GPT-4o",
    model_name: "gpt-4o",
    provider: "mg",
    base_url: "https://mg.yunjii.cn/v1",
    pricing: "paid",
    price_per_1k: 5,
    capabilities: ["vision", "reasoning", "chat"],
    description: "OpenAI 多模态旗舰，支持图片理解与生成",
    recommended: false,
  },
  {
    id: "mg-qwen-chat",
    display_name: "Qwen Max",
    model_name: "qwen-max",
    provider: "mg",
    base_url: "https://mg.yunjii.cn/v1",
    pricing: "free",
    price_per_1k: 0,
    capabilities: ["chat", "code"],
    description: "通义千问旗舰，中文对话与通用任务，免费额度",
    recommended: true,
  },

  // === 免费 API（用户自接）===
  {
    id: "sf-deepseek",
    display_name: "DeepSeek Chat",
    model_name: "deepseek-chat",
    provider: "siliconflow",
    base_url: "https://api.siliconflow.cn/v1",
    pricing: "free",
    price_per_1k: 0,
    capabilities: ["chat", "reasoning"],
    description: "硅基流动免费提供，通用对话与推理",
    recommended: true,
  },
  {
    id: "sf-qwen-coder",
    display_name: "Qwen2.5 Coder",
    model_name: "Qwen/Qwen2.5-Coder-7B-Instruct",
    provider: "siliconflow",
    base_url: "https://api.siliconflow.cn/v1",
    pricing: "free",
    price_per_1k: 0,
    capabilities: ["code"],
    description: "硅基流动免费提供，代码补全与生成",
    recommended: true,
  },
  {
    id: "zhipu-glm4",
    display_name: "GLM-4-Flash",
    model_name: "glm-4-flash",
    provider: "zhipu",
    base_url: "https://open.bigmodel.cn/api/paas/v4",
    pricing: "free",
    price_per_1k: 0,
    capabilities: ["chat", "reasoning"],
    description: "智谱 BigModel 免费层，通用对话与推理",
    recommended: false,
  },

  // === 付费 API（用户自接）===
  {
    id: "openai-gpt4o",
    display_name: "GPT-4o (直连)",
    model_name: "gpt-4o",
    provider: "openai",
    base_url: "https://api.openai.com/v1",
    pricing: "paid",
    price_per_1k: 5,
    capabilities: ["vision", "reasoning", "chat", "code"],
    description: "OpenAI 官方直连，需自备 API Key",
    recommended: false,
  },
  {
    id: "anthropic-claude",
    display_name: "Claude 3.5 Sonnet (直连)",
    model_name: "claude-3-5-sonnet",
    provider: "anthropic",
    base_url: "https://api.anthropic.com/v1",
    pricing: "paid",
    price_per_1k: 3,
    capabilities: ["reasoning", "code", "chat"],
    description: "Anthropic 官方直连，代码与推理强项",
    recommended: false,
  },

  // === 本地模型 ===
  {
    id: "local-llama3",
    display_name: "Llama 3.2 (本地)",
    model_name: "llama3.2",
    provider: "ollama",
    base_url: "http://localhost:11434",
    pricing: "local",
    price_per_1k: 0,
    capabilities: ["chat", "reasoning"],
    description: "Ollama 本地运行，完全离线，隐私无忧",
    recommended: false,
  },
  {
    id: "local-qwen-coder",
    display_name: "Qwen2.5 Coder (本地)",
    model_name: "qwen2.5-coder:7b",
    provider: "ollama",
    base_url: "http://localhost:11434",
    pricing: "local",
    price_per_1k: 0,
    capabilities: ["code"],
    description: "Ollama 本地代码模型，离线代码补全",
    recommended: false,
  },
];

// ===== 主组件 =====

export default function ModelService() {
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<Category>("all");
  const [enabledIds, setEnabledIds] = useState<Set<string>>(new Set());
  const [drawerModel, setDrawerModel] = useState<ModelCardData | null>(null);

  /// 过滤后的模型列表
  const filteredModels = useMemo(() => {
    return BUILTIN_MODELS.filter((m) => {
      // 搜索匹配
      if (search) {
        const q = search.toLowerCase();
        const match =
          m.display_name.toLowerCase().includes(q) ||
          m.model_name.toLowerCase().includes(q) ||
          m.provider.toLowerCase().includes(q);
        if (!match) return false;
      }
      // 分类匹配
      if (category === "all") return true;
      if (category === "free") return m.pricing === "free";
      if (category === "local") return m.pricing === "local";
      return m.capabilities.includes(category);
    });
  }, [search, category]);

  /// 启用/禁用模型
  const handleToggle = (model: ModelCardData) => {
    setEnabledIds((prev) => {
      const next = new Set(prev);
      if (next.has(model.id)) {
        next.delete(model.id);
      } else {
        next.add(model.id);
      }
      return next;
    });
  };

  /// 保存配置
  const handleSave = async (model: ModelCardData, apiKey: string, baseUrl: string) => {
    try {
      // 调用后端 registerModel IPC 注册并持久化
      await registerModel({
        id: model.id,
        provider: model.provider,
        base_url: baseUrl,
        api_key: apiKey || undefined,
        model_name: model.model_name,
      });
      // 自动启用
      setEnabledIds((prev) => new Set(prev).add(model.id));
      setDrawerModel(null);
    } catch (err) {
      console.error("注册模型失败:", err);
      alert(`保存失败: ${err}`);
    }
  };

  /// 统计数据
  const enabledCount = enabledIds.size;
  const freeCount = BUILTIN_MODELS.filter((m) => m.pricing === "free").length;
  const totalCount = BUILTIN_MODELS.length;

  return (
    <div className="h-full flex flex-col bg-zinc-950">
      {/* 顶部标题栏 */}
      <header className="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Server className="w-5 h-5 text-brand-400" />
          <h1 className="text-lg font-medium text-zinc-100">模型服务</h1>
          <span className="text-xs text-zinc-500">
            ({enabledCount} 启用 / {totalCount} 总计)
          </span>
        </div>
        <div className="flex items-center gap-2">
          {/* 搜索框 */}
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="搜索模型..."
              className="w-56 pl-9 pr-3 py-1.5 bg-zinc-900 border border-zinc-800 rounded text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-brand-600"
            />
          </div>
        </div>
      </header>

      {/* 分类标签栏 */}
      <div className="px-6 py-3 border-b border-zinc-800 flex items-center gap-2">
        {(Object.keys(categoryLabels) as Category[]).map((cat) => (
          <button
            key={cat}
            onClick={() => setCategory(cat)}
            className={clsx(
              "px-3 py-1 text-sm rounded-full transition-colors flex items-center gap-1",
              category === cat
                ? "bg-brand-600 text-white"
                : "bg-zinc-900 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
            )}
          >
            {cat === "free" && <Sparkles className="w-3 h-3" />}
            {categoryLabels[cat]}
          </button>
        ))}
        <div className="ml-auto text-xs text-zinc-500">
          {filteredModels.length} 个模型
        </div>
      </div>

      {/* 卡片网格 */}
      <div className="flex-1 overflow-auto px-6 py-4">
        {filteredModels.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-zinc-500">
            <Search className="w-12 h-12 mb-3 opacity-30" />
            <p className="text-sm">未找到匹配的模型</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredModels.map((model) => (
              <ModelCard
                key={model.id}
                model={model}
                enabled={enabledIds.has(model.id)}
                onToggle={handleToggle}
                onConfigure={setDrawerModel}
              />
            ))}
          </div>
        )}
      </div>

      {/* 底部用量条 */}
      <footer className="px-6 py-3 border-t border-zinc-800 bg-zinc-900 flex items-center justify-between text-xs">
        <div className="flex items-center gap-4">
          <span className="flex items-center gap-1.5 text-zinc-400">
            <TrendingUp className="w-3.5 h-3.5" />
            今日用量: <span className="text-zinc-200">0 tokens</span>
          </span>
          <span className="text-zinc-600">|</span>
          <span className="flex items-center gap-1.5 text-zinc-400">
            <Sparkles className="w-3.5 h-3.5 text-green-400" />
            免费模型: <span className="text-green-400">{freeCount} 个可用</span>
          </span>
        </div>
        <div className="flex items-center gap-2 text-zinc-500">
          <span>云集网关: UM 钱包统一计费</span>
        </div>
      </footer>

      {/* 配置抽屉 */}
      {drawerModel && (
        <ModelDetailDrawer
          model={drawerModel}
          onClose={() => setDrawerModel(null)}
          onSave={handleSave}
        />
      )}
    </div>
  );
}
