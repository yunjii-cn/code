import { useTranslation } from "react-i18next";

const footerColumns = [
  { titleKey: "footer.products", linksKey: "footer.productLinks" },
  { titleKey: "footer.resources", linksKey: "footer.resourceLinks" },
  { titleKey: "footer.about", linksKey: "footer.aboutLinks" },
];

export default function Footer() {
  const { t } = useTranslation();

  const productLinks = t("footer.productLinks", { returnObjects: true }) as string[];
  const resourceLinks = t("footer.resourceLinks", { returnObjects: true }) as string[];
  const aboutLinks = t("footer.aboutLinks", { returnObjects: true }) as string[];

  const linkMap: Record<string, string[]> = {
    "footer.productLinks": productLinks,
    "footer.resourceLinks": resourceLinks,
    "footer.aboutLinks": aboutLinks,
  };

  return (
    <footer className="border-t border-[var(--color-border)] px-4 sm:px-6 py-12 sm:py-16">
      <div className="max-w-7xl mx-auto">
        <div className="grid sm:grid-cols-2 lg:grid-cols-5 gap-8 mb-12">
          {/* Brand */}
          <div className="lg:col-span-2">
            <a href="https://yunjii.cn" target="_blank" rel="noopener" className="flex items-center gap-2.5 text-[var(--color-text)] font-semibold text-lg mb-4 hover:text-[var(--color-yj-red)] transition-colors">
              <div className="w-8 h-8 rounded-lg bg-[var(--color-yj-red)] flex items-center justify-center text-white text-sm font-bold">
                云
              </div>
              云集智能
            </a>
            <p className="text-[var(--color-text-muted)] text-sm leading-relaxed max-w-xs">
              {t("footer.tagline")}
            </p>
          </div>

          {footerColumns.map((col) => (
            <div key={col.titleKey}>
              <h4 className="text-[var(--color-text)] text-sm font-semibold mb-4">{t(col.titleKey)}</h4>
              <ul className="space-y-2.5">
                {linkMap[col.linksKey]?.map((link) => (
                  <li key={link}>
                    <a href="#" className="text-[var(--color-text-muted)] hover:text-[var(--color-text)] text-sm transition-colors">
                      {link}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        <div className="border-t border-[var(--color-border)] pt-8 flex flex-col sm:flex-row items-center justify-between gap-4">
          <p className="text-[var(--color-text-faint)] text-xs">
            {t("footer.copyright")}
          </p>
          <div className="flex items-center gap-6 text-xs text-[var(--color-text-faint)]">
            <a href="#" className="hover:text-[var(--color-text-muted)] transition-colors">{t("footer.privacy")}</a>
            <a href="#" className="hover:text-[var(--color-text-muted)] transition-colors">{t("footer.terms")}</a>
            <a href="https://beian.miit.gov.cn/" target="_blank" rel="noopener" className="hover:text-[var(--color-text-muted)] transition-colors">{t("footer.icp")}</a>
          </div>
        </div>
      </div>
    </footer>
  );
}
