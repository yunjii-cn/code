import { Outlet, NavLink } from "react-router-dom";
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
  Sun,
  Moon,
} from "lucide-react";
import { clsx } from "clsx";
import { useTheme } from "@/lib/theme-store";

const navItems = [
  { to: "/chat", label: "AI 互动", icon: MessageSquare },
  { to: "/timeline", label: "时间轴", icon: Clock },
  { to: "/branches", label: "分支", icon: GitBranch },
  { to: "/search", label: "语义搜索", icon: Search },
  { to: "/team", label: "团队协作", icon: Users },
  { to: "/tasks", label: "任务看板", icon: ClipboardList },
  { to: "/training", label: "员工培训", icon: GraduationCap },
  { to: "/marketplace", label: "模板市场", icon: Store },
  { to: "/modelservice", label: "模型服务", icon: Server },
  { to: "/settings", label: "设置", icon: SettingsIcon },
];

export default function Layout() {
  const { isLight, toggleTheme } = useTheme();

  return (
    <div className="flex h-screen bg-zinc-950 text-zinc-100">
      {/* 侧边栏 */}
      <aside className="w-16 flex flex-col items-center py-4 border-r border-zinc-800 bg-zinc-900">
        {/* 导航 */}
        <nav className="flex-1 flex flex-col gap-2">
          {navItems.map(({ to, label, icon: Icon }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                clsx(
                  "w-10 h-10 rounded-lg flex items-center justify-center transition-colors",
                  isActive
                    ? "bg-brand-600 text-white"
                    : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
                )
              }
              title={label}
            >
              <Icon className="w-5 h-5" />
            </NavLink>
          ))}
        </nav>

        {/* 亮/暗切换按钮 */}
        <button
          onClick={toggleTheme}
          className="w-10 h-10 rounded-lg flex items-center justify-center text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-colors"
          title={isLight ? "切换到暗黑" : "切换到明亮"}
        >
          {isLight ? (
            <Moon className="w-5 h-5" />
          ) : (
            <Sun className="w-5 h-5" />
          )}
        </button>
      </aside>

      {/* 主内容区 */}
      <main className="flex-1 overflow-auto">
        <Outlet />
      </main>
    </div>
  );
}
