import { useTranslation } from "react-i18next";
import { motion, AnimatePresence } from "framer-motion";

interface Props {
  productId: string;
  onClose: () => void;
}

const generationLabels: Record<string, string> = {
  pc: "第 1 代 · Free 档",
  web: "第 2 代 · Pro 档",
  team: "第 3 代 · Business 档",
  agentwork: "第 4 代 · Enterprise 档 · Yunjii AgentWork",
};

const techLabels: Record<string, string> = {
  pc: "PyQt6 + Python 3.12 + FastAPI（内嵌）",
  web: "FastAPI + Vue 3 + Vite + Capacitor",
  team: "FastAPI + Platformkit + Vue 3 + Yjs",
  agentwork: "Tauri 2 + Rust + React + TimeFlow",
};

const userLabels: Record<string, string> = {
  pc: "程序员 / 学生 / 隐私党 / 离线党",
  web: "自由职业 / 小团队 / 移动办公",
  team: "中型团队 / 企业 / 政府",
  agentwork: "企业研发团队 / 50+ 人企业",
};

const i18nPrefixes: Record<string, string> = {
  pc: "products.pc",
  web: "products.web",
  team: "products.team",
  agentwork: "products.agentwork",
};

export default function ProductDetailPage({ productId, onClose }: Props) {
  const { t } = useTranslation();
  const prefix = i18nPrefixes[productId] ?? "products.pc";
  const d = t(`${prefix}.detail`, { returnObjects: true }) as Record<string, unknown>;
  const features = (d.features as Array<{ title: string; desc: string }>) ?? [];
  const isAW = productId === "agentwork";
  const genLabel = generationLabels[productId] ?? "";
  const tech = techLabels[productId] ?? "";
  const users = userLabels[productId] ?? "";

  const heading = isAW ? "Yunjii AgentWork" : (d.heading as string);
  const intro = isAW
    ? "Yunjii AgentWork 是云集工作台第 4 代旗舰产品，Enterprise 档 AI 员工与智能体编排平台。独立技术栈 Tauri 2 + Rust + React，核心差异化为五大支柱。"
    : (d.intro as string);

  const awFeatures: Array<{ title: string; desc: string }> = [
    { title: "AI 团队协作引擎", desc: "多 Agent DAG 真并行 + 异构模型差异化绑定（Qwen3.7 / GLM5.2 / MiniMax3）+ Git 分支隔离 + 冲突自动解决。" },
    { title: "AST-Native 代码认知", desc: "Tree-sitter + LanceDB 构建代码知识图谱，Token 消耗降低 70%，跨语言契约监听。" },
    { title: "编译期验证护栏", desc: "五级验证管道（Lint/类型/编译/契约/调用链），破坏类型安全的代码直接拒绝合并，零幻觉交付。" },
    { title: "TimeFlow 本地版本控制", desc: "不依赖 git 的本地优先 VCS，自动快照、自由回滚、AI 语义版本管理。" },
    { title: "全链路自动发布", desc: "AI 整理正式版本 → 多目标构建 → 多平台分发 → 自动生成 Release Notes。" },
    { title: "行业模板 + 自定义", desc: "电商/教育/金融/SaaS 预置模板，支持行业定制扩展。" },
  ];

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={{ duration: 0.25 }}
        className="fixed inset-0 z-50 bg-[var(--color-bg)] overflow-y-auto"
      >
        <div className="min-h-screen">
          {/* Top nav bar */}
          <div className="sticky top-0 z-10 bg-[var(--color-bg)]/90 backdrop-blur-xl border-b border-[var(--color-border)]">
            <div className="max-w-5xl mx-auto px-4 sm:px-6 h-14 flex items-center justify-between">
              <button
                onClick={onClose}
                className="flex items-center gap-2 text-sm text-[var(--color-text-muted)] hover:text-[var(--color-text)] transition-colors"
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M19 12H5M12 19l-7-7 7-7" />
                </svg>
                返回产品矩阵
              </button>
              {isAW && (
                <a
                  href="https://aw.yunjii.cn"
                  target="_blank"
                  rel="noopener"
                  className="flex items-center gap-1.5 text-sm font-medium text-[var(--color-aw-soft)] hover:text-[var(--color-aw-primary)] transition-colors"
                >
                  访问 AW 官网
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                    <path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3" />
                  </svg>
                </a>
              )}
            </div>
          </div>

          {/* Content */}
          <div className="max-w-4xl mx-auto px-4 sm:px-6 py-12 sm:py-20">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1 }}
            >
              {/* Generation badge */}
              <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-border)] text-xs text-[var(--color-text-faint)] mb-8">
                {genLabel}
              </div>

              {/* Heading */}
              <h1 className="text-3xl sm:text-4xl md:text-5xl font-extrabold text-[var(--color-text)] mb-6 tracking-tight">
                {heading}
              </h1>

              {/* Intro */}
              <p className="text-base sm:text-lg text-[var(--color-text-muted)] leading-relaxed mb-14 max-w-3xl">
                {intro}
              </p>

              {/* Features */}
              <div className="grid sm:grid-cols-2 gap-5 mb-14">
                {(isAW ? awFeatures : features).map((f, i) => (
                  <motion.div
                    key={f.title}
                    initial={{ opacity: 0, y: 16 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.15 + i * 0.06 }}
                    className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-6 hover:border-[var(--color-border-hover)] transition-colors"
                  >
                    <h4 className="text-sm font-bold text-[var(--color-text)] mb-2.5">{f.title}</h4>
                    <p className="text-xs text-[var(--color-text-muted)] leading-relaxed">{f.desc}</p>
                  </motion.div>
                ))}
              </div>

              {/* Meta bar */}
              <div className="flex flex-wrap items-center gap-6 text-xs text-[var(--color-text-faint)] border-t border-[var(--color-border)] pt-8">
                <span className="font-mono">{tech}</span>
                <span className="w-1 h-1 rounded-full bg-[var(--color-border)]" />
                <span>{users}</span>
              </div>

              {/* CTA */}
              <div className="mt-10">
                {isAW ? (
                  <a
                    href="https://aw.yunjii.cn"
                    target="_blank"
                    rel="noopener"
                    className="inline-flex items-center gap-2 px-8 py-3.5 bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white rounded-xl font-semibold text-base transition-colors shadow-[0_0_30px_rgba(30,108,240,0.2)]"
                  >
                    访问 AgentWork 官网
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                      <path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3" />
                    </svg>
                  </a>
                ) : (
                  <button
                    onClick={onClose}
                    className="px-8 py-3.5 border border-[var(--color-border)] hover:border-[var(--color-border-hover)] text-[var(--color-text)] rounded-xl font-semibold text-base transition-colors"
                  >
                    返回产品矩阵
                  </button>
                )}
              </div>
            </motion.div>
          </div>
        </div>
      </motion.div>
    </AnimatePresence>
  );
}
