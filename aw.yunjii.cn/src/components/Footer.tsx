import { useTranslation } from "react-i18next";

export default function Footer() {
  const { t } = useTranslation();
  const links = t("footer.links", { returnObjects: true }) as string[];

  return (
    <footer className="border-t border-[var(--color-aw-primary)]/10 px-4 sm:px-6 py-16">
      <div className="max-w-7xl mx-auto">
        <div className="flex flex-col sm:flex-row items-start justify-between gap-12 mb-12">
          <div className="max-w-xs">
            <a href="#" className="flex items-center gap-2.5 text-[var(--color-text)] font-semibold text-lg mb-4">
              <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-[var(--color-aw-primary)] to-[var(--color-aw-soft)] flex items-center justify-center text-white text-xs font-bold shadow-[0_0_16px_rgba(30,108,240,0.3)]">
                AW
              </div>
              AgentWork
            </a>
            <p className="text-[var(--color-text-muted)] text-sm leading-relaxed">{t("footer.tagline")}</p>
          </div>

          <div>
            <h4 className="text-[var(--color-text)] text-sm font-semibold mb-4">{t("footer.products")}</h4>
            <ul className="space-y-3">
              {links.map((link) => (
                <li key={link}>
                  <a href="#" className="text-[var(--color-text-muted)] hover:text-[var(--color-aw-soft)] text-sm transition-colors">
                    {link}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        </div>

        <div className="border-t border-[var(--color-aw-primary)]/8 pt-8 flex flex-col sm:flex-row items-center justify-between gap-4">
          <p className="text-[var(--color-text-faint)] text-xs">{t("footer.copyright")}</p>
          <div className="flex items-center gap-6 text-xs text-[var(--color-text-faint)]">
            <a href="#" className="hover:text-[var(--color-aw-soft)] transition-colors">{t("footer.privacy")}</a>
            <a href="#" className="hover:text-[var(--color-aw-soft)] transition-colors">{t("footer.terms")}</a>
          </div>
        </div>
      </div>
    </footer>
  );
}
