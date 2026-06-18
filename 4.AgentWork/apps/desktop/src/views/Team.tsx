import { useEffect, useState } from "react";
import {
  Users,
  CheckCircle2,
  AlertCircle,
  Loader2,
  Cpu,
  Cloud,
  Plus,
  Play,
  RefreshCw,
} from "lucide-react";
import {
  listTeamTemplates,
  loadTeamTemplate,
  getTeamConfig,
  listModels,
  registerModel,
  getTeamStatus,
  executeTeamTask,
  type TeamTemplateInfo,
  type RoleInfo,
  type ModelInfo,
  type WorkerInfo,
} from "@/lib/tauri";

export default function Team() {
  const [templates, setTemplates] = useState<TeamTemplateInfo[]>([]);
  const [currentRoles, setCurrentRoles] = useState<RoleInfo[]>([]);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [workers, setWorkers] = useState<WorkerInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  // 模型注册表单
  const [newModelId, setNewModelId] = useState("");
  const [newModelProvider, setNewModelProvider] = useState<"openai" | "ollama">("openai");
  const [newModelBaseUrl, setNewModelBaseUrl] = useState("");
  const [newModelApiKey, setNewModelApiKey] = useState("");
  const [newModelName, setNewModelName] = useState("");
  const [registering, setRegistering] = useState(false);

  // 任务执行
  const [taskRole, setTaskRole] = useState("");
  const [taskPrompt, setTaskPrompt] = useState("");
  const [taskResult, setTaskResult] = useState<string | null>(null);
  const [executing, setExecuting] = useState(false);

  useEffect(() => {
    refreshAll();
  }, []);

  async function refreshAll() {
    setLoading(true);
    try {
      const [tmpls, cfg, mdls, wks] = await Promise.all([
        listTeamTemplates().catch(() => []),
        getTeamConfig().catch(() => []),
        listModels().catch(() => []),
        getTeamStatus().catch(() => []),
      ]);
      setTemplates(tmpls);
      setCurrentRoles(cfg);
      setModels(mdls);
      setWorkers(wks);
      if (cfg.length > 0 && !taskRole) {
        setTaskRole(cfg[0].id);
      }
    } finally {
      setLoading(false);
    }
  }

  async function handleLoadTemplate(name: string) {
    try {
      setLoading(true);
      setMessage(null);
      const result = await loadTeamTemplate(name);
      setMessage({ type: "success", text: result });
      await refreshAll();
    } catch (err) {
      setMessage({ type: "error", text: String(err) });
    } finally {
      setLoading(false);
    }
  }

  async function handleRegisterModel() {
    if (registering || !newModelId.trim() || !newModelName.trim()) return;
    try {
      setRegistering(true);
      setMessage(null);
      const baseUrl =
        newModelBaseUrl.trim() ||
        (newModelProvider === "openai"
          ? "https://api.openai.com/v1"
          : "http://localhost:11434");
      const result = await registerModel({
        id: newModelId.trim(),
        provider: newModelProvider,
        base_url: baseUrl,
        api_key: newModelApiKey.trim() || undefined,
        model_name: newModelName.trim(),
      });
      setMessage({ type: "success", text: result });
      setNewModelId("");
      setNewModelBaseUrl("");
      setNewModelApiKey("");
      setNewModelName("");
      await refreshAll();
    } catch (err) {
      setMessage({ type: "error", text: String(err) });
    } finally {
      setRegistering(false);
    }
  }

  async function handleExecuteTask() {
    if (executing || !taskRole || !taskPrompt.trim()) return;
    try {
      setExecuting(true);
      setTaskResult(null);
      setMessage(null);
      const result = await executeTeamTask(taskRole, taskPrompt.trim());
      setTaskResult(result);
      setMessage({ type: "success", text: "任务执行完成" });
      await refreshAll();
    } catch (err) {
      setMessage({ type: "error", text: String(err) });
    } finally {
      setExecuting(false);
    }
  }

  return (
    <div className="flex flex-col h-full">
      <header className="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <h1 className="text-lg font-semibold flex items-center gap-2">
          <Users className="w-5 h-5" />
          团队协作
        </h1>
        <button
          onClick={refreshAll}
          disabled={loading}
          className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-zinc-800 hover:bg-zinc-700 disabled:opacity-50 transition-colors"
        >
          <RefreshCw className={`w-3 h-3 ${loading ? "animate-spin" : ""}`} />
          刷新
        </button>
      </header>

      <div className="flex-1 overflow-auto p-6 space-y-6">
        {/* 消息提示 */}
        {message && (
          <div
            className={`flex items-start gap-2 text-sm rounded p-3 ${
              message.type === "success"
                ? "text-green-400 bg-green-950/30"
                : "text-red-400 bg-red-950/30"
            }`}
          >
            {message.type === "success" ? (
              <CheckCircle2 className="w-4 h-4 mt-0.5 flex-shrink-0" />
            ) : (
              <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
            )}
            <span className="break-all">{message.text}</span>
          </div>
        )}

        {/* 团队模板选择 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">团队模板</h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            {templates.map((tmpl) => (
              <button
                key={tmpl.name}
                onClick={() => handleLoadTemplate(tmpl.name)}
                disabled={loading}
                className="text-left p-3 rounded-lg border border-zinc-700 bg-zinc-800/50 hover:border-brand-500 hover:bg-zinc-800 disabled:opacity-50 transition-all"
              >
                <div className="flex items-center justify-between mb-1">
                  <span className="text-sm font-medium text-zinc-200">{tmpl.team_name}</span>
                  <span className="text-xs text-zinc-500">{tmpl.role_count} 角色</span>
                </div>
                <p className="text-xs text-zinc-500 mb-2">{tmpl.description}</p>
                <div className="flex flex-wrap gap-1">
                  {tmpl.role_ids.map((id) => (
                    <span
                      key={id}
                      className="px-1.5 py-0.5 text-xs rounded bg-zinc-700 text-zinc-300"
                    >
                      {id}
                    </span>
                  ))}
                </div>
              </button>
            ))}
          </div>
        </section>

        {/* 当前团队角色 */}
        {currentRoles.length > 0 && (
          <section>
            <h2 className="text-sm font-medium text-zinc-400 mb-3">
              当前团队角色（{currentRoles.length}）
            </h2>
            <div className="space-y-2">
              {currentRoles.map((role) => (
                <div
                  key={role.id}
                  className="bg-zinc-900 rounded-lg p-3 border border-zinc-800"
                >
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <p className="text-sm font-medium text-zinc-200">
                        {role.name}{" "}
                        <span className="text-xs text-zinc-500">({role.id})</span>
                      </p>
                      <p className="text-xs text-zinc-500 mt-0.5">{role.description}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-xs text-brand-400 font-mono">{role.model}</p>
                      {role.model_fallback.length > 0 && (
                        <p className="text-xs text-zinc-600 mt-0.5">
                          备用: {role.model_fallback.join(", ")}
                        </p>
                      )}
                    </div>
                  </div>
                  <div className="flex flex-wrap gap-1 mt-2">
                    {role.skills.map((s) => (
                      <span
                        key={s}
                        className="px-1.5 py-0.5 text-xs rounded bg-blue-950/50 text-blue-300"
                      >
                        {s}
                      </span>
                    ))}
                    {role.permissions.map((p) => (
                      <span
                        key={p}
                        className="px-1.5 py-0.5 text-xs rounded bg-purple-950/50 text-purple-300"
                      >
                        {p}
                      </span>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </section>
        )}

        {/* Worker 状态 */}
        {workers.length > 0 && (
          <section>
            <h2 className="text-sm font-medium text-zinc-400 mb-3">Worker 状态</h2>
            <div className="bg-zinc-900 rounded-lg p-3 space-y-2">
              {workers.map((w) => (
                <div
                  key={w.role_id}
                  className="flex items-center justify-between text-sm py-1 border-b border-zinc-800 last:border-0"
                >
                  <div className="flex items-center gap-2">
                    <span
                      className={`w-2 h-2 rounded-full ${
                        w.state === "running"
                          ? "bg-green-500 animate-pulse"
                          : w.state === "error"
                          ? "bg-red-500"
                          : "bg-zinc-600"
                      }`}
                    />
                    <span className="text-zinc-200">{w.role_name}</span>
                    <span className="text-xs text-zinc-500">({w.role_id})</span>
                  </div>
                  <div className="flex items-center gap-3 text-xs text-zinc-500">
                    <span>模型: {w.current_model}</span>
                    <span>完成: {w.tasks_completed}</span>
                    <span>失败: {w.tasks_failed}</span>
                  </div>
                </div>
              ))}
            </div>
          </section>
        )}

        {/* 任务执行 */}
        {currentRoles.length > 0 && (
          <section>
            <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
              <Play className="w-4 h-4" />
              执行任务（单角色测试）
            </h2>
            <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
              <div className="flex gap-2">
                <select
                  value={taskRole}
                  onChange={(e) => setTaskRole(e.target.value)}
                  className="bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                >
                  {currentRoles.map((r) => (
                    <option key={r.id} value={r.id}>
                      {r.name} ({r.id})
                    </option>
                  ))}
                </select>
                <input
                  type="text"
                  value={taskPrompt}
                  onChange={(e) => setTaskPrompt(e.target.value)}
                  placeholder="任务描述，例如：分析需求并拆分任务"
                  className="flex-1 bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
                <button
                  onClick={handleExecuteTask}
                  disabled={executing || !taskPrompt.trim()}
                  className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-sm"
                >
                  {executing ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Play className="w-4 h-4" />
                  )}
                  <span>{executing ? "执行中..." : "执行"}</span>
                </button>
              </div>
              {taskResult && (
                <div className="bg-zinc-800/50 rounded p-3 text-sm text-zinc-300 max-h-60 overflow-auto whitespace-pre-wrap">
                  {taskResult}
                </div>
              )}
            </div>
          </section>
        )}

        {/* 已注册模型 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3">
            已注册模型（{models.length}）
          </h2>
          <div className="bg-zinc-900 rounded-lg p-3 space-y-1">
            {models.length === 0 ? (
              <p className="text-sm text-zinc-500 text-center py-2">暂无模型</p>
            ) : (
              models.map((m) => (
                <div
                  key={m.id}
                  className="flex items-center justify-between text-sm py-1.5 border-b border-zinc-800 last:border-0"
                >
                  <div className="flex items-center gap-2">
                    {m.provider === "ollama" ? (
                      <Cpu className="w-3 h-3 text-zinc-400" />
                    ) : (
                      <Cloud className="w-3 h-3 text-zinc-400" />
                    )}
                    <span className="text-zinc-200 font-mono">{m.id}</span>
                    <span className="text-xs text-zinc-500">{m.model_name}</span>
                  </div>
                  <div className="flex items-center gap-2 text-xs text-zinc-500">
                    <span>{m.provider}</span>
                    {m.has_api_key ? (
                      <CheckCircle2 className="w-3 h-3 text-green-400" />
                    ) : (
                      <span className="text-yellow-500">无 key</span>
                    )}
                  </div>
                </div>
              ))
            )}
          </div>
        </section>

        {/* 注册新模型 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
            <Plus className="w-4 h-4" />
            注册自定义模型
          </h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div>
                <label className="text-xs text-zinc-500">模型 ID</label>
                <input
                  type="text"
                  value={newModelId}
                  onChange={(e) => setNewModelId(e.target.value)}
                  placeholder="例如：my-glm"
                  className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
              </div>
              <div>
                <label className="text-xs text-zinc-500">提供方</label>
                <div className="mt-1 grid grid-cols-2 gap-2">
                  {(["openai", "ollama"] as const).map((p) => (
                    <button
                      key={p}
                      onClick={() => setNewModelProvider(p)}
                      className={`p-2 rounded-lg border text-xs transition-all ${
                        newModelProvider === p
                          ? "border-brand-500 bg-brand-950/30 text-brand-300"
                          : "border-zinc-700 bg-zinc-800/50 text-zinc-400"
                      }`}
                    >
                      {p === "openai" ? "OpenAI 兼容" : "Ollama 本地"}
                    </button>
                  ))}
                </div>
              </div>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div>
                <label className="text-xs text-zinc-500">Base URL</label>
                <input
                  type="text"
                  value={newModelBaseUrl}
                  onChange={(e) => setNewModelBaseUrl(e.target.value)}
                  placeholder={
                    newModelProvider === "openai"
                      ? "https://api.openai.com/v1"
                      : "http://localhost:11434"
                  }
                  className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
              </div>
              <div>
                <label className="text-xs text-zinc-500">模型名</label>
                <input
                  type="text"
                  value={newModelName}
                  onChange={(e) => setNewModelName(e.target.value)}
                  placeholder="例如：gpt-4o-mini / qwen2.5-coder:7b"
                  className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
              </div>
            </div>
            {newModelProvider === "openai" && (
              <div>
                <label className="text-xs text-zinc-500">API Key</label>
                <input
                  type="password"
                  value={newModelApiKey}
                  onChange={(e) => setNewModelApiKey(e.target.value)}
                  placeholder="sk-..."
                  className="mt-1 w-full bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                />
              </div>
            )}
            <button
              onClick={handleRegisterModel}
              disabled={registering || !newModelId.trim() || !newModelName.trim()}
              className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-sm"
            >
              {registering ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Plus className="w-4 h-4" />
              )}
              <span>{registering ? "注册中..." : "注册模型"}</span>
            </button>
          </div>
        </section>
      </div>
    </div>
  );
}
