import { useRef, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { motion, useScroll, useTransform } from "framer-motion";
import HeroParticles from "./HeroParticles";

const stats = [
  { key: "hero.statLines", value: "4 代" },
  { key: "hero.statPrice", value: "¥0" },
  { key: "hero.statTech", value: "4 套" },
];

const quickJumps = [
  { label: "云集 PC 版", href: "#desktop" },
  { label: "云集 Web 版", href: "#web" },
  { label: "云集团队版", href: "#team" },
  { label: "AgentWork", href: "#agentwork" },
];

export default function Hero() {
  const { t } = useTranslation();
  const ref = useRef<HTMLElement>(null);
  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ["start start", "end start"],
  });
  const y = useTransform(scrollYProgress, [0, 1], [0, 80]);
  const opacity = useTransform(scrollYProgress, [0, 0.6], [1, 0]);

  const [counters, setCounters] = useState([0, 0, 0]);
  const targets = [4, 0, 4];

  useEffect(() => {
    const timer = setTimeout(() => {
      const interval = setInterval(() => {
        setCounters((prev) => {
          const next = [...prev];
          let done = true;
          for (let i = 0; i < 3; i++) {
            const t = targets[i];
            const n = next[i];
            if (t !== undefined && n !== undefined && n < t) {
              next[i] = Math.min(n + 1, t);
              done = false;
            }
          }
          if (done) clearInterval(interval);
          return next;
        });
      }, 80);
      return () => clearInterval(interval);
    }, 800);
    return () => clearTimeout(timer);
  }, []);

  return (
    <section ref={ref} className="relative min-h-screen flex items-center justify-center overflow-hidden">
      {/* Particle canvas background */}
      <HeroParticles />

      {/* Warm glow layer */}
      <motion.div style={{ y, opacity }} className="absolute inset-0 pointer-events-none">
        <div className="absolute inset-0 bg-grid-pattern opacity-[0.15]" />
        <div className="absolute top-1/3 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[700px] h-[350px] bg-[var(--color-yj-red)]/[0.04] blur-[120px] rounded-full" />
        <div className="absolute bottom-1/4 left-0 w-[350px] h-[250px] bg-amber-500/[0.03] blur-[100px] rounded-full" />
      </motion.div>

      <div className="relative z-10 max-w-5xl mx-auto px-4 sm:px-6 text-center pt-24 sm:pt-16">
        {/* Status badge */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className="inline-flex items-center gap-2.5 px-4 py-2 rounded-full border border-[var(--color-yj-red)]/15 bg-[var(--color-yj-red)]/[0.04] backdrop-blur-sm text-[var(--color-yj-red)] text-sm font-medium mb-10"
        >
          <span className="relative flex h-2 w-2">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--color-yj-red)] opacity-60" />
            <span className="relative inline-flex rounded-full h-2 w-2 bg-[var(--color-yj-red)]" />
          </span>
          {t("hero.badge")}
        </motion.div>

        {/* Title with red gradient */}
        <motion.h1
          initial={{ opacity: 0, y: 28 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, delay: 0.2 }}
          className="text-4xl sm:text-5xl md:text-6xl lg:text-7xl font-bold tracking-tight leading-[1.1] mb-6"
        >
          <span className="text-[var(--color-text)]">{t("hero.title1")}</span>
          <br />
          <span className="bg-gradient-to-r from-[var(--color-yj-red)] via-[#ff4d47] to-[#ff7a6e] bg-clip-text text-transparent">
            {t("hero.title2")}
          </span>
        </motion.h1>

        {/* Description */}
        <motion.p
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.35 }}
          className="text-base sm:text-lg text-[var(--color-text-muted)] max-w-2xl mx-auto mb-12 leading-relaxed"
        >
          {t("hero.description")}
        </motion.p>

        {/* CTA buttons */}
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.48 }}
          className="flex flex-col sm:flex-row items-center justify-center gap-3 sm:gap-4"
        >
          <motion.a
            whileHover={{ scale: 1.04 }}
            whileTap={{ scale: 0.97 }}
            href="#products"
            className="group w-full sm:w-auto inline-flex items-center gap-2.5 px-8 py-3.5 bg-[var(--color-yj-red)] hover:bg-[var(--color-yj-red-deep)] text-white rounded-xl font-semibold text-base transition-all shadow-[0_0_30px_rgba(230,25,18,0.2)]"
          >
            {t("hero.ctaProducts")}
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="group-hover:translate-x-0.5 transition-transform">
              <path d="M5 12h14M12 5l7 7-7 7" />
            </svg>
          </motion.a>
          <motion.a
            whileHover={{ scale: 1.04 }}
            whileTap={{ scale: 0.97 }}
            href="#pricing"
            className="w-full sm:w-auto px-8 py-3.5 border border-[var(--color-border)] hover:border-[var(--color-border-hover)] text-[var(--color-text)] rounded-xl font-medium text-base transition-all"
          >
            {t("hero.ctaPricing")}
          </motion.a>
        </motion.div>

        {/* Animated stats */}
        <motion.div
          initial={{ opacity: 0, y: 28 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.55, delay: 0.6 }}
          className="mt-16 sm:mt-20 grid grid-cols-3 gap-6 max-w-sm mx-auto"
        >
          {stats.map((stat, i) => (
            <motion.div key={stat.key} className="text-center">
              <div className="text-2xl sm:text-3xl font-bold text-[var(--color-text)] tabular-nums">
                {i === 1 ? "¥" : ""}{counters[i]}{i === 0 || i === 2 ? " 代" : ""}
              </div>
              <div className="text-xs text-[var(--color-text-faint)] mt-1.5">
                {t(stat.key)}
              </div>
            </motion.div>
          ))}
        </motion.div>

        {/* Quick-jump pills */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.75 }}
          className="mt-14 flex flex-wrap items-center justify-center gap-2"
        >
          {quickJumps.map((p) => (
            <a
              key={p.label}
              href={p.href}
              className="px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-xs text-[var(--color-text-muted)] hover:text-[var(--color-text)] hover:border-[var(--color-border-hover)] hover:bg-[var(--color-surface)] transition-all"
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
        transition={{ delay: 1.4 }}
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
