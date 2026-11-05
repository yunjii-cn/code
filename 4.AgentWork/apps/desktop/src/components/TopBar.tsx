// 顶部横栏（TopBar）
// 左侧：侧栏展开/收起按钮（无 Logo，版本号改到系统顶栏）
// 中间：命令面板搜索框（常驻，点击或 Ctrl+K 唤起）
// 右侧：右侧栏展开/收起按钮
// 设计一致，逻辑对称

import {
  Search,
  PanelLeftClose,
  PanelLeft,
  PanelRightClose,
  PanelRight,
} from "lucide-react";
import { clsx } from "clsx";
import { useI18n } from "@/i18n";

interface TopBarProps {
  /** 点击搜索框唤起命令面板 */
  onOpenPalette: () => void;
  /** 移动端汉堡按钮 */
  onToggleMobileSidebar?: () => void;
  /** 侧栏是否展开 */
  sidebarExpanded: boolean;
  /** 切换侧栏展开/收起 */
  onToggleSidebar: () => void;
  /** 右侧栏是否展开 */
  rightSidebarExpanded: boolean;
  /** 切换右侧栏展开/收起 */
  onToggleRightSidebar: () => void;
}

export default function TopBar({
  onOpenPalette,
  onToggleMobileSidebar,
  sidebarExpanded,
  onToggleSidebar,
  rightSidebarExpanded,
  onToggleRightSidebar,
}: TopBarProps) {
  const { t } = useI18n();

  return (
    <header className="flex items-center gap-2 h-12 px-2 sm:px-3 border-b border-zinc-800 bg-zinc-900/80 backdrop-blur-sm flex-shrink-0">
      {/* 左侧：侧栏展开/收起按钮（无 Logo） */}
      <div className="flex items-center gap-1 flex-shrink-0">
        {/* 移动端汉堡按钮 */}
        {onToggleMobileSidebar && (
          <button
            onClick={onToggleMobileSidebar}
            className="md:hidden p-1.5 rounded-lg hover:bg-zinc-800 text-zinc-300"
            aria-label="menu"
          >
            <svg
              className="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 6h16M4 12h16M4 18h16"
              />
            </svg>
          </button>
        )}

        {/* 桌面端侧栏展开/收起按钮 */}
        <button
          onClick={onToggleSidebar}
          className="hidden md:flex p-1.5 rounded-lg hover:bg-zinc-800 text-zinc-400 hover:text-zinc-200 transition-colors"
          title={
            sidebarExpanded
              ? t("theme.collapseSidebar")
              : t("theme.expandSidebar")
          }
        >
          {sidebarExpanded ? (
            <PanelLeftClose className="w-4 h-4" />
          ) : (
            <PanelLeft className="w-4 h-4" />
          )}
        </button>
      </div>

      {/* 中间：命令面板搜索框 */}
      <button
        onClick={onOpenPalette}
        className={clsx(
          "flex items-center gap-2 flex-1 max-w-md mx-auto px-3 py-1.5 rounded-lg",
          "bg-zinc-800/60 border border-zinc-700/50 hover:border-zinc-600 hover:bg-zinc-800",
          "text-zinc-500 hover:text-zinc-400 transition-colors text-sm"
        )}
      >
        <Search className="w-4 h-4 flex-shrink-0" />
        <span className="flex-1 text-left truncate hidden sm:inline">
          {t("topbar.searchPlaceholder")}
        </span>
        <span className="flex-1 text-left sm:hidden">Ctrl+K</span>
        <kbd className="hidden sm:inline text-[10px] px-1.5 py-0.5 rounded bg-zinc-700/50 text-zinc-400 border border-zinc-600/50">
          Ctrl+K
        </kbd>
      </button>

      {/* 右侧：右侧栏展开/收起按钮（与左侧对称） */}
      <div className="flex items-center gap-1 flex-shrink-0">
        <button
          onClick={onToggleRightSidebar}
          className="p-1.5 rounded-lg text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-colors"
          title={t("rightbar.title")}
        >
          {rightSidebarExpanded ? (
            <PanelRightClose className="w-4 h-4" />
          ) : (
            <PanelRight className="w-4 h-4" />
          )}
        </button>
      </div>
    </header>
  );
}
