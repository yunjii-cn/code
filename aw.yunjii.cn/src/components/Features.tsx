import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

const icons: Record<string, ReactNode> = {
  agent: <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><circle cx="12" cy="10" r="3"/><path d="M12 2a8 8 0 00-8 8c0 5.4 8 12 8 12s8-6.6 8-12a8 8 0 00-8-8z"/></svg>,
  ast: <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>,
  guard: <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>,
  timeflow: <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>,
};

export default function Features() {
  const { t } = useTranslation();
  const items = t("features.items", { returnObjects: true }) as Array<{ icon: string; title: string; desc: string }>;

  return (
    <section id="features" className="relative py-24 sm:py-32 px-4 sm:px-6">
      {/* Section bg accent */}
      <div className="absolute inset-0 bg-[var(--color-aw-primary)]/[0.015]" />

      <div className="relative max-w-7xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5 }}
          className="text-center mb-16 sm:mb-24"
        >
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-aw-primary)]/10 bg-[var(--color-aw-primary)]/3 text-[var(--color-aw-soft)] text-xs font-medium mb-6">
            核心能力
          </div>
          <h2 className="text-3xl sm:text-5xl font-bold text-[var(--color-text)] mb-4">
            {t("features.title")}
          </h2>
        </motion.div>

        <div className="grid sm:grid-cols-2 gap-6 lg:gap-8">
          {items.map((item, i) => (
            <motion.div
              key={item.title}
              initial={{ opacity: 0, y: 32 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: i * 0.12 }}
              className="group relative rounded-2xl bg-[var(--color-aw-primary)]/[0.02] border border-[var(--color-aw-primary)]/10 p-8 hover:border-[var(--color-aw-primary)]/25 transition-all duration-300"
            >
              {/* Blue accent line */}
              <div className="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-[var(--color-aw-primary)]/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />

              <div className="w-14 h-14 rounded-2xl bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-soft)] flex items-center justify-center mb-6 group-hover:bg-[var(--color-aw-primary)]/20 transition-all group-hover:scale-105">
                {icons[item.icon]}
              </div>
              <h3 className="text-xl font-bold text-[var(--color-text)] mb-3">{item.title}</h3>
              <p className="text-[var(--color-text-muted)] text-sm leading-relaxed">{item.desc}</p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
