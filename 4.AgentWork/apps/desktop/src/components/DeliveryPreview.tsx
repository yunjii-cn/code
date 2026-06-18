// 交付物预览组件（M4.0 D5）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.0 D5
//
// 功能：
//   1. DiffViewer — 文件变更对比（轻量级，不依赖 Monaco）
//   2. TestResults — 测试结果（通过/失败/覆盖率）
//   3. CommitPreview — commit msg + 文件列表
//   4. 一键 approve / request changes
//
// MVP 阶段：用模拟数据展示 UI
// 后续：接入真实 git diff + cargo test 结果

import { useState } from "react";
import {
  FileCode,
  CheckCircle2,
  XCircle,
  GitCommit,
  ThumbsUp,
  ThumbsDown,
  FilePlus,
  FileEdit,
  FileMinus,
  ChevronDown,
  ChevronRight,
  Loader2,
} from "lucide-react";
import { cn } from "@/lib/utils";

/** 文件变更 */
interface FileChange {
  path: string;
  action: "added" | "modified" | "deleted";
  additions: number;
  deletions: number;
  /** diff 内容（简化版，按行展示） */
  diff_lines?: DiffLine[];
}

/** diff 行 */
interface DiffLine {
  type: "add" | "remove" | "context";
  content: string;
}

/** 测试结果 */
interface TestResult {
  name: string;
  status: "passed" | "failed" | "skipped";
  duration_ms: number;
}

/** 测试摘要 */
interface TestSummary {
  total: number;
  passed: number;
  failed: number;
  skipped: number;
  coverage_percent: number;
  results: TestResult[];
}

/** 交付物预览 Props */
interface DeliveryPreviewProps {
  /** 任务标题 */
  taskTitle: string;
  /** commit message */
  commitMessage: string;
  /** 文件变更列表 */
  fileChanges: FileChange[];
  /** 测试结果 */
  testSummary: TestSummary;
  /** approve 回调 */
  onApprove?: () => void;
  /** request changes 回调 */
  onRequestChanges?: () => void;
}

/** 模拟数据（MVP 演示用） */
export const MOCK_DELIVERY: DeliveryPreviewProps = {
  taskTitle: "实现用户登录 API",
  commitMessage: "feat(auth): 实现用户登录 API + JWT token\n\n- 新增 /api/login 端点\n- 密码 bcrypt 加密\n- JWT token 签发与验证\n- 5 个单元测试",
  fileChanges: [
    {
      path: "src/api/auth.rs",
      action: "added",
      additions: 85,
      deletions: 0,
      diff_lines: [
        { type: "add", content: "pub async fn login(email: &str, password: &str) -> Result<Token> {" },
        { type: "add", content: "    let user = User::find_by_email(email).await?;" },
        { type: "add", content: "    user.verify_password(password)?;" },
        { type: "add", content: "    let token = jwt::sign(&user.id)?;" },
        { type: "add", content: "    Ok(Token::new(token))" },
        { type: "add", content: "}" },
      ],
    },
    {
      path: "src/models/user.rs",
      action: "modified",
      additions: 12,
      deletions: 3,
      diff_lines: [
        { type: "context", content: "impl User {" },
        { type: "remove", content: "    pub fn find_by_email(email: &str) -> Option<User> {" },
        { type: "add", content: "    pub async fn find_by_email(email: &str) -> Result<User> {" },
        { type: "context", content: "        // ..." },
        { type: "add", content: "    pub async fn verify_password(&self, password: &str) -> Result<()> {" },
        { type: "add", content: "        bcrypt::verify(password, &self.password_hash)" },
        { type: "add", content: "            .map_err(|_| AuthError::InvalidPassword)" },
        { type: "add", content: "    }" },
      ],
    },
    {
      path: "src/api/mod.rs",
      action: "modified",
      additions: 5,
      deletions: 0,
      diff_lines: [
        { type: "add", content: "pub mod auth;" },
      ],
    },
  ],
  testSummary: {
    total: 5,
    passed: 4,
    failed: 1,
    skipped: 0,
    coverage_percent: 82,
    results: [
      { name: "test_login_success", status: "passed", duration_ms: 15 },
      { name: "test_login_wrong_password", status: "passed", duration_ms: 8 },
      { name: "test_login_user_not_found", status: "passed", duration_ms: 6 },
      { name: "test_jwt_sign_and_verify", status: "passed", duration_ms: 12 },
      { name: "test_login_empty_email", status: "failed", duration_ms: 3 },
    ],
  },
};

