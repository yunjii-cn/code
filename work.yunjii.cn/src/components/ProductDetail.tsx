import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

function DesktopDetail() {
  const { t } = useTranslation();
  const d = t("products.pc.detail", { returnObjects: true }) as Record<string, unknown>;
  const features = (d.features as Array<{ title: string; desc: string }>) ?? [];
  return (
    <section id="desktop-detail" className="py-20 sm:py-28 px-4 sm:px-6 bg-[var(--color-surface)]/30">
      <div className="max-w-4xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.5 }}
        >
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-border)] text-xs text-[var(--color-text-faint)] mb-6">
            第 1 代 · Free 档
          </div>
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">
            {d.heading as string}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base leading-relaxed mb-10 max-w-2xl">
            {d.intro as string}
          </p>

          <div className="grid sm:grid-cols-2 gap-5 mb-10">
            {features.map((f, i) => (
              <div key={i} className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-5">
                <h4 className="text-sm font-bold text-[var(--color-text)] mb-2">{f.title}</h4>
                <p className="text-xs text-[var(--color-text-muted)] leading-relaxed">{f.desc}</p>
              </div>
            ))}
          </div>

          <div className="flex flex-wrap items-center gap-6 text-xs text-[var(--color-text-faint)] border-t border-[var(--color-border)] pt-6">
            <span className="font-mono">{d.tech as string}</span>
            <span>{(d.users as string)}</span>
          </div>
        </motion.div>
      </div>
    </section>
  );
}

function WebDetail() {
  const { t } = useTranslation();
  const d = t("products.web.detail", { returnObjects: true }) as Record<string, unknown>;
  const features = (d.features as Array<{ title: string; desc: string }>) ?? [];
  return (
    <section id="web-detail" className="py-20 sm:py-28 px-4 sm:px-6 bg-[var(--color-surface)]/30">
      <div className="max-w-4xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.5 }}
        >
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-border)] text-xs text-[var(--color-text-faint)] mb-6">
            第 2 代 · Pro 档
          </div>
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">
            {d.heading as string}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base leading-relaxed mb-10 max-w-2xl">
            {d.intro as string}
          </p>

          <div className="grid sm:grid-cols-2 gap-5 mb-10">
            {features.map((f, i) => (
              <div key={i} className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-5">
                <h4 className="text-sm font-bold text-[var(--color-text)] mb-2">{f.title}</h4>
                <p className="text-xs text-[var(--color-text-muted)] leading-relaxed">{f.desc}</p>
              </div>
            ))}
          </div>

          <div className="flex flex-wrap items-center gap-6 text-xs text-[var(--color-text-faint)] border-t border-[var(--color-border)] pt-6">
            <span className="font-mono">{d.tech as string}</span>
            <span>{(d.users as string)}</span>
          </div>
        </motion.div>
      </div>
    </section>
  );
}

function TeamDetail() {
  const { t } = useTranslation();
  const d = t("products.team.detail", { returnObjects: true }) as Record<string, unknown>;
  const features = (d.features as Array<{ title: string; desc: string }>) ?? [];
  return (
    <section id="team-detail" className="py-20 sm:py-28 px-4 sm:px-6 bg-[var(--color-surface)]/30">
      <div className="max-w-4xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.5 }}
        >
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-border)] text-xs text-[var(--color-text-faint)] mb-6">
            第 3 代 · Business 档
          </div>
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">
            {d.heading as string}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base leading-relaxed mb-10 max-w-2xl">
            {d.intro as string}
          </p>

          <div className="grid sm:grid-cols-2 gap-5 mb-10">
            {features.map((f, i) => (
              <div key={i} className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-5">
                <h4 className="text-sm font-bold text-[var(--color-text)] mb-2">{f.title}</h4>
                <p className="text-xs text-[var(--color-text-muted)] leading-relaxed">{f.desc}</p>
              </div>
            ))}
          </div>

          <div className="flex flex-wrap items-center gap-6 text-xs text-[var(--color-text-faint)] border-t border-[var(--color-border)] pt-6">
            <span className="font-mono">{d.tech as string}</span>
            <span>{(d.users as string)}</span>
          </div>
        </motion.div>
      </div>
    </section>
  );
}

export { DesktopDetail, WebDetail, TeamDetail };
