import { useState, useEffect, useCallback } from "react";
import { Outlet, NavLink, useNavigate } from "react-router-dom";
import {
  Clock,
  Settings as SettingsIcon,
  GitBranch,
  Search,
  Users,
  ClipboardList,
  MessageSquare,
  GraduationCap,
  Store,
  Server,
  Sparkles,
  Wrench,
  Sun,
  Moon,
  PanelLeftClose,
  PanelLeft,
  Command as CommandIcon,
} from "lucide-react";
import { clsx } from "clsx";
import { useTheme } from "@/lib/theme-store";
import { useI18n } from "@/i18n";
import CommandPalette from "./CommandPalette";

export default function Layout() {
  const { isLight, toggleTheme } = useTheme();
  const { t } = useI18n();
  const navigate = useNavigate();
  const [expanded, setExpanded] = useState(true);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);

  const navItems = [
    { to: "/chat", key: "nav.chat", icon: MessageSquare },
    { to: "/timeline", key: "nav.timeline", icon: Clock },
    { to: "/branches", key: "nav.branches", icon: GitBranch },
    { to: "/search", key: "nav.search", icon: Search },
    { to: "/team", key: "nav.team", icon: Users },
    { to: "/tasks", key: "nav.tasks", icon: ClipboardList },
    { to: "/training", key: "nav.training", icon: GraduationCap },
    { to: "/marketplace", key: "nav.marketplace", icon: Store },
    { to: "/modelservice", key: "nav.modelservice", icon: Server },
    { to: "/evolution", key: "nav.evolution", icon: Sparkles },
    { to: "/toolbuilder", key: "nav.toolbuilder", icon: Wrench },
    { to: "/settings", key: "nav.settings", icon: SettingsIcon },
  ];

  const toggleSidebar = useCallback(() => {
    setExpanded((v) => !v);
  }, []);

  // 全局快捷键
  useEffect(() => {
    function handler(e: KeyboardEvent) {
      // Ctrl+K / Cmd+K：命令面板
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen((v) => !v);
        return;
      }
      // Ctrl+B：切换侧栏
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "b") {
        e.preventDefault();
        setExpanded((v) => !v);
        return;
      }
      // Ctrl+J：切换主题
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "j") {
        e.preventDefault();
        toggleTheme();
        return;
      }
      // Ctrl+1~9：切换视图
      if ((e.ctrlKey || e.metaKey) && /^[1-9]$/.test(e.key)) {
        e.preventDefault();
        const idx = parseInt(e.key, 10) - 1;
        const item = navItems[idx];
        if (item) navigate(item.to);
        return;
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigate, toggleTheme]);

  return (
    <div className="flex h-screen bg-zinc-950 text-zinc-50">
      {/* 移动端遮罩 */}
      {mobileSidebarOpen && (
        <div
          className="fixed inset-0 z-30 bg-black/60 md:hidden"
          onClick={() => setMobileSidebarOpen(false)}
        />
      )}

      <aside
        className={clsx(
          "flex flex-col py-4 border-r border-zinc-800 bg-zinc-900 transition-all duration-200 z-40",
          // 桌面端：宽度随 expanded 变化
          "md:relative md:translate-x-0",
          expanded ? "w-56" : "w-16",
          // 移动端：抽屉式
          "fixed inset-y-0 left-0",
          mobileSidebarOpen ? "translate-x-0 w-56" : "-translate-x-full md:translate-x-0"
        )}
      >
        <nav className="flex-1 flex flex-col gap-1 px-2 overflow-y-auto">
          {navItems.map(({ to, key, icon: Icon }, idx) => (
            <NavLink
              key={to}
              to={to}
              onClick={() => setMobileSidebarOpen(false)}
              className={({ isActive }) =>
                clsx(
                  "flex items-center gap-3 rounded-lg transition-colors group relative",
                  expanded ? "px-3 py-2" : "w-10 h-10 justify-center mx-auto",
                  isActive
                    ? "bg-brand-600 text-white"
                    : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
                )
              }
              title={!expanded ? t(key) : undefined}
            >
              <Icon className="w-5 h-5 flex-shrink-0" />
              {expanded && <span className="text-sm font-medium">{t(key)}</span>}
              {/* 快捷键角标（仅折叠态显示前 9 个） */}
              {!expanded && idx < 9 && (
                <span className="absolute top-0.5 right-0.5 text-[9px] text-zinc-600 group-hover:text-zinc-400">
                  {idx + 1}
                </span>
              )}
            </NavLink>
          ))}
        </nav>

        <div className="px-2 mt-2 space-y-1 border-t border-zinc-800 pt-3">
          {/* 命令面板入口 */}
          <button
            onClick={() => setPaletteOpen(true)}
            className={clsx(
              "flex items-center gap-3 rounded-lg transition-colors text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200",
              expanded ? "px-3 py-2 w-full" : "w-10 h-10 justify-center mx-auto"
            )}
            title={t("cmdpalette.title")}
          >
            <CommandIcon className="w-5 h-5 flex-shrink-0" />
            {expanded && (
              <>
                <span className="text-sm flex-1 text-left">{t("cmdpalette.title")}</span>
                <kbd className="text-[10px] px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-500 border border-zinc-700">
                  Ctrl+K
                </kbd>
              </>
            )}
          </button>

          <button
            onClick={toggleTheme}
            className={clsx(
              "flex items-center gap-3 rounded-lg transition-colors text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200",
              expanded ? "px-3 py-2 w-full" : "w-10 h-10 justify-center mx-auto"
            )}
            title={isLight ? t("theme.toggleDark") : t("theme.toggleLight")}
          >
            {isLight ? <Moon className="w-5 h-5 flex-shrink-0" /> : <Sun className="w-5 h-5 flex-shrink-0" />}
            {expanded && <span className="text-sm">{isLight ? t("theme.light") : t("theme.dark")}</span>}
          </button>
          <button
            onClick={toggleSidebar}
            className={clsx(
              "flex items-center gap-3 rounded-lg transition-colors text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300",
              expanded ? "px-3 py-2 w-full" : "w-10 h-10 justify-center mx-auto"
            )}
            title={expanded ? t("cmdpalette.action.toggleSidebar") : t("cmdpalette.action.toggleSidebar")}
          >
            {expanded ? (
              <PanelLeftClose className="w-5 h-5 flex-shrink-0" />
            ) : (
              <PanelLeft className="w-5 h-5 flex-shrink-0" />
            )}
            {expanded && <span className="text-sm">{t("cmdpalette.action.toggleSidebar")}</span>}
          </button>
        </div>
      </aside>

      <main className="flex-1 overflow-auto">
        {/* 移动端顶部栏 */}
        <div className="md:hidden flex items-center gap-3 px-4 py-2 border-b border-zinc-800 bg-zinc-900 sticky top-0 z-20">
          <button
            onClick={() => setMobileSidebarOpen(true)}
            className="p-1.5 rounded-lg hover:bg-zinc-800 text-zinc-300"
          >
            <PanelLeft className="w-5 h-5" />
          </button>
          <span className="text-sm text-zinc-400 flex-1">{t("common.appName")}</span>
          <button
            onClick={() => setPaletteOpen(true)}
            className="p-1.5 rounded-lg hover:bg-zinc-800 text-zinc-300"
          >
            <CommandIcon className="w-5 h-5" />
          </button>
        </div>
        <Outlet />
      </main>

      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        onToggleSidebar={toggleSidebar}
      />
    </div>
  );
}