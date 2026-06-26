import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { products } from "../content/products";
import type { Product } from "../content/products";

const statusDot: Record<Product["status"], string> = {
  stable: "bg-[var(--color-success)]",
  beta: "bg-[var(--color-warning)]",
  coming: "bg-[var(--color-aw-primary)]",
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
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-60px" }}
      transition={{ duration: 0.35, delay: index * 0.06 }}
      className={`group relative rounded-xl border p-5 sm:p-6 transition-all duration-200 ${
        isAW
          ? "border-[var(--color-aw-primary)]/20 bg-[var(--color-aw-primary)]/[0.02]"
          : "border-[var(--color-border)] bg-[var(--color-surface)] hover:border-[var(--color-border-hover)]"
      }`}
    >
      {/* Header row: name + status + tier */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2.5">
          <h3 className="text-base font-semibold text-[var(--color-text)]">
            {t(`${productI18nPrefix[product.id]}.name`)}
          </h3>
          <span className={`w-1.5 h-1.5 rounded-full ${statusDot[product.status]}`} />
        </div>
        <span className="text-[11px] font-medium text-[var(--color-text-faint)] bg-[var(--color-border)] px-2 py-0.5 rounded">
          {product.tier}
        </span>
      </div>

      {/* Tagline */}
      <p className="text-sm text-[var(--color-text-muted)] leading-relaxed mb-3">
        {t(`${productI18nPrefix[product.id]}.tagline`)}
      </p>

      {/* Price + CTA row — compact */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <span className="text-lg font-bold text-[var(--color-text)]">{product.price}</span>
          <span className="text-xs text-[var(--color-text-faint)] ml-1">{product.priceNote.split("，")[0]}</span>
        </div>
        <a
          href={product.cta.href}
          className={`text-xs font-medium px-3 py-1.5 rounded-lg transition-colors ${
            isAW
              ? "bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-soft)] hover:bg-[var(--color-aw-primary)]/20"
              : "text-[var(--color-text-muted)] hover:bg-[var(--color-border)]"
          }`}
        >
          {product.cta.label}
        </a>
      </div>

      {/* Highlights — 2-column compact */}
      <div className="grid grid-cols-2 gap-x-3 gap-y-1.5">
        {product.highlights.map((h, i) => (
          <div key={i} className="flex items-center gap-1.5 text-xs text-[var(--color-text-muted)]">
            <svg className="w-3 h-3 shrink-0 text-[var(--color-text-faint)]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2.5">
              <path d="M5 13l4 4L19 7" />
            </svg>
            <span className="truncate">{h}</span>
          </div>
        ))}
      </div>

      {/* Tech + target — subtle footer */}
      <div className="mt-4 pt-3 border-t border-[var(--color-border)] flex items-center justify-between text-[11px] text-[var(--color-text-faint)]">
        <span className="font-mono">{product.tech}</span>
        <span className="truncate ml-2">{product.targetUsers.split("/")[0]}</span>
      </div>
    </motion.div>
  );
}

export default function ProductMatrix() {
  const { t } = useTranslation();

  return (
    <section id="products" className="py-16 sm:py-24 px-4 sm:px-6 bg-[var(--color-surface)]/30">
      <div className="max-w-7xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.4 }}
          className="mb-10 sm:mb-14"
        >
          <div className="flex items-center gap-3 mb-3">
            <div className="w-1 h-5 rounded-full bg-[var(--color-yj-red)]" />
            <h2 className="text-2xl sm:text-3xl font-bold text-[var(--color-text)]">
              {t("products.title")}
            </h2>
          </div>
          <p className="text-[var(--color-text-muted)] text-sm sm:text-base max-w-xl">
            {t("products.subtitle")}
          </p>
        </motion.div>

        <div className="grid sm:grid-cols-2 xl:grid-cols-4 gap-4 sm:gap-5">
          {products.map((p, i) => (
            <ProductCard key={p.id} product={p} index={i} />
          ))}
        </div>

        {/* Bottom guide text */}
        <motion.p
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          className="mt-8 text-center text-xs text-[var(--color-text-faint)]"
        >
          不确定选哪个？查看下方 <a href="#pricing" className="text-[var(--color-aw-soft)] hover:underline">价格对比</a> 获取帮助。
        </motion.p>
      </div>
    </section>
  );
}
