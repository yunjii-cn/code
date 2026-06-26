import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function CtaSection() {
  const { t } = useTranslation();

  return (
    <section className="relative py-24 sm:py-36 px-4 sm:px-6 overflow-hidden">
      {/* Blue CTA atmosphere */}
      <div className="absolute inset-0 bg-[var(--color-aw-primary)]/[0.03]" />
      <motion.div
        animate={{ opacity: [0.15, 0.3, 0.15] }}
        transition={{ duration: 5, repeat: Infinity }}
        className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[300px] bg-[var(--color-aw-primary)]/8 blur-[160px] rounded-full"
      />

      <div className="relative z-10 max-w-2xl mx-auto text-center">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.6 }}
        >
          <h2 className="text-3xl sm:text-5xl font-extrabold text-[var(--color-text)] mb-4 tracking-tight">
            {t("cta_section.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg mb-10">
            {t("cta_section.subtitle")}
          </p>
          <motion.a
            whileHover={{ scale: 1.04 }}
            whileTap={{ scale: 0.96 }}
            href="#"
            className="inline-flex items-center gap-3 px-12 py-4 bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white rounded-xl font-semibold text-lg transition-colors aw-ring-glow"
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
