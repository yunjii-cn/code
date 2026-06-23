// AI 互动面板（M4.0 D3 + D4）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.0 D3 + D4
//
// 功能：
//   1. 接收 agent_stream 事件（thinking / tool_call / progress / error / done）
//   2. 流式显示 AI 思考过程（打字机效果）
//   3. 打字指示器（"AI 正在思考..."）
//   4. 可中断（停止生成）
//   5. 多轮对话上下文（D4）
//   6. 任务触发（聊天里说"修这个"/"创建任务"→ 跳转 TaskBoard）（D4）
//   7. 消息状态（发送中 / 已发送 / 失败）（D4）
//
// MVP 阶段：调用 stream_agent_thinking 模拟流式输出
// D4 阶段：多轮对话上下文 + 任务触发 + 消息状态

import { useEffect, useState, useRef, useCallback } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useNavigate } from "react-router-dom";
import {
  Send,
  Square,
  Brain,
  Wrench,
  TrendingUp,
  AlertCircle,
  CheckCircle2,
  Loader2,
  Sparkles,
  ClipboardList,
  Trash2,
} from "lucide-react";
import {
  streamAgentThinking,
  type StreamEvent,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { useI18n } from "@/i18n";

/** 消息角色 */
type MessageRole = "user" | "assistant" | "system";

/** 消息状态 */
type MessageStatus = "sending" | "sent" | "failed";

/** 聊天消息 */
interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: number;
  /** 是否正在生成中（流式） */
  streaming?: boolean;
  /** 消息状态（D4） */
  status?: MessageStatus;
  /** 是否触发了任务（D4） */
  triggeredTask?: boolean;
}

/** 流式事件展示项 */
interface StreamItem {
  id: string;
  type: "thinking" | "tool_call" | "progress" | "error" | "done";
  icon: typeof Brain;
  color: string;
  text: string;
  timestamp: number;
}

/** 任务触发关键词（D4） */
const TASK_TRIGGER_KEYWORDS = [
  "修这个",
  "创建任务",
  "建任务",
  "加任务",
  "拆任务",
  "新建任务",
  "添加任务",
];

/** 检测是否包含任务触发关键词 */
function detectTaskTrigger(text: string): boolean {
  return TASK_TRIGGER_KEYWORDS.some((kw) => text.includes(kw));
}

/** 初始欢迎消息 */
const WELCOME_MESSAGE: ChatMessage = {
  id: "welcome",
  role: "assistant",
  content:
    "你好！我是 AgentWork AI 助手。输入你的需求，我会拆解为任务并实时展示思考过程。\n\n例如：\n- 创建一个电商客服员工\n- 开发一个用户登录页面\n- 分析这份销售数据\n\n提示：包含「修这个」或「创建任务」的消息会自动跳转到任务看板。",
  timestamp: Date.now(),
  status: "sent",
};

