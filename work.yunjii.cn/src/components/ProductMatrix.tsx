import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { products } from "../content/products";
import type { Product } from "../content/products";

const statusDot: Record<Product["status"], string> = {
  stable: "bg-emerald-400 shadow-[0_0_8px_rgba(34,197,94,0.4)]",
  beta: "bg-amber-400 shadow-[0_0_8px_rgba(234,179,8,0.4)]",
  coming: "bg-[var(--color-aw-primary)] shadow-[0_0_8px_rgba(30,108,240,0.4)]",
};

const statusText: Record<Product["status"], string> = {
  stable: "稳定版",
  beta: "Beta",
  coming: "即将发布",
};

const productI18nPrefix: Record<string, string> = {
  pc: "products.pc",
  web: "products.web",
  team: "products.team",
  agentwork: "products.agentwork",
};

const productIcons: Record<string, ReactNode> = {
  pc: (
    <svg width="44" height="44" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.3">
      <rect x="2" y="3" width="20" height="14" rx="2" />
      <path d="M8 21h8M12 17v4" />
    </svg>
  ),
  web: (
    <svg width="44" height="44" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.3">
      <circle cx="12" cy="12" r="10" />
      <path d="M2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z" />
    </svg>
  ),
  team: (
    <svg width="44" height="44" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.3">
      <path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2" />
      <circle cx="9" cy="7" r="4" />
      <path d="M23 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75" />
    </svg>
  ),
  agentwork: (
    <svg width="44" height="44" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.3">
      <rect x="3" y="3" width="7" height="7" rx="1" />
      <rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="14" y="14" width="7" height="7" rx="1" />
    </svg>
  ),
};

const sectionAnchors: Record<string, string> = {
  pc: "#desktop-detail",
  web: "#web-detail",
  team: "#team-detail",
  agentwork: "https://aw.yunjii.cn",
};

const linkTargets: Record<string, string | undefined> = {
  pc: undefined,
  web: undefined,
  team: undefined,
  agentwork: "_blank",
};

function ProductFeatureCard({ product, index }: { product: Product; index: number }) {
  const { t } = useTranslation();
  const isAW = product.id === "agentwork";
  const prefix = productI18nPrefix[product.id];
  const href = sectionAnchors[product.id] ?? "#";
  const target = linkTargets[product.id];

  const cardBorder = isAW
    ? "border-[var(--color-aw-primary)]/20 hover:border-[var(--color-aw-primary)]/40"
    : "border-[var(--color-border)] hover:border-[var(--color-border-hover)]";

  const cardBg = isAW
    ? "bg-gradient-to-b from-[var(--color-aw-primary)]/[0.025] to-[var(--color-surface)]/50 backdrop-blur-sm"
    : "bg-gradient-to-b from-[var(--color-surface)] to-[var(--color-surface)]/50 backdrop-blur-sm";

  const iconColor = isAW
    ? "text-[var(--color-aw-soft)]"
    : "text-[var(--color-text-muted)]";

  const iconBg = isAW
    ? "bg-[var(--color-aw-primary)]/10"
    : "bg-[var(--color-border)]";

  return (
    <motion.div
      initial={{ opacity: 0, y: 28 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-40px" }}
      transition={{ duration: 0.45, delay: index * 0.08 }}
      whileHover={{ y: -4 }}
      className={`group relative rounded-2xl border ${cardBorder} ${cardBg} p-6 sm:p-7 transition-all duration-300 hover:shadow-[var(--shadow-card-hover)] flex flex-col`}
    >
      {/* Top row: icon + status */}
      <div className="flex items-start justify-between mb-4">
        <div className={`w-16 h-16 rounded-xl ${iconBg} ${iconColor} flex items-center justify-center group-hover:scale-105 transition-transform duration-300`}>
          {productIcons[product.id]}
        </div>
        <div className="flex items-center gap-2">
          <span className={`w-2 h-2 rounded-full ${statusDot[product.status]}`} />
          <span className="text-[11px] font-medium text-[var(--color-text-faint)]">{statusText[product.status]}</span>
        </div>
      </div>

      {/* Product name — bigger */}
      <h3 className={`text-xl font-bold text-[var(--color-text)] mb-3 ${isAW ? "group-hover:text-[var(--color-aw-soft)]" : "group-hover:text-[var(--color-yj-red)]"} transition-colors`}>
        {t(`${prefix}.name`)}
      </h3>

      {/* Description — fuller */}
      <p className="text-sm text-[var(--color-text-muted)] leading-relaxed mb-5 flex-1">
        {t(`${prefix}.desc`)}
      </p>

      {/* Feature chips — show all highlights */}
      <div className="flex flex-wrap gap-1.5 mb-6">
        {product.highlights.map((h, i) => (
          <span
            key={i}
            className={`text-[11px] leading-none px-2.5 py-1.5 rounded-lg border text-[var(--color-text-muted)] ${
              isAW ? "border-[var(--color-aw-primary)]/15 bg-[var(--color-aw-primary)]/5" : "border-[var(--color-border)] bg-[var(--color-bg)]/30"
            }`}
          >
            {h}
          </span>
        ))}
      </div>

      {/* Detail button — prominent */}
      <a
        href={href}
        target={target}
        rel={target ? "noopener" : undefined}
        className={`group/btn inline-flex items-center justify-center gap-2 w-full py-3 rounded-xl font-semibold text-sm transition-all ${
          isAW
            ? "bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white shadow-[0_0_20px_rgba(30,108,240,0.2)]"
            : "border border-[var(--color-border)] text-[var(--color-text)] hover:border-[var(--color-yj-red)]/30 hover:text-[var(--color-yj-red)] hover:bg-[var(--color-yj-red)]/[0.03]"
        }`}
      >
        {isAW ? "访问 AgentWork 官网" : "查看详情"}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="group-hover/btn:translate-x-0.5 transition-transform">
          <path d="M5 12h14M12 5l7 7-7 7" />
        </svg>
      </a>
    </motion.div>
  );
}

export default function ProductMatrix() {
  const { t } = useTranslation();

  return (
    <section id="products" className="py-20 sm:py-28 px-4 sm:px-6">
      <div className="max-w-7xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.5 }}
          className="mb-12 sm:mb-16 text-center"
        >
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text-faint)] text-xs font-medium mb-5">
            产品矩阵
          </div>
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-3">
            {t("products.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-sm sm:text-base max-w-xl mx-auto leading-relaxed">
            {t("products.subtitle")}
          </p>
        </motion.div>

        <div className="grid sm:grid-cols-2 xl:grid-cols-4 gap-4 sm:gap-5">
          {products.map((p, i) => (
            <ProductFeatureCard key={p.id} product={p} index={i} />
          ))}
        </div>
      </div>
    </section>
  );
}
