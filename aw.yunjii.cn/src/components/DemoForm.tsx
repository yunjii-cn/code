import { useState, useRef, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { motion, AnimatePresence } from "framer-motion";

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

export default function DemoForm({ isOpen, onClose }: Props) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [company, setCompany] = useState("");
  const [email, setEmail] = useState("");
  const [needs, setNeeds] = useState("");
  const [submitted, setSubmitted] = useState(false);

  const nameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = "hidden";
      setTimeout(() => nameRef.current?.focus(), 200);
    } else {
      document.body.style.overflow = "";
      setSubmitted(false);
    }
    return () => { document.body.style.overflow = ""; };
  }, [isOpen]);

  useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    if (isOpen) window.addEventListener("keydown", handleEsc);
    return () => window.removeEventListener("keydown", handleEsc);
  }, [isOpen, onClose]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !email.trim()) return;

    const subject = encodeURIComponent(`AgentWork 预约演示 - ${company || name}`);
    const body = encodeURIComponent(
      `姓名：${name}\n公司：${company || "未填"}\n邮箱：${email}\n需求：${needs || "未填"}`
    );
    window.location.href = `mailto:sales@yunjii.cn?subject=${subject}&body=${body}`;
    setSubmitted(true);
    setName(""); setCompany(""); setEmail(""); setNeeds("");
  };

  return (
    <AnimatePresence>
      {isOpen && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            className="absolute inset-0 bg-black/60 backdrop-blur-sm"
          />

          {/* Modal */}
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: 20 }}
            transition={{ duration: 0.25, ease: "easeOut" }}
            className="relative w-full max-w-md bg-[var(--color-elevated)] border border-[var(--color-aw-primary)]/10 rounded-2xl shadow-2xl overflow-hidden"
          >
            {/* Top gradient accent */}
            <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-[var(--color-aw-primary)] via-[var(--color-aw-soft)] to-[var(--color-aw-primary)]/30" />

            {/* Close button */}
            <button
              onClick={onClose}
              className="absolute top-4 right-4 w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)] transition-colors"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </button>

            <div className="p-6 sm:p-8">
              {submitted ? (
                <motion.div
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  className="text-center py-8"
                >
                  <div className="w-14 h-14 rounded-full bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-soft)] flex items-center justify-center mx-auto mb-5">
                    <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <path d="M22 11.08V12a10 10 0 11-5.93-9.14" />
                      <path d="M22 4L12 14.01l-3-3" />
                    </svg>
                  </div>
                  <h3 className="text-xl font-bold text-[var(--color-text)] mb-2">提交成功</h3>
                  <p className="text-sm text-[var(--color-text-muted)]">
                    邮件客户端已打开，我们会尽快与你联系。
                  </p>
                  <button
                    onClick={onClose}
                    className="mt-6 px-6 py-2.5 bg-[var(--color-aw-primary)]/10 text-[var(--color-aw-soft)] rounded-xl text-sm font-medium hover:bg-[var(--color-aw-primary)]/20 transition-colors"
                  >
                    关闭
                  </button>
                </motion.div>
              ) : (
                <>
                  <h3 className="text-xl font-bold text-[var(--color-text)] mb-1">
                    预约演示
                  </h3>
                  <p className="text-sm text-[var(--color-text-muted)] mb-6">
                    填写以下信息，我们将在 24 小时内与你联系。
                  </p>

                  <form onSubmit={handleSubmit} className="space-y-4">
                    <div>
                      <label className="block text-xs font-medium text-[var(--color-text-muted)] mb-1.5">
                        姓名 *
                      </label>
                      <input
                        ref={nameRef}
                        type="text"
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        required
                        placeholder="你的姓名"
                        className="w-full px-4 py-2.5 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-xl text-sm text-[var(--color-text)] placeholder:text-[var(--color-text-faint)] focus:outline-none focus:border-[var(--color-aw-primary)]/40 focus:ring-2 focus:ring-[var(--color-aw-primary)]/10 transition-all"
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-medium text-[var(--color-text-muted)] mb-1.5">
                        公司
                      </label>
                      <input
                        type="text"
                        value={company}
                        onChange={(e) => setCompany(e.target.value)}
                        placeholder="公司名称"
                        className="w-full px-4 py-2.5 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-xl text-sm text-[var(--color-text)] placeholder:text-[var(--color-text-faint)] focus:outline-none focus:border-[var(--color-aw-primary)]/40 focus:ring-2 focus:ring-[var(--color-aw-primary)]/10 transition-all"
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-medium text-[var(--color-text-muted)] mb-1.5">
                        邮箱 *
                      </label>
                      <input
                        type="email"
                        value={email}
                        onChange={(e) => setEmail(e.target.value)}
                        required
                        placeholder="you@company.com"
                        className="w-full px-4 py-2.5 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-xl text-sm text-[var(--color-text)] placeholder:text-[var(--color-text-faint)] focus:outline-none focus:border-[var(--color-aw-primary)]/40 focus:ring-2 focus:ring-[var(--color-aw-primary)]/10 transition-all"
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-medium text-[var(--color-text-muted)] mb-1.5">
                        需求描述
                      </label>
                      <textarea
                        value={needs}
                        onChange={(e) => setNeeds(e.target.value)}
                        rows={3}
                        placeholder="请简述你的团队规模和使用场景…"
                        className="w-full px-4 py-2.5 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-xl text-sm text-[var(--color-text)] placeholder:text-[var(--color-text-faint)] focus:outline-none focus:border-[var(--color-aw-primary)]/40 focus:ring-2 focus:ring-[var(--color-aw-primary)]/10 transition-all resize-none"
                      />
                    </div>

                    <motion.button
                      type="submit"
                      whileHover={{ scale: 1.01 }}
                      whileTap={{ scale: 0.98 }}
                      className="w-full py-3 bg-[var(--color-aw-primary)] hover:bg-[var(--color-aw-hover)] text-white rounded-xl font-semibold text-sm transition-colors"
                    >
                      提交预约
                    </motion.button>
                  </form>

                  <p className="text-[10px] text-[var(--color-text-faint)] text-center mt-4">
                    提交即表示同意我们的隐私政策和服务条款。
                  </p>
                </>
              )}
            </div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
}
