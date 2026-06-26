import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function Hero() {
  const { t } = useTranslation();

  return (
    <section className="relative min-h-screen flex items-center justify-center overflow-hidden">
      {/* Deep immersive AW blue atmosphere */}
      <div className="absolute inset-0 bg-aw-mesh" />
      <div className="absolute inset-0 bg-aw-lines opacity-60" />

      {/* Animated glow orbs */}
      <motion.div
        animate={{ scale: [1, 1.1, 1], opacity: [0.3, 0.5, 0.3] }}
        transition={{ duration: 8, repeat: Infinity, ease: "easeInOut" }}
        className="aw-drift-slow absolute top-1/4 left-1/4 w-[500px] h-[500px] bg-[var(--color-aw-primary)]/8 blur-[150px] rounded-full"
      />
      <motion.div
        animate={{ scale: [1, 1.15, 1], opacity: [0.2, 0.4, 0.2] }}
        transition={{ duration: 10, repeat: Infinity, ease: "easeInOut", delay: 2 }}
        className="aw-drift-fast absolute bottom-1/4 right-1/4 w-[400px] h-[400px] bg-[var(--color-aw-soft)]/6 blur-[120px] rounded-full"
      />
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[300px] bg-gradient-to-r from-[var(--color-aw-primary)]/5 via-transparent to-[var(--color-aw-soft)]/5 blur-[180px] rounded-full aw-pulse" />

      {/* Subtle dot grid — tech feel */}
      <div className="absolute inset-0 bg-[radial-gradient(circle,rgba(30,108,240,0.06)_1px,transparent_1px)] bg-[size:40px_40px] opacity-50" />

      <div className="relative z-10 max-w-4xl mx-auto px-4 sm:px-6 text-center pt-24 sm:pt-20">
        {/* Enterprise badge */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className="inline-flex items-center gap-2.5 px-4 py-2 rounded-full border border-[var(--color-aw-primary)]/20 bg-[var(--color-aw-primary)]/5 text-[var(--color-aw-soft)] text-sm mb-10"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-[var(--color-aw-primary)]">
            <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
          </svg>
          {t("hero.badge")}
        </motion.div>

        {/* Hero title — huge, bold, blue gradient */}
        <motion.h1
          initial={{ opacity: 0, y: 36 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.7, delay: 0.2 }}
          className="text-4xl sm:text-5xl md:text-7xl lg:text-8xl font-extrabold tracking-tight leading-[1.05] mb-8"
        >
          <span className="text-[var(--color-text)]">{t("hero.title1")}</span>
          <br />
          <span className="text-gradient-aw">{t("hero.title2")}</span>
        </motion.h1>

        {/* Description */}
        <motion.p
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, delay: 0.4 }}
          className="text-base sm:text-lg md:text-xl text-[var(--color-text-muted)] max-w-3xl mx-auto mb-12 leading-relaxed"
        >
          {t("hero.description")}
        </motion.p>

        {/* CTA */}
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, delay: 0.55 }}
          className="flex flex-col sm:flex-row items-center justify-center gap-4"
        >
          <motion.a
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.96 }}
            href="#pricing"
            className="w-full sm:w-auto px-10 py-4 bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white rounded-xl font-semibold text-lg transition-colors aw-ring-glow group"
          >
            <span className="flex items-center justify-center gap-2">
              {t("hero.cta")}
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="group-hover:translate-x-0.5 transition-transform">
                <path d="M5 12h14M12 5l7 7-7 7" />
              </svg>
            </span>
          </motion.a>
          <motion.a
            whileHover={{ scale: 1.03 }}
            href="#features"
            className="w-full sm:w-auto px-8 py-4 border border-[var(--color-aw-primary)]/20 text-[var(--color-aw-soft)] rounded-xl font-medium text-base hover:border-[var(--color-aw-primary)]/40 hover:bg-[var(--color-aw-primary)]/5 transition-all"
          >
            探索能力
          </motion.a>
        </motion.div>

        {/* Floating tech chips */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.8, duration: 0.6 }}
          className="mt-16 flex flex-wrap items-center justify-center gap-3"
        >
          {["Tauri 2", "Rust", "Multi-Agent DAG", "AST-Native", "TimeFlow"].map((tech, i) => (
            <motion.span
              key={tech}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.85 + i * 0.06 }}
              className="text-xs font-mono text-[var(--color-aw-soft)]/70 border border-[var(--color-aw-primary)]/10 rounded-lg px-3 py-1.5 bg-[var(--color-aw-primary)]/3"
            >
              {tech}
            </motion.span>
          ))}
        </motion.div>
      </div>

      {/* Bottom gradient fade */}
      <div className="absolute bottom-0 left-0 right-0 h-32 bg-gradient-to-t from-[var(--color-bg)] to-transparent pointer-events-none" />

      {/* Scroll indicator */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 1.5 }}
        className="absolute bottom-8 left-1/2 -translate-x-1/2 z-10"
      >
        <motion.div
          animate={{ y: [0, 8, 0] }}
          transition={{ repeat: Infinity, duration: 2, ease: "easeInOut" }}
          className="flex flex-col items-center gap-2"
        >
          <span className="text-[10px] text-[var(--color-text-faint)] uppercase tracking-widest">Scroll</span>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-[var(--color-text-faint)]">
            <path d="M6 9l6 6 6-6" />
          </svg>
        </motion.div>
      </motion.div>
    </section>
  );
}
