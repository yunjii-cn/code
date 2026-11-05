// 主布局组件
// 顶部横栏（TopBar）+ 左侧分组折叠导航 + 主内容区 + 右侧 Agent 配置面板
// 侧栏分组：互动 / 版本 / 团队 / 生态（点击组名展开/折叠子菜单）
// 左下角：用户头像菜单（设置/主题/更新/积分/活动/消息/扫码/帮助/退出）

import { useState, useEffect, useCallback } from "react";
import { Outlet, NavLink, useNavigate } from "react-router-dom";
import {
  Clock,
  GitBranch,
  Search,
  Users,
  ClipboardList,
  MessageSquare,
  GraduationCap,
  Store,
  Wrench,
  Cpu,
  Zap,
  Lightbulb,
  Workflow,
  Bot,
  TrendingUp,
  ChevronRight,
  ChevronDown,
  type LucideIcon,
} from "lucide-react";
import { clsx } from "clsx";
import { useI18n } from "@/i18n";
import { useTheme } from "@/lib/theme-store";
import CommandPalette from "./CommandPalette";
import TopBar from "./TopBar";
import UserMenu from "./UserMenu";
import AgentConfigPanel from "./AgentConfigPanel";

const SIDEBAR_KEY = "yunji-sidebar-expanded";
const RIGHT_SIDEBAR_KEY = "yunji-right-sidebar-expanded";
const COLLAPSED_GROUPS_KEY = "yunji-sidebar-collapsed-groups";

/** 导航项定义 */
interface NavItem {
  to: string;
  key: string;
  icon: LucideIcon;
}

/** 导航分组定义 */
interface NavGroup {
  groupKey: string;
  items: NavItem[];
}

// 侧栏分组：互动 / 版本 / 团队 / 生态
// 团队组包含进化仪表盘（从生态移回）+ 模型服务
// 生态组：技能 / 插件(=工具构建器) / 模板(=模板市场) / 灵感 / 自动化 / 智能体
const NAV_GROUPS: NavGroup[] = [
  {
    groupKey: "nav.group.interaction",
    items: [
      { to: "/chat", key: "nav.chat", icon: MessageSquare },
      { to: "/search", key: "nav.search", icon: Search },
    ],
  },
  {
    groupKey: "nav.group.version",
    items: [
      { to: "/timeline", key: "nav.timeline", icon: Clock },
      { to: "/branches", key: "nav.branches", icon: GitBranch },
    ],
  },
  {
    groupKey: "nav.group.team",
    items: [
      { to: "/team", key: "nav.team", icon: Users },
      { to: "/tasks", key: "nav.tasks", icon: ClipboardList },
      { to: "/training", key: "nav.training", icon: GraduationCap },
      { to: "/evolution", key: "nav.evolution", icon: TrendingUp },
      { to: "/modelservice", key: "nav.modelservice", icon: Cpu },
    ],
  },
  {
    groupKey: "nav.group.ecosystem",
    items: [
      { to: "/skills", key: "nav.skills", icon: Zap },
      { to: "/toolbuilder", key: "nav.plugins", icon: Wrench },
      { to: "/marketplace", key: "nav.templates", icon: Store },
      { to: "/inspiration", key: "nav.inspiration", icon: Lightbulb },
      { to: "/automation", key: "nav.automation", icon: Workflow },
      { to: "/agents", key: "nav.agents", icon: Bot },
    ],
  },
];

// 所有导航项（用于快捷键 Ctrl+1-9）
const ALL_NAV_ITEMS: NavItem[] = NAV_GROUPS.flatMap((g) => g.items);