/** 文件变更图标 */
function fileActionIcon(action: "added" | "modified" | "deleted") {
  switch (action) {
    case "added":
      return { icon: FilePlus, color: "text-green-400" };
    case "modified":
      return { icon: FileEdit, color: "text-amber-400" };
    case "deleted":
      return { icon: FileMinus, color: "text-red-400" };
  }
}

/** DiffViewer 子组件 */
function DiffViewer({ changes }: { changes: FileChange[] }) {
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(
    new Set(changes.map((c) => c.path))
  );

  const toggleFile = (path: string) => {
    setExpandedFiles((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  return (
    <div className="space-y-2">
      <h3 className="text-sm font-semibold text-zinc-300 flex items-center gap-2">
        <FileCode className="w-4 h-4" />
        文件变更（{changes.length} 个文件）
      </h3>
      {changes.map((change) => {
        const { icon: ActionIcon, color } = fileActionIcon(change.action);
        const isExpanded = expandedFiles.has(change.path);
        return (
          <div
            key={change.path}
            className="rounded-lg border border-zinc-800 overflow-hidden"
          >
            {/* 文件头 */}
            <button
              onClick={() => toggleFile(change.path)}
              className="w-full flex items-center gap-2 px-3 py-2 bg-zinc-900 hover:bg-zinc-800/50 transition-colors"
            >
              {isExpanded ? (
                <ChevronDown className="w-3.5 h-3.5 text-zinc-500" />
              ) : (
                <ChevronRight className="w-3.5 h-3.5 text-zinc-500" />
              )}
              <ActionIcon className={cn("w-4 h-4", color)} />
              <span className="text-sm font-mono text-zinc-200">{change.path}</span>
              <span className="ml-auto text-xs">
                <span className="text-green-400">+{change.additions}</span>
                {" "}
                <span className="text-red-400">-{change.deletions}</span>
              </span>
            </button>
            {/* diff 内容 */}
            {isExpanded && change.diff_lines && (
              <div className="bg-zinc-950 font-mono text-xs overflow-x-auto">
                {change.diff_lines.map((line, i) => (
                  <div
                    key={i}
                    className={cn(
                      "px-3 py-0.5 whitespace-pre",
                      line.type === "add" && "bg-green-950/30 text-green-300",
                      line.type === "remove" && "bg-red-950/30 text-red-300",
                      line.type === "context" && "text-zinc-500"
                    )}
                  >
                    <span className="select-none mr-2">
                      {line.type === "add" ? "+" : line.type === "remove" ? "-" : " "}
                    </span>
                    {line.content}
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

/** TestResults 子组件 */
function TestResults({ summary }: { summary: TestSummary }) {
  const passRate = summary.total > 0
    ? Math.round((summary.passed / summary.total) * 100)
    : 0;

  return (
    <div className="space-y-2">
      <h3 className="text-sm font-semibold text-zinc-300 flex items-center gap-2">
        <CheckCircle2 className="w-4 h-4" />
        测试结果
      </h3>
      {/* 摘要 */}
      <div className="grid grid-cols-5 gap-2 text-center">
        <div className="rounded-lg bg-zinc-900 border border-zinc-800 px-2 py-1.5">
          <div className="text-lg font-bold text-zinc-200">{summary.total}</div>
          <div className="text-xs text-zinc-500">总计</div>
        </div>
        <div className="rounded-lg bg-green-950/30 border border-green-900/50 px-2 py-1.5">
          <div className="text-lg font-bold text-green-400">{summary.passed}</div>
          <div className="text-xs text-zinc-500">通过</div>
        </div>
        <div className="rounded-lg bg-red-950/30 border border-red-900/50 px-2 py-1.5">
          <div className="text-lg font-bold text-red-400">{summary.failed}</div>
          <div className="text-xs text-zinc-500">失败</div>
        </div>
        <div className="rounded-lg bg-zinc-900 border border-zinc-800 px-2 py-1.5">
          <div className="text-lg font-bold text-zinc-400">{summary.skipped}</div>
          <div className="text-xs text-zinc-500">跳过</div>
        </div>
        <div className="rounded-lg bg-blue-950/30 border border-blue-900/50 px-2 py-1.5">
          <div className="text-lg font-bold text-blue-400">{summary.coverage_percent}%</div>
          <div className="text-xs text-zinc-500">覆盖率</div>
        </div>
      </div>
      {/* 通过率进度条 */}
      <div className="flex items-center gap-2">
        <span className="text-xs text-zinc-500">通过率</span>
        <div className="flex-1 h-2 rounded-full bg-zinc-800 overflow-hidden">
          <div
            className={cn(
              "h-full rounded-full transition-all",
              passRate === 100 ? "bg-green-500" : passRate >= 80 ? "bg-amber-500" : "bg-red-500"
            )}
            style={{ width: `${passRate}%` }}
          />
        </div>
        <span className="text-xs text-zinc-400">{passRate}%</span>
      </div>
      {/* 详细结果 */}
      <div className="space-y-1 max-h-40 overflow-y-auto">
        {summary.results.map((result) => (
          <div
            key={result.name}
            className="flex items-center gap-2 px-2 py-1 rounded text-xs"
          >
            {result.status === "passed" ? (
              <CheckCircle2 className="w-3.5 h-3.5 text-green-400 flex-shrink-0" />
            ) : result.status === "failed" ? (
              <XCircle className="w-3.5 h-3.5 text-red-400 flex-shrink-0" />
            ) : (
              <Loader2 className="w-3.5 h-3.5 text-zinc-500 flex-shrink-0" />
            )}
            <span className="font-mono text-zinc-300">{result.name}</span>
            <span className="ml-auto text-zinc-600">{result.duration_ms}ms</span>
          </div>
        ))}
      </div>
    </div>
  );
}

/** CommitPreview 子组件 */
function CommitPreview({ message }: { message: string }) {
  return (
    <div className="space-y-2">
      <h3 className="text-sm font-semibold text-zinc-300 flex items-center gap-2">
        <GitCommit className="w-4 h-4" />
        Commit Message
      </h3>
      <div className="rounded-lg bg-zinc-900 border border-zinc-800 p-3">
        <pre className="text-xs font-mono text-zinc-300 whitespace-pre-wrap">
          {message}
        </pre>
      </div>
    </div>
  );
}

/** 交付物预览主组件 */
export default function DeliveryPreview({
  taskTitle,
  commitMessage,
  fileChanges,
  testSummary,
  onApprove,
  onRequestChanges,
}: DeliveryPreviewProps) {
  const [decision, setDecision] = useState<"none" | "approved" | "changes_requested">("none");

  const handleApprove = () => {
    setDecision("approved");
    onApprove?.();
  };

  const handleRequestChanges = () => {
    setDecision("changes_requested");
    onRequestChanges?.();
  };

  const hasFailedTests = testSummary.failed > 0;

  return (
    <div className="flex flex-col h-full bg-zinc-950 text-zinc-100">
      {/* 顶部标题栏 */}
      <header className="flex items-center gap-2 px-6 py-3 border-b border-zinc-800 bg-zinc-900/50">
        <FileCode className="w-5 h-5 text-brand-400" />
        <h1 className="text-lg font-semibold">交付物预览</h1>
        <span className="text-xs text-zinc-500 ml-2 truncate">{taskTitle}</span>
      </header>

      {/* 内容区 */}
      <div className="flex-1 overflow-y-auto px-6 py-4 space-y-6">
        <CommitPreview message={commitMessage} />
        <DiffViewer changes={fileChanges} />
        <TestResults summary={testSummary} />
      </div>

      {/* 底部操作栏 */}
      <div className="border-t border-zinc-800 bg-zinc-900/50 px-6 py-4">
        {decision === "none" ? (
          <div className="flex items-center gap-3">
            <button
              onClick={handleApprove}
              disabled={hasFailedTests}
              className={cn(
                "flex items-center gap-1.5 rounded-xl px-5 py-2.5",
                "bg-green-600 hover:bg-green-700 text-white text-sm font-medium",
                "transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              )}
              title={hasFailedTests ? "有测试失败，无法 approve" : "通过验收"}
            >
              <ThumbsUp className="w-4 h-4" />
              Approve
            </button>
            <button
              onClick={handleRequestChanges}
              className={cn(
                "flex items-center gap-1.5 rounded-xl px-5 py-2.5",
                "bg-amber-600 hover:bg-amber-700 text-white text-sm font-medium",
                "transition-colors"
              )}
            >
              <ThumbsDown className="w-4 h-4" />
              Request Changes
            </button>
            {hasFailedTests && (
              <span className="text-xs text-red-400 flex items-center gap-1">
                <XCircle className="w-3.5 h-3.5" />
                有 {testSummary.failed} 个测试失败，无法 approve
              </span>
            )}
          </div>
        ) : decision === "approved" ? (
          <div className="flex items-center gap-2 text-green-400">
            <CheckCircle2 className="w-5 h-5" />
            <span className="text-sm font-medium">已通过验收，任务合并完成</span>
          </div>
        ) : (
          <div className="flex items-center gap-2 text-amber-400">
            <ThumbsDown className="w-5 h-5" />
            <span className="text-sm font-medium">已请求修改，任务退回执行</span>
          </div>
        )}
      </div>
    </div>
  );
}
