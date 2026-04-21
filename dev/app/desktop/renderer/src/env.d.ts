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
    // QWebChannel 全局对象（由 PyQt6 QWebEngineView 注入）
    QWebChannel: new (transport: any, initCallback: (channel: { objects: { backend: BackendBridge } }) => void) => void;
    qt: {
      webChannelTransport: any;
    };
  }
}

export {};
