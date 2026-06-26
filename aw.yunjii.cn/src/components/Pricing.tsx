import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function Pricing() {
  const { t } = useTranslation();
  const features = t("pricing.features", { returnObjects: true }) as string[];

  return (
    <section id="pricing" className="relative py-28 sm:py-36 px-4 sm:px-6">
      <div className="absolute inset-0 bg-[var(--color-aw-primary)]/[0.012]" />

      <div className="relative max-w-xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.55 }}
          className="text-center mb-18"
        >
          <div className="inline-flex items-center gap-2.5 px-4 py-1.5 rounded-full border border-[var(--color-aw-primary)]/8 bg-[var(--color-aw-primary)]/3 text-[var(--color-aw-soft)] text-xs font-semibold tracking-wider uppercase mb-7">
            定价
          </div>
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold text-[var(--color-text)] mb-5 tracking-tight">
            {t("pricing.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg">
            {t("pricing.subtitle")}
          </p>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 40 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          whileHover={{ y: -4 }}
          className="relative rounded-3xl bg-[var(--color-surface)] border border-[var(--color-aw-primary)]/15 p-10 sm:p-12 text-center aw-ring-glow hover:shadow-[0_0_50px_rgba(30,108,240,0.12)] transition-all duration-500"
        >
          {/* Top gradient line */}
          <div className="absolute top-0 left-8 right-8 h-px bg-gradient-to-r from-transparent via-[var(--color-aw-primary)]/40 to-transparent" />

          {/* Badge */}
          <div className="text-xs font-bold text-[var(--color-aw-soft)] bg-[var(--color-aw-primary)]/10 px-5 py-2 rounded-full inline-flex items-center gap-2.5 mb-10 tracking-wider uppercase">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor" className="text-[var(--color-aw-primary)]">
              <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
            </svg>
            Enterprise
          </div>

          {/* Price */}
          <div className="mb-12">
            <span className="text-7xl font-extrabold text-[var(--color-text)] tracking-tight">{t("pricing.price")}</span>
            <span className="text-[var(--color-text-muted)] text-xl ml-2 font-medium">{t("pricing.period")}</span>
          </div>

          {/* Feature list */}
          <ul className="space-y-4 mb-14 text-left max-w-sm mx-auto">
            {features.map((f, i) => (
              <motion.li
                key={i}
                initial={{ opacity: 0, x: -8 }}
                whileInView={{ opacity: 1, x: 0 }}
                viewport={{ once: true }}
                transition={{ delay: i * 0.06 }}
                className="flex items-start gap-3.5 text-sm text-[var(--color-text-muted)]"
              >
                <div className="w-6 h-6 rounded-full bg-[var(--color-aw-primary)]/10 flex items-center justify-center shrink-0 mt-0.5">
                  <svg className="w-3.5 h-3.5 text-[var(--color-aw-soft)]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="3">
                    <path d="M5 13l4 4L19 7" />
                  </svg>
                </div>
                <span className="leading-relaxed">{f}</span>
              </motion.li>
            ))}
          </ul>

          {/* CTA */}
          <motion.a
            whileHover={{ scale: 1.03 }}
            whileTap={{ scale: 0.97 }}
            href="#"
            className="block py-4 bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white rounded-xl font-bold text-lg transition-all shadow-[0_0_30px_rgba(30,108,240,0.25)] hover:shadow-[0_0_45px_rgba(30,108,240,0.4)] mb-6"
          >
            {t("pricing.cta")}
          </motion.a>

          <p className="text-xs text-[var(--color-text-faint)]">{t("pricing.footer")}</p>
        </motion.div>
      </div>
    </section>
  );
}
