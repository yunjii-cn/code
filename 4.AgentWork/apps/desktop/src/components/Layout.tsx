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
} from "lucide-react";
import { clsx } from "clsx";

const navItems = [
  { to: "/chat", label: "AI 互动", icon: MessageSquare },
  { to: "/timeline", label: "时间轴", icon: Clock },
  { to: "/branches", label: "分支", icon: GitBranch },
  { to: "/search", label: "语义搜索", icon: Search },
  { to: "/team", label: "团队协作", icon: Users },
  { to: "/tasks", label: "任务看板", icon: ClipboardList },
  { to: "/training", label: "员工培训", icon: GraduationCap },
  { to: "/settings", label: "设置", icon: SettingsIcon },
];

export default function Layout() {
  return (
    <div className="flex h-screen bg-zinc-950 text-zinc-100">
      {/* 侧边栏 */}
      <aside className="w-16 flex flex-col items-center py-4 border-r border-zinc-800 bg-zinc-900">
        {/* Logo */}
        <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-brand-500 to-accent-500 flex items-center justify-center mb-8">
          <GitBranch className="w-5 h-5 text-white" />
        </div>

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
      </aside>

      {/* 主内容区 */}
      <main className="flex-1 overflow-auto">
        <Outlet />
      </main>
    </div>
  );
}
