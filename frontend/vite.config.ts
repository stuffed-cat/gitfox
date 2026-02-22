import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    host: '0.0.0.0',
    port: 3000,
    proxy: {
      // WebIDE (VS Code Web) - 独立运行，与主 SPA 分离
      // 开发时代理到 webide 的 vite dev server (不做路径重写)
      '/-/ide': {
        target: 'http://localhost:3002',
        changeOrigin: true,
        ws: true,  // WebSocket 支持 (HMR + WebIDE 通信)
      },
      '/api': {
        target: 'http://localhost:8081',
        changeOrigin: true,
        ws: true,  // 启用 WebSocket 支持 (用于 runner 连接)
      },
      '/assets': {
        target: 'http://localhost:8081',
        changeOrigin: true,
      },
      // OAuth API 端点转发 (token, revoke, userinfo)
      // 注意：/oauth/authorize 由前端 Vue router 处理（OAuthAuthorizeView.vue）
      // 前端会调用 /api/v1/oauth/authorize/info 和 /api/v1/oauth/authorize/confirm
      '/oauth/token': {
        target: 'http://localhost:8081',
        changeOrigin: true,
      },
      '/oauth/revoke': {
        target: 'http://localhost:8081',
        changeOrigin: true,
      },
      '/oauth/userinfo': {
        target: 'http://localhost:8081',
        changeOrigin: true,
      },
      // Git HTTP 协议转发 (clone/push/fetch)
      '^/[^/]+/[^/]+\\.git': {
        target: 'http://localhost:8081',
        changeOrigin: true,
        ws: true,  // Git 协议也可能需要 WebSocket 支持
      },
      '^/[^/]+/[^/]+/info/': {
        target: 'http://localhost:8081',
        changeOrigin: true,
      },
      '^/[^/]+/[^/]+/git-': {
        target: 'http://localhost:8081',
        changeOrigin: true,
      },
    },
  },
  css: {
    preprocessorOptions: {
      scss: {
        additionalData: `@import "@/styles/variables.scss";`,
      },
    },
  },
})
