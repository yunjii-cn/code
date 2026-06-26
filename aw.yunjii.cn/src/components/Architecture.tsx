import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

export default function Architecture() {
  const { t } = useTranslation();
  const layers = t("architecture.layers", { returnObjects: true }) as Array<{ name: string; tech: string; desc: string }>;

  return (
    <section id="architecture" className="relative py-24 sm:py-32 px-4 sm:px-6">
      <div className="max-w-4xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5 }}
          className="text-center mb-16 sm:mb-24"
        >
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-aw-primary)]/10 bg-[var(--color-aw-primary)]/3 text-[var(--color-aw-soft)] text-xs font-medium mb-6">
            技术架构
          </div>
          <h2 className="text-3xl sm:text-5xl font-bold text-[var(--color-text)] mb-4">{t("architecture.title")}</h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg">{t("architecture.subtitle")}</p>
        </motion.div>

        {/* Visual architecture diagram */}
        <div className="relative">
          {/* Connecting vertical line */}
          <div className="absolute left-8 top-0 bottom-0 w-px bg-gradient-to-b from-[var(--color-aw-primary)]/30 via-[var(--color-aw-primary)]/10 to-[var(--color-aw-primary)]/5" />

          <div className="space-y-0">
            {layers.map((layer, i) => (
              <motion.div
                key={layer.name}
                initial={{ opacity: 0, x: -24 }}
                whileInView={{ opacity: 1, x: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5, delay: i * 0.12 }}
                className="relative pl-16 pb-12 last:pb-0"
              >
                {/* Node dot on the line */}
                <div className="absolute left-[30px] top-2 w-2.5 h-2.5 rounded-full bg-[var(--color-aw-primary)] ring-4 ring-[var(--color-bg)]" />

                {/* Horizontal connector */}
                <div className="absolute left-[35px] top-[13px] w-7 h-px bg-[var(--color-aw-primary)]/20" />

                <div className="rounded-2xl bg-[var(--color-aw-primary)]/[0.02] border border-[var(--color-aw-primary)]/10 p-6 sm:p-8 hover:border-[var(--color-aw-primary)]/25 transition-all">
                  <div className="flex items-center gap-3 mb-3">
                    <span className="text-[var(--color-text)] font-bold text-lg">{layer.name}</span>
                    <span className="text-xs font-mono text-[var(--color-aw-soft)] bg-[var(--color-aw-primary)]/10 px-2.5 py-0.5 rounded-full">
                      {layer.tech}
                    </span>
                  </div>
                  <p className="text-[var(--color-text-muted)] text-sm leading-relaxed">{layer.desc}</p>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
