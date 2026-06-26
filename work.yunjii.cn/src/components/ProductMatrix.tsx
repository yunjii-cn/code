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

const productSlugs: Record<string, string> = {
  pc: "desktop",
  web: "web",
  team: "team",
  agentwork: "agentwork",
};

const productIcons: Record<string, ReactNode> = {
  pc: (
    <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4">
      <rect x="2" y="3" width="20" height="14" rx="2" />
      <path d="M8 21h8M12 17v4" />
    </svg>
  ),
  web: (
    <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4">
      <circle cx="12" cy="12" r="10" />
      <path d="M2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z" />
    </svg>
  ),
  team: (
    <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4">
      <path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2" />
      <circle cx="9" cy="7" r="4" />
      <path d="M23 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75" />
    </svg>
  ),
  agentwork: (
    <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4">
      <rect x="3" y="3" width="7" height="7" rx="1" />
      <rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="14" y="14" width="7" height="7" rx="1" />
    </svg>
  ),
};

function ProductFeatureCard({ product, index }: { product: Product; index: number }) {
  const { t } = useTranslation();
  const isAW = product.id === "agentwork";
  const slug = productSlugs[product.id];

  const cardBorder = isAW
    ? "border-[var(--color-aw-primary)]/20 hover:border-[var(--color-aw-primary)]/40"
    : "border-[var(--color-border)] hover:border-[var(--color-border-hover)]";

  const cardBg = isAW
    ? "bg-[var(--color-aw-primary)]/[0.015]"
    : "bg-[var(--color-surface)]";

  const iconColor = isAW
    ? "text-[var(--color-aw-soft)]"
    : "text-[var(--color-text-muted)]";

  const iconBg = isAW
    ? "bg-[var(--color-aw-primary)]/10"
    : "bg-[var(--color-border)]";

  return (
    <motion.a
      href={`#${slug}`}
      initial={{ opacity: 0, y: 28 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-40px" }}
      transition={{ duration: 0.45, delay: index * 0.08 }}
      whileHover={{ y: -4 }}
      className={`group relative rounded-2xl border ${cardBorder} ${cardBg} p-6 sm:p-7 transition-all duration-300 hover:shadow-[var(--shadow-card-hover)] flex flex-col`}
    >
      {/* Top row: icon + status + tier */}
      <div className="flex items-start justify-between mb-5">
        <div className={`w-14 h-14 rounded-xl ${iconBg} ${iconColor} flex items-center justify-center group-hover:scale-105 transition-transform duration-300`}>
          {productIcons[product.id]}
        </div>
        <div className="flex items-center gap-2">
          <span className={`w-2 h-2 rounded-full ${statusDot[product.status]}`} />
          <span className="text-[11px] font-medium text-[var(--color-text-faint)]">{statusText[product.status]}</span>
        </div>
      </div>

      {/* Product name + Tier badge */}
      <div className="flex items-center gap-2.5 mb-2.5">
        <h3 className="text-lg font-bold text-[var(--color-text)] group-hover:text-[var(--color-yj-red)] transition-colors">
          {t(`${productI18nPrefix[product.id]}.name`)}
        </h3>
        <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-text-faint)] bg-[var(--color-border)] px-2 py-0.5 rounded-md">
          {product.tier}
        </span>
      </div>

      {/* Core value proposition — big */}
      <p className="text-sm text-[var(--color-text-muted)] leading-relaxed mb-5 flex-1">
        {t(`${productI18nPrefix[product.id]}.tagline`)}
      </p>

      {/* Feature chips */}
      <div className="flex flex-wrap gap-1.5 mb-5">
        {product.highlights.slice(0, 3).map((h, i) => (
          <span
            key={i}
            className="text-[11px] leading-none px-2.5 py-1.5 rounded-lg border border-[var(--color-border)] text-[var(--color-text-muted)] bg-[var(--color-bg)]/50"
          >
            {h}
          </span>
        ))}
      </div>

      {/* Bottom info bar */}
      <div className="pt-4 border-t border-[var(--color-border)] flex items-center justify-between">
        <span className="text-[11px] font-mono text-[var(--color-text-faint)]">{product.tech}</span>
        <div className="flex items-center gap-1 text-[11px] text-[var(--color-text-faint)] group-hover:text-[var(--color-text-muted)] transition-colors">
          <span>了解详情</span>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="group-hover:translate-x-0.5 transition-transform">
            <path d="M5 12h14M12 5l7 7-7 7" />
          </svg>
        </div>
      </div>
    </motion.a>
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
