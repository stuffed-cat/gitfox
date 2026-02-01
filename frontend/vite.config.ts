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
      '/api': {
        target: 'http://localhost:8081',
        changeOrigin: true,
      },
      // Git HTTP 协议转发 (clone/push/fetch)
      '^/[^/]+/[^/]+\\.git': {
        target: 'http://localhost:8081',
        changeOrigin: true,
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
