import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { products } from "../content/products";
import type { Product } from "../content/products";

const statusLabels: Record<Product["status"], string> = {
  stable: "products.stable",
  beta: "products.beta",
  coming: "products.coming",
};

const statusStyles: Record<Product["status"], string> = {
  stable: "bg-[var(--color-success)]/10 text-[var(--color-success)] border-[var(--color-success)]/20",
  beta: "bg-[var(--color-warning)]/10 text-[var(--color-warning)] border-[var(--color-warning)]/20",
  coming: "bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-soft)] border-[var(--color-aw-primary)]/20",
};

const productI18nPrefix: Record<string, string> = {
  pc: "products.pc",
  web: "products.web",
  team: "products.team",
  agentwork: "products.agentwork",
};

function ProductCard({ product, index }: { product: Product; index: number }) {
  const { t } = useTranslation();
  const isAW = product.id === "agentwork";

  return (
    <motion.div
      initial={{ opacity: 0, y: 30 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-80px" }}
      transition={{ duration: 0.45, delay: index * 0.08 }}
      whileHover={{ y: -6, scale: 1.01 }}
      className={`group relative rounded-2xl border p-6 sm:p-8 transition-all duration-300 cursor-default ${
        isAW
          ? "border-[var(--color-aw-primary)]/30 bg-[var(--color-aw-primary)]/5 hover:border-[var(--color-aw-primary)]/50"
          : "border-[var(--color-border)] bg-[var(--color-surface)] hover:border-[var(--color-border-hover)]"
      }`}
    >
      {/* Hover glow */}
      <div className={`absolute inset-0 rounded-2xl opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none ${
        isAW ? "shadow-[inset_0_0_60px_rgba(30,108,240,0.06)]" : "shadow-[inset_0_0_40px_rgba(0,0,0,0.04)]"
      }`} />

      <div className="relative">
        {/* Status + tech */}
        <div className="flex items-center gap-3 mb-4">
          <span className={`text-xs font-medium px-2.5 py-0.5 rounded-full border ${statusStyles[product.status]}`}>
            {t(statusLabels[product.status])}
          </span>
          <span className="text-xs text-[var(--color-text-faint)] font-mono">{product.tech}</span>
        </div>

        {/* Name + tier */}
        <div className="flex items-start justify-between mb-2">
          <h3 className="text-xl font-bold text-[var(--color-text)]">{t(`${productI18nPrefix[product.id]}.name`)}</h3>
          <span className="text-xs font-semibold text-[var(--color-aw-primary)] bg-[var(--color-aw-primary)]/10 px-2 py-0.5 rounded">
            {product.tier}
          </span>
        </div>

        <p className="text-[var(--color-text-muted)] text-sm leading-relaxed mb-4">
          {t(`${productI18nPrefix[product.id]}.tagline`)}
        </p>

        {/* Price */}
        <div className="mb-5">
          <span className="text-2xl font-bold text-[var(--color-text)]">{product.price}</span>
          <p className="text-xs text-[var(--color-text-faint)] mt-1">{product.priceNote}</p>
        </div>

        {/* Highlights */}
        <ul className="space-y-2.5 mb-6">
          {product.highlights.map((h, i) => (
            <li key={i} className="flex items-start gap-2.5 text-sm text-[var(--color-text-muted)]">
              <svg className="w-4 h-4 mt-0.5 shrink-0 text-[var(--color-aw-soft)]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                <path d="M5 13l4 4L19 7" />
              </svg>
              {h}
            </li>
          ))}
        </ul>

        {/* Target */}
        <p className="text-xs text-[var(--color-text-faint)] mb-6">
          <span className="text-[var(--color-text-muted)]">{t("products.suitableFor")}</span>
          {product.targetUsers}
        </p>

        {/* CTA */}
        <motion.a
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
          href={product.cta.href}
          className={`block text-center py-2.5 rounded-lg font-medium text-sm transition-all ${
            product.cta.primary || isAW
              ? "bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white shadow-[var(--shadow-glow)]"
              : "border border-[var(--color-border)] text-[var(--color-text)] hover:border-[var(--color-border-hover)] hover:bg-[var(--color-border)]"
          }`}
        >
          {product.cta.label}
        </motion.a>
      </div>
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
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5 }}
          className="text-center mb-14 sm:mb-20"
        >
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">
            {t("products.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg max-w-2xl mx-auto leading-relaxed">
            {t("products.subtitle")}
          </p>
        </motion.div>

        <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-5 sm:gap-6">
          {products.map((p, i) => (
            <ProductCard key={p.id} product={p} index={i} />
          ))}
        </div>
      </div>
    </section>
  );
}
