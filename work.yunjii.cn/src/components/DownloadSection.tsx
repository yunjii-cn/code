import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";

const downloads = [
  {
    name: "云集工作台·桌面版",
    desc: "PyQt6 桌面应用，双击即用",
    size: "~80 MB",
    platform: "Windows 10+",
    href: "#",
  },
  {
    name: "云集工作台·网络版",
    desc: "浏览器即用，PWA 可安装",
    size: "在线服务",
    platform: "全平台",
    href: "#",
  },
  {
    name: "云集工作台·团队版",
    desc: "团队协作工作空间",
    size: "在线服务",
    platform: "全平台",
    href: "#",
  },
  {
    name: "AgentWork",
    desc: "Tauri 2 桌面 + Rust 后端",
    size: "~30 MB",
    platform: "Windows / macOS / Linux",
    href: "#",
    badge: "即将发布",
  },
];

export default function DownloadSection() {
  const { t } = useTranslation();

  return (
    <section id="download" className="py-20 sm:py-28 px-4 sm:px-6 bg-[var(--color-surface)]/40">
      <div className="max-w-7xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5 }}
          className="text-center mb-14 sm:mb-20"
        >
          <h2 className="text-3xl sm:text-4xl font-bold text-[var(--color-text)] mb-4">
            {t("download.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-base sm:text-lg max-w-2xl mx-auto">
            {t("download.subtitle")}
          </p>
        </motion.div>

        <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-5 sm:gap-6">
          {downloads.map((item, i) => (
            <motion.a
              key={item.name}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: "-60px" }}
              transition={{ duration: 0.4, delay: i * 0.08 }}
              whileHover={{ y: -4 }}
              href={item.href}
              className="group relative rounded-2xl border border-[var(--color-border)] bg-[var(--color-surface)] p-6 hover:border-[var(--color-border-hover)] transition-all"
            >
              {item.badge && (
                <span className="absolute top-4 right-4 text-[10px] font-medium px-2 py-0.5 rounded-full bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-soft)] border border-[var(--color-aw-primary)]/20">
                  {item.badge}
                </span>
              )}
              <h3 className="text-[var(--color-text)] font-semibold mb-2 group-hover:text-[var(--color-aw-soft)] transition-colors">
                {item.name}
              </h3>
              <p className="text-[var(--color-text-muted)] text-sm mb-4">{item.desc}</p>
              <div className="flex items-center gap-3 text-xs text-[var(--color-text-faint)]">
                <span>{item.size}</span>
                <span className="w-1 h-1 rounded-full bg-[var(--color-border)]" />
                <span>{item.platform}</span>
              </div>
              <motion.div
                initial={{ opacity: 0, x: -4 }}
                whileHover={{ opacity: 1, x: 0 }}
                className="mt-4 flex items-center gap-1 text-[var(--color-aw-soft)] text-sm font-medium opacity-0 group-hover:opacity-100 transition-opacity"
              >
                下载
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                  <path d="M5 12h14M12 5l7 7-7 7" />
                </svg>
              </motion.div>
            </motion.a>
          ))}
        </div>
      </div>
    </section>
  );
}
