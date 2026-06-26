import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

const icons: Record<string, ReactNode> = {
  agent: <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><circle cx="12" cy="10" r="3"/><path d="M12 2a8 8 0 00-8 8c0 5.4 8 12 8 12s8-6.6 8-12a8 8 0 00-8-8z"/></svg>,
  ast: <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>,
  guard: <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>,
  timeflow: <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>,
};

export default function Features() {
  const { t } = useTranslation();
  const items = t("features.items", { returnObjects: true }) as Array<{ icon: string; title: string; desc: string }>;

  return (
    <section id="features" className="py-20 sm:py-28 px-4 sm:px-6">
      <div className="max-w-7xl mx-auto">
        <motion.div initial={{ opacity: 0, y: 20 }} whileInView={{ opacity: 1, y: 0 }} viewport={{ once: true, margin: "-80px" }} transition={{ duration: 0.5 }} className="text-center mb-14 sm:mb-20">
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">{t("features.title")}</h2>
        </motion.div>

        <div className="grid sm:grid-cols-2 gap-6 sm:gap-8">
          {items.map((item, i) => (
            <motion.div
              key={item.title}
              initial={{ opacity: 0, y: 24 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.4, delay: i * 0.1 }}
              className="group relative rounded-2xl border border-[var(--color-border)] bg-[var(--color-surface)] p-6 sm:p-8 hover:border-[var(--color-border-hover)] transition-all"
            >
              <div className="w-12 h-12 rounded-xl bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-soft)] flex items-center justify-center mb-5 group-hover:bg-[var(--color-aw-primary)]/20 transition-colors">
                {icons[item.icon]}
              </div>
              <h3 className="text-lg font-semibold text-[var(--color-text)] mb-3">{item.title}</h3>
              <p className="text-[var(--color-text-muted)] text-sm leading-relaxed">{item.desc}</p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
