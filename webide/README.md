# GitFox WebIDE

基于 VS Code (openvscode-server) 的 Web IDE，与 GitFox 深度集成。

## 架构

```
webide/
├── packages/
│   ├── webide/              # 主入口包 - 提供 start() API
│   ├── bootstrap/           # VS Code Workbench 引导器
│   ├── gitfox-extension/    # GitFox 集成扩展
│   ├── api-client/          # GitFox REST API 客户端
│   ├── oauth-client/        # OAuth2 PKCE 认证
│   └── fs/                  # 虚拟文件系统 (BrowserFS)
├── static/                  # VS Code 静态资源 (构建产物)
└── scripts/                 # 构建脚本
```

## 工作原理

1. **主应用调用**: GitFox 主应用通过 `@gitfox/web-ide` 的 `start()` 方法初始化 WebIDE
2. **OAuth 认证**: 主应用传递 access token，WebIDE 使用它调用 GitFox API
3. **文件系统**: 使用 BrowserFS 实现内存中的虚拟文件系统，支持变更追踪

## 管理员配置

在 Admin > Settings 中可配置：

### VS Code 扩展市场
- 启用/禁用扩展市场
- 配置 Open VSX Registry URL
- 自定义扩展源

### Gitpod 集成
- 启用/禁用 Gitpod 集成
- 配置 Gitpod 实例 URL (gitpod.io 或自托管)

## 开发

```bash
# 安装依赖（会下载vscode静态资源）
npm install

# 开发模式
npm run dev

# 构建生产版本
npm run build
```

## 与 GitLab WebIDE 的区别

- GitLab 使用自己的 VS Code Fork (`gitlab-org/gitlab-web-ide-vscode-fork`)
- GitFox 使用 `@gitpod/openvscode-server` 作为基座
- 默认无后端，配置 Gitpod 集成后可获得完整功能（终端、调试等）
