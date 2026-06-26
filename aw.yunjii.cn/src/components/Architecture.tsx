import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function Architecture() {
  const { t } = useTranslation();
  const layers = t("architecture.layers", { returnObjects: true }) as Array<{ name: string; tech: string; desc: string }>;

  return (
    <section id="architecture" className="py-20 sm:py-28 px-4 sm:px-6">
      <div className="max-w-4xl mx-auto">
        <motion.div initial={{ opacity: 0, y: 20 }} whileInView={{ opacity: 1, y: 0 }} viewport={{ once: true, margin: "-80px" }} transition={{ duration: 0.5 }} className="text-center mb-14 sm:mb-20">
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">{t("architecture.title")}</h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg">{t("architecture.subtitle")}</p>
        </motion.div>

        <div className="space-y-4">
          {layers.map((layer, i) => (
            <motion.div
              key={layer.name}
              initial={{ opacity: 0, x: -20 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.4, delay: i * 0.1 }}
              className="flex items-start gap-5 sm:gap-6 p-5 sm:p-6 rounded-2xl border border-[var(--color-border)] bg-[var(--color-surface)] hover:border-[var(--color-border-hover)] transition-all"
            >
              <div className="w-2 h-2 mt-2 rounded-full bg-[var(--color-aw-primary)] shrink-0" />
              <div>
                <div className="flex items-center gap-3 mb-1.5">
                  <span className="text-[var(--color-text)] font-semibold">{layer.name}</span>
                  <span className="text-xs font-mono text-[var(--color-aw-soft)] bg-[var(--color-aw-primary)]/10 px-2 py-0.5 rounded">{layer.tech}</span>
                </div>
                <p className="text-[var(--color-text-muted)] text-sm leading-relaxed">{layer.desc}</p>
              </div>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
