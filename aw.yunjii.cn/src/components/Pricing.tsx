import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function Pricing() {
  const { t } = useTranslation();
  const features = t("pricing.features", { returnObjects: true }) as string[];

  return (
    <section id="pricing" className="py-20 sm:py-28 px-4 sm:px-6 bg-[var(--color-surface)]/40">
      <div className="max-w-lg mx-auto">
        <motion.div initial={{ opacity: 0, y: 20 }} whileInView={{ opacity: 1, y: 0 }} viewport={{ once: true, margin: "-80px" }} transition={{ duration: 0.5 }} className="text-center mb-12">
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">{t("pricing.title")}</h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg">{t("pricing.subtitle")}</p>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className="rounded-2xl border border-[var(--color-aw-primary)]/30 bg-[var(--color-aw-primary)]/5 p-8 sm:p-10 text-center ring-1 ring-[var(--color-aw-primary)]/20"
        >
          <div className="text-xs font-semibold text-[var(--color-aw-soft)] bg-[var(--color-aw-primary)]/10 px-3 py-1 rounded-full inline-block mb-6">
            Enterprise
          </div>

          <div className="mb-8">
            <span className="text-5xl font-bold text-[var(--color-text)]">{t("pricing.price")}</span>
            <span className="text-[var(--color-text-muted)] ml-1">{t("pricing.period")}</span>
          </div>

          <ul className="space-y-3 mb-10 text-left">
            {features.map((f, i) => (
              <li key={i} className="flex items-start gap-3 text-sm text-[var(--color-text-muted)]">
                <svg className="w-4 h-4 mt-0.5 shrink-0 text-[var(--color-aw-soft)]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                  <path d="M5 13l4 4L19 7" />
                </svg>
                {f}
              </li>
            ))}
          </ul>

          <motion.a
            whileHover={{ scale: 1.03 }}
            whileTap={{ scale: 0.97 }}
            href="#"
            className="block py-3.5 bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white rounded-xl font-semibold transition-colors shadow-[var(--shadow-glow)] mb-4"
          >
            {t("pricing.cta")}
          </motion.a>

          <p className="text-xs text-[var(--color-text-faint)]">{t("pricing.footer")}</p>
        </motion.div>
      </div>
    </section>
  );
}
