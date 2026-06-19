// 模板市场（M4.2 D4）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.2 D4
//
// 功能：
//   - 浏览 / 搜索 / 预览公开模板
//   - 一键安装
//   - 评分 / 评论
//   - 按行业 / 标签 / 评分 / 下载量筛选

import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  Search,
  Star,
  Download,
  Tag,
  Filter,
  ShoppingCart,
  CheckCircle2,
  Eye,
  TrendingUp,
  Upload,
} from "lucide-react";
import { clsx } from "clsx";

// ============================================================
// 类型定义（与 Rust 后端 template_market.rs 对应）
// ============================================================

interface TemplateRating {
  user_id: string;
  stars: number;
  comment: string;
  rated_at: string;
}

interface RatingSummary {
  count: number;
  average: number;
  stars_count: number[];
}

interface TemplateMarketEntry {
  id: string;
  template_id: string;
  name: string;
  version: string;
  industry: string;
  sub_industry: string;
  description: string;
  author_id: string;
  author_name: string;
  pricing: "free" | "paid";
  price_cents: number;
  review_status: string;
  tags: string[];
  employee_count: number;
  download_count: number;
  rating: RatingSummary;
  ratings: TemplateRating[];
  created_at: string;
  updated_at: string;
  download_url: string | null;
  preview_images: string[];
}

// ============================================================
// 模拟数据（MVP 阶段，后续接入 Tauri command 调用 Rust 后端）
// ============================================================

const MOCK_TEMPLATES: TemplateMarketEntry[] = [
  {
    id: "mkt_001",
    template_id: "ecommerce-fashion",
    name: "电商-穿搭",
    version: "1.0.0",
    industry: "ecommerce",
    sub_industry: "fashion",
    description: "淘宝/抖音/小红书穿搭商家客服 + 选品 + 搭配团队",
    author_id: "official",
    author_name: "云集官方",
    pricing: "free",
    price_cents: 0,
    review_status: "approved",
    tags: ["官方", "电商", "穿搭", "客服"],
    employee_count: 3,
    download_count: 1280,
    rating: { count: 56, average: 4.7, stars_count: [0, 0, 2, 10, 44] },
    ratings: [],
    created_at: "2026-06-19T10:00:00Z",
    updated_at: "2026-06-19T10:00:00Z",
    download_url: null,
    preview_images: [],
  },
  {
    id: "mkt_002",
    template_id: "ecommerce-electronics",
    name: "电商-电子",
    version: "1.0.0",
    industry: "ecommerce",
    sub_industry: "electronics",
    description: "3C 数码 / 家电 / 智能硬件商家",
    author_id: "official",
    author_name: "云集官方",
    pricing: "free",
    price_cents: 0,
    review_status: "approved",
    tags: ["官方", "电商", "3C", "家电"],
    employee_count: 3,
    download_count: 856,
    rating: { count: 32, average: 4.5, stars_count: [0, 0, 1, 8, 23] },
    ratings: [],
    created_at: "2026-06-19T10:00:00Z",
    updated_at: "2026-06-19T10:00:00Z",
    download_url: null,
    preview_images: [],
  },
  {
    id: "mkt_003",
    template_id: "finance-securities",
    name: "金融-证券股票",
    version: "1.0.0",
    industry: "finance",
    sub_industry: "securities",
    description: "券商投顾 / 财经媒体 / 第三方投研机构（合规最严）",
    author_id: "official",
    author_name: "云集官方",
    pricing: "paid",
    price_cents: 9900,
    review_status: "approved",
    tags: ["官方", "金融", "证券", "合规"],
    employee_count: 3,
    download_count: 423,
    rating: { count: 18, average: 4.9, stars_count: [0, 0, 0, 2, 16] },
    ratings: [],
    created_at: "2026-06-19T10:00:00Z",
    updated_at: "2026-06-19T10:00:00Z",
    download_url: null,
    preview_images: [],
  },
  {
    id: "mkt_004",
    template_id: "education-early-childhood",
    name: "教育-早教",
    version: "1.0.0",
    industry: "education",
    sub_industry: "early_childhood",
    description: "0-6 岁早教机构 / 托育中心 / 亲子号",
    author_id: "official",
    author_name: "云集官方",
    pricing: "free",
    price_cents: 0,
    review_status: "approved",
    tags: ["官方", "教育", "早教", "亲子"],
    employee_count: 3,
    download_count: 612,
    rating: { count: 28, average: 4.6, stars_count: [0, 0, 1, 6, 21] },
    ratings: [],
    created_at: "2026-06-19T10:00:00Z",
    updated_at: "2026-06-19T10:00:00Z",
    download_url: null,
    preview_images: [],
  },
  {
    id: "mkt_005",
    template_id: "education-arts-training",
    name: "教育-素质培训",
    version: "1.0.0",
    industry: "education",
    sub_industry: "arts_training",
    description: "少儿编程 / AI 启蒙 / 美术 / 书法 / 音乐机构",
    author_id: "official",
    author_name: "云集官方",
    pricing: "free",
    price_cents: 0,
    review_status: "approved",
    tags: ["官方", "教育", "素质培训", "编程"],
    employee_count: 3,
    download_count: 389,
    rating: { count: 15, average: 4.4, stars_count: [0, 0, 1, 4, 10] },
    ratings: [],
    created_at: "2026-06-19T10:00:00Z",
    updated_at: "2026-06-19T10:00:00Z",
    download_url: null,
    preview_images: [],
  },
  {
    id: "mkt_006",
    template_id: "community-medical-clinic",
    name: "社区诊所接待",
    version: "1.2.0",
    industry: "medical",
    sub_industry: "clinic",
    description: "社区诊所前台接待 + 分诊 + 健康宣教",
    author_id: "user_med_001",
    author_name: "健康先锋",
    pricing: "paid",
    price_cents: 4900,
    review_status: "approved",
    tags: ["社区", "医疗", "诊所", "分诊"],
    employee_count: 3,
    download_count: 156,
    rating: { count: 8, average: 4.3, stars_count: [0, 0, 0, 3, 5] },
    ratings: [],
    created_at: "2026-06-15T10:00:00Z",
    updated_at: "2026-06-18T10:00:00Z",
    download_url: null,
    preview_images: [],
  },
];

