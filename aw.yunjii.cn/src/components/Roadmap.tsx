import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

interface Milestone {
  name: string;
  date: string;
  tag: string;
  status: "done" | "active" | "planned" | "future";
  items: string[];
}

const statusStyles: Record<string, { dot: string; tagBg: string; tagText: string; cardBorder: string }> = {
  done: {
    dot: "bg-[var(--color-aw-soft)] shadow-[0_0_8px_var(--color-aw-soft)]",
    tagBg: "bg-[var(--color-aw-soft)]/15",
    tagText: "text-[var(--color-aw-soft)]",
    cardBorder: "border-[var(--color-aw-soft)]/15",
  },
  active: {
    dot: "bg-[var(--color-aw-primary)] shadow-[0_0_12px_var(--color-aw-primary)] animate-pulse",
    tagBg: "bg-[var(--color-aw-primary)]/15",
    tagText: "text-[var(--color-aw-primary)]",
    cardBorder: "border-[var(--color-aw-primary)]/25 hover:border-[var(--color-aw-primary)]/45",
  },
  planned: {
    dot: "bg-[var(--color-text-faint)]",
    tagBg: "bg-[var(--color-text-faint)]/10",
    tagText: "text-[var(--color-text-faint)]",
    cardBorder: "border-[var(--color-border)] hover:border-[var(--color-aw-primary)]/20",
  },
  future: {
    dot: "bg-[var(--color-text-faint)]/40",
    tagBg: "bg-[var(--color-text-faint)]/5",
    tagText: "text-[var(--color-text-faint)]/50",
    cardBorder: "border-[var(--color-border)]/50 hover:border-[var(--color-aw-primary)]/10",
  },
};

export default function Roadmap() {
  const { t } = useTranslation();
  const milestones = t("roadmap.milestones", { returnObjects: true }) as Milestone[];

  return (
    <section id="roadmap" className="relative py-28 sm:py-36 px-4 sm:px-6 overflow-hidden">
      {/* Ambient glow */}
      <motion.div
        animate={{ opacity: [0.03, 0.08, 0.03] }}
        transition={{ duration: 12, repeat: Infinity, ease: "easeInOut" }}
        className="absolute top-1/3 left-1/2 -translate-x-1/2 w-[600px] h-[400px] bg-[var(--color-aw-primary)]/6 blur-[180px] rounded-full pointer-events-none"
      />

      <div className="max-w-4xl mx-auto">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.55 }}
          className="text-center mb-18 sm:mb-24"
        >
          <div className="inline-flex items-center gap-2.5 px-4 py-1.5 rounded-full border border-[var(--color-aw-primary)]/8 bg-[var(--color-aw-primary)]/3 backdrop-blur-md text-[var(--color-aw-soft)] text-xs font-semibold tracking-wider uppercase mb-7">
            {t("roadmap.badge")}
          </div>
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold text-[var(--color-text)] mb-5 tracking-tight">
            {t("roadmap.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg max-w-xl mx-auto">
            {t("roadmap.subtitle")}
          </p>
        </motion.div>

        {/* Timeline */}
        <div className="relative">
          {/* Central timeline line */}
          <div className="absolute left-10 sm:left-1/2 top-0 bottom-0 w-px -translate-x-px bg-gradient-to-b from-[var(--color-aw-primary)]/30 via-[var(--color-aw-primary)]/10 to-[var(--color-aw-primary)]/3" />

          <div className="space-y-6">
            {milestones.map((m, i) => {
              const isLeft = i % 2 === 0;
              const s = statusStyles[m.status] ?? statusStyles.planned;

              return (
                <motion.div
                  key={m.name}
                  initial={{ opacity: 0, y: 30 }}
                  whileInView={{ opacity: 1, y: 0 }}
                  viewport={{ once: true, margin: "-40px" }}
                  transition={{ duration: 0.45, delay: i * 0.08 }}
                  className={`relative flex items-start gap-4 ${
                    isLeft ? "sm:flex-row" : "sm:flex-row-reverse"
                  }`}
                >
                  {/* Node on timeline */}
                  <div className="absolute left-10 sm:left-1/2 top-8 -translate-x-1/2 z-10">
                    <div className={`w-4 h-4 rounded-full ${s.dot} ring-4 ring-[var(--color-bg)] border-2 border-current/20`}
                      style={{ borderColor: m.status === "active" ? "var(--color-aw-primary)" : undefined }}
                    />
                  </div>

                  {/* Spacer for center alignment on desktop */}
                  <div className="hidden sm:block w-1/2" />

                  {/* Card */}
                  <div className="sm:w-1/2 pl-16 sm:pl-0 sm:pr-10">
                    <div
                      className={`rounded-2xl border ${s.cardBorder} bg-gradient-to-b from-[var(--color-surface)] to-[var(--color-surface)]/60 backdrop-blur-sm p-6 transition-all duration-300`}
                    >
                      {/* Header row */}
                      <div className="flex items-center gap-3 mb-4 flex-wrap">
                        <h3 className="text-[var(--color-text)] font-bold text-base sm:text-lg">
                          {m.name}
                        </h3>
                        <span className={`text-[11px] font-semibold px-2.5 py-0.5 rounded-full ${s.tagBg} ${s.tagText}`}>
                          {m.tag}
                        </span>
                      </div>

                      {/* Date */}
                      <p className="text-xs font-mono text-[var(--color-text-faint)] mb-4">
                        {m.date}
                      </p>

                      {/* Feature items */}
                      <ul className="space-y-2">
                        {m.items.map((item, j) => (
                          <li
                            key={j}
                            className="flex items-start gap-2.5 text-sm text-[var(--color-text-muted)]"
                          >
                            <svg
                              className="w-4 h-4 mt-0.5 shrink-0 text-[var(--color-aw-soft)]/60"
                              fill="none"
                              viewBox="0 0 24 24"
                              stroke="currentColor"
                              strokeWidth="2"
                            >
                              <path d="M5 13l4 4L19 7" />
                            </svg>
                            {item}
                          </li>
                        ))}
                      </ul>
                    </div>
                  </div>

                  {/* Right side spacer for alternating layout */}
                  <div className="hidden sm:block w-1/2" />
                </motion.div>
              );
            })}
          </div>
        </div>

        {/* Bottom note */}
        <motion.p
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ delay: 0.8 }}
          className="text-center text-xs text-[var(--color-text-faint)] mt-16"
        >
          路线图可能根据用户反馈和技术进展调整。
        </motion.p>
      </div>
    </section>
  );
}
