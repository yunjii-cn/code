// 主题二选一 toggle（segmented control 风格）
// 两个并排按钮：太阳 | 月亮，当前激活的有高亮背景
// 比 VS Code 的更精致：带平滑滑动指示器

import { Sun, Moon } from "lucide-react";
import { clsx } from "clsx";
import { useTheme } from "@/lib/theme-store";
import { useI18n } from "@/i18n";

interface ThemeToggleProps {
  /** 折叠态：只显示图标，不显示容器边框 */
  compact?: boolean;
}

export default function ThemeToggle({ compact = false }: ThemeToggleProps) {
  const { isLight, toggleTheme } = useTheme();
  const { t } = useI18n();

  if (compact) {
    // 折叠态：单个图标按钮，点击切换
    return (
      <button
        onClick={toggleTheme}
        className="w-10 h-10 flex items-center justify-center mx-auto rounded-lg text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-colors"
        title={isLight ? t("theme.toggleDark") : t("theme.toggleLight")}
      >
        {isLight ? <Moon className="w-5 h-5" /> : <Sun className="w-5 h-5" />}
      </button>
    );
  }

  // 展开态：segmented control
  return (
    <div
      className="relative flex items-center gap-0.5 p-0.5 rounded-lg bg-zinc-800/60 border border-zinc-700/50"
      role="group"
      aria-label={t("theme.toggle")}
    >
      {/* 滑动指示器 */}
      <div
        className={clsx(
          "absolute top-0.5 bottom-0.5 w-[calc(50%-2px)] rounded-md transition-transform duration-200 ease-out",
          "bg-brand-600 shadow-sm",
          isLight ? "translate-x-0" : "translate-x-[calc(100%+2px)]"
        )}
      />
      {/* 明亮按钮 */}
      <button
        onClick={isLight ? undefined : toggleTheme}
        className={clsx(
          "relative z-10 flex-1 flex items-center justify-center gap-1.5 py-1.5 px-2 rounded-md text-xs font-medium transition-colors",
          isLight
            ? "text-white"
            : "text-zinc-400 hover:text-zinc-200"
        )}
        title={t("theme.light")}
      >
        <Sun className="w-3.5 h-3.5" />
        <span>{t("theme.light")}</span>
      </button>
      {/* 暗黑按钮 */}
      <button
        onClick={!isLight ? undefined : toggleTheme}
        className={clsx(
          "relative z-10 flex-1 flex items-center justify-center gap-1.5 py-1.5 px-2 rounded-md text-xs font-medium transition-colors",
          !isLight
            ? "text-white"
            : "text-zinc-400 hover:text-zinc-200"
        )}
        title={t("theme.dark")}
      >
        <Moon className="w-3.5 h-3.5" />
        <span>{t("theme.dark")}</span>
      </button>
    </div>
  );
}
