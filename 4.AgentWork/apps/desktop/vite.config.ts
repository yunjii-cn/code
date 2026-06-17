import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

// Tauri 桌面端 Vite 配置
// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
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
});
