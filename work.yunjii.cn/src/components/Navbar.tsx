import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { motion, AnimatePresence } from "framer-motion";
import { useTheme } from "../context/ThemeContext";

const navLinks = [
  { labelKey: "nav.products", href: "#products" },
  { labelKey: "nav.wizard", href: "#wizard" },
  { labelKey: "nav.pricing", href: "#pricing" },
  { labelKey: "nav.download", href: "#download" },
  { labelKey: "nav.faq", href: "#faq" },
  { labelKey: "nav.docs", href: "#" },
];

const themeIcons = {
  dark: (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
      <path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z" />
    </svg>
  ),
  light: (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
      <circle cx="12" cy="12" r="5" />
      <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
    </svg>
  ),
  system: (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
      <rect x="2" y="3" width="20" height="14" rx="2" />
      <path d="M8 21h8M12 17v4" />
    </svg>
  ),
};

export default function Navbar({ onLogoClick, onNavigate }: { onLogoClick?: () => void; onNavigate?: (href: string) => void }) {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();
  const [open, setOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);
  const [themeMenuOpen, setThemeMenuOpen] = useState(false);
  const themeBtnRef = useRef<HTMLButtonElement>(null);

  const toggleLang = () => {
    i18n.changeLanguage(i18n.language === "zh" ? "en" : "zh");
  };

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (themeBtnRef.current && !themeBtnRef.current.contains(e.target as Node)) {
        setThemeMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  useEffect(() => {
    document.body.style.overflow = open ? "hidden" : "";
    return () => { document.body.style.overflow = ""; };
  }, [open]);

  const cycleTheme = () => {
    const order: Array<"dark" | "light" | "system"> = ["dark", "light", "system"];
    const idx = order.indexOf(theme);
    setTheme(order[Math.max(0, (idx + 1) % 3)] ?? "dark");
  };

  return (
    <>
      <motion.nav
        initial={{ y: -80 }}
        animate={{ y: 0 }}
        transition={{ duration: 0.5, ease: "easeOut" }}
        className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
          scrolled
            ? "bg-[var(--color-nav-bg)] backdrop-blur-xl border-b border-[var(--color-border)]"
            : "bg-transparent"
        }`}
      >
        <div className="max-w-7xl mx-auto px-4 sm:px-6 h-16 flex items-center justify-between">
          {/* Logo */}
          <motion.a
            href="#"
            onClick={(e) => {
              if (onLogoClick) { e.preventDefault(); onLogoClick(); }
            }}
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            className="flex items-center gap-2.5 text-[var(--color-text)] font-semibold text-lg tracking-tight shrink-0 cursor-pointer"
          >
            <div className="w-8 h-8 rounded-lg bg-[var(--color-yj-red)] flex items-center justify-center text-white text-sm font-bold">
              云
            </div>
            <span className="hidden sm:inline">云集工作台</span>
          </motion.a>

          {/* Desktop nav */}
          <div className="hidden md:flex items-center gap-1">
            {navLinks.map((link) => (
              <a
                key={link.labelKey}
                href={onNavigate ? "#" : link.href}
                onClick={onNavigate ? ((e) => { e.preventDefault(); onNavigate(link.href); }) : undefined}
                className="px-3 py-2 text-sm text-[var(--color-text-muted)] hover:text-[var(--color-text)] rounded-lg hover:bg-[var(--color-border)] transition-colors"
              >
                {t(link.labelKey)}
              </a>
            ))}
          </div>

          {/* Right controls */}
          <div className="flex items-center gap-1 sm:gap-2">
            {/* Language toggle */}
            <motion.button
              whileHover={{ scale: 1.05 }}
              whileTap={{ scale: 0.95 }}
              onClick={toggleLang}
              className="p-2 text-xs font-medium text-[var(--color-text-muted)] hover:text-[var(--color-text)] rounded-lg hover:bg-[var(--color-border)] transition-colors"
              title={t("lang.switch")}
            >
              {i18n.language === "zh" ? "EN" : "中"}
            </motion.button>

            {/* Theme toggle */}
            <div className="relative">
              <motion.button
                ref={themeBtnRef}
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
                onClick={() => setThemeMenuOpen(!themeMenuOpen)}
                className="p-2 text-[var(--color-text-muted)] hover:text-[var(--color-text)] rounded-lg hover:bg-[var(--color-border)] transition-colors"
                aria-label="切换主题"
              >
                {themeIcons[theme]}
              </motion.button>
              <AnimatePresence>
                {themeMenuOpen && (
                  <motion.div
                    initial={{ opacity: 0, y: -4, scale: 0.95 }}
                    animate={{ opacity: 1, y: 0, scale: 1 }}
                    exit={{ opacity: 0, y: -4, scale: 0.95 }}
                    transition={{ duration: 0.15 }}
                    className="absolute right-0 top-full mt-2 w-32 bg-[var(--color-elevated)] border border-[var(--color-border)] rounded-xl shadow-xl overflow-hidden backdrop-blur-xl"
                  >
                    {(["dark", "light", "system"] as const).map((themeOption) => (
                      <button
                        key={themeOption}
                        onClick={() => { setTheme(themeOption); setThemeMenuOpen(false); }}
                        className={`w-full px-4 py-2.5 text-left text-sm flex items-center gap-2.5 transition-colors hover:bg-[var(--color-border)] ${
                          theme === themeOption ? "text-[var(--color-aw-primary)]" : "text-[var(--color-text-muted)]"
                        }`}
                      >
                        {themeIcons[themeOption]}
                        {t(`theme.${themeOption}`)}
                      </button>
                    ))}
                  </motion.div>
                )}
              </AnimatePresence>
            </div>

            {/* Desktop auth buttons */}
            <div className="hidden md:flex items-center gap-2 ml-2">
              <button className="text-sm text-[var(--color-text-muted)] hover:text-[var(--color-text)] transition-colors px-3 py-2 rounded-lg">
                {t("nav.login")}
              </button>
              <motion.button
                whileHover={{ scale: 1.03 }}
                whileTap={{ scale: 0.97 }}
                className="text-sm px-4 py-2 bg-[var(--color-yj-red)] hover:bg-[var(--color-yj-red-deep)] text-white rounded-lg font-medium transition-all"
              >
                {t("nav.startFree")}
              </motion.button>
            </div>

            {/* Mobile menu toggle */}
            <motion.button
              whileTap={{ scale: 0.9 }}
              className="md:hidden p-2 -mr-1 text-[var(--color-text)]"
              onClick={() => setOpen(!open)}
              aria-label="菜单"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                {open ? (
                  <path d="M6 6l12 12M18 6L6 18" />
                ) : (
                  <path d="M4 6h16M4 12h16M4 18h16" />
                )}
              </svg>
            </motion.button>
          </div>
        </div>
      </motion.nav>

      {/* Mobile full-screen menu */}
      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="fixed inset-0 z-40 md:hidden bg-[var(--color-bg)]/95 backdrop-blur-xl flex flex-col pt-20 px-6"
          >
            <nav className="flex flex-col gap-2">
              {navLinks.map((link, i) => (
                <motion.a
                  key={link.labelKey}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: 0.05 * i }}
                  href={onNavigate ? "#" : link.href}
                  onClick={() => { setOpen(false); if (onNavigate) onNavigate(link.href); }}
                  className="text-xl text-[var(--color-text-muted)] hover:text-[var(--color-text)] py-3 transition-colors"
                >
                  {t(link.labelKey)}
                </motion.a>
              ))}
            </nav>
            <div className="mt-auto pb-8 space-y-3">
              <button
                onClick={() => { setOpen(false); cycleTheme(); }}
                className="w-full py-3 text-sm text-[var(--color-text-muted)] border border-[var(--color-border)] rounded-xl flex items-center justify-center gap-2"
              >
                {themeIcons[theme]}
                {t(`theme.${theme}`)}
              </button>
              <motion.button
                whileTap={{ scale: 0.98 }}
                className="w-full py-3.5 bg-[var(--color-yj-red)] text-white rounded-xl font-semibold text-base"
              >
                {t("nav.startFree")}
              </motion.button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}
