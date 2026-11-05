// 生态模块占位页
// 用于技能/灵感/自动化/智能体等尚未实现的功能
// 统一展示"即将推出"提示，保持侧栏导航完整性

import { useNavigate } from "react-router-dom";
import { ArrowLeft, type LucideIcon } from "lucide-react";
import { useI18n } from "@/i18n";

interface EcoPlaceholderProps {
  /** 模块图标 */
  icon: LucideIcon;
  /** 模块标题 i18n key */
  titleKey: string;
  /** 模块描述 i18n key */
  descKey: string;
  /** 即将推出的功能列表 i18n keys */
  features: string[];
}

export default function EcoPlaceholder({
  icon: Icon,
  titleKey,
  descKey,
  features,
}: EcoPlaceholderProps) {
  const { t } = useI18n();
  const navigate = useNavigate();

  return (
    <div className="flex flex-col h-full">
      <header className="px-4 sm:px-6 py-4 border-b border-zinc-800 flex items-center gap-3">
        <button
          onClick={() => navigate(-1)}
          className="p-1.5 rounded-lg hover:bg-zinc-800 text-zinc-400"
        >
          <ArrowLeft className="w-5 h-5" />
        </button>
        <Icon className="w-5 h-5 text-brand-400" />
        <h1 className="text-lg font-semibold">{t(titleKey)}</h1>
      </header>

      <div className="flex-1 flex items-center justify-center p-6">
        <div className="max-w-md text-center space-y-4">
          <div className="w-16 h-16 mx-auto rounded-2xl bg-brand-600/20 flex items-center justify-center">
            <Icon className="w-8 h-8 text-brand-400" />
          </div>
          <h2 className="text-xl font-semibold text-zinc-100">
            {t(titleKey)}
          </h2>
          <p className="text-sm text-zinc-500">{t(descKey)}</p>

          {features.length > 0 && (
            <div className="text-left space-y-2 mt-6 p-4 rounded-lg bg-zinc-900 border border-zinc-800">
              <p className="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-2">
                {t("eco.comingSoon")}
              </p>
              {features.map((feature, i) => (
                <div
                  key={i}
                  className="flex items-center gap-2 text-sm text-zinc-400"
                >
                  <span className="w-1 h-1 rounded-full bg-brand-500" />
                  <span>{t(feature)}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