// ============================================================
// 工具函数
// ============================================================

function formatPrice(pricing: string, priceCents: number): string {
  if (pricing === "free") return "免费";
  return `¥${(priceCents / 100).toFixed(2)}`;
}

function formatDownloads(count: number): string {
  if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
  return count.toString();
}

function industryLabel(industry: string): string {
  const labels: Record<string, string> = {
    ecommerce: "电商",
    education: "教育",
    finance: "金融",
    medical: "医疗",
  };
  return labels[industry] || industry;
}

// ============================================================
// 子组件：星级显示
// ============================================================

function StarRating({ average, size = 14 }: { average: number; size?: number }) {
  return (
    <div className="flex items-center gap-0.5">
      {[1, 2, 3, 4, 5].map((star) => (
        <Star
          key={star}
          size={size}
          className={clsx(
            star <= Math.round(average)
              ? "fill-yellow-400 text-yellow-400"
              : "text-zinc-600"
          )}
        />
      ))}
      <span className="ml-1 text-xs text-zinc-400">{average.toFixed(1)}</span>
    </div>
  );
}

// ============================================================
// 子组件：模板卡片
// ============================================================

interface TemplateCardProps {
  template: TemplateMarketEntry;
  installed: boolean;
  onInstall: (template: TemplateMarketEntry) => void;
  onPreview: (template: TemplateMarketEntry) => void;
}

