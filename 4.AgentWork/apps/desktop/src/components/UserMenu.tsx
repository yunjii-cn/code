// 左下角用户头像菜单
// 参考扣子 / WorkBuddy：头像+昵称+身份作为入口
// 展开菜单包含：设置、主题、检查更新、积分、活动、系统消息、手机扫码、帮助与反馈、退出登录

import { useState, useRef, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  Settings,
  Moon,
  Sun,
  Download,
  Gift,
  Coins,
  Bell,
  Smartphone,
  HelpCircle,
  LogOut,
  LogIn,
  ChevronUp,
  ChevronDown,
  Copy,
  Crown,
  Sparkles,
  User,
  type LucideIcon,
} from "lucide-react";
import { clsx } from "clsx";
import { useI18n } from "@/i18n";
import { useTheme } from "@/lib/theme-store";
import { useAuth, type UserInfo } from "@/lib/auth-store";
import { getAppVersion } from "@/lib/version";

interface MenuItem {
  id: string;
  key: string;
  icon: LucideIcon;
  action?: () => void;
  suffix?: React.ReactNode;
  danger?: boolean;
}

interface UserMenuProps {
  compact?: boolean;
}

export default function UserMenu({ compact = false }: UserMenuProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { isLight, toggleTheme } = useTheme();
  const { user, logout, openLoginModal } = useAuth();
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // 点击外部关闭
  useEffect(() => {
    function handler(e: MouseEvent) {
      if (!containerRef.current?.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    if (open) window.addEventListener("mousedown", handler);
    return () => window.removeEventListener("mousedown", handler);
  }, [open]);

  const copyUserId = async () => {
    if (!user?.userId) return;
    try {
      await navigator.clipboard.writeText(user.userId);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // ignore
    }
  };

  const handleLogout = () => {
    logout();
    setOpen(false);
    // 可选：调用后端清理 UM 会话
  };

  const handleLogin = () => {
    setOpen(false);
    openLoginModal("usermenu");
  };

  const menuItems: MenuItem[] = [
    {
      id: "theme",
      key: "usermenu.theme",
      icon: isLight ? Moon : Sun,
      action: toggleTheme,
      suffix: (
        <span className="text-xs text-zinc-500">
          {t(isLight ? "theme.light" : "theme.dark")}
        </span>
      ),
    },
    {
      id: "settings",
      key: "usermenu.settings",
      icon: Settings,
      action: () => navigate("/settings"),
    },
    {
      id: "update",
      key: "usermenu.checkUpdate",
      icon: Download,
      action: () => navigate("/update"),
    },
    {
      id: "points",
      key: "usermenu.points",
      icon: Coins,
      action: () => navigate("/points"),
      suffix: <span className="text-xs font-medium text-brand-400">455</span>,
    },
    {
      id: "events",
      key: "usermenu.events",
      icon: Gift,
      action: () => navigate("/events"),
    },
    {
      id: "notifications",
      key: "usermenu.notifications",
      icon: Bell,
      action: () => navigate("/notifications"),
      suffix:
        user && user.unreadCount > 0 ? (
          <span className="min-w-[18px] h-[18px] px-1 flex items-center justify-center rounded-full bg-red-500/80 text-[10px] text-white font-medium">
            {user.unreadCount}
          </span>
        ) : undefined,
    },
    {
      id: "mobile",
      key: "usermenu.mobile",
      icon: Smartphone,
      action: () => navigate("/mobile"),
    },
    {
      id: "help",
      key: "usermenu.help",
      icon: HelpCircle,
      action: () => navigate("/help"),
    },
    user
      ? {
          id: "logout",
          key: "usermenu.logout",
          icon: LogOut,
          danger: true,
          action: handleLogout,
        }
      : {
          id: "login",
          key: "nav.login",
          icon: LogIn,
          action: handleLogin,
        },
  ];

  // 未登录状态（折叠态）
  if (compact && !user) {
    return (
      <button
        onClick={handleLogin}
        className="w-10 h-10 rounded-lg flex items-center justify-center text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-colors"
        title={t("nav.login")}
      >
        <LogIn className="w-5 h-5" />
      </button>
    );
  }

  // 折叠态：仅显示头像按钮
  if (compact) {
    return (
      <div ref={containerRef} className="relative">
        <button
          onClick={() => setOpen((v) => !v)}
          className={clsx(
            "w-10 h-10 rounded-lg flex items-center justify-center transition-colors relative",
            open
              ? "bg-brand-600 text-white"
              : "text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200"
          )}
          title={user?.name}
        >
          <UserAvatar user={user!} size="sm" />
          {user && user.unreadCount > 0 && (
            <span className="absolute top-0.5 right-0.5 w-2 h-2 rounded-full bg-red-500" />
          )}
        </button>

        {open && (
          <div className="absolute left-full bottom-0 ml-2 w-64 bg-zinc-900 border border-zinc-700 rounded-xl shadow-2xl overflow-hidden z-50">
            <CompactMenuContent
              user={user!}
              menuItems={menuItems}
              onClose={() => setOpen(false)}
            />
          </div>
        )}
      </div>
    );
  }

  // 未登录展开态
  if (!user) {
    return (
      <button
        onClick={handleLogin}
        className="group w-full flex items-center gap-2.5 px-2 py-2 rounded-xl transition-colors text-zinc-300 hover:bg-zinc-800/60 hover:text-zinc-100"
      >
        <span className="w-8 h-8 rounded-full bg-gradient-to-br from-brand-500 to-brand-700 flex items-center justify-center">
          <User className="w-4 h-4 text-white" />
        </span>
        <span className="text-sm font-medium text-[#2563eb] group-hover:text-[#0ea5e9] transition-colors">
          {t("nav.login")}
        </span>
      </button>
    );
  }

  // 展开态：头像+昵称+身份卡片
  return (
    <div ref={containerRef} className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        className={clsx(
          "group w-full flex items-center gap-2.5 px-2 py-2 rounded-xl transition-colors",
          open
            ? "bg-zinc-800 text-zinc-100"
            : "text-zinc-300 hover:bg-zinc-800/60 hover:text-zinc-100"
        )}
      >
        <span className="relative flex-shrink-0">
          <UserAvatar user={user} size="md" />
          {user.unreadCount > 0 && (
            <span className="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-red-500 border-2 border-zinc-900" />
          )}
        </span>

        <div className="flex-1 min-w-0 text-left">
          <div className="flex items-center gap-1.5">
            <span className="text-sm font-medium truncate text-[#2563eb] dark:text-zinc-200 group-hover:text-[#0ea5e9] dark:group-hover:text-zinc-100 transition-colors">
              {user.name}
            </span>
            <Crown className="w-3 h-3 text-amber-400 flex-shrink-0" />
          </div>
          <div className="flex items-center gap-1.5 text-[11px] text-zinc-500">
            <span>{user.role}</span>
            <span className="w-1 h-1 rounded-full bg-zinc-600" />
            <span>{user.plan}</span>
          </div>
        </div>

        {open ? (
          <ChevronDown className="w-4 h-4 text-zinc-500 flex-shrink-0" />
        ) : (
          <ChevronUp className="w-4 h-4 text-zinc-500 flex-shrink-0" />
        )}
      </button>

      {open && (
        <div className="absolute left-0 right-0 bottom-full mb-2 bg-zinc-900 border border-zinc-700 rounded-xl shadow-2xl overflow-hidden z-50">
          <ExpandedMenuContent
            user={user}
            copied={copied}
            onCopy={copyUserId}
            menuItems={menuItems}
            onClose={() => setOpen(false)}
          />
        </div>
      )}
    </div>
  );
}

function UserAvatar({ user, size }: { user: UserInfo; size: "sm" | "md" | "lg" }) {
  const dims =
    size === "sm" ? "w-7 h-7 text-xs" : size === "md" ? "w-8 h-8 text-sm" : "w-11 h-11 text-lg";
  if (user.avatar) {
    return (
      <img
        src={user.avatar}
        alt={user.name}
        className={clsx(dims, "rounded-full object-cover")}
      />
    );
  }
  return (
    <span
      className={clsx(
        dims,
        "rounded-full bg-gradient-to-br from-brand-500 to-brand-700 flex items-center justify-center font-bold text-white"
      )}
    >
      {user.name.slice(0, 1)}
    </span>
  );
}

function CompactMenuContent({
  user,
  menuItems,
  onClose,
}: {
  user: UserInfo;
  menuItems: MenuItem[];
  onClose: () => void;
}) {
  const { t } = useI18n();
  return (
    <>
      {/* 用户信息 */}
      <div className="p-3 border-b border-zinc-800">
        <div className="flex items-center gap-3">
          <UserAvatar user={user} size="md" />
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-1.5">
              <span className="text-sm font-medium text-zinc-100 truncate">
                {user.name}
              </span>
              <Crown className="w-3 h-3 text-amber-400" />
            </div>
            <div className="text-[11px] text-zinc-500">
              {user.role} · {user.plan}
            </div>
          </div>
        </div>
      </div>
      {/* 菜单项 */}
      <div className="p-1.5">
        {menuItems.map((item) => (
          <MenuRow key={item.id} item={item} t={t} onClose={onClose} />
        ))}
      </div>
    </>
  );
}

function ExpandedMenuContent({
  user,
  copied,
  onCopy,
  menuItems,
  onClose,
}: {
  user: UserInfo;
  copied: boolean;
  onCopy: () => void;
  menuItems: MenuItem[];
  onClose: () => void;
}) {
  const { t } = useI18n();
  return (
    <>
      {/* 用户信息 + ID 复制 */}
      <div className="p-3 border-b border-zinc-800">
        <div className="flex items-center gap-3 mb-3">
          <UserAvatar user={user} size="lg" />
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-1.5">
              <span className="text-sm font-semibold text-zinc-100 truncate">
                {user.name}
              </span>
              <Crown className="w-3.5 h-3.5 text-amber-400" />
            </div>
            <div className="text-[11px] text-zinc-500">
              {user.role} · {user.plan}
            </div>
          </div>
        </div>
        {user.userId && (
          <button
            onClick={onCopy}
            className="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg bg-zinc-800/60 hover:bg-zinc-800 text-zinc-400 hover:text-zinc-300 transition-colors"
          >
            <span className="text-xs font-mono">ID: {user.userId}</span>
            {copied ? (
              <span className="text-[10px] text-brand-400">{t("common.success")}</span>
            ) : (
              <Copy className="w-3 h-3" />
            )}
          </button>
        )}
      </div>

      {/* 活动卡片（占位） */}
      <div className="mx-3 mt-3 p-3 rounded-xl bg-gradient-to-r from-brand-900/40 to-emerald-900/30 border border-brand-700/30">
        <div className="flex items-center gap-2 mb-1">
          <Sparkles className="w-3.5 h-3.5 text-brand-400" />
          <span className="text-xs font-semibold text-zinc-200">
            {t("usermenu.eventCard.title")}
          </span>
        </div>
        <p className="text-[11px] text-zinc-500 mb-2">
          {t("usermenu.eventCard.desc")}
        </p>
        <button
          onClick={() => {
            onClose();
            // 占位跳转
          }}
          className="text-[11px] px-2.5 py-1 rounded-md bg-brand-600/80 hover:bg-brand-600 text-white transition-colors"
        >
          {t("usermenu.eventCard.action")}
        </button>
      </div>

      {/* 菜单项 */}
      <div className="p-1.5 mt-1">
        {menuItems.map((item) => (
          <MenuRow key={item.id} item={item} t={t} onClose={onClose} />
        ))}
      </div>

      {/* 版本号：封装时间注入，开发模式为启动时间 */}
      <div className="pb-2 text-center text-[10px] font-mono text-zinc-600">
        v{getAppVersion()}
      </div>
    </>
  );
}

function MenuRow({
  item,
  t,
  onClose,
}: {
  item: MenuItem;
  t: (key: string) => string;
  onClose: () => void;
}) {
  const Icon = item.icon;
  return (
    <button
      onClick={() => {
        item.action?.();
        onClose();
      }}
      className={clsx(
        "w-full flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-sm transition-colors",
        item.danger
          ? "text-red-400 hover:bg-red-500/10"
          : "text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100"
      )}
    >
      <Icon className="w-4 h-4 flex-shrink-0" />
      <span className="flex-1 text-left">{t(item.key)}</span>
      {item.suffix}
    </button>
  );
}
