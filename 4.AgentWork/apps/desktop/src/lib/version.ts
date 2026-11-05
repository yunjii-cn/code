// 版本号工具
// 开发模式：以启动时间为准（每次启动更新）
// EXE/APP 模式：以封装时间为准（构建时注入，固定不变）

/** 版本号格式：YYYY.MM.DD.HHMM */
export type VersionString = string;

// 构建时由 vite define 注入的裸标识符；用 typeof 守卫避免未注入时抛 ReferenceError
declare const __APP_VERSION__: string;

const VERSION_STORAGE_KEY = "yunji-app-version";

/**
 * 获取应用版本号
 *
 * 优先级：
 * 1. __APP_VERSION__（构建时注入，EXE 模式）— 封装时间
 * 2. localStorage 缓存（开发模式，首次启动时生成）
 * 3. 实时生成（开发模式，首次启动）
 */
export function getAppVersion(): VersionString {
  // 1. 构建时注入的版本号（EXE 模式）
  // Vite 在构建时通过 define 替换裸标识符（globalThis.__APP_VERSION__ 这种成员表达式不会被替换）
  const injected = typeof __APP_VERSION__ === "string" ? __APP_VERSION__ : undefined;
  if (injected && injected !== "dev") {
    return injected;
  }

  // 2. 开发模式：检查 localStorage 缓存
  try {
    const cached = localStorage.getItem(VERSION_STORAGE_KEY);
    if (cached) {
      return cached;
    }
  } catch {
    // ignore
  }

  // 3. 开发模式：以当前时间（启动时间）生成版本号
  const now = new Date();
  const version = formatVersion(now);

  try {
    localStorage.setItem(VERSION_STORAGE_KEY, version);
  } catch {
    // ignore
  }

  return version;
}

/** 格式化日期为版本号 YYYY.MM.DD.HHMM */
function formatVersion(date: Date): VersionString {
  const yyyy = date.getFullYear();
  const mm = String(date.getMonth() + 1).padStart(2, "0");
  const dd = String(date.getDate()).padStart(2, "0");
  const hh = String(date.getHours()).padStart(2, "0");
  const min = String(date.getMinutes()).padStart(2, "0");
  return `${yyyy}.${mm}.${dd}.${hh}${min}`;
}

/** 重置开发模式版本号（下次调用 getAppVersion 会重新生成） */
export function resetDevVersion(): void {
  try {
    localStorage.removeItem(VERSION_STORAGE_KEY);
  } catch {
    // ignore
  }
}
