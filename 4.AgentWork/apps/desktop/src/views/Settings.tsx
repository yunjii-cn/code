import { useEffect, useState } from "react";
import { Palette, CheckCircle2, AlertCircle } from "lucide-react";
import { useTheme } from "@/lib/theme-store";
import {
  FolderOpen,
  Loader2,
  Eye,
  RefreshCw,
  Upload,
  GitBranch,
  Brain,
  Cpu,
  Cloud,
  FolderTree,
  RotateCcw,
} from "lucide-react";
import {
  getAppInfo,
  getSystemInfo,
  initRepository,
  currentBranch,
  getGitMode,
  setGitMode,
  getGitStatus,
  gitPush,
  getAiConfig,
  setAiConfig,
  setAiEnabled,
  getDefaultRepoPath,
  openFolder,
  type AppInfo,
  type SystemInfo,
  type GitStatusInfo,
  type AiConfigInfo,
} from "@/lib/tauri";

type GitMode = "stealth" | "sync" | "release";

const MODE_INFO: Record<GitMode, { label: string; desc: string; icon: typeof Eye }> = {
  stealth: {
    label: "隐身模式",
    desc: "纯本地 TimeFlow，不碰 git，代码永不上云。适合政府/金融/隐私敏感企业。",
    icon: Eye,
  },
  sync: {
    label: "同步模式",
    desc: "每个 TimeFlow 快照自动镜像为 git commit，可选自动 push。适合开源项目/个人开发者。",
    icon: RefreshCw,
  },
  release: {
    label: "发布模式",
    desc: "只把正式版本（Release 类型快照）推到 git，WIP 留本地。适合商业软件/团队开发。",
    icon: Upload,
  },
};

