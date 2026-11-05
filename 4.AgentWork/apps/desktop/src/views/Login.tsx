import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  Loader2,
  RefreshCw,
  Smartphone,
  Shield,
  Bot,
  Users,
  Cpu,
  Sparkles,
  Sun,
  Moon,
} from "lucide-react";
import { useI18n } from "@/i18n";
import { useTheme } from "@/lib/theme-store";
import { startUmLogin, cancelUmLogin, openExternalUrl } from "@/lib/tauri";
import { listen } from "@tauri-apps/api/event";
import { useAuth, storeUser, type StoredUser } from "@/lib/auth-store";
import { BRAND } from "@/lib/brand";

interface UmLoginResult {
  session_id: string;
  login_url: string;
  qrcode_url: string;
  qr_image_url?: string;
  redirect_uri: string;
  oauth_code?: string;
}

// 官方云集登录入口（PC 扫码）和视频创意站参考实现一致
const OFFICIAL_LOGIN_URL =
  "https://vi.yunjii.cn/sl/login.php?next=" +
  encodeURIComponent("https://agentwork.yunjii.cn/");

// Tauri 2 没有直接的 window.open，保持对原生 openExternalUrl 的兼容
async function openExternal(url: string) {
  try {
    await openExternalUrl(url);
  } catch (err) {
    console.warn("[Login] openExternalUrl failed, fallback to window.open:", err);
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

const HIGHLIGHTS = [
  { icon: Users, title: "全民上手", desc: "无代码向导，5 分钟即可创建专属 AI 团队" },
  { icon: Bot, title: "AI 员工队伍", desc: "多 Agent 分工协作，像管理员工一样管理 AI" },
  { icon: Cpu, title: "自进化能力", desc: "可视化进化仪表盘，经验可回放、可交易" },
];

const NETWORK_NODES = ["智能", "协作", "进化", "生态", "安全", "交互"];

function CollaborationNetwork({
  isLight,
  containerRef,
  buttonRef,
}: {
  isLight: boolean;
  containerRef: React.RefObject<HTMLDivElement | null>;
  buttonRef: React.RefObject<HTMLButtonElement | null>;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const hoverRef = useRef(false);
  const burstParticles = useRef<
    { x: number; y: number; vx: number; vy: number; life: number }[]
  >([]);
  const speedMultiplier = useRef(1);

  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animationId: number;

    const getCenter = () => {
      const btn = buttonRef.current;
      if (!btn) return { cx: window.innerWidth * 0.76, cy: window.innerHeight * 0.15 };
      const bRect = btn.getBoundingClientRect();
      return {
        cx: bRect.left + bRect.width / 2,
        cy: bRect.top + bRect.height / 2,
      };
    };

    const measure = () => {
      const dpr = window.devicePixelRatio || 1;
      const w = window.innerWidth;
      const h = window.innerHeight;
      canvas.width = w * dpr;
      canvas.height = h * dpr;
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    measure();

    const onResize = () => measure();
    window.addEventListener("resize", onResize);

    const primaryColor = isLight ? "59, 130, 246" : "96, 165, 250";
    const secondaryColor = isLight ? "139, 92, 246" : "167, 139, 250";
    const tertiaryColor = isLight ? "6, 182, 212" : "34, 211, 238";
    const glowColor = isLight ? "234, 179, 8" : "129, 140, 248";

    const nodeCount = 6;
    const baseOrbitRadius = 145;
    const baseOrbitSpeed = 0.002;
    let orbitAngle = 0;
    let width = canvas.width;
    let height = canvas.height;

    const agents = NETWORK_NODES.map((role, i) => ({
      role,
      angleOffset: (i / nodeCount) * Math.PI * 2,
      radius: baseOrbitRadius + (Math.random() - 0.5) * 20,
      pulse: Math.random() * Math.PI * 2,
    }));

    const dataPackets: {
      agentIdx: number;
      progress: number;
      speed: number;
      color: string;
    }[] = [];

    const bgParticles: {
      x: number;
      y: number;
      vx: number;
      vy: number;
      alpha: number;
    }[] = Array.from({ length: 14 }, () => ({
      x: 0,
      y: 0,
      vx: (Math.random() - 0.5) * 0.3,
      vy: (Math.random() - 0.5) * 0.3,
      alpha: Math.random() * 0.08 + 0.03,
    }));

    const initParticles = () => {
      width = window.innerWidth;
      height = window.innerHeight;
      bgParticles.forEach((p) => {
        p.x = Math.random() * width;
        p.y = Math.random() * height;
      });
    };
    initParticles();

    const spawnPacket = () => {
      const agentIdx = Math.floor(Math.random() * nodeCount);
      dataPackets.push({
        agentIdx,
        progress: 0,
        speed: (0.005 + Math.random() * 0.005) * speedMultiplier.current,
        color: Math.random() > 0.5 ? primaryColor : tertiaryColor,
      });
    };

    const spawnBurst = (x: number, y: number, count: number) => {
      for (let i = 0; i < count; i++) {
        const angle = Math.random() * Math.PI * 2;
        const speed = 0.6 + Math.random() * 1.8;
        burstParticles.current.push({
          x,
          y,
          vx: Math.cos(angle) * speed,
          vy: Math.sin(angle) * speed,
          life: 0.5 + Math.random() * 0.7,
        });
      }
    };

    let lastPacketSpawn = 0;
    let lastBurstTime = 0;

    const render = (time: number) => {
      ctx.clearRect(0, 0, width, height);
      const { cx, cy } = getCenter();

      const targetSpeed = hoverRef.current ? 2.5 : 1;
      speedMultiplier.current +=
        (targetSpeed - speedMultiplier.current) * 0.05;

      if (hoverRef.current && time - lastBurstTime > 400) {
        spawnBurst(cx, cy, 5);
        lastBurstTime = time;
      }

      bgParticles.forEach((p) => {
        p.x += p.vx;
        p.y += p.vy;
        if (p.x < 0 || p.x > width) p.vx *= -1;
        if (p.y < 0 || p.y > height) p.vy *= -1;
        ctx.beginPath();
        ctx.arc(p.x, p.y, 1, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${primaryColor}, ${p.alpha})`;
        ctx.fill();
      });

      for (let i = burstParticles.current.length - 1; i >= 0; i--) {
        const bp = burstParticles.current[i];
        bp.x += bp.vx;
        bp.y += bp.vy;
        bp.vx *= 0.97;
        bp.vy *= 0.97;
        bp.life -= 0.015;

        if (bp.life <= 0) {
          burstParticles.current.splice(i, 1);
          continue;
        }

        ctx.beginPath();
        ctx.arc(bp.x, bp.y, 1.4, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${glowColor}, ${bp.life * 0.7})`;
        ctx.shadowColor = `rgba(${glowColor}, ${bp.life * 0.6})`;
        ctx.shadowBlur = 5;
        ctx.fill();
        ctx.shadowBlur = 0;
      }

      orbitAngle += baseOrbitSpeed * speedMultiplier.current;

      const agentPositions = agents.map((agent) => {
        const angle = orbitAngle + agent.angleOffset;
        const x = cx + Math.cos(angle) * agent.radius;
        const y = cy + Math.sin(angle) * agent.radius;
        return { x, y, angle };
      });

      const lineAlpha = hoverRef.current ? 0.18 : 0.08;
      agentPositions.forEach((pos, i) => {
        const pulse = Math.sin(time * 0.003 + agents[i].pulse) * 0.5 + 0.5;
        ctx.beginPath();
        ctx.moveTo(cx, cy);
        ctx.lineTo(pos.x, pos.y);
        ctx.strokeStyle = `rgba(${primaryColor}, ${lineAlpha * (0.5 + pulse * 0.5)})`;
        ctx.lineWidth = hoverRef.current ? 1.5 : 0.7;
        ctx.stroke();
      });

      const ringAlpha = hoverRef.current ? 0.08 : 0.035;
      for (let i = 0; i < agentPositions.length; i++) {
        for (let j = i + 1; j < agentPositions.length; j++) {
          const dx = agentPositions[i].x - agentPositions[j].x;
          const dy = agentPositions[i].y - agentPositions[j].y;
          const dist = Math.sqrt(dx * dx + dy * dy);
          if (dist < baseOrbitRadius * 1.3) {
            ctx.beginPath();
            ctx.moveTo(agentPositions[i].x, agentPositions[i].y);
            ctx.lineTo(agentPositions[j].x, agentPositions[j].y);
            ctx.strokeStyle = `rgba(${secondaryColor}, ${ringAlpha})`;
            ctx.lineWidth = 0.5;
            ctx.stroke();
          }
        }
      }

      const spawnInterval = hoverRef.current ? 350 : 1400;
      if (time - lastPacketSpawn > spawnInterval) {
        spawnPacket();
        lastPacketSpawn = time;
      }

      for (let i = dataPackets.length - 1; i >= 0; i--) {
        const packet = dataPackets[i];
        packet.progress += packet.speed;
        if (packet.progress >= 1) {
          dataPackets.splice(i, 1);
          continue;
        }

        const pos = agentPositions[packet.agentIdx];
        const x = cx + (pos.x - cx) * packet.progress;
        const y = cy + (pos.y - cy) * packet.progress;

        ctx.beginPath();
        ctx.arc(x, y, 2, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${packet.color}, ${1 - packet.progress * 0.5})`;
        ctx.shadowColor = `rgba(${packet.color}, 0.7)`;
        ctx.shadowBlur = 6;
        ctx.fill();
        ctx.shadowBlur = 0;
      }

      agentPositions.forEach((pos, i) => {
        const pulse = Math.sin(time * 0.003 + agents[i].pulse) * 0.5 + 0.5;

        ctx.beginPath();
        ctx.arc(pos.x, pos.y, 6 + pulse * 1.2, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${tertiaryColor}, ${hoverRef.current ? 0.12 : 0.06})`;
        ctx.fill();

        ctx.beginPath();
        ctx.arc(pos.x, pos.y, 3.2, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${tertiaryColor}, 0.78)`;
        ctx.shadowColor = `rgba(${tertiaryColor}, 0.45)`;
        ctx.shadowBlur = 8;
        ctx.fill();
        ctx.shadowBlur = 0;

        const fontSize = hoverRef.current ? "10.5px" : "10px";
        ctx.font = `${fontSize} sans-serif`;
        ctx.textAlign = "center";
        ctx.fillStyle = isLight
          ? "rgba(15, 23, 42, 0.4)"
          : "rgba(255, 255, 255, 0.3)";
        ctx.fillText(agents[i].role, pos.x, pos.y + 16);
      });

      // 装饰性中心光球
      const centerPulse = Math.sin(time * 0.0025) * 0.5 + 0.5;
      const coreRadius = 16 + centerPulse * 6;

      const coreGrad = ctx.createRadialGradient(cx, cy, 0, cx, cy, coreRadius * 2.5);
      coreGrad.addColorStop(0, `rgba(${glowColor}, 0.6)`);
      coreGrad.addColorStop(0.3, `rgba(${glowColor}, 0.25)`);
      coreGrad.addColorStop(0.6, `rgba(${glowColor}, 0.05)`);
      coreGrad.addColorStop(1, "transparent");

      ctx.beginPath();
      ctx.arc(cx, cy, coreRadius * 2.5, 0, Math.PI * 2);
      ctx.fillStyle = coreGrad;
      ctx.fill();

      ctx.beginPath();
      ctx.arc(cx, cy, coreRadius, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(${glowColor}, 0.5)`;
      ctx.shadowColor = `rgba(${glowColor}, 0.45)`;
      ctx.shadowBlur = 22;
      ctx.fill();
      ctx.shadowBlur = 0;

      ctx.beginPath();
      ctx.arc(cx, cy, coreRadius * 0.4, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(255, 255, 255, 0.75)`;
      ctx.fill();

      animationId = requestAnimationFrame(render);
    };

    const onMouseEnter = () => {
      hoverRef.current = true;
    };
    const onMouseLeave = () => {
      hoverRef.current = false;
    };
    container.addEventListener("mouseenter", onMouseEnter);
    container.addEventListener("mouseleave", onMouseLeave);

    animationId = requestAnimationFrame(render);

    return () => {
      cancelAnimationFrame(animationId);
      window.removeEventListener("resize", onResize);
      container.removeEventListener("mouseenter", onMouseEnter);
      container.removeEventListener("mouseleave", onMouseLeave);
    };
  }, [isLight, containerRef, buttonRef]);

  return (
    <canvas
      ref={canvasRef}
      className="fixed inset-0 pointer-events-none"
      style={{ zIndex: 5 }}
    />
  );
}

export default function Login() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { toggleTheme, isLight } = useTheme();
  const brandPanelRef = useRef<HTMLDivElement>(null);
  const themeBtnRef = useRef<HTMLButtonElement>(null);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [loginData, setLoginData] = useState<UmLoginResult | null>(null);
  const [status, setStatus] = useState<"idle" | "waiting" | "success">("idle");
  const [iframeLoaded, setIframeLoaded] = useState(false);

  useEffect(() => {
    if (user) navigate("/", { replace: true });
  }, [user, navigate]);

  useEffect(() => {
    let unlistenSuccess: (() => void) | undefined;
    let unlistenFailed: (() => void) | undefined;

    listen("um:login-success", (event) => {
      const payload = event.payload as { session_id: string; user: StoredUser };
      setStatus("success");
      storeUser(payload.user);
      setTimeout(() => navigate("/", { replace: true }), 1000);
    }).then((fn) => {
      unlistenSuccess = fn;
    });

    listen("um:login-failed", (event) => {
      setStatus("idle");
      setError(String(event.payload));
      setLoading(false);
    }).then((fn) => {
      unlistenFailed = fn;
    });

    return () => {
      unlistenSuccess?.();
      unlistenFailed?.();
      cancelUmLogin().catch(() => {});
    };
  }, [navigate]);

  useEffect(() => {
    if (status !== "idle" || loginData) return;
    async function init() {
      try {
        setLoading(true);
        setError("");
        const data = await startUmLogin();
        setLoginData(data);
        setStatus("waiting");
      } catch (err) {
        setError(String(err));
      } finally {
        setLoading(false);
      }
    }
    init();
  }, [status, loginData]);

  async function handleRefresh() {
    try {
      setLoading(true);
      setError("");
      const data = await startUmLogin();
      setLoginData(data);
      setStatus("waiting");
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div
      className={`fixed inset-0 z-50 flex overflow-hidden ${isLight ? "light bg-[#f8fafc]" : "bg-[#02040a]"}`}
    >
      <div
        className={`absolute inset-0 pointer-events-none ${
          isLight
            ? "bg-[radial-gradient(ellipse_at_60%_32%,rgba(59,130,246,0.08),transparent_45%)]"
            : "bg-[radial-gradient(ellipse_at_60%_32%,rgba(37,99,235,0.12),transparent_45%)]"
        }`}
      />

      <div className="relative z-10 flex w-full h-full items-center justify-center px-6 lg:px-16 xl:px-24">
        <div className="flex w-full max-w-6xl items-stretch gap-8 lg:gap-16">
          {/* ===== 左侧：品牌区 ===== */}
          <div
            ref={brandPanelRef}
            className="relative flex-1 max-w-xl min-h-[520px] flex flex-col justify-center overflow-visible"
          >
            <CollaborationNetwork
              isLight={isLight}
              containerRef={brandPanelRef}
              buttonRef={themeBtnRef}
            />

            {/* 主题切换 — 置于雷达轨道中心 */}
            <button
              ref={themeBtnRef}
              onClick={toggleTheme}
              className="absolute z-20 w-14 h-14 rounded-full flex items-center justify-center
                         transition-all duration-300 hover:scale-110 active:scale-95"
              style={{
                right: "14%",
                top: "12%",
                background: isLight
                  ? "radial-gradient(circle at 35% 35%, rgba(251,191,36,0.9), rgba(234,179,8,0.6))"
                  : "radial-gradient(circle at 35% 35%, rgba(167,139,250,0.9), rgba(99,102,241,0.6))",
                boxShadow: isLight
                  ? "0 0 30px rgba(251,191,36,0.5), 0 0 60px rgba(251,191,36,0.25), 0 4px 20px rgba(234,179,8,0.4)"
                  : "0 0 30px rgba(129,140,248,0.5), 0 0 60px rgba(129,140,248,0.25), 0 4px 20px rgba(99,102,241,0.4)",
                border: isLight
                  ? "2px solid rgba(251,191,36,0.5)"
                  : "2px solid rgba(167,139,250,0.5)",
              }}
              title="切换主题"
            >
              {isLight ? (
                <Sun className="w-6 h-6 text-white drop-shadow-lg" />
              ) : (
                <Moon className="w-6 h-6 text-white drop-shadow-lg" />
              )}
            </button>

            {/* 品牌内容 */}
            <div className="relative z-10 animate-login-fade-up px-2">
              <div className="flex items-center gap-3 mb-8">
                <img src={BRAND.logo} alt={BRAND.name} className="w-10 h-10 rounded-xl shadow-lg shadow-brand-500/25" />
                <div
                  className="text-[26px] font-extrabold tracking-tight leading-none"
                  style={{ color: isLight ? "#0f172a" : "#ffffff" }}
                >
                  AgentWork
                </div>
              </div>

              <div
                className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium mb-6 ${
                  isLight
                    ? "bg-brand-50/80 text-brand-600 border border-brand-100"
                    : "bg-brand-500/10 text-brand-400 border border-brand-500/20"
                }`}
              >
                <Sparkles className="w-3.5 h-3.5" />
                <span>AI 原生智能工作空间</span>
              </div>

              <h1
                className={`text-3xl lg:text-4xl xl:text-5xl font-extrabold tracking-tight mb-6 ${
                  isLight ? "text-slate-900" : "text-white"
                }`}
                style={{ lineHeight: 1.15 }}
              >
                <div
                  className="login-title-glow"
                  style={{ animationDelay: "0s" }}
                >
                  打造数字化协作团队
                </div>
                <div
                  className="login-title-glow"
                  style={{ animationDelay: "0.7s" }}
                >
                  培养自进化AI员工
                </div>
              </h1>

              <p
                className={`text-base lg:text-lg leading-relaxed mb-10 max-w-md ${
                  isLight ? "text-slate-500" : "text-zinc-400"
                }`}
              >
                用对话即可为团队创建专属 AI 员工，自动分工、沉淀经验、持续进化。
                覆盖研发、运营、营销等多种业务场景，如同管理一支真实的数字化团队。
              </p>

              <div className="flex gap-3">
                {HIGHLIGHTS.map((item, idx) => {
                  const Icon = item.icon;
                  return (
                    <div
                      key={item.title}
                      className={`flex-1 p-3.5 rounded-2xl border transition-all ${
                        isLight
                          ? "bg-white/60 border-slate-200/70 hover:bg-white/90 hover:border-brand-200 shadow-sm backdrop-blur-sm"
                          : "bg-white/5 border-white/10 hover:bg-white/10 hover:border-brand-500/30 backdrop-blur-sm"
                      }`}
                      style={{ animationDelay: `${0.3 + idx * 0.08}s` }}
                    >
                      <div className="w-8 h-8 rounded-lg bg-brand-500/10 flex items-center justify-center mb-2">
                        <Icon className="w-4 h-4 text-brand-500" />
                      </div>
                      <div
                        className={`text-xs font-semibold mb-0.5 ${
                          isLight ? "text-slate-800" : "text-zinc-200"
                        }`}
                      >
                        {item.title}
                      </div>
                      <div
                        className={`text-[10px] leading-relaxed ${
                          isLight ? "text-slate-500" : "text-zinc-500"
                        }`}
                      >
                        {item.desc}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>

          <div className="hidden lg:block w-px self-stretch bg-gradient-to-b from-transparent via-brand-500/30 to-transparent" />

          {/* ===== 右侧：登录卡片 ===== */}
          <div
            className="w-full max-w-[380px] flex flex-col justify-center animate-login-scale-in"
            style={{ animationDelay: "0.15s" }}
          >
            <div
              className={`rounded-3xl border p-8 shadow-2xl backdrop-blur-xl ${
                isLight
                  ? "bg-white/70 border-white/60 shadow-slate-200/40"
                  : "bg-white/5 border-white/10 shadow-black/40"
              }`}
            >
              <div className="text-center mb-6">
                <h2
                  className={`text-xl font-bold ${
                    isLight ? "text-slate-900" : "text-white"
                  }`}
                >
                  云集智能体工作台
                </h2>
              </div>

              {status === "success" ? (
                <div className="aspect-square flex flex-col items-center justify-center gap-4">
                  <img src={BRAND.logo} alt={BRAND.name} className="w-20 h-20 rounded-full animate-login-pulse-glow" />
                  <div className="text-center">
                    <div
                      className={`text-lg font-semibold ${
                        isLight ? "text-slate-900" : "text-white"
                      }`}
                    >
                      {t("login.modal.successTitle")}
                    </div>
                    <div
                      className={
                        isLight ? "text-sm text-slate-500" : "text-sm text-zinc-500"
                      }
                    >
                      {t("login.modal.successDesc")}
                    </div>
                  </div>
                </div>
              ) : (
                <>
                  {/* 二维码提示 — 移到二维码上方 */}
                  <div className="text-center space-y-2 mb-5">
                    <div
                      className={`flex items-center justify-center gap-2 text-sm font-medium ${
                        isLight ? "text-slate-700" : "text-zinc-200"
                      }`}
                    >
                      <Smartphone className="w-4 h-4 text-brand-500" />
                      {loading ? t("login.loading") : t("login.modal.qrTitle")}
                    </div>
                    <p
                      className={`text-xs ${
                        isLight ? "text-slate-400" : "text-zinc-500"
                      }`}
                    >
                      {t("login.modal.qrDesc")}
                    </p>
                  </div>

                  {/* 二维码区域：200px 居中 */}
                  <div className="relative w-[200px] h-[200px] mx-auto rounded-2xl bg-white p-0 mb-5 overflow-hidden animate-login-scan-line shadow-inner border border-slate-200/40">
                    {!iframeLoaded && (
                      <div className="absolute inset-0 z-10 flex items-center justify-center bg-white">
                        <div className="flex flex-col items-center gap-2 text-zinc-500">
                          <Loader2 className="w-8 h-8 text-brand-500 animate-spin" />
                          <span className="text-xs">正在连接云集登录服务...</span>
                        </div>
                      </div>
                    )}

                    {loginData?.qr_image_url ? (
                      <img
                        src={loginData.qr_image_url}
                        alt="UM login QR"
                        className="w-full h-full object-contain"
                        onLoad={() => {
                          setIframeLoaded(true);
                        }}
                        onError={() => {
                          console.warn("[Login] qr_image failed, fallback to iframe");
                          setIframeLoaded(true);
                        }}
                      />
                    ) : (
                      <iframe
                        src={OFFICIAL_LOGIN_URL}
                        title="Yunji Login"
                        className="w-full h-full border-0"
                        onLoad={() => setIframeLoaded(true)}
                        onError={() => {
                          setIframeLoaded(true);
                          setError(
                            (prev) => prev ||
                              "官方登录页加载失败，请检查网络或点击刷新重试"
                          );
                        }}
                      />
                    )}
                  </div>

                  {error && (
                    <div className="mb-4 p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-xs text-red-500 text-center">
                      {error}
                    </div>
                  )}

                  {!loading && (
                    <>
                      <button
                        onClick={handleRefresh}
                        className={`w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs transition-colors ${
                          isLight
                            ? "bg-slate-100 hover:bg-slate-200 text-slate-600"
                            : "bg-white/10 hover:bg-white/15 text-zinc-300"
                        }`}
                      >
                        <RefreshCw className="w-3.5 h-3.5" />
                        {t("login.refreshQr")}
                      </button>

                      <button
                        onClick={() => openExternal(OFFICIAL_LOGIN_URL)}
                        className={`mt-2 w-full flex items-center justify-center gap-2 px-4 py-2 rounded-xl text-[10px] transition-colors ${
                          isLight
                            ? "bg-slate-50 hover:bg-slate-100 text-slate-500"
                            : "bg-white/5 hover:bg-white/10 text-zinc-500"
                        }`}
                        title="在系统浏览器中打开官方登录页"
                      >
                        <span>🌐 在浏览器中打开登录页</span>
                      </button>
                    </>
                  )}

                  {/* 调试：跳过登录直接进入工作台（不写入真实用户） */}
                  <button
                    onClick={() => {
                      storeUser({
                        nickname: "调试用户",
                        role: "游客",
                        social_uid: "guest-debug",
                      });
                      navigate("/", { replace: true });
                    }}
                    className={`mt-4 w-full flex items-center justify-center gap-2 px-4 py-2 rounded-xl text-[10px] transition-colors border ${
                      isLight
                        ? "bg-amber-50/80 hover:bg-amber-100/80 text-amber-700 border-amber-200/70"
                        : "bg-amber-500/5 hover:bg-amber-500/10 text-amber-400 border-amber-500/20"
                    }`}
                    title="跳过登录直接进入工作台（调试用）"
                  >
                    🐞 以游客身份进入（调试）
                  </button>
                </>
              )}
            </div>

            <div
              className={`mt-5 flex items-center justify-center gap-2 text-xs ${
                isLight ? "text-slate-400" : "text-zinc-500"
              }`}
            >
              <Shield className="w-3.5 h-3.5 text-brand-500/70" />
              <span>{t("login.modal.agreement")}</span>
            </div>
          </div>
        </div>
      </div>

      <div
        className={`absolute bottom-6 left-0 right-0 z-10 flex justify-center text-[11px] ${
          isLight ? "text-slate-400" : "text-zinc-600"
        }`}
      >
        <span>
          {BRAND.copyright}
        </span>
      </div>
    </div>
  );
}