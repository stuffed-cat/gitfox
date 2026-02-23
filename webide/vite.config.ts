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
    // 提供 VS Code 静态文件
    {
      name: 'serve-vscode',
      enforce: 'pre',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url && req.url.startsWith('/vscode/')) {
            const filePath = req.url.split('?')[0];
            const fullPath = resolve(__dirname, 'static', filePath);
            
            if (fs.existsSync(fullPath) && fs.statSync(fullPath).isFile()) {
              let contentType = 'application/octet-stream';
              if (fullPath.endsWith('.js')) {
                contentType = 'application/javascript; charset=utf-8';
              } else if (fullPath.endsWith('.json')) {
                contentType = 'application/json; charset=utf-8';
              } else if (fullPath.endsWith('.css')) {
                contentType = 'text/css; charset=utf-8';
              } else if (fullPath.endsWith('.html')) {
                contentType = 'text/html; charset=utf-8';
              } else if (fullPath.endsWith('.ttf') || fullPath.endsWith('.woff') || fullPath.endsWith('.woff2')) {
                contentType = 'font/ttf';
              }
              
              res.setHeader('Content-Type', contentType);
              res.setHeader('Cache-Control', 'public, max-age=31536000');
              res.end(fs.readFileSync(fullPath));
              return;
            }
            
            res.statusCode = 404;
            res.end('Not found');
            return;
          }
          next();
        });
      },
    },
    // 修复 MIME 类型 - 必须最先执行
    {
      name: 'fix-mime-types',
      enforce: 'pre',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url && req.url.endsWith('.js')) {
            res.setHeader('Content-Type', 'application/javascript; charset=utf-8');
          } else if (req.url && req.url.endsWith('.json')) {
            res.setHeader('Content-Type', 'application/json; charset=utf-8');
          }
          next();
        });
      },
    },
    // 提供扩展文件的插件 - 必须在 Vite 内部中间件之前注册
    {
      name: 'serve-extensions',
      enforce: 'pre',
      configureServer(server) {
        // 返回函数会在 Vite 内部中间件之后注册
        // 但直接调用 server.middlewares.use 会在内部中间件之前注册
        server.middlewares.use((req, res, next) => {
          if (req.url && req.url.startsWith('/-/ide/extensions/')) {
            console.log(`[serve-extensions] Request: ${req.method} ${req.url}`);
            const extensionPath = req.url.replace('/-/ide/extensions/', '');
            const filePath = resolve(__dirname, 'extensions', extensionPath);
            
            // 如果请求的是目录（没有扩展名），返回 package.json
            if (fs.existsSync(filePath)) {
              const stat = fs.statSync(filePath);
              if (stat.isDirectory()) {
                const packageJsonPath = resolve(filePath, 'package.json');
                if (fs.existsSync(packageJsonPath)) {
                  console.log(`[serve-extensions] Serving package.json for ${extensionPath}`);
                  res.setHeader('Content-Type', 'application/json');
                  res.setHeader('Access-Control-Allow-Origin', '*');
                  const content = fs.readFileSync(packageJsonPath);
                  res.end(content);
                  return;
                }
              } else if (stat.isFile()) {
                let contentType = 'application/javascript';
                if (filePath.endsWith('.json')) {
                  contentType = 'application/json';
                } else if (filePath.endsWith('.html')) {
                  contentType = 'text/html';
                } else if (filePath.endsWith('.css')) {
                  contentType = 'text/css';
                }
                
                console.log(`[serve-extensions] Serving file ${extensionPath} as ${contentType}`);
                res.setHeader('Content-Type', contentType);
                res.setHeader('Access-Control-Allow-Origin', '*');
                const content = fs.readFileSync(filePath);
                res.end(content);
                return;
              }
            }
            
            console.log(`[serve-extensions] Not found: ${extensionPath}`);
            res.statusCode = 404;
            res.end('Not found');
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
