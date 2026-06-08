/// <reference types="vite/client" />

type ChatState = {
  sessionId: string;
  model: string;
  busy: boolean;
  settings: Record<string, string>;
  workspacePath: string;
};

type ChatSendResult = {
  ok: boolean;
  requestId?: string;
  text?: string;
  error?: string;
  sessionId?: string;
  stopped?: boolean;
};

type DeltaPayload = { requestId: string; text: string };
type StatusPayload = { busy: boolean; requestId: string };
type ModelEntry = { id: string; name: string; provider: string; toolSupport?: boolean };

type DesktopApi = {
  getState: () => Promise<ChatState>;
  newSession: () => Promise<{ sessionId: string }>;
  sendMessage: (payload: { prompt: string; provider: string; model: string; [key: string]: any }) => Promise<ChatSendResult>;
  stopMessage: () => Promise<{ ok: boolean; error?: string; sessionId?: string }>;
  getWorkspace: () => Promise<{ path: string }>;
  chooseWorkspace: () => Promise<{ ok: boolean; path?: string; error?: string; canceled?: boolean }>;
  getSettings: () => Promise<Record<string, string>>;
  saveSettings: (payload: Record<string, string>) => Promise<Record<string, string>>;
  clearModelSettings: () => Promise<Record<string, string>>;
  listModels: (payload: { source: string; baseUrl?: string; apiKey?: string }) => Promise<{ ok: boolean; models?: ModelEntry[]; error?: string }>;
  onDelta: (handler: (payload: DeltaPayload) => void) => () => void;
  onStatus: (handler: (payload: StatusPayload) => void) => () => void;
};

// QWebChannel 后端桥接对象类型
type BackendBridge = {
  getState: () => Promise<string>;           // 返回 JSON 字符串
  newSession: () => Promise<string>;
  sendMessage: (payloadJson: string) => Promise<string>;
  stopMessage: () => Promise<string>;
  getWorkspace: () => Promise<string>;
  chooseWorkspace: () => Promise<string>;
  getSettings: () => Promise<string>;
  saveSettings: (payloadJson: string) => Promise<string>;
  clearModelSettings: () => Promise<string>;
  listModels: (payloadJson: string) => Promise<string>;
  // 信号
  deltaReceived: { connect: (handler: (jsonStr: string) => void) => void; disconnect: (handler: (jsonStr: string) => void) => void };
  statusReceived: { connect: (handler: (jsonStr: string) => void) => void; disconnect: (handler: (jsonStr: string) => void) => void };
};

declare global {
  interface Window {
    desktopApi?: DesktopApi;
    QWebChannel: new (transport: any, initCallback: (channel: { objects: { backend: BackendBridge } }) => void) => void;
    qt: {
      webChannelTransport: any;
    };
  }
}

export {};
