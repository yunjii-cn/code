import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function Pricing() {
  const { t } = useTranslation();
  const features = t("pricing.features", { returnObjects: true }) as string[];

  return (
    <section id="pricing" className="relative py-24 sm:py-32 px-4 sm:px-6">
      <div className="absolute inset-0 bg-[var(--color-aw-primary)]/[0.015]" />

      <div className="relative max-w-xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5 }}
          className="text-center mb-16"
        >
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-aw-primary)]/10 bg-[var(--color-aw-primary)]/3 text-[var(--color-aw-soft)] text-xs font-medium mb-6">
            定价
          </div>
          <h2 className="text-3xl sm:text-5xl font-bold text-[var(--color-text)] mb-4">{t("pricing.title")}</h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg">{t("pricing.subtitle")}</p>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 32 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="relative rounded-2xl bg-[var(--color-surface)] border border-[var(--color-aw-primary)]/20 p-8 sm:p-10 text-center aw-ring-glow"
        >
          {/* Top accent */}
          <div className="absolute top-0 left-4 right-4 h-px bg-gradient-to-r from-transparent via-[var(--color-aw-primary)]/30 to-transparent" />

          <div className="text-xs font-semibold text-[var(--color-aw-soft)] bg-[var(--color-aw-primary)]/10 px-4 py-1.5 rounded-full inline-flex items-center gap-2 mb-8">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
            </svg>
            Enterprise
          </div>

          <div className="mb-10">
            <span className="text-6xl font-extrabold text-[var(--color-text)] tracking-tight">{t("pricing.price")}</span>
            <span className="text-[var(--color-text-muted)] text-lg ml-2">{t("pricing.period")}</span>
          </div>

          <ul className="space-y-4 mb-12 text-left max-w-sm mx-auto">
            {features.map((f, i) => (
              <li key={i} className="flex items-start gap-3 text-sm text-[var(--color-text-muted)]">
                <div className="w-5 h-5 rounded-full bg-[var(--color-aw-primary)]/10 flex items-center justify-center shrink-0 mt-0.5">
                  <svg className="w-3 h-3 text-[var(--color-aw-soft)]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="3">
                    <path d="M5 13l4 4L19 7" />
                  </svg>
                </div>
                {f}
              </li>
            ))}
          </ul>

          <motion.a
            whileHover={{ scale: 1.03 }}
            whileTap={{ scale: 0.97 }}
            href="#"
            className="block py-4 bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white rounded-xl font-semibold text-lg transition-colors shadow-[var(--shadow-glow)] hover:shadow-[var(--shadow-glow-strong)] mb-5"
          >
            {t("pricing.cta")}
          </motion.a>

          <p className="text-xs text-[var(--color-text-faint)]">{t("pricing.footer")}</p>
        </motion.div>
      </div>
    </section>
  );
}
