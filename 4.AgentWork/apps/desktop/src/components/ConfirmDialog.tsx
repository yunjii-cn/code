import { type ReactNode } from "react";
import { AlertTriangle, X } from "lucide-react";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  description: ReactNode;
  confirmText?: string;
  cancelText?: string;
  variant?: "danger" | "warning" | "info";
  loading?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

const variantConfig = {
  danger: {
    icon: AlertTriangle,
    iconColor: "text-red-400",
    buttonClass: "bg-red-600 hover:bg-red-700",
  },
  warning: {
    icon: AlertTriangle,
    iconColor: "text-amber-400",
    buttonClass: "bg-amber-600 hover:bg-amber-700",
  },
  info: {
    icon: AlertTriangle,
    iconColor: "text-brand-400",
    buttonClass: "bg-brand-600 hover:bg-brand-700",
  },
};

export default function ConfirmDialog({
  open,
  title,
  description,
  confirmText = "确认",
  cancelText = "取消",
  variant = "warning",
  loading = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  if (!open) return null;

  const cfg = variantConfig[variant];
  const Icon = cfg.icon;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={() => !loading && onCancel()}
    >
      <div
        className="bg-zinc-900 border border-zinc-700 rounded-xl shadow-2xl max-w-md w-full mx-4 overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* 头部 */}
        <div className="flex items-start justify-between p-5 pb-3">
          <div className="flex items-start gap-3">
            <Icon className={`w-5 h-5 mt-0.5 ${cfg.iconColor}`} />
            <h2 className="text-base font-semibold text-zinc-100">{title}</h2>
          </div>
          <button
            onClick={() => !loading && onCancel()}
            className="text-zinc-500 hover:text-zinc-300 transition-colors"
            disabled={loading}
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* 内容 */}
        <div className="px-5 pb-5">
          <div className="text-sm text-zinc-400 leading-relaxed">{description}</div>
        </div>

        {/* 按钮 */}
        <div className="flex justify-end gap-2 px-5 py-3 bg-zinc-950/50 border-t border-zinc-800">
          <button
            onClick={onCancel}
            disabled={loading}
            className="px-4 py-2 rounded-lg text-sm text-zinc-300 hover:bg-zinc-800 disabled:opacity-50 transition-colors"
          >
            {cancelText}
          </button>
          <button
            onClick={onConfirm}
            disabled={loading}
            className={`px-4 py-2 rounded-lg text-sm text-white disabled:opacity-50 transition-colors ${cfg.buttonClass}`}
          >
            {loading ? "处理中..." : confirmText}
          </button>
        </div>
      </div>
    </div>
  );
}
