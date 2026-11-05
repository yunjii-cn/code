// 软件更新页面
// 多源竞速检查 + 版本列表 + 下载进度 + 切换提示
// 移植自云集智能视频创意站的更新机制

import { useState, useEffect, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import {
  RefreshCw,
  Download,
  CheckCircle2,
  AlertCircle,
  ArrowLeft,
  Loader2,
  FolderOpen,
  Zap,
} from "lucide-react";
import { clsx } from "clsx";
import { useI18n } from "@/i18n";
import {
  checkForUpdates,
  downloadVersion,
  switchVersion,
  openVerDir,
  type UpdateCheckResult,
  type VersionInfo,
  type DownloadProgress,
} from "@/lib/tauri";
import { listen } from "@tauri-apps/api/event";

export default function UpdateManager() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<UpdateCheckResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [switching, setSwitching] = useState<string | null>(null);
  const [switchMessage, setSwitchMessage] = useState<string | null>(null);

  // 监听下载进度事件
  useEffect(() => {
    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      setDownloadProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleCheck = useCallback(async () => {
    setLoading(true);
    setError(null);
    setSwitchMessage(null);
    try {
      const res = await checkForUpdates("auto");
      setResult(res);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const handleDownload = useCallback(
    async (version: VersionInfo) => {
      if (!version.download_url) {
        setError(t("update.updateFailed").replace("{error}", "无下载链接"));
        return;
      }
      setDownloading(version.version);
      setDownloadProgress(null);
      setError(null);
      try {
        await downloadVersion(version.version, version.download_url);
        // 刷新本地版本列表
        const res = await checkForUpdates("auto");
        setResult(res);
      } catch (err) {
        setError(t("update.updateFailed").replace("{error}", String(err)));
      } finally {
        setDownloading(null);
        setDownloadProgress(null);
      }
    },
    [t]
  );

  const handleSwitch = useCallback(
    async (version: string) => {
      setSwitching(version);
      setError(null);
      try {
        const msg = await switchVersion(version);
        setSwitchMessage(msg);
      } catch (err) {
        setError(t("update.updateFailed").replace("{error}", String(err)));
      } finally {
        setSwitching(null);
      }
    },
    [t]
  );

  // 判断版本状态
  const getVersionStatus = (
    version: VersionInfo
  ): "current" | "downloaded" | "new" | "available" => {
    if (!result) return "available";
    if (version.version === result.current_version) return "current";
    if (result.local_versions.includes(version.version)) return "downloaded";
    if (result.has_update && version.version === result.latest_version)
      return "new";
    return "available";
  };

  return (
    <div className="flex flex-col h-full">
      {/* 头部 */}
      <header className="px-4 sm:px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate("/settings")}
            className="p-1.5 rounded-lg hover:bg-zinc-800 text-zinc-400"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <h1 className="text-lg font-semibold">{t("update.title")}</h1>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => openVerDir()}
            className="p-1.5 rounded-lg hover:bg-zinc-800 text-zinc-400"
            title="ver/"
          >
            <FolderOpen className="w-4 h-4" />
          </button>
          <button
            onClick={handleCheck}
            disabled={loading}
            className={clsx(
              "flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors",
              "bg-brand-600 hover:bg-brand-500 text-white",
              loading && "opacity-50 cursor-wait"
            )}
          >
            {loading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )}
            {loading ? t("update.checking") : t("update.checkUpdate")}
          </button>
        </div>
      </header>

      {/* 内容区 */}
      <div className="flex-1 overflow-auto p-4 sm:p-6 space-y-4">
        {/* 错误提示 */}
        {error && (
          <div className="flex items-center gap-2 px-4 py-3 rounded-lg bg-red-950/30 border border-red-900/50 text-red-400 text-sm">
            <AlertCircle className="w-4 h-4 flex-shrink-0" />
            <span>{error}</span>
          </div>
        )}

        {/* 切换成功提示 */}
        {switchMessage && (
          <div className="flex items-center gap-2 px-4 py-3 rounded-lg bg-green-950/30 border border-green-900/50 text-green-400 text-sm">
            <CheckCircle2 className="w-4 h-4 flex-shrink-0" />
            <span>{switchMessage}</span>
          </div>
        )}

        {/* 当前版本信息 */}
        {result && (
          <div className="p-4 rounded-lg bg-zinc-900 border border-zinc-800">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm text-zinc-500">
                {t("update.currentVersion")}
              </span>
              <span className="text-sm font-mono font-medium text-zinc-200">
                v{result.current_version}
              </span>
            </div>
            {result.latest_version && (
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm text-zinc-500">
                  {t("update.latestVersion")}
                </span>
                <span
                  className={clsx(
                    "text-sm font-mono font-medium",
                    result.has_update ? "text-brand-400" : "text-green-400"
                  )}
                >
                  v{result.latest_version}
                  {result.has_update && (
                    <span className="ml-2 text-xs px-1.5 py-0.5 rounded bg-brand-600/20 text-brand-400">
                      {t("update.new")}
                    </span>
                  )}
                </span>
              </div>
            )}
            <div className="flex items-center justify-between">
              <span className="text-sm text-zinc-500">
                {t("update.source")}
              </span>
              <span className="text-sm text-zinc-400">{result.source}</span>
            </div>
            {result.has_update && (
              <div className="mt-3 px-3 py-2 rounded bg-brand-600/10 border border-brand-600/30 text-brand-400 text-sm">
                {t("update.hasUpdate").replace(
                  "{version}",
                  result.latest_version!
                )}
              </div>
            )}
            {!result.has_update && result.latest_version && (
              <div className="mt-3 px-3 py-2 rounded bg-green-950/30 border border-green-900/50 text-green-400 text-sm">
                {t("update.upToDate")}
              </div>
            )}
          </div>
        )}

        {/* 下载进度条 */}
        {downloading && downloadProgress && (
          <div className="p-4 rounded-lg bg-zinc-900 border border-zinc-800">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm text-zinc-300">
                {t("update.downloadProgress").replace(
                  "{percent}",
                  String(downloadProgress.percent)
                )}
              </span>
              <span className="text-xs text-zinc-500 font-mono">
                {(downloadProgress.downloaded / 1024 / 1024).toFixed(1)} /{" "}
                {(downloadProgress.total / 1024 / 1024).toFixed(1)} MB
              </span>
            </div>
            <div className="h-2 rounded-full bg-zinc-800 overflow-hidden">
              <div
                className="h-full bg-brand-600 transition-all duration-300"
                style={{ width: `${downloadProgress.percent}%` }}
              />
            </div>
          </div>
        )}

        {/* 版本列表 */}
        {result && result.versions.length > 0 && (
          <div className="space-y-2">
            <h2 className="text-sm font-semibold text-zinc-300">
              {t("update.versions")} ({result.versions.length})
            </h2>
            {result.versions.map((version) => {
              const status = getVersionStatus(version);
              const isDownloading = downloading === version.version;
              const isSwitching = switching === version.version;
              const isDownloaded = result.local_versions.includes(
                version.version
              );

              return (
                <div
                  key={version.version}
                  className={clsx(
                    "p-4 rounded-lg border transition-colors",
                    status === "current"
                      ? "bg-green-950/20 border-green-900/50"
                      : status === "new"
                        ? "bg-brand-600/5 border-brand-600/30"
                        : status === "downloaded"
                          ? "bg-zinc-900 border-zinc-700"
                          : "bg-zinc-900 border-zinc-800"
                  )}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-mono font-medium text-zinc-100">
                          v{version.version}
                        </span>
                        {status === "current" && (
                          <span className="text-xs px-1.5 py-0.5 rounded bg-green-900/50 text-green-400">
                            {t("update.current")}
                          </span>
                        )}
                        {status === "new" && (
                          <span className="text-xs px-1.5 py-0.5 rounded bg-brand-600/20 text-brand-400 flex items-center gap-0.5">
                            <Zap className="w-2.5 h-2.5" />
                            {t("update.new")}
                          </span>
                        )}
                        {status === "downloaded" && (
                          <span className="text-xs px-1.5 py-0.5 rounded bg-zinc-700 text-zinc-300">
                            {t("update.downloaded")}
                          </span>
                        )}
                      </div>
                      <p className="text-sm text-zinc-400 mb-1">
                        {version.message}
                      </p>
                      <span className="text-xs text-zinc-600">
                        {version.date}
                      </span>
                      {version.changes.length > 0 && (
                        <ul className="mt-2 space-y-0.5">
                          {version.changes.slice(0, 5).map((change, i) => (
                            <li
                              key={i}
                              className="text-xs text-zinc-500 flex items-start gap-1"
                            >
                              <span className="text-zinc-600">•</span>
                              <span>{change}</span>
                            </li>
                          ))}
                        </ul>
                      )}
                    </div>

                    {/* 操作按钮 */}
                    <div className="flex flex-col gap-1.5 flex-shrink-0">
                      {status !== "current" && version.download_url && !isDownloaded && (
                        <button
                          onClick={() => handleDownload(version)}
                          disabled={isDownloading}
                          className={clsx(
                            "flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition-colors",
                            "bg-brand-600 hover:bg-brand-500 text-white",
                            isDownloading && "opacity-50 cursor-wait"
                          )}
                        >
                          {isDownloading ? (
                            <Loader2 className="w-3 h-3 animate-spin" />
                          ) : (
                            <Download className="w-3 h-3" />
                          )}
                          {isDownloading
                            ? t("update.downloading")
                            : t("update.download")}
                        </button>
                      )}
                      {isDownloaded && status !== "current" && (
                        <button
                          onClick={() => handleSwitch(version.version)}
                          disabled={isSwitching}
                          className={clsx(
                            "flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition-colors",
                            "bg-green-700 hover:bg-green-600 text-white",
                            isSwitching && "opacity-50 cursor-wait"
                          )}
                        >
                          {isSwitching ? (
                            <Loader2 className="w-3 h-3 animate-spin" />
                          ) : (
                            <RefreshCw className="w-3 h-3" />
                          )}
                          {isSwitching
                            ? t("update.switching")
                            : t("update.switchVersion")}
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* 空状态 */}
        {!result && !loading && !error && (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <RefreshCw className="w-12 h-12 text-zinc-700 mb-3" />
            <p className="text-sm text-zinc-500">
              {t("update.checkUpdate")}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