function TemplateCard({ template, installed, onInstall, onPreview }: TemplateCardProps) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4 hover:border-zinc-700 transition-colors flex flex-col gap-3">
      {/* 头部：名称 + 行业 */}
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <h3 className="font-medium text-zinc-100 truncate">{template.name}</h3>
          <div className="flex items-center gap-2 mt-1 text-xs text-zinc-500">
            <span className="px-1.5 py-0.5 rounded bg-zinc-800">
              {industryLabel(template.industry)}
            </span>
            <span>v{template.version}</span>
            <span>·</span>
            <span>{template.employee_count} 员工</span>
          </div>
        </div>
        <div
          className={clsx(
            "px-2 py-0.5 rounded text-xs font-medium",
            template.pricing === "free"
              ? "bg-green-900/50 text-green-400"
              : "bg-yellow-900/50 text-yellow-400"
          )}
        >
          {formatPrice(template.pricing, template.price_cents)}
        </div>
      </div>

      {/* 描述 */}
      <p className="text-sm text-zinc-400 line-clamp-2 min-h-[2.5rem]">
        {template.description}
      </p>

      {/* 标签 */}
      <div className="flex flex-wrap gap-1">
        {template.tags.slice(0, 4).map((tag) => (
          <span
            key={tag}
            className="px-1.5 py-0.5 text-xs rounded bg-zinc-800 text-zinc-400"
          >
            <Tag className="inline w-3 h-3 mr-0.5" />
            {tag}
          </span>
        ))}
      </div>

      {/* 评分 + 下载 */}
      <div className="flex items-center justify-between text-xs">
        <StarRating average={template.rating.average} />
        <div className="flex items-center gap-3 text-zinc-500">
          <span className="flex items-center gap-1">
            <Download className="w-3 h-3" />
            {formatDownloads(template.download_count)}
          </span>
          <span className="text-zinc-600">·</span>
          <span>作者: {template.author_name}</span>
        </div>
      </div>

      {/* 操作按钮 */}
      <div className="flex gap-2 pt-2 border-t border-zinc-800">
        <button
          onClick={() => onPreview(template)}
          className="flex-1 px-3 py-1.5 text-sm rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 flex items-center justify-center gap-1"
        >
          <Eye className="w-4 h-4" />
          预览
        </button>
        <button
          onClick={() => onInstall(template)}
          disabled={installed}
          className={clsx(
            "flex-1 px-3 py-1.5 text-sm rounded flex items-center justify-center gap-1 transition-colors",
            installed
              ? "bg-green-900/50 text-green-400 cursor-default"
              : "bg-brand-600 hover:bg-brand-500 text-white"
          )}
        >
          {installed ? (
            <>
              <CheckCircle2 className="w-4 h-4" />
              已安装
            </>
          ) : (
            <>
              <ShoppingCart className="w-4 h-4" />
              安装
            </>
          )}
        </button>
      </div>
    </div>
  );
}

// ============================================================
// 主组件
// ============================================================

type SortBy = "popular" | "rating" | "newest";

