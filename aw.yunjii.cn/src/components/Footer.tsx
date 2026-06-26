import { useTranslation } from "react-i18next";

export default function Footer() {
  const { t } = useTranslation();
  const links = t("footer.links", { returnObjects: true }) as string[];

  return (
    <footer className="border-t border-[var(--color-aw-primary)]/8 px-4 sm:px-6 py-16 sm:py-20">
      <div className="max-w-7xl mx-auto">
        <div className="flex flex-col md:flex-row items-start justify-between gap-12 mb-16">
          {/* Brand */}
          <div className="max-w-xs">
            <a href="#" className="flex items-center gap-3 text-[var(--color-text)] font-bold text-lg mb-4">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-[var(--color-aw-primary)] to-[var(--color-aw-soft)] flex items-center justify-center text-white text-xs font-extrabold shadow-[0_0_20px_rgba(30,108,240,0.3)]">
                AW
              </div>
              AgentWork
            </a>
            <p className="text-[var(--color-text-muted)] text-sm leading-relaxed">
              {t("footer.tagline")}
            </p>
          </div>

          {/* Links */}
          <div className="grid grid-cols-2 sm:grid-cols-3 gap-10 sm:gap-16">
            <div>
              <h4 className="text-[var(--color-text)] text-sm font-semibold mb-4">{t("footer.products")}</h4>
              <ul className="space-y-3">
                {links.slice(0, 3).map((link) => (
                  <li key={link}>
                    <a href="#" className="text-[var(--color-text-muted)] hover:text-[var(--color-aw-soft)] text-sm transition-colors">
                      {link}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
            <div>
              <h4 className="text-[var(--color-text)] text-sm font-semibold mb-4">资源</h4>
              <ul className="space-y-3">
                {links.slice(3).map((link) => (
                  <li key={link}>
                    <a href="#" className="text-[var(--color-text-muted)] hover:text-[var(--color-aw-soft)] text-sm transition-colors">
                      {link}
                    </a>
                  </li>
                ))}
                <li><a href="#" className="text-[var(--color-text-muted)] hover:text-[var(--color-aw-soft)] text-sm transition-colors">路线图</a></li>
              </ul>
            </div>
            <div>
              <h4 className="text-[var(--color-text)] text-sm font-semibold mb-4">关于</h4>
              <ul className="space-y-3">
                <li><a href="#" className="text-[var(--color-text-muted)] hover:text-[var(--color-aw-soft)] text-sm transition-colors">联系我们</a></li>
                <li><a href="#" className="text-[var(--color-text-muted)] hover:text-[var(--color-aw-soft)] text-sm transition-colors">博客</a></li>
                <li><a href="#" className="text-[var(--color-text-muted)] hover:text-[var(--color-aw-soft)] text-sm transition-colors">加入我们</a></li>
              </ul>
            </div>
          </div>
        </div>

        {/* Bottom bar */}
        <div className="border-t border-[var(--color-aw-primary)]/6 pt-8 flex flex-col sm:flex-row items-center justify-between gap-5">
          <p className="text-[var(--color-text-faint)] text-xs">{t("footer.copyright")}</p>
          <div className="flex items-center gap-8 text-xs text-[var(--color-text-faint)]">
            <a href="#" className="hover:text-[var(--color-aw-soft)] transition-colors">{t("footer.privacy")}</a>
            <a href="#" className="hover:text-[var(--color-aw-soft)] transition-colors">{t("footer.terms")}</a>
            <span>备案号</span>
          </div>
        </div>
      </div>
    </footer>
  );
}