export default function Settings() {
  const { currentTheme, setTheme } = useTheme();
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [sysInfo, setSysInfo] = useState<SystemInfo | null>(null);
  const [repoPath, setRepoPath] = useState("");
  const [initStatus, setInitStatus] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [initMessage, setInitMessage] = useState("");
  const [currentBranchName, setCurrentBranchName] = useState<string | null>(null);

  // Git 模式相关状态
  const [gitMode, setGitModeState] = useState<GitMode>("stealth");
  const [gitStatus, setGitStatus] = useState<GitStatusInfo | null>(null);
  const [modeSwitching, setModeSwitching] = useState(false);
  const [modeMessage, setModeMessage] = useState<{ type: "success" | "error"; text: string } | null>(
    null
  );
  const [pushing, setPushing] = useState(false);

  // AI 配置相关状态
  const [aiConfig, setAiConfigState] = useState<AiConfigInfo | null>(null);
  const [aiProvider, setAiProvider] = useState<"openai" | "ollama" | "mock" | "mg">("ollama");
  const [aiApiKey, setAiApiKey] = useState("");
  const [aiModel, setAiModel] = useState("");
  const [aiBaseUrl, setAiBaseUrl] = useState("");
  const [aiSaving, setAiSaving] = useState(false);
  const [aiMessage, setAiMessage] = useState<{ type: "success" | "error"; text: string } | null>(
    null
  );

  useEffect(() => {
    Promise.all([getAppInfo(), getSystemInfo()])
      .then(([app, sys]) => {
        setAppInfo(app);
        setSysInfo(sys);
      })
      .catch(console.error);
    // 尝试获取当前分支（判断是否已初始化）
    currentBranch()
      .then((name) => setCurrentBranchName(name))
      .catch(() => setCurrentBranchName(null));
    // 加载 Git 模式
    getGitMode()
      .then((m) => setGitModeState(m as GitMode))
      .catch(() => setGitModeState("stealth"));
    refreshGitStatus();
    // 加载 AI 配置
    refreshAiConfig();
  }, []);

  function refreshGitStatus() {
    getGitStatus()
      .then((s) => setGitStatus(s))
      .catch(() => setGitStatus(null));
  }

  function refreshAiConfig() {
    getAiConfig()
      .then((cfg) => {
        setAiConfigState(cfg);
        setAiProvider(cfg.provider);
        setAiModel(cfg.model);
        setAiBaseUrl(cfg.base_url);
        if (cfg.has_api_key) {
          setAiApiKey("(已配置)");
        }
      })
      .catch(() => setAiConfigState(null));
  }

  async function handleSaveAiConfig() {
    if (aiSaving) return;
    try {
      setAiSaving(true);
      setAiMessage(null);
      const result = await setAiConfig({
        provider: aiProvider,
        base_url: aiBaseUrl || undefined,
        api_key: aiApiKey === "(已配置)" ? undefined : aiApiKey || undefined,
        model: aiModel || undefined,
      });
      setAiMessage({ type: "success", text: result });
      refreshAiConfig();
    } catch (err) {
      setAiMessage({ type: "error", text: String(err) });
    } finally {
      setAiSaving(false);
    }
  }

  async function handleToggleAi(enabled: boolean) {
    try {
      await setAiEnabled(enabled);
      refreshAiConfig();
    } catch (err) {
      setAiMessage({ type: "error", text: String(err) });
    }
  }

  async function handleInit() {
    if (!repoPath.trim()) {
      setInitStatus("error");
      setInitMessage("请输入仓库路径");
      return;
    }
    try {
      setInitStatus("loading");
      setInitMessage("正在初始化...");
      const result = await initRepository(repoPath.trim());
      setInitStatus("success");
      setInitMessage(result);
      const branch = await currentBranch();
      setCurrentBranchName(branch);
      // 重新加载 git 模式和状态
      const mode = await getGitMode();
      setGitModeState(mode as GitMode);
      refreshGitStatus();
    } catch (err) {
      setInitStatus("error");
      setInitMessage(String(err));
    }
  }

  // 在系统资源管理器中打开当前仓库文件夹
  async function handleOpenFolder() {
    if (!repoPath.trim()) {
      setInitStatus("error");
      setInitMessage("请先输入或初始化仓库路径");
      return;
    }
    try {
      await openFolder(repoPath.trim());
    } catch (err) {
      setInitStatus("error");
      setInitMessage(`打开文件夹失败: ${String(err)}`);
    }
  }

  // 使用默认仓库路径（<app_dir>/data/workspace）并自动初始化
  async function handleUseDefaultPath() {
    try {
      setInitStatus("loading");
      setInitMessage("正在切换到默认仓库...");
      const defaultPath = await getDefaultRepoPath();
      setRepoPath(defaultPath);
      const result = await initRepository(defaultPath);
      setInitStatus("success");
      setInitMessage(result);
      const branch = await currentBranch();
      setCurrentBranchName(branch);
      const mode = await getGitMode();
      setGitModeState(mode as GitMode);
      refreshGitStatus();
    } catch (err) {
      setInitStatus("error");
      setInitMessage(`使用默认路径失败: ${String(err)}`);
    }
  }

  async function handleModeSwitch(newMode: GitMode) {
    if (newMode === gitMode || modeSwitching) return;
    try {
      setModeSwitching(true);
      setModeMessage(null);
      const result = await setGitMode(newMode);
      setGitModeState(newMode);
      setModeMessage({ type: "success", text: result });
      refreshGitStatus();
    } catch (err) {
      setModeMessage({ type: "error", text: String(err) });
    } finally {
      setModeSwitching(false);
    }
  }

  async function handlePush() {
    if (pushing) return;
    try {
      setPushing(true);
      const result = await gitPush();
      setModeMessage({ type: "success", text: result });
      refreshGitStatus();
    } catch (err) {
      setModeMessage({ type: "error", text: String(err) });
    } finally {
      setPushing(false);
    }
  }

  return (
    <div className="flex flex-col h-full">
      <header className="px-6 py-4 border-b border-zinc-800">
        <h1 className="text-lg font-semibold">设置</h1>
      </header>

      <div className="flex-1 overflow-auto p-6 space-y-6">
        {/* 仓库初始化 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">TimeFlow 仓库</h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-4">
            <div>
              <label className="text-sm text-zinc-300">仓库路径</label>
              <div className="mt-1 flex gap-2">
                <input
                  type="text"
                  value={repoPath}
                  onChange={(e) => setRepoPath(e.target.value)}
                  placeholder="例如：D:\projects\my-app 或 /home/user/project"
                  className="flex-1 bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
                <button
                  onClick={handleInit}
                  disabled={initStatus === "loading"}
                  className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors"
                >
                  {initStatus === "loading" ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <FolderOpen className="w-4 h-4" />
                  )}
                  <span className="text-sm">
                    {initStatus === "loading" ? "初始化中..." : "初始化仓库"}
                  </span>
                </button>
              </div>
              {/* 辅助按钮：打开文件夹 + 使用默认路径 */}
              <div className="mt-2 flex gap-2">
                <button
                  onClick={handleOpenFolder}
                  disabled={!repoPath.trim()}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors text-xs text-zinc-300"
                  title="在系统资源管理器中打开当前仓库文件夹"
                >
                  <FolderTree className="w-3.5 h-3.5" />
                  <span>打开文件夹</span>
                </button>
                <button
                  onClick={handleUseDefaultPath}
                  disabled={initStatus === "loading"}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors text-xs text-zinc-300"
                  title="使用软件目录下的 data/workspace 作为默认仓库并自动初始化"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  <span>使用默认路径</span>
                </button>
              </div>
              <p className="mt-2 text-xs text-zinc-500">
                默认路径为软件目录下的 <code className="px-1 py-0.5 bg-zinc-800 rounded text-zinc-400">data/workspace</code>，首次启动已自动初始化。可随时更改路径并重新初始化。
              </p>
            </div>

            {/* 当前状态 */}
            {currentBranchName && initStatus !== "loading" && (
              <div className="flex items-center gap-2 text-sm text-green-400">
                <CheckCircle2 className="w-4 h-4" />
                <span>已初始化，当前分支：{currentBranchName}</span>
              </div>
            )}

            {/* 初始化反馈 */}
            {initStatus === "success" && (
              <div className="flex items-start gap-2 text-sm text-green-400 bg-green-950/30 rounded p-3">
                <CheckCircle2 className="w-4 h-4 mt-0.5 flex-shrink-0" />
                <span className="break-all">{initMessage}</span>
              </div>
            )}
            {initStatus === "error" && (
              <div className="flex items-start gap-2 text-sm text-red-400 bg-red-950/30 rounded p-3">
                <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                <span className="break-all">{initMessage}</span>
              </div>
            )}
          </div>
        </section>

        {/* Git 兼容模式 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">Git 兼容模式</h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-4">
            {/* 模式选择卡片 */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              {(Object.keys(MODE_INFO) as GitMode[]).map((mode) => {
                const info = MODE_INFO[mode];
                const Icon = info.icon;
                const isActive = gitMode === mode;
                return (
                  <button
                    key={mode}
                    onClick={() => handleModeSwitch(mode)}
                    disabled={modeSwitching}
                    className={`text-left p-3 rounded-lg border transition-all ${
                      isActive
                        ? "border-brand-500 bg-brand-950/30"
                        : "border-zinc-700 bg-zinc-800/50 hover:border-zinc-600"
                    } ${modeSwitching ? "opacity-50 cursor-not-allowed" : ""}`}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <Icon className={`w-4 h-4 ${isActive ? "text-brand-400" : "text-zinc-400"}`} />
                      <span className={`text-sm font-medium ${isActive ? "text-brand-300" : "text-zinc-200"}`}>
                        {info.label}
                      </span>
                      {isActive && <CheckCircle2 className="w-3 h-3 text-brand-400 ml-auto" />}
                    </div>
                    <p className="text-xs text-zinc-500 leading-relaxed">{info.desc}</p>
                  </button>
                );
              })}
            </div>

            {/* 模式切换反馈 */}
            {modeSwitching && (
              <div className="flex items-center gap-2 text-sm text-zinc-400">
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>正在切换模式...</span>
              </div>
            )}
            {modeMessage && (
              <div
                className={`flex items-start gap-2 text-sm rounded p-3 ${
                  modeMessage.type === "success"
                    ? "text-green-400 bg-green-950/30"
                    : "text-red-400 bg-red-950/30"
                }`}
              >
                {modeMessage.type === "success" ? (
                  <CheckCircle2 className="w-4 h-4 mt-0.5 flex-shrink-0" />
                ) : (
                  <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                )}
                <span className="break-all">{modeMessage.text}</span>
              </div>
            )}

            {/* Git 状态信息 */}
            {gitStatus && gitStatus.initialized && (
              <div className="bg-zinc-800/50 rounded-lg p-3 space-y-2">
                <div className="flex items-center gap-2 text-sm text-zinc-300 mb-2">
                  <GitBranch className="w-4 h-4" />
                  <span>Git 仓库状态</span>
                </div>
                <Row label="当前分支" value={gitStatus.current_branch || "(无)"} />
                <Row label="最近 commit" value={gitStatus.last_commit ? gitStatus.last_commit.slice(0, 8) : "(无)"} />
                <Row label="远程仓库" value={gitStatus.remote_url || "(未配置)"} />
                <Row label="待推送" value={`${gitStatus.unpushed_commits} 个 commit`} />

                {/* 推送按钮 */}
                {gitMode !== "stealth" && (
                  <button
                    onClick={handlePush}
                    disabled={pushing || gitStatus.unpushed_commits === 0}
                    className="mt-2 flex items-center gap-2 px-3 py-1.5 rounded-lg bg-zinc-700 hover:bg-zinc-600 disabled:opacity-50 transition-colors text-sm"
                  >
                    {pushing ? (
                      <Loader2 className="w-3 h-3 animate-spin" />
                    ) : (
                      <Upload className="w-3 h-3" />
                    )}
                    <span>{pushing ? "推送中..." : "推送到远程"}</span>
                  </button>
                )}
              </div>
            )}

            {/* 隐身模式提示 */}
            {gitMode === "stealth" && (
              <div className="flex items-start gap-2 text-xs text-zinc-500 bg-zinc-800/30 rounded p-3">
                <Eye className="w-3 h-3 mt-0.5 flex-shrink-0" />
                <span>
                  隐身模式下不会创建 .git 目录，所有版本数据仅保存在 .yunji/timeflow/ 中。
                  切换到同步或发布模式时会自动初始化 Git 仓库。
                </span>
              </div>
            )}
          </div>
        </section>

        {/* AI 配置 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
            <Brain className="w-4 h-4" />
            AI 自动生成 commit msg
          </h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-4">
            {/* 启用开关 */}
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-zinc-200">启用 AI 自动生成</p>
                <p className="text-xs text-zinc-500 mt-0.5">
                  快照后自动调用 LLM 生成 Conventional Commits 格式的提交信息
                </p>
              </div>
              <button
                onClick={() => handleToggleAi(!aiConfig?.enabled)}
                disabled={!aiConfig}
                className={`relative w-11 h-6 rounded-full transition-colors ${
                  aiConfig?.enabled ? "bg-brand-600" : "bg-zinc-700"
                } disabled:opacity-50`}
              >
                <span
                  className={`absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white transition-transform ${
                    aiConfig?.enabled ? "translate-x-5" : ""
                  }`}
                />
              </button>
            </div>

            {/* LLM 提供方选择 */}
            <div>
              <label className="text-sm text-zinc-300">LLM 提供方</label>
              <div className="mt-1 grid grid-cols-2 gap-2">
                {([
                  { value: "ollama", label: "Ollama 本地", icon: Cpu, desc: "不上云" },
                  { value: "openai", label: "OpenAI 云端", icon: Cloud, desc: "更快更准" },
                  { value: "mg", label: "云集网关", icon: Cloud, desc: "UM 计费" },
                  { value: "mock", label: "Mock 测试", icon: Brain, desc: "不调 API" },
                ] as const).map(({ value, label, icon: Icon, desc }) => (
                  <button
                    key={value}
                    onClick={() => setAiProvider(value)}
                    className={`p-2 rounded-lg border text-left transition-all ${
                      aiProvider === value
                        ? "border-brand-500 bg-brand-950/30"
                        : "border-zinc-700 bg-zinc-800/50 hover:border-zinc-600"
                    }`}
                  >
                    <div className="flex items-center gap-1.5 mb-0.5">
                      <Icon className={`w-3 h-3 ${aiProvider === value ? "text-brand-400" : "text-zinc-400"}`} />
                      <span className={`text-xs font-medium ${aiProvider === value ? "text-brand-300" : "text-zinc-200"}`}>
                        {label}
                      </span>
                    </div>
                    <p className="text-xs text-zinc-500">{desc}</p>
                  </button>
                ))}
              </div>
            </div>

            {/* API 配置 */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div>
                <label className="text-sm text-zinc-300">Base URL</label>
                <input
                  type="text"
                  value={aiBaseUrl}
                  onChange={(e) => setAiBaseUrl(e.target.value)}
                  placeholder={
                    aiProvider === "openai"
                      ? "https://api.openai.com/v1"
                      : aiProvider === "ollama"
                      ? "http://localhost:11434"
                      : "mock://localhost"
                  }
                  className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
              </div>
              <div>
                <label className="text-sm text-zinc-300">模型名</label>
                <input
                  type="text"
                  value={aiModel}
                  onChange={(e) => setAiModel(e.target.value)}
                  placeholder={
                    aiProvider === "openai"
                      ? "gpt-4o-mini"
                      : aiProvider === "ollama"
                      ? "qwen2.5-coder:7b"
                      : "mock-model"
                  }
                  className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
              </div>
            </div>

            {/* API Key（仅 OpenAI 需要） */}
            {aiProvider === "openai" && (
              <div>
                <label className="text-sm text-zinc-300">API Key</label>
                <input
                  type="password"
                  value={aiApiKey}
                  onChange={(e) => setAiApiKey(e.target.value)}
                  placeholder="sk-..."
                  className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
                <p className="text-xs text-zinc-500 mt-1">
                  配置自动持久化，重启后保留
                </p>
              </div>
            )}

            {/* 保存按钮 */}
            <div className="flex items-center gap-3">
              <button
                onClick={handleSaveAiConfig}
                disabled={aiSaving}
                className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-sm"
              >
                {aiSaving ? <Loader2 className="w-4 h-4 animate-spin" /> : <CheckCircle2 className="w-4 h-4" />}
                <span>{aiSaving ? "保存中..." : "保存配置"}</span>
              </button>

              {/* 当前状态 */}
              {aiConfig && (
                <span className={`text-xs ${aiConfig.enabled ? "text-green-400" : "text-zinc-500"}`}>
                  {aiConfig.enabled ? "● 已启用" : "○ 已禁用"} · {aiConfig.provider}
                </span>
              )}
            </div>

            {/* 反馈消息 */}
            {aiMessage && (
              <div
                className={`flex items-start gap-2 text-sm rounded p-3 ${
                  aiMessage.type === "success"
                    ? "text-green-400 bg-green-950/30"
                    : "text-red-400 bg-red-950/30"
                }`}
              >
                {aiMessage.type === "success" ? (
                  <CheckCircle2 className="w-4 h-4 mt-0.5 flex-shrink-0" />
                ) : (
                  <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                )}
                <span className="break-all">{aiMessage.text}</span>
              </div>
            )}

            {/* 说明 */}
            <div className="text-xs text-zinc-500 bg-zinc-800/30 rounded p-3 space-y-1">
              <p>
                <strong className="text-zinc-400">Ollama 本地模式</strong>：代码不上云，适合隐私敏感场景。需先安装 Ollama 并拉取模型。
              </p>
              <p>
                <strong className="text-zinc-400">OpenAI 云端模式</strong>：更快更准，但代码 diff 会上云。适合开源项目。
              </p>
              <p>
                <strong className="text-zinc-400">Mock 模式</strong>：返回固定响应，仅用于测试。
              </p>
            </div>
          </div>
        </section>

        {/* 主题外观 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
            <Palette className="w-4 h-4" />
            主题外观
          </h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
            <div className="grid grid-cols-2 gap-2">
              {([
                { value: "dark", label: "暗黑" },
                { value: "light", label: "明亮" },
              ] as const).map((opt) => (
                <button
                  key={opt.value}
                  onClick={() => setTheme(opt.value)}
                  className={`p-3 rounded-lg border text-left transition-all ${
                    currentTheme === opt.value
                      ? "border-brand-500 bg-brand-950/30"
                      : "border-zinc-700 bg-zinc-800/50 hover:border-zinc-600"
                  }`}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className={`text-sm font-medium ${currentTheme === opt.value ? "text-brand-300" : "text-zinc-200"}`}>
                      {opt.label}
                    </span>
                    {currentTheme === opt.value && <CheckCircle2 className="w-4 h-4 text-brand-400" />}
                  </div>
                </button>
              ))}
            </div>
          </div>
        </section>

        {/* 应用信息 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">应用信息</h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-2">
            {appInfo ? (
              <>
                <Row label="名称" value={appInfo.name} />
                <Row label="版本" value={appInfo.version} />
                <Row label="描述" value={appInfo.description} />
              </>
            ) : (
              <p className="text-sm text-zinc-500">加载中...</p>
            )}
          </div>
        </section>

        {/* 系统信息 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">系统信息</h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-2">
            {sysInfo ? (
              <>
                <Row label="操作系统" value={sysInfo.os} />
                <Row label="架构" value={sysInfo.arch} />
                <Row label="CPU 核心数" value={String(sysInfo.cpu_count)} />
              </>
            ) : (
              <p className="text-sm text-zinc-500">加载中...</p>
            )}
          </div>
        </section>

        {/* 关于 */}
        <section className="text-center text-xs text-zinc-600 pt-8">
          <p>云集智能体工作台 © 2026 Yunji AI</p>
          <p className="mt-1">Apache 2.0 License</p>
        </section>
      </div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between text-sm">
      <span className="text-zinc-500">{label}</span>
      <span className="text-zinc-200">{value}</span>
    </div>
  );
}
