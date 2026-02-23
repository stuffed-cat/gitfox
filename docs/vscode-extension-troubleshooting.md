# VS Code 扩展不激活问题排查指南

## 问题描述

VS Code 控制台显示 "Found additional builtin location extensions"（发现了扩展），但扩展的 `extension.js` 没有被请求，扩展代码不执行。

## 快速诊断步骤

### Step 1: 检查 package.json 是否可访问

打开浏览器开发者工具 → Network 标签，刷新页面，查找：

```
✅ GET /-/ide/extensions/gitfox-provider/package.json  (应该是 200 OK)
```

如果是 404：
- 检查 Vite 配置是否允许访问 extensions 目录
- 检查路径拼写是否正确

### Step 2: 检查 package.json 的关键字段

使用 curl 或直接访问 URL，查看 package.json 内容：

```bash
curl http://localhost:3002/-/ide/extensions/gitfox-provider/package.json
```

**必须存在的字段：**

```json
{
  "name": "...",
  "publisher": "...",
  "version": "...",
  "engines": { "vscode": "^1.x.x" },
  "browser": "./dist/extension.js",    // ⚠️ 很多人忘记这个
  "activationEvents": ["*"],            // ⚠️ 或其他激活事件
  "contributes": {}                     // ⚠️ 即使为空也要有
}
```

### Step 3: 检查 extension.js 是否被请求

在 Network 标签中查找：

```
✅ GET /-/ide/extensions/gitfox-provider/dist/extension.js  (应该是 200 OK)
```

**如果没有这个请求**，可能的原因：

1. **package.json 缺少 `browser` 字段**
   ```json
   // ❌ 错误：只有 main
   { "main": "./dist/extension.js" }
   
   // ✅ 正确：包含 browser
   { 
     "main": "./dist/extension.js",
     "browser": "./dist/extension.js"
   }
   ```

2. **activationEvents 配置错误**
   ```json
   // ❌ 错误：空数组
   { "activationEvents": [] }
   
   // ✅ 正确：至少一个事件
   { "activationEvents": ["*"] }
   ```

3. **缺少 contributes 字段**
   ```json
   // ❌ 错误：没有 contributes
   { "name": "...", "version": "..." }
   
   // ✅ 正确：包含 contributes（即使为空）
   { 
     "name": "...", 
     "version": "...",
     "contributes": {}
   }
   ```

### Step 4: 检查扩展代码是否正确导出

打开 `src/extension.ts`，确认：

```typescript
// ✅ 正确：直接导出函数
export function activate(context: vscode.ExtensionContext) {
  console.log('Extension activating...');
  // ...
}

export function deactivate() {
  // ...
}

// ❌ 错误：没有导出
function activate(context: vscode.ExtensionContext) { /* ... */ }

// ❌ 错误：使用 default export
export default {
  activate,
  deactivate
};
```

### Step 5: 检查构建配置

查看 `package.json` 的 build 脚本：

```json
{
  "scripts": {
    "build": "esbuild src/extension.ts --bundle --outfile=dist/extension.js --external:vscode --format=esm --platform=browser --target=es2020"
  }
}
```

**关键参数：**
- `--format=esm` ✅ 必须
- `--platform=browser` ✅ 必须
- `--external:vscode` ✅ 必须

运行构建：
```bash
npm run build
```

检查输出：
```bash
ls -lh dist/extension.js  # 应该有文件且大小合理（几 KB）
```

### Step 6: 检查 Vite 服务器配置

在 `vite.config.ts` 中：

```typescript
export default defineConfig({
  server: {
    fs: {
      allow: [
        resolve(__dirname),
        resolve(__dirname, 'extensions'),  // ✅ 必须包含
      ],
    },
  },
  plugins: [
    {
      name: 'serve-extensions',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url?.startsWith('/-/ide/extensions/')) {
            // ✅ 确保这个中间件存在
            // ...
          }
          next();
        });
      },
    },
  ],
});
```

## 常见错误模式

### 模式 1: 扩展被发现但 extension.js 404

**症状：**
- 控制台显示 "Found additional builtin location extensions"
- Network 显示 package.json 200 但 extension.js 404

**原因：**
- package.json 中的 `browser` 字段路径错误
- 或者 extension.js 未构建

**解决：**
```bash
# 1. 检查文件是否存在
ls extensions/gitfox-provider/dist/extension.js

# 2. 如果不存在，构建它
cd extensions/gitfox-provider && npm run build

# 3. 检查 package.json 的 browser 字段
grep "browser" extensions/gitfox-provider/package.json
# 应该输出: "browser": "./dist/extension.js"
```

