import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function Templates() {
  const { t } = useTranslation();
  const items = t("templates.items", { returnObjects: true }) as Array<{ name: string; desc: string; icon: string }>;

  return (
    <section id="templates" className="py-20 sm:py-28 px-4 sm:px-6 bg-[var(--color-surface)]/40">
      <div className="max-w-7xl mx-auto">
        <motion.div initial={{ opacity: 0, y: 20 }} whileInView={{ opacity: 1, y: 0 }} viewport={{ once: true, margin: "-80px" }} transition={{ duration: 0.5 }} className="text-center mb-14 sm:mb-20">
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">{t("templates.title")}</h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg">{t("templates.subtitle")}</p>
        </motion.div>

        <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-5 sm:gap-6">
          {items.map((item, i) => (
            <motion.div
              key={item.name}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.4, delay: i * 0.08 }}
              whileHover={{ y: -4 }}
              className="relative rounded-2xl border border-[var(--color-border)] bg-[var(--color-surface)] p-6 text-center hover:border-[var(--color-border-hover)] transition-all"
            >
              <div className="w-12 h-12 rounded-xl bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-soft)] flex items-center justify-center mx-auto mb-4 text-sm font-bold">
                {item.icon}
              </div>
              <h3 className="text-[var(--color-text)] font-semibold mb-2">{item.name}</h3>
              <p className="text-[var(--color-text-muted)] text-sm leading-relaxed">{item.desc}</p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
