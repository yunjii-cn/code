import type { ReactNode } from "react";
import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { motion, AnimatePresence } from "framer-motion";

interface Slide {
  title: string;
  subtitle: string;
  render: () => ReactNode;
}

function DashboardMock() {
  return (
    <div className="w-full aspect-[16/10] rounded-xl border border-[var(--color-aw-primary)]/10 bg-[var(--color-surface)] overflow-hidden shadow-[0_0_60px_rgba(30,108,240,0.08)]">
      {/* Top bar */}
      <div className="h-10 border-b border-[var(--color-border)] flex items-center gap-2 px-4">
        <div className="flex gap-1.5">
          <div className="w-2.5 h-2.5 rounded-full bg-red-400/60" />
          <div className="w-2.5 h-2.5 rounded-full bg-yellow-400/60" />
          <div className="w-2.5 h-2.5 rounded-full bg-green-400/60" />
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="w-64 h-5 rounded-full bg-[var(--color-border)]/50" />
        </div>
      </div>
      {/* Body — dashboard grid */}
      <div className="p-4 sm:p-5 grid grid-cols-2 sm:grid-cols-4 gap-3">
        {["任务总数", "活跃 Agent", "成功率", "平均耗时"].map((_label, i) => (
          <div key={i} className="rounded-lg border border-[var(--color-border)] p-3 space-y-2">
            <div className="h-2 w-12 rounded bg-[var(--color-border)]" />
            <div className="h-5 w-16 rounded bg-[var(--color-aw-primary)]/20" />
            <div className="h-1.5 w-20 rounded bg-[var(--color-border)]" />
          </div>
        ))}
      </div>
      {/* Chart area */}
      <div className="px-4 sm:px-5 pb-4 grid grid-cols-2 gap-3">
        <div className="rounded-lg border border-[var(--color-border)] p-3 space-y-3">
          <div className="h-3 w-20 rounded bg-[var(--color-border)]" />
          <div className="space-y-2">
            {[80, 55, 72, 40].map((h, i) => (
              <div key={i} className="flex items-end gap-1">
                <div className="h-3 w-8 rounded bg-[var(--color-border)]" />
                <div className="h-2 rounded bg-[var(--color-aw-primary)]/15" style={{ width: `${h}%` }} />
              </div>
            ))}
          </div>
        </div>
        <div className="rounded-lg border border-[var(--color-border)] p-3 space-y-3">
          <div className="h-3 w-16 rounded bg-[var(--color-border)]" />
          <div className="space-y-1.5">
            {[1, 2, 3].map((i) => (
              <div key={i} className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-[var(--color-aw-primary)]/20" />
                <div className="h-2 rounded bg-[var(--color-border)] flex-1" />
                <div className="h-2 w-10 rounded bg-[var(--color-border)]" />
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function WorkflowMock() {
  const nodes = [
    { x: 30, y: 25 }, { x: 60, y: 15 }, { x: 70, y: 45 },
    { x: 48, y: 55 }, { x: 22, y: 50 }, { x: 15, y: 30 },
  ];

  return (
    <div className="w-full aspect-[16/10] rounded-xl border border-[var(--color-aw-primary)]/10 bg-[var(--color-surface)] overflow-hidden shadow-[0_0_60px_rgba(30,108,240,0.08)]">
      <div className="h-10 border-b border-[var(--color-border)] flex items-center gap-2 px-4">
        <div className="flex gap-1.5">
          <div className="w-2.5 h-2.5 rounded-full bg-red-400/60" />
          <div className="w-2.5 h-2.5 rounded-full bg-yellow-400/60" />
          <div className="w-2.5 h-2.5 rounded-full bg-green-400/60" />
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="w-48 h-5 rounded-full bg-[var(--color-border)]/50" />
        </div>
      </div>
      <div className="relative flex-1 p-6" style={{ height: "calc(100% - 40px)" }}>
        {/* Connection lines */}
        <svg className="absolute inset-0 w-full h-full pointer-events-none">
          {nodes.slice(0, -1).map((_, i) => (
            <line key={i}
              x1={`${nodes[i]?.x ?? 0}%`} y1={`${nodes[i]?.y ?? 0}%`}
              x2={`${nodes[i + 1]?.x ?? 0}%`} y2={`${nodes[i + 1]?.y ?? 0}%`}
              stroke="var(--color-aw-primary)" strokeOpacity="0.2" strokeWidth="1.5"
              strokeDasharray="4 3"
            />
          ))}
        </svg>
        {/* Nodes */}
        {nodes.map((n, i) => (
          <div key={i} className="absolute -translate-x-1/2 -translate-y-1/2" style={{ left: `${n.x}%`, top: `${n.y}%` }}>
            <div className="w-10 h-10 rounded-xl bg-[var(--color-aw-primary)]/15 border border-[var(--color-aw-primary)]/30 flex items-center justify-center">
              <div className="w-3 h-3 rounded-sm bg-[var(--color-aw-primary)]/40" />
            </div>
            <div className="absolute -bottom-5 left-1/2 -translate-x-1/2 whitespace-nowrap">
              <div className="h-1.5 w-10 rounded bg-[var(--color-border)]" />
            </div>
          </div>
        ))}
        {/* Side panel */}
        <div className="absolute right-3 top-1/2 -translate-y-1/2 w-24 space-y-2">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-8 rounded-lg border border-[var(--color-border)] bg-[var(--color-bg)]/30 flex items-center px-2 gap-1.5">
              <div className="w-2 h-2 rounded-full bg-[var(--color-aw-primary)]/30" />
              <div className="h-1.5 flex-1 rounded bg-[var(--color-border)]" />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function CodeMock() {
  return (
    <div className="w-full aspect-[16/10] rounded-xl border border-[var(--color-aw-primary)]/10 bg-[var(--color-surface)] overflow-hidden shadow-[0_0_60px_rgba(30,108,240,0.08)]">
      <div className="h-10 border-b border-[var(--color-border)] flex items-center gap-2 px-4">
        <div className="flex gap-1.5">
          <div className="w-2.5 h-2.5 rounded-full bg-red-400/60" />
          <div className="w-2.5 h-2.5 rounded-full bg-yellow-400/60" />
          <div className="w-2.5 h-2.5 rounded-full bg-green-400/60" />
        </div>
        <div className="flex gap-2 ml-4">
          <div className="h-5 w-16 rounded bg-[var(--color-border)]/30" />
          <div className="h-5 w-16 rounded bg-[var(--color-border)]/20" />
        </div>
      </div>
      <div className="flex" style={{ height: "calc(100% - 40px)" }}>
        {/* File tree */}
        <div className="w-28 sm:w-36 border-r border-[var(--color-border)] p-3 space-y-2">
          {["src/", "agents/", "config/"].map((_d, i) => (
            <div key={i} className="flex items-center gap-1.5">
              <div className="w-3 h-3 rounded bg-[var(--color-aw-primary)]/15" />
              <div className="h-1.5 flex-1 rounded bg-[var(--color-border)]" style={{ width: [70, 85, 60][i] }} />
            </div>
          ))}
        </div>
        {/* Code area */}
        <div className="flex-1 p-4 space-y-3">
          {[
            { indent: 0, w: 60, c: "text-[var(--color-aw-soft)]/30" },
            { indent: 4, w: 80, c: "text-[var(--color-aw-soft)]/15" },
            { indent: 8, w: 70, c: "text-[var(--color-aw-primary)]/20" },
            { indent: 8, w: 50, c: "text-[var(--color-text-faint)]" },
            { indent: 4, w: 40, c: "text-[var(--color-aw-soft)]/15" },
            { indent: 0, w: 55, c: "text-[var(--color-aw-soft)]/30" },
            { indent: 4, w: 90, c: "text-[var(--color-aw-primary)]/20" },
            { indent: 4, w: 65, c: "text-[var(--color-text-faint)]" },
          ].map((line, i) => (
            <div key={i} className="flex items-center gap-3">
              <span className="w-6 text-right text-[10px] text-[var(--color-text-faint)]">{i + 1}</span>
              <div className={`h-2 rounded ${line.c}`} style={{ width: `${line.w}%`, marginLeft: line.indent * 2 }} />
            </div>
          ))}
          {/* Cursor blink */}
          <div className="flex items-center gap-3">
            <span className="w-6 text-right text-[10px] text-[var(--color-text-faint)]">9</span>
            <div className="w-0.5 h-4 rounded bg-[var(--color-aw-primary)]/50 animate-pulse" />
          </div>
        </div>
      </div>
    </div>
  );
}

function TeamMock() {
  return (
    <div className="w-full aspect-[16/10] rounded-xl border border-[var(--color-aw-primary)]/10 bg-[var(--color-surface)] overflow-hidden shadow-[0_0_60px_rgba(30,108,240,0.08)]">
      <div className="h-10 border-b border-[var(--color-border)] flex items-center gap-2 px-4">
        <div className="flex gap-1.5">
          <div className="w-2.5 h-2.5 rounded-full bg-red-400/60" />
          <div className="w-2.5 h-2.5 rounded-full bg-yellow-400/60" />
          <div className="w-2.5 h-2.5 rounded-full bg-green-400/60" />
        </div>
        <div className="flex-1" />
        <div className="h-5 w-5 rounded bg-[var(--color-aw-primary)]/15" />
      </div>
      <div className="flex" style={{ height: "calc(100% - 40px)" }}>
        {/* Avatars side */}
        <div className="w-14 border-r border-[var(--color-border)] flex flex-col items-center gap-3 pt-3">
          {["#1E6CF0", "#5B9BFF", "#8AB4FF", "#1553C7"].map((c, i) => (
            <div key={i} className="w-8 h-8 rounded-xl flex items-center justify-center text-[10px] font-bold text-white" style={{ backgroundColor: c, opacity: 0.6 + i * 0.1 }}>
              A{i + 1}
            </div>
          ))}
        </div>
        {/* Chat area */}
        <div className="flex-1 p-4 space-y-3">
          {[
            { align: "left", w: 70, c: "bg-[var(--color-border)]" },
            { align: "right", w: 55, c: "bg-[var(--color-aw-primary)]/15" },
            { align: "left", w: 85, c: "bg-[var(--color-border)]" },
            { align: "left", w: 60, c: "bg-[var(--color-border)]" },
            { align: "right", w: 75, c: "bg-[var(--color-aw-primary)]/15" },
            { align: "left", w: 50, c: "bg-[var(--color-aw-primary)]/10" },
          ].map((msg, i) => (
            <div key={i} className={`flex ${msg.align === "right" ? "justify-end" : "justify-start"}`}>
              <div className={`h-5 rounded-lg ${msg.c}`} style={{ width: `${msg.w}%` }} />
            </div>
          ))}
          {/* Typing indicator */}
          <div className="flex justify-start items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-[var(--color-aw-primary)]/30 animate-bounce" style={{ animationDelay: "0ms" }} />
            <div className="w-1.5 h-1.5 rounded-full bg-[var(--color-aw-primary)]/30 animate-bounce" style={{ animationDelay: "150ms" }} />
            <div className="w-1.5 h-1.5 rounded-full bg-[var(--color-aw-primary)]/30 animate-bounce" style={{ animationDelay: "300ms" }} />
          </div>
        </div>
      </div>
    </div>
  );
}

export default function ProductShowcase() {
  const { t } = useTranslation();
  const [current, setCurrent] = useState(0);
  const [isPaused, setIsPaused] = useState(false);

  const slidesData = t("showcase.slides", { returnObjects: true }) as Array<{ title: string; subtitle: string }>;

  const slides: Slide[] = [
    { ...slidesData[0], render: () => <DashboardMock /> } as Slide,
    { ...slidesData[1], render: () => <WorkflowMock /> } as Slide,
    { ...slidesData[2], render: () => <CodeMock /> } as Slide,
    { ...slidesData[3], render: () => <TeamMock /> } as Slide,
  ].filter((s): s is Slide => s.title !== undefined);

  const next = useCallback(() => {
    setCurrent((c) => (c + 1) % slides.length);
  }, []);

  useEffect(() => {
    if (isPaused) return;
    const timer = setInterval(next, 5000);
    return () => clearInterval(timer);
  }, [isPaused, next]);

  const slide = slides[current];
  if (!slide) return null;

  return (
    <section className="relative py-20 sm:py-28 px-4 sm:px-6 overflow-hidden">
      <div className="absolute inset-0 bg-[var(--color-aw-primary)]/[0.008]" />

      <div className="relative max-w-5xl mx-auto">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.5 }}
          className="text-center mb-12 sm:mb-16"
        >
          <div className="inline-flex items-center gap-2.5 px-4 py-1.5 rounded-full border border-[var(--color-aw-primary)]/8 bg-[var(--color-aw-primary)]/3 text-[var(--color-aw-soft)] text-xs font-semibold tracking-wider uppercase mb-6">
            {t("showcase.badge")}
          </div>
          <h2 className="text-2xl sm:text-3xl md:text-4xl font-bold text-[var(--color-text)] mb-3">
            {t("showcase.title")}
          </h2>
          <p className="text-[var(--color-text-muted)] text-sm sm:text-base max-w-lg mx-auto">
            {t("showcase.subtitle")}
          </p>
        </motion.div>

        {/* Slideshow */}
        <div
          className="relative"
          onMouseEnter={() => setIsPaused(true)}
          onMouseLeave={() => setIsPaused(false)}
        >
          {/* Navigation dots */}
          <div className="flex justify-center gap-2 mb-8">
            {slides.map((s, i) => (
              <button
                key={s.title}
                onClick={() => setCurrent(i)}
                className={`h-1.5 rounded-full transition-all duration-300 ${
                  i === current
                    ? "w-8 bg-[var(--color-aw-primary)]"
                    : "w-1.5 bg-[var(--color-border)] hover:bg-[var(--color-aw-primary)]/40"
                }`}
              />
            ))}
          </div>

          {/* Slide content */}
          <AnimatePresence mode="wait">
            <motion.div
              key={current}
              initial={{ opacity: 0, y: 24, scale: 0.97 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -16, scale: 0.97 }}
              transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
            >
              {/* Mockup */}
              {slide.render()}

              {/* Caption */}
              <motion.div
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.2 }}
                className="mt-6 text-center"
              >
                <h3 className="text-lg font-bold text-[var(--color-text)] mb-1">
                  {slide.title}
                </h3>
                <p className="text-sm text-[var(--color-text-muted)]">
                  {slide.subtitle}
                </p>
              </motion.div>
            </motion.div>
          </AnimatePresence>

          {/* Arrow buttons — only on desktop */}
          <div className="hidden sm:block">
            <button
              onClick={() => setCurrent((c) => (c - 1 + slides.length) % slides.length)}
              className="absolute left-0 top-[45%] -translate-y-1/2 -translate-x-4 w-10 h-10 rounded-full bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-text-muted)] hover:text-[var(--color-text)] hover:border-[var(--color-aw-primary)]/30 flex items-center justify-center transition-all"
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M15 18l-6-6 6-6" />
              </svg>
            </button>
            <button
              onClick={next}
              className="absolute right-0 top-[45%] -translate-y-1/2 translate-x-4 w-10 h-10 rounded-full bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-text-muted)] hover:text-[var(--color-text)] hover:border-[var(--color-aw-primary)]/30 flex items-center justify-center transition-all"
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M9 18l6-6-6-6" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </section>
  );
}
