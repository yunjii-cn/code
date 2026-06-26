import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import HeroCodeFlow from "./HeroCodeFlow";

const techStack = ["Tauri 2", "Rust", "Multi-Agent DAG", "AST-Native", "TimeFlow"];

const fadeUp = (delay: number) => ({
  initial: { opacity: 0, y: 36 },
  animate: { opacity: 1, y: 0 },
  transition: { duration: 0.7, delay, ease: [0.22, 1, 0.36, 1] as const },
});

export default function Hero() {
  const { t } = useTranslation();

  return (
    <section className="relative min-h-[90vh] sm:min-h-screen flex items-center justify-center overflow-hidden">
      {/* Deep immersive AW blue atmosphere — multi-layered */}
      <div className="absolute inset-0 bg-aw-mesh" />
      <div className="absolute inset-0" style={{ background: "radial-gradient(ellipse 80% 50% at 50% 35%, rgba(30,108,240,0.12) 0%, transparent 70%)" }} />

      {/* Code flow canvas */}
      <HeroCodeFlow />

      {/* Animated glow orbs — premium floating lights */}
      <motion.div
        animate={{ scale: [1, 1.08, 1], opacity: [0.25, 0.45, 0.25] }}
        transition={{ duration: 9, repeat: Infinity, ease: "easeInOut" }}
        className="aw-drift-slow absolute top-1/4 left-1/4 w-[500px] h-[500px] bg-[var(--color-aw-primary)]/6 blur-[160px] rounded-full pointer-events-none"
      />
      <motion.div
        animate={{ scale: [1, 1.12, 1], opacity: [0.15, 0.35, 0.15] }}
        transition={{ duration: 11, repeat: Infinity, ease: "easeInOut", delay: 2 }}
        className="aw-drift-fast absolute bottom-1/3 right-1/4 w-[400px] h-[400px] bg-[var(--color-aw-soft)]/4 blur-[130px] rounded-full pointer-events-none"
      />
      <motion.div
        animate={{ opacity: [0.2, 0.5, 0.2] }}
        transition={{ duration: 4.5, repeat: Infinity }}
        className="aw-pulse absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[650px] h-[250px] bg-gradient-to-r from-[var(--color-aw-primary)]/4 via-transparent to-[var(--color-aw-soft)]/4 blur-[180px] rounded-full pointer-events-none"
      />

      <div className="relative z-10 max-w-5xl mx-auto px-4 sm:px-6 text-center pt-24 sm:pt-16">
        {/* Premium enterprise badge — glass morphism */}
        <motion.div
          {...fadeUp(0.1)}
          className="inline-flex items-center gap-3 px-5 py-2.5 rounded-full border border-[var(--color-aw-primary)]/15 bg-[var(--color-aw-primary)]/[0.04] backdrop-blur-xl text-[var(--color-aw-soft)] text-sm font-medium mb-12"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
            <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
          </svg>
          {t("hero.badge")}
        </motion.div>

        {/* Hero title */}
        <motion.h1
          {...fadeUp(0.2)}
          className="text-5xl sm:text-6xl lg:text-7xl font-extrabold tracking-tight leading-[1.06] mb-6"
        >
          <span className="text-[var(--color-text)]">{t("hero.title1")}</span>
        </motion.h1>

        {/* Tagline — confident weight, AW soft gradient, sized for impact */}
        <motion.p
          {...fadeUp(0.3)}
          className="text-2xl sm:text-3xl lg:text-5xl font-semibold tracking-tight leading-snug text-[var(--color-aw-soft)] mb-10"
        >
          {t("hero.title2a")}
          <br />
          {t("hero.title2b")}
        </motion.p>

        {/* Description — refined, larger */}
        <motion.p
          {...fadeUp(0.4)}
          className="text-base sm:text-lg text-[var(--color-text-muted)] max-w-2xl mx-auto mb-12 leading-relaxed"
        >
          {t("hero.description")}
        </motion.p>

        {/* CTA — premium glow button */}
        <motion.div
          {...fadeUp(0.55)}
          className="flex flex-col sm:flex-row items-center justify-center gap-4"
        >
          <motion.a
            whileHover={{ scale: 1.04 }}
            whileTap={{ scale: 0.97 }}
            href="#pricing"
            className="group w-full sm:w-auto inline-flex items-center gap-3 px-10 py-4 bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white rounded-xl font-semibold text-lg transition-all aw-ring-glow hover:shadow-[0_0_40px_rgba(30,108,240,0.35)]"
          >
            {t("hero.cta")}
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="group-hover:translate-x-0.5 transition-transform">
              <path d="M5 12h14M12 5l7 7-7 7" />
            </svg>
          </motion.a>
          <motion.a
            whileHover={{ scale: 1.03 }}
            href="#features"
            className="w-full sm:w-auto px-9 py-4 border border-[var(--color-aw-primary)]/15 text-[var(--color-aw-soft)] rounded-xl font-medium text-base hover:border-[var(--color-aw-primary)]/35 hover:bg-[var(--color-aw-primary)]/5 transition-all"
          >
            探索能力
          </motion.a>
        </motion.div>

        {/* Tech stack chips — refined */}
        <motion.div
          {...fadeUp(0.75)}
          className="mt-20 flex flex-wrap items-center justify-center gap-3"
        >
          {techStack.map((tech, i) => (
            <motion.span
              key={tech}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.8 + i * 0.06 }}
              className="text-xs font-mono text-[var(--color-aw-soft)]/60 border border-[var(--color-aw-primary)]/8 rounded-lg px-3 py-1.5 backdrop-blur-sm bg-[var(--color-aw-primary)]/3"
            >
              {tech}
            </motion.span>
          ))}
        </motion.div>
      </div>

      {/* Scroll indicator — refined */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 1.6 }}
        className="absolute bottom-10 left-1/2 -translate-x-1/2 z-10"
      >
        <motion.div
          animate={{ y: [0, 8, 0] }}
          transition={{ repeat: Infinity, duration: 2.2, ease: "easeInOut" }}
          className="flex flex-col items-center gap-2.5"
        >
          <span className="text-[10px] text-[var(--color-text-faint)] uppercase tracking-[0.2em] font-medium">Scroll</span>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-[var(--color-text-faint)]">
            <path d="M6 9l6 6 6-6" />
          </svg>
        </motion.div>
      </motion.div>

      {/* Bottom fade */}
      <div className="absolute bottom-0 left-0 right-0 h-40 bg-gradient-to-t from-[var(--color-bg)] via-[var(--color-bg)]/80 to-transparent pointer-events-none" />
    </section>
  );
}
