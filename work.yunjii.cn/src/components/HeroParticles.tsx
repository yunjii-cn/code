import { useEffect, useRef } from "react";

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  r: number;
  alpha: number;
  hue: string;
  pulse: number;
}

const COLORS = ["#e61912", "#ff4d47", "#ff7a6e", "#ffb347", "#ff8c42"];
const COUNT = 70;
const MAX_DIST = 130;
const MARGIN = 40;

export default function HeroParticles() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animId = 0;
    let particles: Particle[] = [];
    let w = 0;
    let h = 0;

    const resize = () => {
      const dpr = window.devicePixelRatio || 1;
      w = canvas.width = canvas.offsetWidth * dpr;
      h = canvas.height = canvas.offsetHeight * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const init = () => {
      particles = [];
      for (let i = 0; i < COUNT; i++) {
        particles.push({
          x: Math.random() * w,
          y: Math.random() * h,
          vx: (Math.random() - 0.5) * 0.3,
          vy: (Math.random() - 0.5) * 0.3,
          r: Math.random() * 2.5 + 0.8,
          alpha: Math.random() * 0.4 + 0.15,
          hue: COLORS[Math.floor(Math.random() * COLORS.length)] ?? "#e61912",
          pulse: Math.random() * Math.PI * 2,
        });
      }
    };

    resize();
    init();
    window.addEventListener("resize", () => { resize(); init(); });

    const tick = () => {
      ctx.clearRect(0, 0, w, h);

      for (let i = 0; i < particles.length; i++) {
        const p = particles[i];
        if (!p) continue;
        p.pulse += 0.008;
        const a = p.alpha * (0.7 + 0.3 * Math.sin(p.pulse));

        p.x += p.vx;
        p.y += p.vy;

        if (p.x < -MARGIN) p.x = w + MARGIN;
        if (p.x > w + MARGIN) p.x = -MARGIN;
        if (p.y < -MARGIN) p.y = h + MARGIN;
        if (p.y > h + MARGIN) p.y = -MARGIN;

        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        ctx.fillStyle = p.hue;
        ctx.globalAlpha = a;
        ctx.fill();

        for (let j = i + 1; j < particles.length; j++) {
          const q = particles[j];
          if (!q) continue;
          const dx = p.x - q.x;
          const dy = p.y - q.y;
          const dist = Math.sqrt(dx * dx + dy * dy);

          if (dist < MAX_DIST) {
            ctx.beginPath();
            ctx.moveTo(p.x, p.y);
            ctx.lineTo(q.x, q.y);
            ctx.globalAlpha = a * (1 - dist / MAX_DIST) * 0.25;
            ctx.strokeStyle = p.hue;
            ctx.lineWidth = 0.5;
            ctx.stroke();
          }
        }
      }

      ctx.globalAlpha = 1;
      animId = requestAnimationFrame(tick);
    };

    tick();

    return () => {
      cancelAnimationFrame(animId);
      window.removeEventListener("resize", resize);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className="absolute inset-0 w-full h-full pointer-events-none"
      style={{ maskImage: "radial-gradient(ellipse 80% 70% at 50% 50%, black 30%, transparent 70%)", WebkitMaskImage: "radial-gradient(ellipse 80% 70% at 50% 50%, black 30%, transparent 70%)" }}
    />
  );
}
