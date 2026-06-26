import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { motion, useScroll, useTransform } from "framer-motion";

const stats = [
  { key: "hero.statLines", value: "4 代" },
  { key: "hero.statPrice", value: "¥0" },
  { key: "hero.statTech", value: "4 套" },
];

export default function Hero() {
  const { t } = useTranslation();
  const ref = useRef<HTMLElement>(null);
  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ["start start", "end start"],
  });
  const y = useTransform(scrollYProgress, [0, 1], [0, 80]);
  const opacity = useTransform(scrollYProgress, [0, 0.5], [1, 0]);

  return (
    <section ref={ref} className="relative min-h-screen flex items-center justify-center overflow-hidden">
      {/* Subtle background — cleaner, more portal-like */}
      <motion.div style={{ y, opacity }} className="absolute inset-0">
        <div className="absolute inset-0 bg-grid-pattern opacity-[0.25]" />
        {/* Single warm red-toned glow — not blue */}
        <div className="absolute top-1/4 left-1/2 -translate-x-1/2 w-[800px] h-[400px] bg-[var(--color-yj-red)]/[0.03] blur-[140px] rounded-full" />
        <div className="absolute bottom-0 left-0 w-[400px] h-[300px] bg-[var(--color-yj-red)]/[0.02] blur-[100px] rounded-full" />
      </motion.div>

      <div className="relative z-10 max-w-5xl mx-auto px-4 sm:px-6 text-center pt-24 sm:pt-20">
        {/* Status badge */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.45, delay: 0.1 }}
          className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-[var(--color-border)] bg-[var(--color-surface)]/70 backdrop-blur-sm text-[var(--color-text-muted)] text-sm mb-8"
        >
          <span className="w-1.5 h-1.5 rounded-full bg-[var(--color-success)]" />
          {t("hero.badge")}
        </motion.div>

        {/* Title — neutral dark tones, 云集红 on key phrase */}
        <motion.h1
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.2 }}
          className="text-4xl sm:text-5xl md:text-6xl lg:text-7xl font-bold tracking-tight leading-[1.12] mb-6"
        >
          <span className="text-[var(--color-text)]">{t("hero.title1")}</span>
          <br />
          <span
            className="bg-gradient-to-r from-[var(--color-yj-red)] to-[#ff4d47] bg-clip-text text-transparent"
          >
            {t("hero.title2")}
          </span>
        </motion.h1>

        {/* Description */}
        <motion.p
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.32 }}
          className="text-base sm:text-lg md:text-xl text-[var(--color-text-muted)] max-w-2xl mx-auto mb-10 leading-relaxed"
        >
          {t("hero.description")}
        </motion.p>

        {/* CTA buttons — 云集红 primary */}
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.44 }}
          className="flex flex-col sm:flex-row items-center justify-center gap-3 sm:gap-4"
        >
          <motion.a
            whileHover={{ scale: 1.04 }}
            whileTap={{ scale: 0.97 }}
            href="#products"
            className="w-full sm:w-auto px-8 py-3.5 bg-[var(--color-yj-red)] hover:bg-[var(--color-yj-red-deep)] text-white rounded-xl font-semibold text-base transition-colors"
          >
            {t("hero.ctaProducts")}
          </motion.a>
          <motion.a
            whileHover={{ scale: 1.04 }}
            whileTap={{ scale: 0.97 }}
            href="#pricing"
            className="w-full sm:w-auto px-8 py-3.5 border border-[var(--color-border)] hover:border-[var(--color-border-hover)] text-[var(--color-text)] rounded-xl font-medium text-base transition-colors"
          >
            {t("hero.ctaPricing")}
          </motion.a>
        </motion.div>

        {/* Stats — cleaner portal style */}
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.56 }}
          className="mt-16 sm:mt-20 grid grid-cols-3 gap-8 max-w-sm mx-auto"
        >
          {stats.map((stat, i) => (
            <motion.div
              key={stat.key}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 0.65 + i * 0.08 }}
            >
              <div className="text-2xl sm:text-3xl font-bold text-[var(--color-text)]">
                {stat.value}
              </div>
              <div className="text-xs sm:text-sm text-[var(--color-text-faint)] mt-1.5">
                {t(stat.key)}
              </div>
            </motion.div>
          ))}
        </motion.div>

        {/* Quick-jump product pills */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.45, delay: 0.7 }}
          className="mt-12 flex flex-wrap items-center justify-center gap-2"
        >
          <span className="text-xs text-[var(--color-text-faint)] mr-1">快速跳转：</span>
          {[
            { label: "云集 PC 版", href: "#products" },
            { label: "云集 Web 版", href: "#products" },
            { label: "云集团队版", href: "#products" },
            { label: "AgentWork", href: "#products" },
          ].map((p) => (
            <a
              key={p.label}
              href={p.href}
              className="px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-xs text-[var(--color-text-muted)] hover:text-[var(--color-text)] hover:border-[var(--color-border-hover)] transition-colors"
            >
              {p.label}
            </a>
          ))}
        </motion.div>
      </div>

      {/* Scroll indicator */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 1.2 }}
        className="absolute bottom-8 left-1/2 -translate-x-1/2"
      >
        <motion.div
          animate={{ y: [0, 8, 0] }}
          transition={{ repeat: Infinity, duration: 2, ease: "easeInOut" }}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-[var(--color-text-faint)]">
            <path d="M6 9l6 6 6-6" />
          </svg>
        </motion.div>
      </motion.div>
    </section>
  );
}
