import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { tiers } from "../content/products";

const tierFeatures: Record<string, string[]> = {
  Free: [
    "本地 AI（Ollama）",
    "单设备使用",
    "PyQt6 桌面端",
    "社区支持",
  ],
  Pro: [
    "云端 AI 模型",
    "跨端同步（3 设备）",
    "PWA / 移动壳",
    "¥10 等值算力/月",
    "邮件支持",
  ],
  Business: [
    "全部 Pro 功能",
    "5 角色团队协作",
    "知识引擎 + 感知",
    "Yjs 实时协作",
    "SSO + 审计",
    "优先支持",
  ],
  Enterprise: [
    "AI 员工 + AI 团队",
    "行业模板（电商/教育/金融）",
    "AST-Native 代码认知",
    "TimeFlow 版本控制",
    "全链路自动发布",
    "私有化部署可选",
    "专属客户成功经理",
  ],
};

export default function PriceComparison() {
  const { t } = useTranslation();

  return (
    <section id="pricing" className="py-20 sm:py-28 px-4 sm:px-6 bg-[var(--color-surface)]/40">
      <div className="max-w-7xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5 }}
          className="text-center mb-14 sm:mb-20"
        >
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">
            {t("pricing.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg max-w-2xl mx-auto leading-relaxed">
            {t("pricing.subtitle")}
          </p>
        </motion.div>

        {/* Mobile: horizontal scroll */}
        <div className="lg:hidden -mx-4 px-4 overflow-x-auto pb-4 snap-x snap-mandatory scrollbar-none">
          <div className="flex gap-4 min-w-max">
            {tiers.map((tier, i) => (
              <TierCard key={tier.name} tier={tier} index={i} compact />
            ))}
          </div>
        </div>

        {/* Desktop: grid */}
        <div className="hidden lg:grid lg:grid-cols-4 gap-5">
          {tiers.map((tier, i) => (
            <TierCard key={tier.name} tier={tier} index={i} />
          ))}
        </div>
      </div>
    </section>
  );
}

function TierCard({ tier, index, compact }: { tier: typeof tiers[number]; index: number; compact?: boolean }) {
  const { t } = useTranslation();
  const tierKey = tier.name.toLowerCase();

  return (
    <motion.div
      initial={{ opacity: 0, y: 30 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-60px" }}
      transition={{ duration: 0.4, delay: index * 0.08 }}
      whileHover={!compact ? { y: -6, scale: 1.02 } : undefined}
      className={`relative rounded-2xl border p-6 sm:p-8 transition-all duration-300 ${
        compact ? "w-[280px] snap-center shrink-0" : ""
      } ${
        tier.highlighted
        ? "border-[var(--color-yj-red)]/20 bg-[var(--color-yj-red)]/[0.02] ring-1 ring-[var(--color-yj-red)]/10"
        : "border-[var(--color-border)] bg-[var(--color-surface)] hover:border-[var(--color-border-hover)]"
      }`}
    >
      {tier.highlighted && (
        <div className="absolute -top-3 left-1/2 -translate-x-1/2 px-3 py-1 bg-[var(--color-yj-red)] text-white text-xs font-semibold rounded-full">
          最受欢迎
        </div>
      )}

      <div className="mb-6">
        <h3 className="text-lg font-bold text-[var(--color-text)]">{t(`pricing.tiers.${tierKey}.name`)}</h3>
        <div className="mt-3 flex items-baseline gap-0.5">
          <span className="text-3xl sm:text-4xl font-bold text-[var(--color-text)]">{tier.price}</span>
          {t(`pricing.tiers.${tierKey}.period`) !== "" && (
            <span className="text-[var(--color-text-muted)] text-sm">{t(`pricing.tiers.${tierKey}.period`)}</span>
          )}
        </div>
      </div>

      <ul className="space-y-3 mb-8">
        {(tierFeatures[tier.name] || []).map((feature, i) => (
          <li key={i} className="flex items-start gap-2.5 text-sm text-[var(--color-text-muted)]">
            <svg className="w-4 h-4 mt-0.5 shrink-0 text-[var(--color-aw-soft)]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
              <path d="M5 13l4 4L19 7" />
            </svg>
            {feature}
          </li>
        ))}
      </ul>

      <motion.a
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
        href="#"
        className={`block text-center py-3 rounded-xl font-semibold text-sm transition-all ${
          tier.highlighted
            ? "bg-[var(--color-yj-red)] hover:bg-[var(--color-yj-red-deep)] text-white"
            : "border border-[var(--color-border)] text-[var(--color-text)] hover:border-[var(--color-border-hover)]"
        }`}
      >
        {t(`pricing.tiers.${tierKey}.cta`)}
      </motion.a>
    </motion.div>
  );
}
