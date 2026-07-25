import { useState } from "react";
import { useTranslation } from "react-i18next";
import { motion, AnimatePresence } from "framer-motion";

interface FaqItem {
  q: string;
  a: string;
}

export default function Faq() {
  const { t } = useTranslation();
  const items = t("faq.items", { returnObjects: true }) as FaqItem[];
  const [openIdx, setOpenIdx] = useState<number | null>(null);

  const toggle = (i: number) => setOpenIdx(openIdx === i ? null : i);

  return (
    <section id="faq" className="relative py-28 sm:py-36 px-4 sm:px-6">
      <div className="max-w-2xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.55 }}
          className="text-center mb-14"
        >
          <div className="inline-flex items-center gap-2.5 px-4 py-1.5 rounded-full border border-[var(--color-aw-primary)]/8 bg-[var(--color-aw-primary)]/3 backdrop-blur-md text-[var(--color-aw-soft)] text-xs font-semibold tracking-wider uppercase mb-7">
            {t("faq.badge")}
          </div>
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4 tracking-tight">
            {t("faq.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-sm">
            {t("faq.subtitle")}
          </p>
        </motion.div>

        <div className="space-y-3">
          {items.map((item, i) => {
            const isOpen = openIdx === i;
            return (
              <motion.div
                key={i}
                initial={{ opacity: 0, y: 12 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ delay: i * 0.05 }}
                className={`rounded-xl border transition-all duration-300 ${
                  isOpen
                    ? "border-[var(--color-aw-primary)]/20 bg-[var(--color-aw-primary)]/[0.02]"
                    : "border-[var(--color-border)] bg-[var(--color-surface)]/60 hover:border-[var(--color-aw-primary)]/10"
                }`}
              >
                <button
                  onClick={() => toggle(i)}
                  className="w-full text-left px-5 py-4 flex items-center justify-between gap-3"
                >
                  <span className="text-sm font-semibold text-[var(--color-text)] pr-2">
                    {item.q}
                  </span>
                  <motion.svg
                    animate={{ rotate: isOpen ? 180 : 0 }}
                    transition={{ duration: 0.25 }}
                    width="18"
                    height="18"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    className="text-[var(--color-text-faint)] shrink-0"
                  >
                    <path d="M6 9l6 6 6-6" />
                  </motion.svg>
                </button>
                <AnimatePresence>
                  {isOpen && (
                    <motion.div
                      initial={{ height: 0, opacity: 0 }}
                      animate={{ height: "auto", opacity: 1 }}
                      exit={{ height: 0, opacity: 0 }}
                      transition={{ duration: 0.25 }}
                      className="overflow-hidden"
                    >
                      <p className="px-5 pb-5 text-sm text-[var(--color-text-muted)] leading-relaxed">
                        {item.a}
                      </p>
                    </motion.div>
                  )}
                </AnimatePresence>
              </motion.div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