### 模式 2: extension.js 200 但 activate 不调用

**症状：**
- Network 显示 extension.js 200
- 但控制台没有 "[Extension] Activating..." 日志

**原因：**
- activationEvents 不匹配
- activate 函数导出方式错误
- 构建格式错误

**解决：**

1. 检查 activationEvents：
```json
// 临时设置为 "*" 测试
{ "activationEvents": ["*"] }
```

2. 检查导出：
```typescript
// 必须是命名导出，不是 default export
export function activate(context: vscode.ExtensionContext) { /* ... */ }
```

3. 重新构建（确保使用正确参数）：
```bash
npm run build
```

### 模式 3: 模块加载错误

**症状：**
- 控制台显示 "vscode is not defined" 或类似错误
- 或 "Cannot use import statement"

**原因：**
- 构建时未标记 vscode 为外部依赖
- 或未使用 ESM 格式

**解决：**
```bash
# 确保构建命令包含这些参数
esbuild src/extension.ts --bundle --outfile=dist/extension.js \
  --external:vscode \
  --format=esm \
  --platform=browser
```

## 完整的验证流程

执行以下命令验证配置：

```bash
#!/bin/bash
# 在项目根目录执行

echo "=== 检查扩展目录结构 ==="
ls -la webide/extensions/gitfox-provider/

echo -e "\n=== 检查 package.json ==="
cat webide/extensions/gitfox-provider/package.json | jq '{name, publisher, version, engines, browser, activationEvents, contributes}'

echo -e "\n=== 检查 extension.js 是否存在 ==="
ls -lh webide/extensions/gitfox-provider/dist/extension.js

echo -e "\n=== 构建扩展 ==="
cd webide/extensions/gitfox-provider && npm run build && cd ../../..

echo -e "\n=== 验证构建产物 ==="
ls -lh webide/extensions/gitfox-provider/dist/extension.js

echo -e "\n=== 检查 Vite 配置 ==="
grep -A 5 "allow:" webide/vite.config.ts

echo -e "\n=== 测试扩展文件访问（需要服务器运行） ==="
curl -I http://localhost:3002/-/ide/extensions/gitfox-provider/package.json
curl -I http://localhost:3002/-/ide/extensions/gitfox-provider/dist/extension.js
```

## 最小可工作配置

如果以上都不行，使用这个最小配置开始：

### package.json
```json
{
  "name": "test-extension",
  "publisher": "test",
  "version": "1.0.0",
  "engines": { "vscode": "^1.90.0" },
  "browser": "./extension.js",
  "activationEvents": ["*"],
  "contributes": {}
}
```

### extension.js（手动创建测试）
```javascript
export function activate(context) {
  console.log('[TEST] Extension activated!');
}

export function deactivate() {
  console.log('[TEST] Extension deactivated');
}
```

### bootstrap 配置
```typescript
additionalBuiltinExtensions: [
  {
    scheme: 'http',
    authority: 'localhost:3002',
    path: '/-/ide/extensions/test-extension',
  }
]
```

如果这个最小配置能工作，说明：
- VS Code 加载机制正常
- 服务器配置正常

然后逐步恢复复杂功能，找出导致问题的配置项。

## 调试技巧

### 1. 启用详细日志

在浏览器控制台执行：
```javascript
localStorage.setItem('vscode.log.level', 'trace');
location.reload();
```

### 2. 检查扩展扫描结果

在控制台查找：
```javascript
// 扫描到的扩展
[ExtensionScanner] Found additional builtin location extensions

// 加载的扩展
[ExtensionService] Loading extension: ...

// 激活的扩展
[ExtensionService] Activating extension: ...
```

### 3. 手动测试激活

在控制台执行：
```javascript
// 获取扩展 API
const ext = vscode.extensions.getExtension('test.test-extension');
console.log('Extension:', ext);

// 手动激活
ext?.activate().then(() => console.log('Activated!'));
```

### 4. 检查扩展主机状态

在控制台执行：
```javascript
vscode.extensions.all.forEach(e => {
  console.log(`${e.id}: isActive=${e.isActive}`);
});
```

## 获取帮助

如果以上步骤都无法解决问题，请提供以下信息：

1. **浏览器 Network 标签截图**（显示所有 extension 相关请求）
2. **浏览器控制台完整日志**（从页面加载开始）
3. **package.json 完整内容**
4. **extension.js 文件大小**（`ls -lh dist/extension.js`）
5. **Vite 服务器端日志**

这些信息能帮助快速定位问题。
