// 命令面板（Command Palette）
// Ctrl+K 唤起，支持页面导航 + 快捷操作，键盘上下选择 + Enter 确认

import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  Search,
  CornerDownLeft,
  ArrowUp,
  ArrowDown,
  Sun,
  PanelLeft,
  Settings as SettingsIcon,
  Sparkles,
  type LucideIcon,
} from "lucide-react";
import { clsx } from "clsx";
import { useI18n } from "@/i18n";
import { useTheme } from "@/lib/theme-store";

interface CommandItem {
  id: string;
  label: string;
  group: "navigation" | "actions";
  icon: LucideIcon;
  shortcut?: string;
  action: () => void;
}

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  onToggleSidebar: () => void;
}

export default function CommandPalette({
  open,
  onClose,
  onToggleSidebar,
}: CommandPaletteProps) {
  const { t } = useI18n();
  const { toggleTheme } = useTheme();
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // 构建命令列表
  const commands = useMemo<CommandItem[]>(() => {
    const navItems: { to: string; key: string; icon: LucideIcon }[] = [
      { to: "/chat", key: "nav.chat", icon: Sparkles },
      { to: "/timeline", key: "nav.timeline", icon: Sparkles },
      { to: "/branches", key: "nav.branches", icon: Sparkles },
      { to: "/search", key: "nav.search", icon: Search },
      { to: "/team", key: "nav.team", icon: Sparkles },
      { to: "/tasks", key: "nav.tasks", icon: Sparkles },
      { to: "/training", key: "nav.training", icon: Sparkles },
      { to: "/marketplace", key: "nav.marketplace", icon: Sparkles },
      { to: "/modelservice", key: "nav.modelservice", icon: Sparkles },
      { to: "/evolution", key: "nav.evolution", icon: Sparkles },
      { to: "/toolbuilder", key: "nav.toolbuilder", icon: Sparkles },
      { to: "/settings", key: "nav.settings", icon: SettingsIcon },
    ];

    const navCommands: CommandItem[] = navItems.map((item, i) => ({
      id: `nav-${item.to}`,
      label: t(item.key),
      group: "navigation",
      icon: item.icon,
      shortcut: i < 9 ? `Ctrl+${i + 1}` : undefined,
      action: () => {
        navigate(item.to);
        onClose();
      },
    }));

    const actionCommands: CommandItem[] = [
      {
        id: "action-theme",
        label: t("cmdpalette.action.toggleTheme"),
        group: "actions",
        icon: Sun,
        shortcut: "Ctrl+J",
        action: () => {
          toggleTheme();
          onClose();
        },
      },
      {
        id: "action-sidebar",
        label: t("cmdpalette.action.toggleSidebar"),
        group: "actions",
        icon: PanelLeft,
        shortcut: "Ctrl+B",
        action: () => {
          onToggleSidebar();
          onClose();
        },
      },
      {
        id: "action-settings",
        label: t("cmdpalette.action.openSettings"),
        group: "actions",
        icon: SettingsIcon,
        action: () => {
          navigate("/settings");
          onClose();
        },
      },
      {
        id: "action-onboarding",
        label: t("cmdpalette.action.openOnboarding"),
        group: "actions",
        icon: Sparkles,
        action: () => {
          navigate("/onboarding");
          onClose();
        },
      },
    ];

    return [...navCommands, ...actionCommands];
  }, [t, navigate, onClose, toggleTheme, onToggleSidebar]);

  // 过滤
  const filtered = useMemo(() => {
    if (!query.trim()) return commands;
    const q = query.toLowerCase();
    return commands.filter((c) => c.label.toLowerCase().includes(q));
  }, [commands, query]);

  // 重置选中项
  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
      // 延迟聚焦，等待动画
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  // 过滤结果变化时重置索引
  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  // 键盘导航
  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIndex((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const item = filtered[activeIndex];
      if (item) item.action();
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  // 滚动到选中项
  useEffect(() => {
    const el = listRef.current?.querySelector(`[data-idx="${activeIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  if (!open) return null;

  // 分组
  const navItems = filtered.filter((c) => c.group === "navigation");
  const actionItems = filtered.filter((c) => c.group === "actions");

  let runningIndex = -1;

  function renderItem(item: CommandItem) {
    runningIndex += 1;
    const idx = runningIndex;
    const Icon = item.icon;
    return (
      <button
        key={item.id}
        data-idx={idx}
        onMouseEnter={() => setActiveIndex(idx)}
        onClick={() => item.action()}
        className={clsx(
          "w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-left transition-colors",
          idx === activeIndex
            ? "bg-brand-600/20 text-brand-200"
            : "text-zinc-300 hover:bg-zinc-800/50"
        )}
      >
        <Icon className="w-4 h-4 flex-shrink-0 text-zinc-400" />
        <span className="flex-1 text-sm">{item.label}</span>
        {item.shortcut && (
          <kbd className="text-xs px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400 border border-zinc-700">
            {item.shortcut}
          </kbd>
        )}
        {idx === activeIndex && (
          <CornerDownLeft className="w-3.5 h-3.5 text-brand-400 flex-shrink-0" />
        )}
      </button>
    );
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] px-4"
      onClick={onClose}
    >
      {/* 遮罩 */}
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" />

      {/* 面板 */}
      <div
        className="relative w-full max-w-xl bg-zinc-900 rounded-xl border border-zinc-700 shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* 搜索框 */}
        <div className="flex items-center gap-3 px-4 py-3 border-b border-zinc-800">
          <Search className="w-4 h-4 text-zinc-500" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t("cmdpalette.placeholder")}
            className="flex-1 bg-transparent text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none"
          />
          <kbd className="text-xs px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400 border border-zinc-700">
            Esc
          </kbd>
        </div>

        {/* 命令列表 */}
        <div ref={listRef} className="max-h-[50vh] overflow-auto p-2">
          {filtered.length === 0 ? (
            <div className="py-8 text-center text-sm text-zinc-500">
              {t("cmdpalette.empty")}
            </div>
          ) : (
            <>
              {navItems.length > 0 && (
                <div className="mb-2">
                  <p className="px-3 py-1 text-xs font-medium text-zinc-500 uppercase tracking-wide">
                    {t("cmdpalette.group.navigation")}
                  </p>
                  <div className="space-y-0.5">{navItems.map(renderItem)}</div>
                </div>
              )}
              {actionItems.length > 0 && (
                <div>
                  <p className="px-3 py-1 text-xs font-medium text-zinc-500 uppercase tracking-wide">
                    {t("cmdpalette.group.actions")}
                  </p>
                  <div className="space-y-0.5">{actionItems.map(renderItem)}</div>
                </div>
              )}
            </>
          )}
        </div>

        {/* 底部提示 */}
        <div className="flex items-center justify-between px-4 py-2 border-t border-zinc-800 text-xs text-zinc-500">
          <span className="flex items-center gap-3">
            <span className="flex items-center gap-1">
              <ArrowUp className="w-3 h-3" />
              <ArrowDown className="w-3 h-3" />
              {t("cmdpalette.hint")}
            </span>
          </span>
          <span>{t("cmdpalette.shortcut")}</span>
        </div>
      </div>
    </div>
  );
}
