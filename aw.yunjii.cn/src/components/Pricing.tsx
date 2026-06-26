import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

interface Tier {
  name: string;
  price: string;
  period: string;
  features: string[];
  cta: string;
  highlighted: boolean;
}

export default function Pricing() {
  const { t } = useTranslation();
  const tiers = t("pricing.tiers", { returnObjects: true }) as Tier[];

  return (
    <section id="pricing" className="relative py-28 sm:py-36 px-4 sm:px-6 overflow-hidden">
      <div className="absolute inset-0 bg-[var(--color-aw-primary)]/[0.012]" />

      {/* Ambient glow */}
      <motion.div
        animate={{ opacity: [0.05, 0.12, 0.05] }}
        transition={{ duration: 9, repeat: Infinity, ease: "easeInOut" }}
        className="absolute top-1/4 right-0 w-[500px] h-[500px] bg-[var(--color-aw-primary)]/6 blur-[180px] rounded-full pointer-events-none translate-x-1/4"
      />

      <div className="relative max-w-7xl mx-auto">
        {/* Section header */}
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.55 }}
          className="text-center mb-16 sm:mb-20"
        >
          <div className="inline-flex items-center gap-2.5 px-4 py-1.5 rounded-full border border-[var(--color-aw-primary)]/8 bg-[var(--color-aw-primary)]/3 text-[var(--color-aw-soft)] text-xs font-semibold tracking-wider uppercase mb-7">
            定价
          </div>
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold text-[var(--color-text)] mb-5 tracking-tight">
            {t("pricing.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg max-w-xl mx-auto">
            {t("pricing.subtitle")}
          </p>
        </motion.div>

        {/* Pricing grid — 4 tiers */}
        <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-5">
          {tiers.map((tier, i) => (
            <PricingCard key={tier.name} tier={tier} index={i} />
          ))}
        </div>

        {/* Footer */}
        <motion.p
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ delay: 0.6 }}
          className="text-center text-xs text-[var(--color-text-faint)] mt-12"
        >
          {t("pricing.footer")}
        </motion.p>
      </div>
    </section>
  );
}

function PricingCard({ tier, index }: { tier: Tier; index: number }) {
  const { t } = useTranslation();

  const cardBase = tier.highlighted
    ? "border-[var(--color-aw-primary)]/30 bg-[var(--color-aw-primary)]/[0.03] aw-ring-glow hover:shadow-[0_0_60px_rgba(30,108,240,0.18)] scale-[1.02] lg:scale-105 z-10"
    : "border-[var(--color-aw-primary)]/8 bg-gradient-to-b from-[var(--color-aw-primary)]/[0.015] to-[var(--color-surface)] hover:border-[var(--color-aw-primary)]/25";

  const btnBase = tier.highlighted
    ? "bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white shadow-[0_0_30px_rgba(30,108,240,0.25)] hover:shadow-[0_0_45px_rgba(30,108,240,0.4)]"
    : "border border-[var(--color-aw-primary)]/15 text-[var(--color-aw-soft)] hover:border-[var(--color-aw-primary)]/35 hover:bg-[var(--color-aw-primary)]/5";

  return (
    <motion.div
      initial={{ opacity: 0, y: 36 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      transition={{ duration: 0.5, delay: index * 0.08 }}
      whileHover={tier.highlighted ? { y: -6 } : { y: -4 }}
      className={`relative rounded-2xl ${cardBase} p-6 sm:p-7 transition-all duration-400 flex flex-col`}
    >
      {/* Top gradient line */}
      <div className={`absolute top-0 left-6 right-6 h-px bg-gradient-to-r from-transparent via-[var(--color-aw-primary)]/${tier.highlighted ? "50" : "15"} to-transparent`} />

      {/* Recommended badge */}
      {tier.highlighted && (
        <div className="absolute -top-3 left-1/2 -translate-x-1/2 px-4 py-1 bg-[var(--color-aw-primary)] text-white text-[11px] font-bold rounded-full tracking-wide shadow-[0_0_16px_rgba(30,108,240,0.4)]">
          {t("pricing.recommended")}
        </div>
      )}

      {/* Name */}
      <h3 className="text-sm font-bold text-[var(--color-text)] mb-1 tracking-wide uppercase">
        {tier.name}
      </h3>

      {/* Price */}
      <div className="mb-6">
        <span className="text-4xl sm:text-5xl font-extrabold text-[var(--color-text)] tracking-tight">
          {tier.price}
        </span>
        <span className="text-sm text-[var(--color-text-muted)] font-medium ml-1">
          {tier.period}
        </span>
      </div>

      {/* Features */}
      <ul className="space-y-3 mb-8 flex-1">
        {tier.features.map((f, j) => (
          <li key={j} className="flex items-start gap-3 text-sm text-[var(--color-text-muted)]">
            <div className="w-5 h-5 rounded-full bg-[var(--color-aw-primary)]/10 flex items-center justify-center shrink-0 mt-0.5">
              <svg className="w-3 h-3 text-[var(--color-aw-soft)]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="3">
                <path d="M5 13l4 4L19 7" />
              </svg>
            </div>
            <span className="leading-relaxed">{f}</span>
          </li>
        ))}
      </ul>

      {/* CTA */}
      <motion.a
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.97 }}
        href="#"
        className={`block text-center py-3 rounded-xl font-semibold text-sm transition-all ${btnBase}`}
      >
        {tier.cta}
      </motion.a>
    </motion.div>
  );
}
