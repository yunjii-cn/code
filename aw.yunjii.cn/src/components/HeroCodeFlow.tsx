import { useEffect, useRef } from "react";

interface Node {
  x: number;
  y: number;
  r: number;
  hue: number;
  alpha: number;
  pulse: number;
  connections: number[];
}

interface LineParticle {
  fromIdx: number;
  toIdx: number;
  t: number;
  speed: number;
  alpha: number;
}

const NODE_COUNT = 18;
const LINE_COUNT = 30;
const COLORS_HUE = [210, 220, 230, 200];

export default function HeroCodeFlow() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animId = 0;
    let nodes: Node[] = [];
    let lines: LineParticle[] = [];
    let w = 0;
    let h = 0;

    const resize = () => {
      const dpr = window.devicePixelRatio || 1;
      w = canvas.width = canvas.offsetWidth * dpr;
      h = canvas.height = canvas.offsetHeight * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const init = () => {
      nodes = [];
      for (let i = 0; i < NODE_COUNT; i++) {
        const connections: number[] = [];
        const connCount = Math.floor(Math.random() * 3) + 1;
        for (let c = 0; c < connCount; c++) {
          const t = Math.floor(Math.random() * NODE_COUNT);
          if (t !== i && !connections.includes(t)) connections.push(t);
        }
        nodes.push({
          x: Math.random() * w * 0.85 + w * 0.075,
          y: Math.random() * h * 0.8 + h * 0.1,
          r: Math.random() * 3 + 1.5,
          hue: COLORS_HUE[Math.floor(Math.random() * COLORS_HUE.length)] ?? 220,
          alpha: Math.random() * 0.5 + 0.3,
          pulse: Math.random() * Math.PI * 2,
          connections,
        });
      }

      lines = [];
      for (let i = 0; i < LINE_COUNT; i++) {
        const fromIdx = Math.floor(Math.random() * NODE_COUNT);
        const node = nodes[fromIdx];
        if (!node || node.connections.length === 0) continue;
        const toIdx = node.connections[Math.floor(Math.random() * node.connections.length)];
        if (toIdx === undefined) continue;
        lines.push({
          fromIdx,
          toIdx,
          t: Math.random(),
          speed: Math.random() * 0.003 + 0.001,
          alpha: Math.random() * 0.3 + 0.1,
        });
      }
    };

    resize();
    init();
    window.addEventListener("resize", () => { resize(); init(); });

    const tick = () => {
      ctx.clearRect(0, 0, w, h);

      // Draw connections (static, dim)
      for (let i = 0; i < nodes.length; i++) {
        const node = nodes[i];
        if (!node) continue;
        for (const ci of node.connections) {
          const other = nodes[ci];
          if (!other) continue;
          ctx.beginPath();
          ctx.moveTo(node.x, node.y);
          ctx.lineTo(other.x, other.y);
          ctx.strokeStyle = `hsla(${node.hue}, 80%, 50%, 0.06)`;
          ctx.lineWidth = 0.5;
          ctx.stroke();
        }
      }

      // Draw nodes
      for (const node of nodes) {
        if (!node) continue;
        node.pulse += 0.01;
        const a = node.alpha * (0.7 + 0.3 * Math.sin(node.pulse));

        // Glow
        const grad = ctx.createRadialGradient(node.x, node.y, 0, node.x, node.y, node.r * 4);
        grad.addColorStop(0, `hsla(${node.hue}, 80%, 60%, ${a * 0.4})`);
        grad.addColorStop(0.5, `hsla(${node.hue}, 80%, 60%, ${a * 0.1})`);
        grad.addColorStop(1, "transparent");
        ctx.beginPath();
        ctx.arc(node.x, node.y, node.r * 4, 0, Math.PI * 2);
        ctx.fillStyle = grad;
        ctx.fill();

        // Core
        ctx.beginPath();
        ctx.arc(node.x, node.y, node.r, 0, Math.PI * 2);
        ctx.fillStyle = `hsla(${node.hue}, 80%, 65%, ${a})`;
        ctx.fill();
      }

      // Draw flowing line particles
      for (const line of lines) {
        const from = nodes[line.fromIdx];
        const to = nodes[line.toIdx];
        if (!from || !to) continue;

        line.t += line.speed;
        if (line.t > 1) line.t -= 1;

        const px = from.x + (to.x - from.x) * line.t;
        const py = from.y + (to.y - from.y) * line.t;

        ctx.beginPath();
        ctx.arc(px, py, 1.5, 0, Math.PI * 2);
        ctx.fillStyle = `hsla(210, 80%, 70%, ${line.alpha})`;
        ctx.fill();

        // Trail
        const trailLen = 0.06;
        const t2 = Math.max(0, line.t - trailLen);
        const px2 = from.x + (to.x - from.x) * t2;
        const py2 = from.y + (to.y - from.y) * t2;

        const grad = ctx.createLinearGradient(px2, py2, px, py);
        grad.addColorStop(0, "transparent");
        grad.addColorStop(1, `hsla(210, 80%, 70%, ${line.alpha * 0.8})`);
        ctx.beginPath();
        ctx.moveTo(px2, py2);
        ctx.lineTo(px, py);
        ctx.strokeStyle = grad;
        ctx.lineWidth = 2;
        ctx.stroke();
      }

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
      style={{ maskImage: "radial-gradient(ellipse 70% 60% at 50% 40%, black 40%, transparent 75%)", WebkitMaskImage: "radial-gradient(ellipse 70% 60% at 50% 40%, black 40%, transparent 75%)" }}
    />
  );
}
