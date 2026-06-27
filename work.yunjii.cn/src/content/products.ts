export interface Product {
  id: string;
  name: string;
  tagline: string;
  tier: string;
  price: string;
  priceNote: string;
  status: "stable" | "beta" | "coming";
  tech: string;
  targetUsers: string;
  highlights: string[];
  cta: { label: string; href: string; primary?: boolean };
}

export interface Tier {
  name: string;
  price: string;
  period: string;
  desc: string;
  features: string[];
  cta: string;
  highlighted?: boolean;
}

export const products: Product[] = [
  {
    id: "pc",
    name: "云集工作台·桌面版",
    tagline: "本地 AI 编程工具，数据不离开你的电脑",
    tier: "Free",
    price: "永久免费",
    priceNote: "云端模型按算力付费，本地 Ollama 零成本",
    status: "stable",
    tech: "PyQt6",
    targetUsers: "程序员 / 学生 / 隐私党 / 离线党",
    highlights: [
      "永久免费，无功能阉割",
      "离线可用，Ollama 原生支持",
      "隐私第一，数据 100% 留本地",
      "PyQt6 原生窗口，响应更快",
    ],
    cta: { label: "下载使用", href: "#", primary: false },
  },
  {
    id: "web",
    name: "云集工作台·网络版",
    tagline: "跨端跨平台的 AI 编程工作空间",
    tier: "Pro",
    price: "¥99/月起",
    priceNote: "新人送 ¥30 体验金，约 3 个月免费额度",
    status: "beta",
    tech: "FastAPI + Vue 3",
    targetUsers: "自由职业 / 小团队 / 移动办公",
    highlights: [
      "电脑、手机、平板无缝切换",
      "云端 AI 一键使用",
      "PWA / 桌面壳多端覆盖",
      "新人送 ¥30 体验金",
    ],
    cta: { label: "开始试用", href: "#", primary: false },
  },
  {
    id: "team",
    name: "云集工作台·团队版",
    tagline: "全功能的 AI 编程团队工作空间",
    tier: "Business",
    price: "¥199/月起",
    priceNote: "含私有化部署咨询",
    status: "stable",
    tech: "FastAPI + Platformkit + Vue 3",
    targetUsers: "中型团队 / 企业 / 政府",
    highlights: [
      "5 角色团队协作模式",
      "Yjs 实时协作引擎",
      "4 层知识引擎 + 强度演化",
      "SSO + 审计 + 多租户合规",
    ],
    cta: { label: "了解更多", href: "#", primary: false },
  },
  {
    id: "agentwork",
    name: "AgentWork",
    tagline: "AI 智能体工作台，AI 员工 + AI 团队 + 行业模板",
    tier: "Enterprise",
    price: "¥999/月/席位",
    priceNote: "企业定制单独议价，私有化部署 ¥50 万起",
    status: "coming",
    tech: "Tauri 2 + Rust + React",
    targetUsers: "企业研发团队 / 50+ 人企业",
    highlights: [
      "多 Agent DAG 真并行协作",
      "AST-Native 代码认知引擎",
      "编译期验证护栏，零幻觉交付",
      "TimeFlow 本地版本控制",
    ],
    cta: { label: "预约演示", href: "#", primary: true },
  },
];

export const tiers: Tier[] = [
  {
    name: "Free",
    price: "¥0",
    period: "永久",
    desc: "本地 AI 编程，数据不离开你的电脑",
    features: [
      "本地 AI 引擎（Ollama 原生）",
      "单设备使用，断网也能跑",
      "云集工作台·桌面版",
      "代码、对话 100% 留存本地",
      "零注册，双击即用",
      "社区支持",
    ],
    cta: "免费下载",
  },
  {
    name: "Pro",
    price: "¥99",
    period: "起/月",
    desc: "跨端跨平台，电脑手机平板无缝切换",
    features: [
      "云端 AI 模型一键接入",
      "跨端同步（3 设备同时在线）",
      "PWA / 移动端覆盖",
      "¥50 等值算力/月",
      "新人送 ¥30 体验金",
      "邮件支持",
    ],
    cta: "开始试用",
  },
  {
    name: "Business",
    price: "¥199",
    period: "起/月",
    desc: "全功能 AI 编程团队工作空间",
    features: [
      "全部 Pro 功能",
      "5 角色团队协作（主管/架构/开发/测试/文档）",
      "4 层知识引擎 + 强度演化",
      "4 类主动感知（变更/质量/漏洞/任务）",
      "Yjs 实时协作引擎",
      "SSO + 审计 + 多租户合规",
      "优先技术支持",
    ],
    cta: "了解更多",
    highlighted: true,
  },
  {
    name: "Enterprise",
    price: "¥399",
    period: "起/月",
    desc: "AI 智能体工作台，AI 员工组成数字化团队，自进化工作流",
    features: [
      "AI 团队协作引擎（多Agent DAG并行+异构模型）",
      "多 Agent DAG 真并行协作",
      "行业模板（电商/教育/金融/SaaS）",
      "AST-Native 代码认知引擎",
      "编译期五级验证护栏",
      "TimeFlow 本地版本控制",
      "全链路自动发布流水线",
      "私有化部署可选 · 专属客户成功经理",
    ],
    cta: "预约演示",
  },
];

export const navLinks = [
  { label: "产品", href: "#products" },
  { label: "价格", href: "#pricing" },
  { label: "下载", href: "#download" },
  { label: "文档", href: "#" },
];