export default function Marketplace() {
  const navigate = useNavigate();
  const [searchQuery, setSearchQuery] = useState("");
  const [industryFilter, setIndustryFilter] = useState<string>("all");
  const [pricingFilter, setPricingFilter] = useState<string>("all");
  const [sortBy, setSortBy] = useState<SortBy>("popular");
  const [installedIds, setInstalledIds] = useState<Set<string>>(new Set());
  const [previewTemplate, setPreviewTemplate] = useState<TemplateMarketEntry | null>(null);

  // 筛选 + 排序
  const filteredTemplates = useMemo(() => {
    let result = MOCK_TEMPLATES.filter((t) => t.review_status === "approved");

    // 搜索
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        (t) =>
          t.name.toLowerCase().includes(q) ||
          t.description.toLowerCase().includes(q) ||
          t.tags.some((tag) => tag.toLowerCase().includes(q))
      );
    }

    // 行业筛选
    if (industryFilter !== "all") {
      result = result.filter((t) => t.industry === industryFilter);
    }

    // 定价筛选
    if (pricingFilter === "free") {
      result = result.filter((t) => t.pricing === "free");
    } else if (pricingFilter === "paid") {
      result = result.filter((t) => t.pricing === "paid");
    }

    // 排序
    result = [...result].sort((a, b) => {
      switch (sortBy) {
        case "popular":
          return b.download_count - a.download_count;
        case "rating":
          return b.rating.average - a.rating.average;
        case "newest":
          return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
        default:
          return 0;
      }
    });

    return result;
  }, [searchQuery, industryFilter, pricingFilter, sortBy]);

  const handleInstall = (template: TemplateMarketEntry) => {
    // MVP：直接标记为已安装（后续接入 Tauri command 调用 Rust 后端 install）
    setInstalledIds((prev) => new Set(prev).add(template.id));
    // TODO: 调用 invoke("install_template", { templateId: template.template_id })
  };

  const handlePreview = (template: TemplateMarketEntry) => {
    setPreviewTemplate(template);
  };

  // 统计
  const totalTemplates = MOCK_TEMPLATES.length;
  const freeCount = MOCK_TEMPLATES.filter((t) => t.pricing === "free").length;
  const paidCount = totalTemplates - freeCount;

  return (
    <div className="h-full flex flex-col bg-zinc-950 text-zinc-100">
      {/* 头部 */}
      <header className="px-6 py-4 border-b border-zinc-800">
        <div className="flex items-center justify-between mb-3">
          <div>
            <h1 className="text-xl font-semibold">模板市场</h1>
            <p className="text-sm text-zinc-500 mt-0.5">
              浏览 / 搜索 / 安装社区模板 · 共 {totalTemplates} 个模板（{freeCount} 免费 / {paidCount} 付费）
            </p>
          </div>
          <button
            onClick={() => navigate("/marketplace/submit")}
            className="px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-md flex items-center gap-2 text-sm"
          >
            <Upload className="w-4 h-4" />
            提交我的模板
          </button>
        </div>

        {/* 搜索栏 */}
        <div className="flex gap-2">
          <div className="flex-1 relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="搜索模板名称、描述、标签..."
              className="w-full pl-10 pr-3 py-2 bg-zinc-900 border border-zinc-800 rounded text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-brand-600"
            />
          </div>
        </div>

        {/* 筛选栏 */}
        <div className="flex items-center gap-3 mt-3 text-sm">
          <div className="flex items-center gap-1 text-zinc-500">
            <Filter className="w-4 h-4" />
            <span>筛选:</span>
          </div>

          {/* 行业筛选 */}
          <select
            value={industryFilter}
            onChange={(e) => setIndustryFilter(e.target.value)}
            className="px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-zinc-300 focus:outline-none focus:border-brand-600"
          >
            <option value="all">全部行业</option>
            <option value="ecommerce">电商</option>
            <option value="education">教育</option>
            <option value="finance">金融</option>
            <option value="medical">医疗</option>
          </select>

          {/* 定价筛选 */}
          <select
            value={pricingFilter}
            onChange={(e) => setPricingFilter(e.target.value)}
            className="px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-zinc-300 focus:outline-none focus:border-brand-600"
          >
            <option value="all">全部定价</option>
            <option value="free">免费</option>
            <option value="paid">付费</option>
          </select>

          {/* 排序 */}
          <div className="flex items-center gap-1 ml-auto">
            <TrendingUp className="w-4 h-4 text-zinc-500" />
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as SortBy)}
              className="px-2 py-1 bg-zinc-900 border border-zinc-800 rounded text-zinc-300 focus:outline-none focus:border-brand-600"
            >
              <option value="popular">最热门</option>
              <option value="rating">评分最高</option>
              <option value="newest">最新发布</option>
            </select>
          </div>
        </div>
      </header>

      {/* 模板列表 */}
      <div className="flex-1 overflow-auto p-6">
        {filteredTemplates.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-zinc-500">
            <Search className="w-12 h-12 mb-3 opacity-50" />
            <p>没有找到匹配的模板</p>
            <p className="text-sm mt-1">尝试调整搜索条件或筛选器</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredTemplates.map((template) => (
              <TemplateCard
                key={template.id}
                template={template}
                installed={installedIds.has(template.id)}
                onInstall={handleInstall}
                onPreview={handlePreview}
              />
            ))}
          </div>
        )}
      </div>

      {/* 预览弹窗 */}
      {previewTemplate && (
        <div
          className="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4"
          onClick={() => setPreviewTemplate(null)}
        >
          <div
            className="bg-zinc-900 border border-zinc-800 rounded-lg max-w-2xl w-full max-h-[80vh] overflow-auto p-6"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-start justify-between mb-4">
              <div>
                <h2 className="text-xl font-semibold">{previewTemplate.name}</h2>
                <div className="flex items-center gap-2 mt-1 text-sm text-zinc-500">
                  <span className="px-2 py-0.5 rounded bg-zinc-800">
                    {industryLabel(previewTemplate.industry)}
                  </span>
                  <span>v{previewTemplate.version}</span>
                  <span>·</span>
                  <span>作者: {previewTemplate.author_name}</span>
                </div>
              </div>
              <button
                onClick={() => setPreviewTemplate(null)}
                className="text-zinc-500 hover:text-zinc-300 text-2xl leading-none"
              >
                ×
              </button>
            </div>

            <p className="text-zinc-300 mb-4">{previewTemplate.description}</p>

            <div className="grid grid-cols-3 gap-4 mb-4">
              <div className="bg-zinc-950 rounded p-3 text-center">
                <div className="text-2xl font-semibold text-brand-400">
                  {previewTemplate.employee_count}
                </div>
                <div className="text-xs text-zinc-500 mt-1">员工数</div>
              </div>
              <div className="bg-zinc-950 rounded p-3 text-center">
                <div className="text-2xl font-semibold text-brand-400">
                  {formatDownloads(previewTemplate.download_count)}
                </div>
                <div className="text-xs text-zinc-500 mt-1">下载量</div>
              </div>
              <div className="bg-zinc-950 rounded p-3 text-center">
                <div className="text-2xl font-semibold text-yellow-400">
                  {previewTemplate.rating.average.toFixed(1)}
                </div>
                <div className="text-xs text-zinc-500 mt-1">
                  评分 ({previewTemplate.rating.count})
                </div>
              </div>
            </div>

            <div className="mb-4">
              <h3 className="text-sm font-medium text-zinc-400 mb-2">标签</h3>
              <div className="flex flex-wrap gap-1">
                {previewTemplate.tags.map((tag) => (
                  <span
                    key={tag}
                    className="px-2 py-1 text-xs rounded bg-zinc-800 text-zinc-300"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>

            <div className="mb-4">
              <h3 className="text-sm font-medium text-zinc-400 mb-2">定价</h3>
              <div className="text-lg font-semibold">
                {formatPrice(previewTemplate.pricing, previewTemplate.price_cents)}
              </div>
            </div>

            <div className="flex gap-2 pt-4 border-t border-zinc-800">
              <button
                onClick={() => {
                  handleInstall(previewTemplate);
                  setPreviewTemplate(null);
                }}
                disabled={installedIds.has(previewTemplate.id)}
                className={clsx(
                  "flex-1 px-4 py-2 rounded flex items-center justify-center gap-2",
                  installedIds.has(previewTemplate.id)
                    ? "bg-green-900/50 text-green-400"
                    : "bg-brand-600 hover:bg-brand-500 text-white"
                )}
              >
                {installedIds.has(previewTemplate.id) ? (
                  <>
                    <CheckCircle2 className="w-4 h-4" />
                    已安装
                  </>
                ) : (
                  <>
                    <ShoppingCart className="w-4 h-4" />
                    一键安装
                  </>
                )}
              </button>
              <button
                onClick={() => setPreviewTemplate(null)}
                className="px-4 py-2 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300"
              >
                关闭
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
