import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

const icons: Record<string, ReactNode> = {
  agent: <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.3"><circle cx="12" cy="10" r="3"/><path d="M12 2a8 8 0 00-8 8c0 5.4 8 12 8 12s8-6.6 8-12a8 8 0 00-8-8z"/></svg>,
  ast: <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.3"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>,
  guard: <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.3"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>,
  timeflow: <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.3"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>,
};

export default function Features() {
  const { t } = useTranslation();
  const items = t("features.items", { returnObjects: true }) as Array<{ icon: string; title: string; desc: string }>;

  return (
    <section id="features" className="relative py-28 sm:py-36 px-4 sm:px-6 overflow-hidden">
      <div className="absolute inset-0 bg-[var(--color-aw-primary)]/[0.012]" />

      {/* Ambient glow */}
      <motion.div
        animate={{ opacity: [0.06, 0.14, 0.06] }}
        transition={{ duration: 8, repeat: Infinity, ease: "easeInOut" }}
        className="absolute bottom-0 right-0 w-[600px] h-[400px] bg-[var(--color-aw-primary)]/8 blur-[180px] rounded-full pointer-events-none translate-x-1/4 translate-y-1/4"
      />

      <div className="relative max-w-7xl mx-auto">
        {/* Section header */}
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.55 }}
          className="text-center mb-18 sm:mb-24"
        >
          <div className="inline-flex items-center gap-2.5 px-4 py-1.5 rounded-full border border-[var(--color-aw-primary)]/8 bg-[var(--color-aw-primary)]/3 text-[var(--color-aw-soft)] text-xs font-semibold tracking-wider uppercase mb-7">
            核心能力
          </div>
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold text-[var(--color-text)] mb-5 tracking-tight">
            {t("features.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg max-w-xl mx-auto leading-relaxed">
            不是 AI 编程工具，而是 AI 智能体工作台。每一项能力都为真正的自主协作而设计。
          </p>
        </motion.div>

        {/* Feature grid — 2×2 large cards */}
        <div className="grid sm:grid-cols-2 gap-5 lg:gap-6">
          {items.map((item, i) => (
            <motion.div
              key={item.title}
              initial={{ opacity: 0, y: 40 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: i * 0.1 }}
              whileHover={{ y: -4 }}
              className="group relative rounded-2xl bg-gradient-to-b from-[var(--color-aw-primary)]/[0.02] to-[var(--color-surface)] border border-[var(--color-aw-primary)]/8 p-8 sm:p-10 transition-all duration-300 hover:border-[var(--color-aw-primary)]/30 hover:shadow-[0_8px_40px_rgba(30,108,240,0.08)]"
            >
              {/* Top gradient accent line */}
              <div className="absolute top-0 left-6 right-6 h-px bg-gradient-to-r from-transparent via-[var(--color-aw-primary)]/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500" />

              {/* Icon */}
              <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-[var(--color-aw-primary)]/12 to-[var(--color-aw-soft)]/8 text-[var(--color-aw-soft)] flex items-center justify-center mb-7 group-hover:from-[var(--color-aw-primary)]/20 group-hover:to-[var(--color-aw-soft)]/12 transition-all duration-300">
                {icons[item.icon]}
              </div>

              <h3 className="text-xl font-bold text-[var(--color-text)] mb-3.5">{item.title}</h3>
              <p className="text-[var(--color-text-muted)] text-sm leading-relaxed">{item.desc}</p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
