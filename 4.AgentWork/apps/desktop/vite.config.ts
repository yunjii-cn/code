import fs from "node:fs";
import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// 版本号：构建时注入（EXE 模式 = 封装时间）
// 唯一来源是 apps/desktop/.build-version（scripts/build-version.mjs 生成），
// 与 src-tauri/build.rs 读取同一文件，保证前后端版本号一致。
// 开发模式（serve）固定返回 "dev"，由前端 version.ts 改用启动时间。
const STAMP_FILE = path.resolve(__dirname, ".build-version");

function readBuildVersion(): string {
  try {
    const value = fs.readFileSync(STAMP_FILE, "utf8").trim();
    return value || "dev";
  } catch {
    return "dev";
  }
}

// Tauri 桌面端 Vite 配置
// https://vitejs.dev/config/
export default defineConfig(({ command }) => ({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  // 注入全局变量供前端读取版本号（version.ts 读取裸标识符 __APP_VERSION__）
  define: {
    __APP_VERSION__: JSON.stringify(
      command === "build" ? readBuildVersion() : "dev"
    ),
  },
  // Tauri 要求固定端口，避免每次 dev 变端口
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // 忽略 Rust 侧变更，避免触发前端 HMR
      ignored: ["**/src-tauri/**"],
    },
  },
  // 生产构建产物目录（Tauri 会从这里加载）
  build: {
    target: "esnext",
    outDir: "dist",
    emptyOutDir: true,
  },
}));
