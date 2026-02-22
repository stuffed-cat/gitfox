import { defineConfig } from 'vite'
import { resolve } from 'path'
import fs from 'fs'

export default defineConfig({
  root: resolve(__dirname, 'packages/bootstrap'),
  base: '/-/ide/',
  publicDir: resolve(__dirname, 'static'),
  build: {
    outDir: resolve(__dirname, 'dist'),
    emptyOutDir: true,
  },
  server: {
    port: 3002,
    fs: {
      // 允许访问扩展目录
      allow: [
        resolve(__dirname),
        resolve(__dirname, 'extensions'),
      ],
    },
    proxy: {
      // GitFox API
      '/api': {
        target: 'http://localhost:8081',
        changeOrigin: true,
      },
      // OAuth endpoints
      '/oauth': {
        target: 'http://localhost:8081',
        changeOrigin: true,
      },
    },
  },
  plugins: [
    // 提供扩展文件的插件
    {
      name: 'serve-extensions',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url && req.url.startsWith('/-/ide/extensions/')) {
            const extensionPath = req.url.replace('/-/ide/extensions/', '');
            const filePath = resolve(__dirname, 'extensions', extensionPath);
            
            if (fs.existsSync(filePath) && fs.statSync(filePath).isFile()) {
              // 确定 MIME 类型
              let contentType = 'application/javascript';
              if (filePath.endsWith('.json')) {
                contentType = 'application/json';
              } else if (filePath.endsWith('.html')) {
                contentType = 'text/html';
              } else if (filePath.endsWith('.css')) {
                contentType = 'text/css';
              }
              
              res.setHeader('Content-Type', contentType);
              const content = fs.readFileSync(filePath);
              res.end(content);
            } else {
              next();
            }
          } else {
            next();
          }
        });
      },
    },
  ],
  // 为 SPA 添加 HTML5 History API fallback，
  // 所有 /-/ide/ 开头的请求如果找不到文件，返回 index.html
  appType: 'spa',
  resolve: {
    alias: {
      '@gitfox/api-client': resolve(__dirname, 'packages/api-client/src'),
      '@gitfox/oauth-client': resolve(__dirname, 'packages/oauth-client/src'),
      '@gitfox/webide-fs': resolve(__dirname, 'packages/fs/src'),
    },
  },
})
