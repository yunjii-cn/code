import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function CtaSection() {
  const { t } = useTranslation();

  return (
    <section className="relative py-28 sm:py-40 px-4 sm:px-6 overflow-hidden">
      {/* Premium CTA atmosphere */}
      <div className="absolute inset-0 bg-[var(--color-aw-primary)]/[0.025]" />
      <motion.div
        animate={{ opacity: [0.12, 0.28, 0.12] }}
        transition={{ duration: 5.5, repeat: Infinity }}
        className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[700px] h-[350px] bg-[var(--color-aw-primary)]/6 blur-[180px] rounded-full pointer-events-none"
      />
      <motion.div
        animate={{ opacity: [0.06, 0.15, 0.06] }}
        transition={{ duration: 6, repeat: Infinity, delay: 1 }}
        className="absolute top-1/3 left-1/3 w-[400px] h-[200px] bg-[var(--color-aw-soft)]/5 blur-[140px] rounded-full pointer-events-none"
      />

      <div className="relative z-10 max-w-3xl mx-auto text-center">
        <motion.div
          initial={{ opacity: 0, y: 32 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.65 }}
        >
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-extrabold text-[var(--color-text)] mb-5 tracking-tight leading-tight">
            {t("cta_section.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg mb-12 leading-relaxed max-w-xl mx-auto">
            {t("cta_section.subtitle")}
          </p>

          <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
            <motion.a
              whileHover={{ scale: 1.04 }}
              whileTap={{ scale: 0.96 }}
              href="#"
              className="group w-full sm:w-auto inline-flex items-center gap-3 px-12 py-4 bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white rounded-xl font-bold text-lg transition-all aw-ring-glow hover:shadow-[0_0_40px_rgba(30,108,240,0.35)]"
            >
              {t("cta_section.button")}
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="group-hover:translate-x-0.5 transition-transform">
                <path d="M5 12h14M12 5l7 7-7 7" />
              </svg>
            </motion.a>
            <motion.a
              whileHover={{ scale: 1.03 }}
              href="https://work.yunjii.cn"
              className="w-full sm:w-auto px-10 py-4 border border-[var(--color-aw-primary)]/15 backdrop-blur-sm text-[var(--color-aw-soft)] rounded-xl font-medium text-base hover:border-[var(--color-aw-primary)]/35 hover:bg-[var(--color-aw-primary)]/10 transition-all"
            >
              查看产品矩阵
            </motion.a>
          </div>

          <p className="mt-8 text-xs text-[var(--color-text-faint)]">
            无需绑定信用卡 · 14 天免费试用 · 随时取消
          </p>
        </motion.div>
      </div>
    </section>
  );
}
