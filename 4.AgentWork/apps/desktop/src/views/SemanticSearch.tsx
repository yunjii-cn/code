import { useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  Search,
  Loader2,
  AlertCircle,
  CheckCircle2,
  Star,
  TrendingUp,
  ArrowRight,
} from "lucide-react";
import {
  semanticSearch,
  detectCandidates,
  type SearchResultInfo,
  type CandidateScoreInfo,
} from "@/lib/tauri";

export default function SemanticSearch() {
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [searchResults, setSearchResults] = useState<SearchResultInfo[]>([]);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [hasSearched, setHasSearched] = useState(false);

  // 候选版本
  const [candidates, setCandidates] = useState<CandidateScoreInfo[]>([]);
  const [loadingCandidates, setLoadingCandidates] = useState(false);
  const [candidateError, setCandidateError] = useState<string | null>(null);

  async function handleSearch() {
    if (!query.trim() || searching) return;
    try {
      setSearching(true);
      setSearchError(null);
      setHasSearched(true);
      const results = await semanticSearch(query.trim(), 10);
      setSearchResults(results);
    } catch (err) {
      setSearchError(String(err));
      setSearchResults([]);
    } finally {
      setSearching(false);
    }
  }

  async function handleLoadCandidates() {
    if (loadingCandidates) return;
    try {
      setLoadingCandidates(true);
      setCandidateError(null);
      const result = await detectCandidates();
      setCandidates(result);
    } catch (err) {
      setCandidateError(String(err));
    } finally {
      setLoadingCandidates(false);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      handleSearch();
    }
  }

  return (
    <div className="flex flex-col h-full">
      <header className="px-6 py-4 border-b border-zinc-800">
        <h1 className="text-lg font-semibold flex items-center gap-2">
          <Search className="w-5 h-5" />
          语义搜索
        </h1>
      </header>

      <div className="flex-1 overflow-auto p-6 space-y-6">
        {/* 搜索框 */}
        <section>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
            <div className="flex gap-2">
              <input
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder='例如："加登录的版本" / "修复数据库 bug" / "重构认证模块"'
                className="flex-1 bg-zinc-800 text-sm rounded-lg px-3 py-2 border border-zinc-700 focus:border-brand-500 focus:outline-none"
                autoFocus
              />
              <button
                onClick={handleSearch}
                disabled={searching || !query.trim()}
                className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 hover:bg-brand-700 disabled:opacity-50 transition-colors text-sm"
              >
                {searching ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <Search className="w-4 h-4" />
                )}
                <span>{searching ? "搜索中..." : "搜索"}</span>
              </button>
            </div>

            {/* 搜索示例 */}
            <div className="flex flex-wrap gap-2">
              {["登录", "数据库", "修复 bug", "重构", "文档"].map((tag) => (
                <button
                  key={tag}
                  onClick={() => {
                    setQuery(tag);
                  }}
                  className="px-2 py-1 text-xs rounded bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-zinc-200 transition-colors"
                >
                  {tag}
                </button>
              ))}
            </div>

            {/* 错误 */}
            {searchError && (
              <div className="flex items-start gap-2 text-sm text-red-400 bg-red-950/30 rounded p-3">
                <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                <span className="break-all">{searchError}</span>
              </div>
            )}

            {/* 搜索结果 */}
            {hasSearched && !searching && searchResults.length === 0 && !searchError && (
              <div className="text-sm text-zinc-500 text-center py-4">
                未找到相关快照
              </div>
            )}

            {searchResults.length > 0 && (
              <div className="space-y-2">
                <p className="text-xs text-zinc-500">
                  找到 {searchResults.length} 个相关快照（按相似度排序）
                </p>
                {searchResults.map((r) => (
                  <button
                    key={r.snapshot_id}
                    onClick={() => navigate(`/snapshot/${r.snapshot_id}`)}
                    className="w-full text-left p-3 rounded-lg bg-zinc-800/50 hover:bg-zinc-800 border border-zinc-700 hover:border-zinc-600 transition-all"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex-1 min-w-0">
                        <p className="text-sm text-zinc-200 truncate">{r.message}</p>
                        <p className="text-xs text-zinc-500 mt-1">
                          {r.snapshot_id.slice(0, 8)} · {formatTime(r.timestamp)}
                        </p>
                      </div>
                      <div className="flex items-center gap-2 flex-shrink-0">
                        <span className="text-xs text-brand-400 font-mono">
                          {(r.score * 100).toFixed(0)}%
                        </span>
                        <ArrowRight className="w-3 h-3 text-zinc-600" />
                      </div>
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </section>

        {/* 候选版本识别 */}
        <section>
          <h2 className="text-sm font-medium text-zinc-400 mb-3 flex items-center gap-2">
            <Star className="w-4 h-4" />
            候选版本识别
            <span className="text-xs text-zinc-600">（评分 ≥ 70 的可发布版本）</span>
          </h2>
          <div className="bg-zinc-900 rounded-lg p-4 space-y-3">
            <button
              onClick={handleLoadCandidates}
              disabled={loadingCandidates}
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-zinc-700 hover:bg-zinc-600 disabled:opacity-50 transition-colors text-sm"
            >
              {loadingCandidates ? (
                <Loader2 className="w-3 h-3 animate-spin" />
              ) : (
                <TrendingUp className="w-3 h-3" />
              )}
              <span>{loadingCandidates ? "分析中..." : "扫描候选版本"}</span>
            </button>

            {candidateError && (
              <div className="flex items-start gap-2 text-sm text-red-400 bg-red-950/30 rounded p-3">
                <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                <span className="break-all">{candidateError}</span>
              </div>
            )}

            {!loadingCandidates && candidates.length === 0 && !candidateError && (
              <p className="text-xs text-zinc-500">
                点击上方按钮，AI 将扫描所有快照并识别可发布的候选版本
              </p>
            )}

            {candidates.length > 0 && (
              <div className="space-y-2">
                <p className="text-xs text-zinc-500">
                  识别到 {candidates.length} 个候选版本
                </p>
                {candidates.map((c) => (
                  <button
                    key={c.snapshot_id}
                    onClick={() => navigate(`/snapshot/${c.snapshot_id}`)}
                    className="w-full text-left p-3 rounded-lg bg-zinc-800/50 hover:bg-zinc-800 border border-zinc-700 hover:border-brand-500 transition-all"
                  >
                    <div className="flex items-start justify-between gap-2 mb-2">
                      <div className="flex-1 min-w-0">
                        <p className="text-sm text-zinc-200 truncate">{c.message}</p>
                        <p className="text-xs text-zinc-500 mt-1">
                          {c.snapshot_id.slice(0, 8)} · {formatTime(c.timestamp)}
                        </p>
                      </div>
                      <div className="flex items-center gap-1 flex-shrink-0">
                        <CheckCircle2 className="w-3 h-3 text-green-400" />
                        <span className="text-sm text-green-400 font-mono font-semibold">
                          {c.score}
                        </span>
                      </div>
                    </div>

                    {/* 评分维度 */}
                    <div className="grid grid-cols-4 gap-2 text-xs">
                      <ScoreBar label="构建" value={c.build_score} max={40} />
                      <ScoreBar label="规模" value={c.scale_score} max={30} />
                      <ScoreBar label="多样性" value={c.diversity_score} max={15} />
                      <ScoreBar label="msg" value={c.message_score} max={15} />
                    </div>

                    <p className="text-xs text-zinc-500 mt-2">{c.reason}</p>
                  </button>
                ))}
              </div>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}

function ScoreBar({ label, value, max }: { label: string; value: number; max: number }) {
  const pct = (value / max) * 100;
  return (
    <div>
      <div className="flex justify-between text-zinc-500 mb-0.5">
        <span>{label}</span>
        <span>
          {value}/{max}
        </span>
      </div>
      <div className="h-1 bg-zinc-800 rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full ${
            pct >= 80 ? "bg-green-500" : pct >= 50 ? "bg-yellow-500" : "bg-red-500"
          }`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}