export default function Layout() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { toggleTheme } = useTheme();

  // 侧栏展开/折叠
  const [expanded, setExpanded] = useState(() => {
    try {
      return localStorage.getItem(SIDEBAR_KEY) !== "false";
    } catch {
      return true;
    }
  });

  // 右侧栏展开/折叠（默认折叠）
  const [rightExpanded, setRightExpanded] = useState(() => {
    try {
      return localStorage.getItem(RIGHT_SIDEBAR_KEY) === "true";
    } catch {
      return false;
    }
  });

  // 分组折叠状态（哪些组被折叠了，默认全部展开）
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(() => {
    try {
      const saved = localStorage.getItem(COLLAPSED_GROUPS_KEY);
      if (saved) return new Set(JSON.parse(saved) as string[]);
    } catch {
      // ignore
    }
    return new Set();
  });

  const [paletteOpen, setPaletteOpen] = useState(false);
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);

  const toggleSidebar = useCallback(() => {
    setExpanded((v) => {
      const next = !v;
      try {
        localStorage.setItem(SIDEBAR_KEY, String(next));
      } catch {
        // ignore
      }
      return next;
    });
  }, []);

  const toggleRightSidebar = useCallback(() => {
    setRightExpanded((v) => {
      const next = !v;
      try {
        localStorage.setItem(RIGHT_SIDEBAR_KEY, String(next));
      } catch {
        // ignore
      }
      return next;
    });
  }, []);

  const toggleGroup = useCallback((groupKey: string) => {
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(groupKey)) {
        next.delete(groupKey);
      } else {
        next.add(groupKey);
      }
      try {
        localStorage.setItem(
          COLLAPSED_GROUPS_KEY,
          JSON.stringify([...next])
        );
      } catch {
        // ignore
      }
      return next;
    });
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
        toggleSidebar();
        return;
      }
      // Ctrl+J：切换主题
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "j") {
        e.preventDefault();
        toggleTheme();
        return;
      }
      // Ctrl+1~9：切换视图（基于 ALL_NAV_ITEMS 顺序）
      if ((e.ctrlKey || e.metaKey) && /^[1-9]$/.test(e.key)) {
        e.preventDefault();
        const idx = parseInt(e.key, 10) - 1;
        const item = ALL_NAV_ITEMS[idx];
        if (item) navigate(item.to);
        return;
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [navigate, toggleSidebar, toggleTheme]);

  /** 渲染单个导航项 */
  function renderNavItem(item: NavItem, idx: number, indent = false) {
    const Icon = item.icon;

    return (
      <NavLink
        key={item.to}
        to={item.to}
        onClick={() => setMobileSidebarOpen(false)}
        className={({ isActive }) =>
          clsx(
            "flex items-center gap-3 rounded-lg transition-colors group relative",
            expanded
              ? indent
                ? "px-3 py-1.5 ml-3"
                : "px-3 py-2"
              : "w-10 h-10 justify-center mx-auto",
            isActive
              ? "bg-brand-600 text-white hover:bg-[#0ea5e9] dark:hover:bg-brand-700"
              : "text-[#333333] dark:text-zinc-400 hover:text-[#2563eb] hover:bg-brand-50/60 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
          )
        }
        title={!expanded ? t(item.key) : undefined}
      >
        <Icon className="w-5 h-5 flex-shrink-0" />
        {expanded && (
          <span className="text-sm font-medium">{t(item.key)}</span>
        )}
        {/* 快捷键角标（仅折叠态显示前 9 个） */}
        {!expanded && idx < 9 && (
          <span className="absolute top-0.5 right-0.5 text-[9px] text-zinc-600 group-hover:text-zinc-400">
            {idx + 1}
          </span>
        )}
      </NavLink>
    );
  }

  /** 渲染分组（含折叠功能） */
  function renderGroup(group: NavGroup, groupIdx: number) {
    const isCollapsed = collapsedGroups.has(group.groupKey);
    const startIdx = NAV_GROUPS.slice(0, groupIdx).reduce(
      (acc, g) => acc + g.items.length,
      0
    );

    // 折叠态（侧栏 w-16）：不显示组名，只显示分隔线 + 图标
    if (!expanded) {
      return (
        <div key={group.groupKey} className="space-y-0.5">
          {groupIdx > 0 && (
            <div className="mx-2 my-1 border-t border-zinc-800" />
          )}
          {group.items.map((item, i) => renderNavItem(item, startIdx + i))}
        </div>
      );
    }

    // 展开态：显示组名（可点击折叠）+ 子菜单
    return (
      <div key={group.groupKey} className="space-y-0.5">
        <button
          onClick={() => toggleGroup(group.groupKey)}
          className="w-full flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50 transition-colors group"
          title={t(group.groupKey)}
        >
          {isCollapsed ? (
            <ChevronRight className="w-3.5 h-3.5 flex-shrink-0" />
          ) : (
            <ChevronDown className="w-3.5 h-3.5 flex-shrink-0" />
          )}
          <span className="text-[11px] font-semibold uppercase tracking-wider">
            {t(group.groupKey)}
          </span>
          <span className="ml-auto text-[10px] text-zinc-600">
            {group.items.length}
          </span>
        </button>
        {!isCollapsed && (
          <div className="space-y-0.5">
            {group.items.map((item, i) =>
              renderNavItem(item, startIdx + i, true)
            )}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen bg-zinc-950 text-zinc-50">
      {/* 顶部横栏 */}
      <TopBar
        onOpenPalette={() => setPaletteOpen(true)}
        onToggleMobileSidebar={() => setMobileSidebarOpen(true)}
        sidebarExpanded={expanded}
        onToggleSidebar={toggleSidebar}
        rightSidebarExpanded={rightExpanded}
        onToggleRightSidebar={toggleRightSidebar}
      />

      <div className="flex flex-1 overflow-hidden relative">
        {/* 移动端遮罩 */}
        {mobileSidebarOpen && (
          <div
            className="fixed inset-0 z-30 bg-black/60 md:hidden"
            onClick={() => setMobileSidebarOpen(false)}
          />
        )}

        {/* 桌面端左侧栏：flex 相对布局，紧贴 TopBar 无额外空白 */}
        <aside
          className={clsx(
            "hidden md:flex flex-col border-r border-zinc-800 bg-zinc-900 transition-all duration-200 z-40",
            expanded ? "w-56" : "w-16"
          )}
        >
          <nav className="flex-1 flex flex-col gap-2 px-2 pt-1 pb-2 overflow-y-auto">
            {NAV_GROUPS.map((group, idx) => renderGroup(group, idx))}
          </nav>

          {/* 底部：左下角用户头像菜单 */}
          <div className="px-2 pb-2 border-t border-zinc-800 pt-2 flex-shrink-0">
            <UserMenu compact={!expanded} />
          </div>
        </aside>

        {/* 移动端左侧栏：fixed 全屏覆盖 */}
        <aside
          className={clsx(
            "md:hidden flex flex-col border-r border-zinc-800 bg-zinc-900 transition-transform duration-200 z-40",
            "fixed inset-y-0 left-0 w-56",
            mobileSidebarOpen ? "translate-x-0" : "-translate-x-full"
          )}
        >
          <nav className="flex-1 flex flex-col gap-2 px-2 pt-2 pb-2 overflow-y-auto">
            {NAV_GROUPS.map((group, idx) => renderGroup(group, idx))}
          </nav>
          <div className="px-2 pb-3 border-t border-zinc-800 pt-2 flex-shrink-0">
            <UserMenu compact={false} />
          </div>
        </aside>

        {/* 主内容区 */}
        <main className="flex-1 overflow-auto">
          <Outlet />
        </main>

        {/* 右侧栏：Agent 配置面板 */}
        {rightExpanded && (
          <div className="hidden md:flex flex-shrink-0">
            <AgentConfigPanel onClose={toggleRightSidebar} />
          </div>
        )}
      </div>

      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        onToggleSidebar={toggleSidebar}
      />
    </div>
  );
}
