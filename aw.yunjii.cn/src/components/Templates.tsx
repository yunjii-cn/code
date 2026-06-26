import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

const templateIcons: Record<string, string> = {
  "🛒": "from-blue-500/20 to-blue-600/10",
  "📚": "from-violet-500/20 to-violet-600/10",
  "💰": "from-emerald-500/20 to-emerald-600/10",
  "⚙️": "from-sky-500/20 to-sky-600/10",
};

export default function Templates() {
  const { t } = useTranslation();
  const items = t("templates.items", { returnObjects: true }) as Array<{ name: string; desc: string; icon: string }>;

  return (
    <section id="templates" className="relative py-28 sm:py-36 px-4 sm:px-6 overflow-hidden">
      {/* Premium blue atmosphere */}
      <div className="absolute inset-0 bg-[var(--color-aw-primary)]/[0.015]" />
      <motion.div
        animate={{ opacity: [0.1, 0.2, 0.1] }}
        transition={{ duration: 7, repeat: Infinity }}
        className="absolute top-0 right-0 w-[700px] h-[500px] bg-[var(--color-aw-primary)]/4 blur-[200px] rounded-full translate-x-1/4 -translate-y-1/4 pointer-events-none"
      />

      <div className="relative max-w-7xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.55 }}
          className="text-center mb-18 sm:mb-24"
        >
          <div className="inline-flex items-center gap-2.5 px-4 py-1.5 rounded-full border border-[var(--color-aw-primary)]/8 bg-[var(--color-aw-primary)]/3 text-[var(--color-aw-soft)] text-xs font-semibold tracking-wider uppercase mb-7">
            行业模板
          </div>
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold text-[var(--color-text)] mb-5 tracking-tight">
            {t("templates.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg max-w-xl mx-auto">
            {t("templates.subtitle")}
          </p>
        </motion.div>

        <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-5">
          {items.map((item, i) => {
            const gradientClass = templateIcons[item.icon] ?? "from-[var(--color-aw-primary)]/20 to-[var(--color-aw-soft)]/10";
            return (
              <motion.div
                key={item.name}
                initial={{ opacity: 0, y: 32 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.45, delay: i * 0.1 }}
                whileHover={{ y: -6 }}
                className="group relative rounded-2xl bg-[var(--color-surface)] border border-[var(--color-aw-primary)]/6 p-7 text-center transition-all duration-300 hover:border-[var(--color-aw-primary)]/25 hover:shadow-[0_8px_32px_rgba(30,108,240,0.05)]"
              >
                {/* Icon */}
                <div className={`w-16 h-16 rounded-2xl bg-gradient-to-br ${gradientClass} flex items-center justify-center mx-auto mb-5 text-2xl group-hover:scale-110 transition-transform duration-300`}>
                  {item.icon}
                </div>
                <h3 className="text-[var(--color-text)] font-bold text-lg mb-3">{item.name}</h3>
                <p className="text-[var(--color-text-muted)] text-sm leading-relaxed">{item.desc}</p>
              </motion.div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
