import { useState } from "react";
import { useTranslation } from "react-i18next";
import { motion, AnimatePresence } from "framer-motion";

type ProductId = "pc" | "web" | "team" | "agentwork";

interface Option {
  label: string;
  desc: string;
}

export default function SelectorWizard() {
  const { t } = useTranslation();
  const [step, setStep] = useState<0 | 1 | 2 | 3>(0);
  const [answers, setAnswers] = useState<number[]>([]);
  const [result, setResult] = useState<ProductId | null>(null);

  const q1Opts = t("wizard.q1.options", { returnObjects: true }) as Option[];
  const q2Opts = t("wizard.q2.options", { returnObjects: true }) as Option[];
  const q3Opts = t("wizard.q3.options", { returnObjects: true }) as Option[];

  function calcResult(a: number[]): ProductId {
    const [q1, q2, q3] = a;

    // q2: privacy — local-first pushes strongly toward pc
    if (q2 === 0 && q1 !== 3) return "pc";

    // q3: single device → pc or web
    if (q3 === 0 && q2 !== 0) return "web";

    // q1: enterprise → agentwork
    if (q1 === 3) return "agentwork";

    // q1: medium team → team
    if (q1 === 2) return "team";

    // q1: small team + multi-device → web
    if (q1 === 1 && q3 !== 2) return "web";

    // q1: individual + cloud ok → web
    if (q1 === 0 && q2 !== 0 && q3 !== 0) return "web";

    // fallback
    if (q1 === 0) return "pc";
    if (q1 === 1) return "team";
    return "agentwork";
  }

  function handleAnswer(choice: number) {
    const next = [...answers, choice];
    if (step < 3) {
      setAnswers(next);
      setStep((step + 1) as 0 | 1 | 2 | 3);
      if (step === 2) {
        setResult(calcResult(next));
      }
    }
  }

  function reset() {
    setStep(0);
    setAnswers([]);
    setResult(null);
  }

  const stepProgress = ((step === 0 ? 0 : step) / 3) * 100;

  return (
    <section id="wizard" className="relative py-20 sm:py-28 px-4 sm:px-6 bg-[var(--color-surface)]/30">
      <div className="max-w-2xl mx-auto">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.5 }}
          className="text-center mb-12"
        >
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-yj-red)]/15 bg-[var(--color-yj-red)]/[0.04] backdrop-blur-sm text-[var(--color-yj-red)] text-xs font-medium mb-6">
            {t("wizard.badge")}
          </div>
          <h2 className="text-2xl sm:text-3xl font-bold text-[var(--color-text)] mb-3">
            {t("wizard.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-sm sm:text-base">
            {t("wizard.subtitle")}
          </p>
        </motion.div>

        {/* Progress bar */}
        {step > 0 && (
          <div className="mb-8">
            <div className="h-1.5 rounded-full bg-[var(--color-border)] overflow-hidden">
              <motion.div
                initial={{ width: 0 }}
                animate={{ width: `${stepProgress}%` }}
                transition={{ duration: 0.4 }}
                className="h-full rounded-full bg-gradient-to-r from-[var(--color-yj-red)] to-[#ff7a6e]"
              />
            </div>
            <p className="text-xs text-[var(--color-text-faint)] mt-2 text-center">
              {step} / 3
            </p>
          </div>
        )}

        {/* Content */}
        <AnimatePresence mode="wait">
          {step === 0 ? (
            <motion.div
              key="start"
              initial={{ opacity: 0, scale: 0.96 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.96 }}
              className="text-center"
            >
              <motion.button
                whileHover={{ scale: 1.03 }}
                whileTap={{ scale: 0.97 }}
                onClick={() => setStep(1)}
                className="inline-flex items-center gap-2.5 px-8 py-4 bg-[var(--color-yj-red)] hover:bg-[var(--color-yj-red-deep)] text-white rounded-xl font-semibold text-lg transition-colors shadow-[0_0_24px_rgba(230,25,18,0.15)]"
              >
                {t("wizard.start")}
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                  <path d="M5 12h14M12 5l7 7-7 7" />
                </svg>
              </motion.button>
            </motion.div>
          ) : step <= 3 && !result ? (
            <motion.div
              key={`q${step}`}
              initial={{ opacity: 0, x: 40 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -40 }}
              transition={{ duration: 0.35 }}
            >
              <h3 className="text-lg sm:text-xl font-bold text-[var(--color-text)] mb-6 text-center">
                {step === 1 ? t("wizard.q1.title") : step === 2 ? t("wizard.q2.title") : t("wizard.q3.title")}
              </h3>

              <div className="grid gap-3">
                {(step === 1 ? q1Opts : step === 2 ? q2Opts : q3Opts).map((opt, i) => (
                  <motion.button
                    key={opt.label}
                    initial={{ opacity: 0, y: 12 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: i * 0.06 }}
                    whileHover={{ scale: 1.01, borderColor: "var(--color-yj-red)" }}
                    whileTap={{ scale: 0.98 }}
                    onClick={() => handleAnswer(i)}
                    className="w-full text-left p-4 sm:p-5 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] hover:border-[var(--color-yj-red)]/30 hover:bg-[var(--color-yj-red)]/[0.02] transition-all group"
                  >
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface-alt)] flex items-center justify-center text-sm font-mono text-[var(--color-text-muted)] group-hover:border-[var(--color-yj-red)]/30 group-hover:text-[var(--color-yj-red)] shrink-0 transition-colors">
                        {String.fromCharCode(65 + i)}
                      </div>
                      <div>
                        <span className="text-sm font-semibold text-[var(--color-text)] group-hover:text-[var(--color-yj-red)] transition-colors">
                          {opt.label}
                        </span>
                        <p className="text-xs text-[var(--color-text-muted)] mt-0.5">
                          {opt.desc}
                        </p>
                      </div>
                    </div>
                  </motion.button>
                ))}
              </div>
            </motion.div>
          ) : result ? (
            <motion.div
              key="result"
              initial={{ opacity: 0, y: 30 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.45 }}
              className="text-center"
            >
              <h3 className="text-lg font-semibold text-[var(--color-text-muted)] mb-6">
                {t("wizard.result.title")}
              </h3>

              {/* Result card */}
              <div className={`rounded-2xl border p-6 sm:p-8 mb-8 text-left ${
                result === "agentwork"
                  ? "border-[var(--color-aw-primary)]/25 bg-[var(--color-aw-primary)]/[0.03] aw-ring-glow"
                  : "border-[var(--color-yj-red)]/15 bg-[var(--color-yj-red)]/[0.02]"
              }`}>
                <div className="flex items-center gap-4 mb-4">
                  <div className={`w-14 h-14 rounded-xl flex items-center justify-center text-2xl font-bold shrink-0 ${
                    result === "agentwork"
                      ? "bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-primary)]"
                      : "bg-[var(--color-yj-red)]/10 text-[var(--color-yj-red)]"
                  }`}>
                    {result === "pc" ? "PC" : result === "web" ? "Web" : result === "team" ? "TM" : "AW"}
                  </div>
                  <div>
                    <h4 className="text-xl font-bold text-[var(--color-text)]">
                      {t(`wizard.result.${result}.name`)}
                    </h4>
                    <span className={`inline-block text-xs font-medium px-2 py-0.5 rounded-full mt-1 ${
                      result === "agentwork"
                        ? "bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-primary)]"
                        : "bg-[var(--color-yj-red)]/10 text-[var(--color-yj-red)]"
                    }`}>
                      {t(`wizard.result.${result}.tag`)}
                    </span>
                  </div>
                </div>

                <p className="text-sm text-[var(--color-text-muted)] leading-relaxed">
                  {t(`wizard.result.${result}.reason`)}
                </p>
              </div>

              {/* Actions */}
              <div className="flex flex-col sm:flex-row items-center justify-center gap-3">
                <motion.a
                  whileHover={{ scale: 1.03 }}
                  whileTap={{ scale: 0.97 }}
                  href={result === "agentwork" ? "https://aw.yunjii.cn" : result === "pc" ? "#download" : "#pricing"}
                  target={result === "agentwork" ? "_blank" : undefined}
                  rel={result === "agentwork" ? "noopener" : undefined}
                  className={`w-full sm:w-auto inline-flex items-center justify-center gap-2 px-8 py-3.5 rounded-xl font-semibold text-base transition-all ${
                    result === "agentwork"
                      ? "bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white shadow-[0_0_24px_rgba(30,108,240,0.2)]"
                      : "bg-[var(--color-yj-red)] hover:bg-[var(--color-yj-red-deep)] text-white shadow-[0_0_24px_rgba(230,25,18,0.15)]"
                  }`}
                >
                  {result === "pc" ? "免费下载" : result === "agentwork" ? "访问 AgentWork 官网" : "查看方案"}
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                    <path d="M5 12h14M12 5l7 7-7 7" />
                  </svg>
                </motion.a>

                <button
                  onClick={reset}
                  className="w-full sm:w-auto px-6 py-3.5 border border-[var(--color-border)] text-[var(--color-text-muted)] rounded-xl text-sm font-medium hover:text-[var(--color-text)] hover:border-[var(--color-border-hover)] transition-all"
                >
                  {t("wizard.restart")}
                </button>
              </div>
            </motion.div>
          ) : null}
        </AnimatePresence>
      </div>
    </section>
  );
}
