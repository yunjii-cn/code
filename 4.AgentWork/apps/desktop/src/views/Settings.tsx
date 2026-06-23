import { useEffect, useState } from "react";
import {
  Palette,
  CheckCircle2,
  AlertCircle,
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
  Globe,
  Download,
  Stethoscope,
} from "lucide-react";
import { useTheme } from "@/lib/theme-store";
import { useI18n, SUPPORTED_LOCALES } from "@/i18n";
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
  exportLogs,
  type AppInfo,
  type SystemInfo,
  type GitStatusInfo,
  type AiConfigInfo,
} from "@/lib/tauri";

type GitMode = "stealth" | "sync" | "release";

export default function Settings() {
  const { t, displayLanguage, setDisplayLanguage } = useI18n();
  const { currentTheme, setTheme } = useTheme();
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [sysInfo, setSysInfo] = useState<SystemInfo | null>(null);
  const [repoPath, setRepoPath] = useState("");
  const [initStatus, setInitStatus] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [initMessage, setInitMessage] = useState("");
  const [currentBranchName, setCurrentBranchName] = useState<string | null>(null);
  const [gitMode, setGitModeState] = useState<GitMode>("stealth");
  const [gitStatus, setGitStatus] = useState<GitStatusInfo | null>(null);
  const [modeSwitching, setModeSwitching] = useState(false);
  const [modeMessage, setModeMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [pushing, setPushing] = useState(false);
  const [aiConfig, setAiConfigState] = useState<AiConfigInfo | null>(null);
  const [aiProvider, setAiProvider] = useState<"openai" | "ollama" | "mock" | "mg">("ollama");
  const [aiApiKey, setAiApiKey] = useState("");
  const [aiModel, setAiModel] = useState("");
  const [aiBaseUrl, setAiBaseUrl] = useState("");
  const [aiSaving, setAiSaving] = useState(false);
  const [aiMessage, setAiMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [exportingLogs, setExportingLogs] = useState(false);
  const [logMessage, setLogMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  const MODE_INFO: Record<GitMode, { labelKey: string; descKey: string; icon: typeof Eye }> = {
    stealth: {
      labelKey: "settings.gitMode.stealth",
      descKey: "settings.gitMode.stealthDesc",
      icon: Eye,
    },
    sync: {
      labelKey: "settings.gitMode.sync",
      descKey: "settings.gitMode.syncDesc",
      icon: RefreshCw,
    },
    release: {
      labelKey: "settings.gitMode.release",
      descKey: "settings.gitMode.releaseDesc",
      icon: Upload,
    },
  };

  useEffect(() => {
    Promise.all([getAppInfo(), getSystemInfo()])
      .then(([app, sys]) => {
        setAppInfo(app);
        setSysInfo(sys);
      })
      .catch(console.error);
    currentBranch()
      .then((name) => setCurrentBranchName(name))
      .catch(() => setCurrentBranchName(null));
    getGitMode()
      .then((m) => setGitModeState(m as GitMode))
      .catch(() => setGitModeState("stealth"));
    refreshGitStatus();
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
      setInitMessage(t("settings.repo.needPath"));
      return;
    }
    try {
      setInitStatus("loading");
      setInitMessage(t("settings.repo.initing"));
      const result = await initRepository(repoPath.trim());
      setInitStatus("success");
      setInitMessage(result);
      const branch = await currentBranch();
      setCurrentBranchName(branch);
      const mode = await getGitMode();
      setGitModeState(mode as GitMode);
      refreshGitStatus();
    } catch (err) {
      setInitStatus("error");
      setInitMessage(String(err));
    }
  }

  async function handleOpenFolder() {
    if (!repoPath.trim()) {
      return;
    }
    try {
      await openFolder(repoPath.trim());
    } catch (err) {
      setInitStatus("error");
      setInitMessage(t("settings.repo.openFail") + ": " + String(err));
    }
  }

  async function handleUseDefaultPath() {
    try {
      setInitStatus("loading");
      setInitMessage(t("settings.repo.switchingDefault"));
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
      setInitMessage(t("settings.repo.defaultFail") + ": " + String(err));
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

  async function handleExportLogs() {
    if (exportingLogs) return;
    try {
      setExportingLogs(true);
      setLogMessage(null);
      const filePath = await exportLogs();
      setLogMessage({ type: "success", text: t("settings.diagnostics.exportSuccess") + filePath });
      // 尝试打开所在文件夹
      try {
        const dir = filePath.replace(/[\\/][^\\/]+$/, "");
        await openFolder(dir);
      } catch {
        // 打开文件夹失败不影响主流程
      }
    } catch (err) {
      setLogMessage({ type: "error", text: t("settings.diagnostics.exportFail") + String(err) });
    } finally {
      setExportingLogs(false);
    }
  }

  return (
    <div className="flex flex-col h-full">
      <header className="px-4 sm:px-6 py-4 border-b border-zinc-800">
        <h1 className="text-lg font-semibold">{t("settings.title")}</h1>
      </header>

      <div className="flex-1 overflow-auto p-4 sm:p-6 space-y-6">
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">{t("settings.section.repo")}</h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-4">
            <div>
              <label className="text-sm text-zinc-300">{t("settings.repo.path")}</label>
              <div className="mt-1 flex gap-2">
                <input
                  type="text"
                  value={repoPath}
                  onChange={(e) => setRepoPath(e.target.value)}
                  placeholder={t("settings.repo.placeholder")}
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
                    {initStatus === "loading" ? t("settings.repo.initing") : t("settings.repo.init")}
                  </span>
                </button>
              </div>
              <div className="mt-2 flex gap-2">
                <button
                  onClick={handleOpenFolder}
                  disabled={!repoPath.trim()}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors text-xs text-zinc-300"
                >
                  <FolderTree className="w-3.5 h-3.5" />
                  <span>{t("settings.repo.openFolder")}</span>
                </button>
                <button
                  onClick={handleUseDefaultPath}
                  disabled={initStatus === "loading"}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors text-xs text-zinc-300"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  <span>{t("settings.repo.useDefault")}</span>
                </button>
              </div>
              <p className="mt-2 text-xs text-zinc-500">{t("settings.repo.defaultHint")}</p>
            </div>

            {currentBranchName && initStatus !== "loading" && (
              <div className="flex items-center gap-2 text-sm text-green-400">
                <CheckCircle2 className="w-4 h-4" />
                <span>{t("settings.repo.initializedOn")}{currentBranchName}</span>
              </div>
            )}

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

        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">{t("settings.section.gitMode")}</h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-4">
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
                        {t(info.labelKey)}
                      </span>
                      {isActive && <CheckCircle2 className="w-3 h-3 text-brand-400 ml-auto" />}
                    </div>
                    <p className="text-xs text-zinc-500 leading-relaxed">{t(info.descKey)}</p>
                  </button>
                );
              })}
            </div>

            {modeSwitching && (
              <div className="flex items-center gap-2 text-sm text-zinc-400">
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>{t("settings.gitMode.switching")}</span>
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

            {gitStatus && gitStatus.initialized && (
              <div className="bg-zinc-800/50 rounded-lg p-3 space-y-2">
                <div className="flex items-center gap-2 text-sm text-zinc-300 mb-2">
                  <GitBranch className="w-4 h-4" />
                  <span>{t("settings.git.statusPanel")}</span>
                </div>
                <Row label={t("settings.git.currentBranch")} value={gitStatus.current_branch || "(无)"} />
                <Row label={t("settings.git.lastCommit")} value={gitStatus.last_commit ? gitStatus.last_commit.slice(0, 8) : "(无)"} />
                <Row label={t("settings.git.remote")} value={gitStatus.remote_url || "(未配置)"} />
                <Row label={t("settings.git.unpushed")} value={t("settings.git.unpushedCount", { count: gitStatus.unpushed_commits })} />
                {gitMode !== "stealth" && (
                  <button
                    onClick={handlePush}
                    disabled={pushing || gitStatus.unpushed_commits === 0}
                    className="mt-2 flex items-center gap-2 px-3 py-1.5 rounded-lg bg-zinc-700 hover:bg-zinc-600 disabled:opacity-50 transition-colors text-sm"
                  >
                    {pushing ? <Loader2 className="w-3 h-3 animate-spin" /> : <Upload className="w-3 h-3" />}
                    <span>{pushing ? t("settings.git.pushing") : t("settings.git.push")}</span>
                  </button>
                )}
              </div>
            )}

            {gitMode === "stealth" && (
              <div className="flex items-start gap-2 text-xs text-zinc-500 bg-zinc-800/30 rounded p-3">
                <Eye className="w-3 h-3 mt-0.5 flex-shrink-0" />
                <span>{t("settings.gitMode.stealthHint")}</span>
              </div>
            )}
          </div>
        </section>

        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
            <Brain className="w-4 h-4" />
            {t("settings.section.aiConfig")}
          </h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-zinc-200">{t("settings.aiConfig.enable")}</p>
                <p className="text-xs text-zinc-500 mt-0.5">{t("settings.aiConfig.enableDesc")}</p>
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

            <div>
              <label className="text-sm text-zinc-300">{t("settings.aiConfig.provider")}</label>
              <div className="mt-1 grid grid-cols-2 gap-2">
                {([
                  { value: "ollama", labelKey: "settings.aiConfig.providers.ollama", icon: Cpu, descKey: "settings.aiConfig.providers.ollamaDesc" },
                  { value: "openai", labelKey: "settings.aiConfig.providers.openai", icon: Cloud, descKey: "settings.aiConfig.providers.openaiDesc" },
                  { value: "mg", labelKey: "settings.aiConfig.providers.mg", icon: Cloud, descKey: "settings.aiConfig.providers.mgDesc" },
                  { value: "mock", labelKey: "settings.aiConfig.providers.mock", icon: Brain, descKey: "settings.aiConfig.providers.mockDesc" },
                ] as const).map(({ value, labelKey, icon: Icon, descKey }) => (
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
                        {t(labelKey)}
                      </span>
                    </div>
                    <p className="text-xs text-zinc-500">{t(descKey)}</p>
                  </button>
                ))}
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div>
                <label className="text-sm text-zinc-300">{t("settings.aiConfig.baseUrl")}</label>
                <input
                  type="text"
                  value={aiBaseUrl}
                  onChange={(e) => setAiBaseUrl(e.target.value)}
                  placeholder={
                    aiProvider === "openai"
                      ? t("settings.aiConfig.baseUrl.openai")
                      : aiProvider === "ollama"
                      ? t("settings.aiConfig.baseUrl.ollama")
                      : t("settings.aiConfig.baseUrl.mock")
                  }
                  className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
              </div>
              <div>
                <label className="text-sm text-zinc-300">{t("settings.aiConfig.model")}</label>
                <input
                  type="text"
                  value={aiModel}
                  onChange={(e) => setAiModel(e.target.value)}
                  placeholder={
                    aiProvider === "openai"
                      ? t("settings.aiConfig.model.openai")
                      : aiProvider === "ollama"
                      ? t("settings.aiConfig.model.ollama")
                      : t("settings.aiConfig.model.mock")
                  }
                  className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
              </div>
            </div>

            {aiProvider === "openai" && (
              <div>
                <label className="text-sm text-zinc-300">{t("settings.aiConfig.apiKey")}</label>
                <input
                  type="password"
                  value={aiApiKey}
                  onChange={(e) => setAiApiKey(e.target.value)}
                  placeholder={t("settings.aiConfig.apiKeyPlaceholder")}
                  className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
                <p className="text-xs text-zinc-500 mt-1">{t("settings.aiConfig.apiKeyHint")}</p>
              </div>
            )}

            <div className="flex items-center gap-3">
              <button
                onClick={handleSaveAiConfig}
                disabled={aiSaving}
                className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-sm"
              >
                {aiSaving ? <Loader2 className="w-4 h-4 animate-spin" /> : <CheckCircle2 className="w-4 h-4" />}
                <span>{aiSaving ? t("settings.aiConfig.saving") : t("settings.aiConfig.save")}</span>
              </button>
              {aiConfig && (
                <span className={`text-xs ${aiConfig.enabled ? "text-green-400" : "text-zinc-500"}`}>
                  {aiConfig.enabled ? t("settings.aiConfig.statusEnabled") : t("settings.aiConfig.statusDisabled")} · {aiConfig.provider}
                </span>
              )}
            </div>

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

            <div className="text-xs text-zinc-500 bg-zinc-800/30 rounded p-3 space-y-1">
              <p>{t("settings.aiConfig.noteOllama")}</p>
              <p>{t("settings.aiConfig.noteOpenai")}</p>
              <p>{t("settings.aiConfig.noteMock")}</p>
            </div>
          </div>
        </section>

        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
            <Palette className="w-4 h-4" />
            {t("settings.section.theme")}
          </h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
            <p className="text-xs text-zinc-500">{t("theme.desc")}</p>
            <div className="grid grid-cols-2 gap-2">
              {([
                { value: "dark" as const, labelKey: "theme.dark" },
                { value: "light" as const, labelKey: "theme.light" },
              ]).map((opt) => (
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
                    <span
                      className={`text-sm font-medium ${
                        currentTheme === opt.value ? "text-brand-300" : "text-zinc-200"
                      }`}
                    >
                      {t(opt.labelKey)}
                    </span>
                    {currentTheme === opt.value && <CheckCircle2 className="w-4 h-4 text-brand-400" />}
                  </div>
                </button>
              ))}
            </div>
          </div>
        </section>

        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
            <Globe className="w-4 h-4" />
            {t("settings.section.language")}
          </h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
            <p className="text-xs text-zinc-500">{t("language.desc")}</p>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
              {SUPPORTED_LOCALES.map((lang) => (
                <button
                  key={lang}
                  onClick={() => setDisplayLanguage(lang)}
                  className={`p-3 rounded-lg border text-left transition-all ${
                    displayLanguage === lang
                      ? "border-brand-500 bg-brand-950/30"
                      : "border-zinc-700 bg-zinc-800/50 hover:border-zinc-600"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span
                      className={`text-sm font-medium ${
                        displayLanguage === lang ? "text-brand-300" : "text-zinc-200"
                      }`}
                    >
                      {t(`language.${lang}` as any)}
                    </span>
                    {displayLanguage === lang && <CheckCircle2 className="w-4 h-4 text-brand-400" />}
                  </div>
                </button>
              ))}
            </div>
          </div>
        </section>

        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">{t("settings.section.appInfo")}</h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-2">
            {appInfo ? (
              <>
                <Row label={t("settings.appInfo.name")} value={appInfo.name} />
                <Row label={t("settings.appInfo.version")} value={appInfo.version} />
                <Row label={t("settings.appInfo.desc")} value={appInfo.description} />
              </>
            ) : (
              <p className="text-sm text-zinc-500">{t("common.loading")}</p>
            )}
          </div>
        </section>

        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">{t("settings.section.systemInfo")}</h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-2">
            {sysInfo ? (
              <>
                <Row label={t("settings.systemInfo.os")} value={sysInfo.os} />
                <Row label={t("settings.systemInfo.arch")} value={sysInfo.arch} />
                <Row label={t("settings.systemInfo.cpu")} value={String(sysInfo.cpu_count)} />
              </>
            ) : (
              <p className="text-sm text-zinc-500">{t("common.loading")}</p>
            )}
          </div>
        </section>

        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
            <Stethoscope className="w-4 h-4" />
            {t("settings.section.diagnostics")}
          </h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
            <p className="text-xs text-zinc-500 leading-relaxed">{t("settings.diagnostics.desc")}</p>
            <button
              onClick={handleExportLogs}
              disabled={exportingLogs}
              className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-sm"
            >
              {exportingLogs ? <Loader2 className="w-4 h-4 animate-spin" /> : <Download className="w-4 h-4" />}
              <span>{exportingLogs ? t("settings.diagnostics.exporting") : t("settings.diagnostics.exportLogs")}</span>
            </button>
            {logMessage && (
              <div
                className={`flex items-start gap-2 text-sm rounded p-3 ${
                  logMessage.type === "success"
                    ? "text-green-400 bg-green-950/30"
                    : "text-red-400 bg-red-950/30"
                }`}
              >
                {logMessage.type === "success" ? (
                  <CheckCircle2 className="w-4 h-4 mt-0.5 flex-shrink-0" />
                ) : (
                  <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                )}
                <span className="break-all">{logMessage.text}</span>
              </div>
            )}
          </div>
        </section>

        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">{t("settings.section.about")}</h2>
          <div className="bg-zinc-900 rounded-lg p-4 text-center text-xs text-zinc-500 space-y-1">
            <p>{t("settings.footer.copyright")}</p>
            <p>{t("settings.footer.license")}</p>
          </div>
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