export default function ChatPanel() {
  const { t } = useI18n();
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: "welcome",
      role: "assistant",
      content: t("chat.welcome"),
      timestamp: Date.now(),
      status: "sent",
    } as ChatMessage,
  ]);
  const [input, setInput] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [streamItems, setStreamItems] = useState<StreamItem[]>([]);
  /** 对话轮次（D4 多轮上下文） */
  const [roundCount, setRoundCount] = useState(0);

  const navigate = useNavigate();
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const cancelRef = useRef(false);
  /** 多轮对话上下文（最近 N 条消息摘要，D4） */
  const contextRef = useRef<string[]>([]);

  // 自动滚动到底部
  const scrollToBottom = useCallback(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, streamItems, scrollToBottom]);

  // 监听 agent_stream 事件
  useEffect(() => {
    const setupListener = async () => {
      unlistenRef.current = await listen<StreamEvent>(
        "agent_stream",
        (event) => {
          const evt = event.payload;
          const now = Date.now();

          switch (evt.type) {
            case "thinking": {
              setStreamItems((prev) => [
                ...prev,
                {
                  id: `think-${now}-${Math.random()}`,
                  type: "thinking",
                  icon: Brain,
                  color: "text-purple-400",
                  text: evt.delta,
                  timestamp: now,
                },
              ]);
              break;
            }
            case "tool_call": {
              const statusText =
                typeof evt.status === "string"
                  ? evt.status
                  : "succeeded" in evt.status
                    ? `成功: ${evt.status.succeeded.result_summary}`
                    : `失败: ${evt.status.failed.error}`;
              setStreamItems((prev) => [
                ...prev,
                {
                  id: `tool-${now}`,
                  type: "tool_call",
                  icon: Wrench,
                  color: "text-blue-400",
                  text: `${evt.tool_name}(${evt.arguments}) → ${statusText}`,
                  timestamp: now,
                },
              ]);
              break;
            }
            case "progress": {
              setStreamItems((prev) => [
                ...prev,
                {
                  id: `prog-${now}`,
                  type: "progress",
                  icon: TrendingUp,
                  color: "text-amber-400",
                  text: `${evt.task_title} → ${evt.new_state} (${evt.percent}%)`,
                  timestamp: now,
                },
              ]);
              break;
            }
            case "error": {
              setStreamItems((prev) => [
                ...prev,
                {
                  id: `err-${now}`,
                  type: "error",
                  icon: AlertCircle,
                  color: "text-red-400",
                  text: evt.message + (evt.retryable ? "（可重试）" : ""),
                  timestamp: now,
                },
              ]);
              break;
            }
            case "done": {
              setStreamItems((prev) => [
                ...prev,
                {
                  id: `done-${now}`,
                  type: "done",
                  icon: CheckCircle2,
                  color: "text-green-400",
                  text: `${evt.summary}（${evt.tasks_completed}/${evt.tasks_total} 任务，${evt.elapsed_ms}ms）`,
                  timestamp: now,
                },
              ]);
              // 结束生成状态
              setIsGenerating(false);
              // 把流式内容合并为一条 assistant 消息
              setMessages((prev) => {
                const last = prev[prev.length - 1];
                if (last?.streaming) {
                  const newMsg = {
                    ...last,
                    streaming: false,
                    status: "sent" as MessageStatus,
                    content: last.content + "\n\n" + evt.summary,
                  };
                  // 更新上下文（D4）
                  contextRef.current.push(`AI: ${evt.summary}`);
                  if (contextRef.current.length > 10) {
                    contextRef.current.shift();
                  }
                  return [...prev.slice(0, -1), newMsg];
                }
                return prev;
              });
              break;
            }
          }
        },
      );
    };
    setupListener();

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
      }
    };
  }, []);

  /** 发送消息 */
  const handleSend = useCallback(async () => {
    const trimmed = input.trim();
    if (!trimmed || isGenerating) return;

    // 检测任务触发（D4）
    const shouldTriggerTask = detectTaskTrigger(trimmed);

    // 添加用户消息
    const userMsg: ChatMessage = {
      id: `user-${Date.now()}`,
      role: "user",
      content: trimmed,
      timestamp: Date.now(),
      status: "sent",
      triggeredTask: shouldTriggerTask,
    };

    // 添加占位 assistant 消息（流式）
    const assistantMsg: ChatMessage = {
      id: `assistant-${Date.now()}`,
      role: "assistant",
      content: "",
      timestamp: Date.now(),
      streaming: true,
      status: "sending",
    };

    setMessages((prev) => [...prev, userMsg, assistantMsg]);
    setStreamItems([]);
    setInput("");
    setIsGenerating(true);
    setRoundCount((c) => c + 1);
    cancelRef.current = false;

    // 更新上下文（D4 多轮对话）
    contextRef.current.push(`用户: ${trimmed}`);
    if (contextRef.current.length > 10) {
      contextRef.current.shift();
    }

    // 如果触发任务，延迟跳转 TaskBoard（D4）
    if (shouldTriggerTask) {
      setTimeout(() => {
        navigate("/tasks");
      }, 2000);
    }

    try {
      // 构建带上下文的需求（D4 多轮对话）
      const contextPrefix =
        contextRef.current.length > 2
          ? `[对话上下文（最近${Math.min(contextRef.current.length, 6)}轮）]\n${contextRef.current.slice(-6).join("\n")}\n\n[当前需求]\n`
          : "";
      await streamAgentThinking(contextPrefix + trimmed);
    } catch (err) {
      setIsGenerating(false);
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last?.streaming) {
          return [
            ...prev.slice(0, -1),
            {
              ...last,
              streaming: false,
              status: "failed" as MessageStatus,
              content: `调用失败：${err}`,
            },
          ];
        }
        return prev;
      });
    }
  }, [input, isGenerating, navigate]);

  /** 中断生成 */
  const handleStop = useCallback(() => {
    cancelRef.current = true;
    setIsGenerating(false);
    setMessages((prev) => {
      const last = prev[prev.length - 1];
      if (last?.streaming) {
        return [
          ...prev.slice(0, -1),
          {
            ...last,
            streaming: false,
            status: "sent" as MessageStatus,
            content: last.content + "\n\n[已中断]",
          },
        ];
      }
      return prev;
    });
  }, []);

  /** 清空对话（D4） */
  const handleClear = useCallback(() => {
    setMessages([WELCOME_MESSAGE]);
    setStreamItems([]);
    setRoundCount(0);
    contextRef.current = [];
  }, []);

  /** 键盘事件：Enter 发送，Shift+Enter 换行 */
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  /** 消息状态图标（D4） */
  const renderStatusIcon = (msg: ChatMessage) => {
    if (msg.role !== "user") return null;
    if (msg.status === "sending") {
      return <Loader2 className="w-3 h-3 animate-spin text-zinc-500" />;
    }
    if (msg.status === "failed") {
      return <AlertCircle className="w-3 h-3 text-red-400" />;
    }
    return <CheckCircle2 className="w-3 h-3 text-zinc-600" />;
  };

  return (
    <div className="flex flex-col h-full bg-zinc-950 text-zinc-100">
      {/* 顶部标题栏 */}
      <header className="flex items-center px-4 sm:px-6 py-3 border-b border-zinc-800 bg-zinc-900/50">
        <h1 className="text-lg font-semibold">{t("chat.title")}</h1>
        <span className="text-xs text-zinc-500 ml-2">
          {t("chat.subtitle", { round: roundCount })}
        </span>
        <div className="flex-1" />
        <button
          onClick={handleClear}
          className="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
          title={t("chat.clearTitle")}
        >
          <Trash2 className="w-3.5 h-3.5" />
          {t("chat.clear")}
        </button>
      </header>

      {/* 消息列表 + 流式事件 */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
        {messages.map((msg) => (
          <div
            key={msg.id}
            className={cn(
              "flex gap-3",
              msg.role === "user" ? "justify-end" : "justify-start"
            )}
          >
            {msg.role === "assistant" && (
              <div className="w-8 h-8 rounded-full bg-gradient-to-br from-brand-500 to-accent-500 flex items-center justify-center flex-shrink-0">
                <Sparkles className="w-4 h-4 text-white" />
              </div>
            )}
            <div className="flex flex-col items-end gap-1 max-w-[70%]">
              <div
                className={cn(
                  "rounded-2xl px-4 py-2.5",
                  msg.role === "user"
                    ? "bg-brand-600 text-white"
                    : "bg-zinc-800 text-zinc-100",
                  msg.streaming && "border border-brand-500/50",
                  msg.status === "failed" && "border border-red-500/50"
                )}
              >
                <p className="whitespace-pre-wrap text-sm leading-relaxed">
                  {msg.content || (msg.streaming ? "" : "(空)")}
                  {msg.streaming && (
                    <span className="inline-block w-2 h-4 ml-1 bg-brand-400 animate-pulse" />
                  )}
                </p>
              </div>
              {/* 消息底部状态行（D4） */}
              <div className="flex items-center gap-2 text-xs text-zinc-600">
                {renderStatusIcon(msg)}
                {msg.triggeredTask && (
                  <span className="flex items-center gap-1 text-amber-400">
                    <ClipboardList className="w-3 h-3" />
                    {t("chat.triggersTask")}
                  </span>
                )}
                {msg.status === "failed" && (
                  <span className="flex items-center gap-1 text-red-400">
                    <AlertCircle className="w-3 h-3" />
                    {t("chat.sendFailed")}
                  </span>
                )}
              </div>
            </div>
            {msg.role === "user" && (
              <div className="w-8 h-8 rounded-full bg-zinc-700 flex items-center justify-center flex-shrink-0">
                <span className="text-xs text-zinc-300">我</span>
              </div>
            )}
          </div>
        ))}

        {/* 流式事件展示区（生成中显示） */}
        {isGenerating && streamItems.length > 0 && (
          <div className="ml-11 space-y-1.5 border-l-2 border-brand-500/30 pl-4">
            {streamItems.map((item) => {
              const Icon = item.icon;
              return (
                <div key={item.id} className="flex items-start gap-2 text-sm">
                  <Icon className={cn("w-4 h-4 mt-0.5 flex-shrink-0", item.color)} />
                  <span className="text-zinc-400">{item.text}</span>
                </div>
              );
            })}
            {/* 打字指示器 */}
            <div className="flex items-center gap-2 text-xs text-zinc-500">
              <Loader2 className="w-3 h-3 animate-spin" />
              <span>{t("chat.aiThinking")}</span>
            </div>
          </div>
        )}
      </div>

      {/* 输入区 */}
      <div className="border-t border-zinc-800 bg-zinc-900/50 px-6 py-4">
        <div className="flex items-end gap-2">
          <textarea
            ref={inputRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t("chat.placeholder")}
            rows={1}
            className={cn(
              "flex-1 resize-none rounded-xl bg-zinc-800 border border-zinc-700",
              "px-4 py-2.5 text-sm text-zinc-100 placeholder-zinc-500",
              "focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500",
              "max-h-32 overflow-y-auto"
            )}
            style={{ minHeight: "44px" }}
            disabled={isGenerating}
          />
          {isGenerating ? (
            <button
              onClick={handleStop}
              className={cn(
                "flex items-center gap-1.5 rounded-xl px-4 py-2.5",
                "bg-red-600 hover:bg-red-700 text-white text-sm font-medium",
                "transition-colors"
              )}
            >
              <Square className="w-4 h-4" />
              停止
            </button>
          ) : (
            <button
              onClick={handleSend}
              disabled={!input.trim()}
              className={cn(
                "flex items-center gap-1.5 rounded-xl px-4 py-2.5",
                "bg-brand-600 hover:bg-brand-700 text-white text-sm font-medium",
                "transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              )}
            >
              <Send className="w-4 h-4" />
              发送
            </button>
          )}
        </div>
        <p className="mt-2 text-xs text-zinc-600">
          {t("chat.footer", { n: 6 })}
        </p>
      </div>
    </div>
  );
}
