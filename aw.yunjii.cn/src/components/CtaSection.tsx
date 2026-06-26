import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function CtaSection() {
  const { t } = useTranslation();

  return (
    <section className="py-20 sm:py-28 px-4 sm:px-6">
      <div className="max-w-2xl mx-auto text-center">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.5 }}
        >
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">
            {t("cta_section.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg mb-10">
            {t("cta_section.subtitle")}
          </p>
          <motion.a
            whileHover={{ scale: 1.04 }}
            whileTap={{ scale: 0.97 }}
            href="#"
            className="inline-flex items-center gap-2 px-10 py-4 bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white rounded-xl font-semibold text-lg transition-colors shadow-[var(--shadow-glow)] hover:shadow-[var(--shadow-glow-strong)]"
          >
            {t("cta_section.button")}
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M5 12h14M12 5l7 7-7 7" />
            </svg>
          </motion.a>
        </motion.div>
      </div>
    </section>
  );
}
