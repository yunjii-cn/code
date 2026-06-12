import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import Components from 'unplugin-vue-components/vite'
import { VantResolver } from '@vant/auto-import-resolver'
import { VitePWA } from 'vite-plugin-pwa'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [
    vue(),
    Components({
      resolvers: [VantResolver()],
    }),
    // 2026-06-09 TASK-4.3 PWA 完整支持：vite-plugin-pwa + Workbox injectManifest
    // 选择 injectManifest 模式：自动注入 precache manifest，但保留自定义 fetch 逻辑
    VitePWA({
      strategies: 'injectManifest',
      srcDir: 'src',
      filename: 'sw.ts',
      registerType: 'prompt',
      injectRegister: false, // 我们自己控制注册流程
      manifest: false, // 使用 public/manifest.webmanifest
      injectManifest: {
        // precache 范围：vite 构建产物 + 关键资源
        globPatterns: ['**/*.{js,css,html,svg,png,ico,woff,woff2}'],
        // 不缓存过大的文件
        maximumFileSizeToCacheInBytes: 5 * 1024 * 1024,
      },
      devOptions: {
        // 开发模式启用，方便调试
        enabled: false,
        type: 'module',
      },
    }),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      // 2026-06-08 TASK-1.4 引入：feature-sliced 架构别名
      // 新代码: import { xxx } from '@features/chat'
      // 跨 feature 共享: import { YJButton } from '@shared/components'
      '@features': fileURLToPath(new URL('./src/features', import.meta.url)),
      '@shared': fileURLToPath(new URL('./src/shared', import.meta.url)),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:18080',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://127.0.0.1:18080',
        ws: true,
      },
    },
  },
})
