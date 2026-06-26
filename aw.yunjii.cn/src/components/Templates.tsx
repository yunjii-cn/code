import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function Templates() {
  const { t } = useTranslation();
  const items = t("templates.items", { returnObjects: true }) as Array<{ name: string; desc: string; icon: string }>;

  return (
    <section id="templates" className="relative py-24 sm:py-32 px-4 sm:px-6 overflow-hidden">
      {/* Blue atmosphere */}
      <div className="absolute inset-0 bg-[var(--color-aw-primary)]/[0.02]" />
      <motion.div
        animate={{ opacity: [0.15, 0.25, 0.15] }}
        transition={{ duration: 6, repeat: Infinity }}
        className="absolute top-1/3 right-0 w-[600px] h-[400px] bg-[var(--color-aw-primary)]/5 blur-[180px] rounded-full -translate-y-1/2 translate-x-1/4"
      />

      <div className="relative max-w-7xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5 }}
          className="text-center mb-16 sm:mb-24"
        >
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-aw-primary)]/10 bg-[var(--color-aw-primary)]/3 text-[var(--color-aw-soft)] text-xs font-medium mb-6">
            行业模板
          </div>
          <h2 className="text-3xl sm:text-5xl font-bold text-[var(--color-text)] mb-4">{t("templates.title")}</h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg">{t("templates.subtitle")}</p>
        </motion.div>

        <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-5">
          {items.map((item, i) => (
            <motion.div
              key={item.name}
              initial={{ opacity: 0, y: 24 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.4, delay: i * 0.1 }}
              whileHover={{ y: -6 }}
              className="relative rounded-2xl bg-[var(--color-surface)] border border-[var(--color-aw-primary)]/10 p-6 text-center hover:border-[var(--color-aw-primary)]/30 transition-all group"
            >
              <div className="w-14 h-14 rounded-2xl bg-gradient-to-br from-[var(--color-aw-primary)]/15 to-[var(--color-aw-soft)]/10 text-[var(--color-aw-soft)] flex items-center justify-center mx-auto mb-5 text-sm font-bold group-hover:scale-110 transition-transform">
                {item.icon}
              </div>
              <h3 className="text-[var(--color-text)] font-semibold text-lg mb-3">{item.name}</h3>
              <p className="text-[var(--color-text-muted)] text-sm leading-relaxed">{item.desc}</p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
