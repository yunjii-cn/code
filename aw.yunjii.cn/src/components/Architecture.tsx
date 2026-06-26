import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

const layerIcons: Record<number, ReactNode> = {
  0: <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/></svg>,
  1: <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z"/></svg>,
  2: <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>,
  3: <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>,
  4: <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>,
};

export default function Architecture() {
  const { t } = useTranslation();
  const layers = t("architecture.layers", { returnObjects: true }) as Array<{ name: string; tech: string; desc: string }>;

  return (
    <section id="architecture" className="relative py-28 sm:py-36 px-4 sm:px-6 overflow-hidden">
      {/* Ambient glow */}
      <motion.div
        animate={{ opacity: [0.04, 0.1, 0.04] }}
        transition={{ duration: 10, repeat: Infinity, ease: "easeInOut" }}
        className="absolute top-1/3 left-1/2 -translate-x-1/2 w-[500px] h-[300px] bg-[var(--color-aw-primary)]/6 blur-[160px] rounded-full pointer-events-none"
      />
      <div className="max-w-4xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.55 }}
          className="text-center mb-18 sm:mb-24"
        >
          <div className="inline-flex items-center gap-2.5 px-4 py-1.5 rounded-full border border-[var(--color-aw-primary)]/8 bg-[var(--color-aw-primary)]/3 text-[var(--color-aw-soft)] text-xs font-semibold tracking-wider uppercase mb-7">
            技术架构
          </div>
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold text-[var(--color-text)] mb-5 tracking-tight">
            {t("architecture.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg">
            {t("architecture.subtitle")}
          </p>
        </motion.div>

        {/* Visual layered architecture */}
        <div className="relative">
          {/* Central connection line with glow */}
          <div className="absolute left-10 top-8 bottom-8 w-px bg-gradient-to-b from-[var(--color-aw-primary)]/40 via-[var(--color-aw-primary)]/15 to-[var(--color-aw-primary)]/5" />

          <div className="space-y-3">
            {layers.map((layer, i) => (
              <motion.div
                key={layer.name}
                initial={{ opacity: 0, x: -30 }}
                whileInView={{ opacity: 1, x: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5, delay: i * 0.12 }}
                className="relative pl-20 py-3"
              >
                {/* Node on the line */}
                <div className="absolute left-[32px] top-1/2 -translate-y-1/2">
                  <div className="relative">
                    <div className="w-5 h-5 rounded-full bg-[var(--color-aw-primary)]/20 ring-4 ring-[var(--color-bg)] border-2 border-[var(--color-aw-primary)]/40" />
                    {/* Pulse ring */}
                    <div className="absolute inset-0 w-5 h-5 rounded-full bg-[var(--color-aw-primary)]/30 animate-ping" style={{ animationDuration: "3s" }} />
                  </div>
                </div>

                {/* Horizontal connector line */}
                <div className="absolute left-[43px] top-1/2 w-8 h-px bg-gradient-to-r from-[var(--color-aw-primary)]/25 to-transparent" />

                {/* Card */}
                <div className="rounded-2xl bg-[var(--color-surface)] border border-[var(--color-aw-primary)]/6 p-6 sm:p-8 hover:border-[var(--color-aw-primary)]/20 hover:shadow-[0_4px_24px_rgba(30,108,240,0.04)] transition-all duration-300">
                  <div className="flex items-center gap-4 mb-3">
                    <div className="w-10 h-10 rounded-xl bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-soft)] flex items-center justify-center shrink-0">
                      {layerIcons[i] ?? layerIcons[0]}
                    </div>
                    <div className="flex items-center gap-3 min-w-0">
                      <span className="text-[var(--color-text)] font-bold text-lg">{layer.name}</span>
                      <span className="text-[11px] font-mono text-[var(--color-aw-soft)] bg-[var(--color-aw-primary)]/10 px-2.5 py-0.5 rounded-full shrink-0">
                        {layer.tech}
                      </span>
                    </div>
                  </div>
                  <p className="text-[var(--color-text-muted)] text-sm leading-relaxed pl-14">{layer.desc}</p>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
