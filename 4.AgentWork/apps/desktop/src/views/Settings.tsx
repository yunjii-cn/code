import { useEffect, useState } from "react";
import { getAppInfo, getSystemInfo, type AppInfo, type SystemInfo } from "@/lib/tauri";

export default function Settings() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [sysInfo, setSysInfo] = useState<SystemInfo | null>(null);

  useEffect(() => {
    Promise.all([getAppInfo(), getSystemInfo()])
      .then(([app, sys]) => {
        setAppInfo(app);
        setSysInfo(sys);
      })
      .catch(console.error);
  }, []);

  return (
    <div className="flex flex-col h-full">
      <header className="px-6 py-4 border-b border-zinc-800">
        <h1 className="text-lg font-semibold">设置</h1>
      </header>

      <div className="flex-1 overflow-auto p-6 space-y-6">
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

        {/* TimeFlow 设置（占位） */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">TimeFlow 版本控制</h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-4">
            <div>
              <label className="text-sm text-zinc-300">运行模式</label>
              <select
                className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700"
                defaultValue="stealth"
              >
                <option value="stealth">隐身模式（纯本地，代码不上云）</option>
                <option value="sync">同步模式（自动镜像 git）</option>
                <option value="release">发布模式（只推正式版本）</option>
              </select>
            </div>
            <div>
              <label className="text-sm text-zinc-300">自动快照间隔（秒）</label>
              <input
                type="number"
                className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700"
                defaultValue={5}
                min={1}
              />
            </div>
            <div className="flex items-center gap-2">
              <input type="checkbox" id="ai-summary" defaultChecked className="rounded" />
              <label htmlFor="ai-summary" className="text-sm text-zinc-300">
                启用 AI 变更摘要
              </label>
            </div>
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
