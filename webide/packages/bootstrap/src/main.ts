/**
 * GitFox WebIDE Bootstrap
 * 
 * 独立 SPA 入口点，运行在 /-/ide/ 路径下
 * 从 URL 解析项目信息，自行处理 OAuth2 认证
 * 
 * URL 格式: /-/ide/project/:owner/:repo/edit/:ref/-/:path
 */

import { GitFoxOAuthClient } from '@gitfox/oauth-client';
import { GitFoxApiClient } from '@gitfox/api-client';

interface ProjectInfo {
  owner: string;
  repo: string;
  ref: string;
  filePath?: string;
}

interface ExtensionConfig {
  enabled: boolean;
  serviceUrl?: string;
  itemUrl?: string;
  resourceUrl?: string;
}

class WebIDEApp {
  private accessToken: string | null = null;
  private projectInfo: ProjectInfo | null = null;
  private apiClient: GitFoxApiClient | null = null;
  private oauthClient: GitFoxOAuthClient;

  constructor() {
    this.oauthClient = new GitFoxOAuthClient({
      gitfoxUrl: window.location.origin,
      clientId: 'gitfox-webide',
      redirectUri: `${window.location.origin}/-/ide/oauth/callback`,
      scopes: ['openid', 'api', 'read_user', 'read_repository', 'write_repository'],
    });
    
    this.init();
  }

  private async init() {
    try {
      // 处理 OAuth 回调 (先检查，因为回调 URL 可能不包含项目信息)
      if (this.oauthClient.isCallback()) {
        await this.handleOAuthCallback();
        return;
      }
      
      // 解析 URL 获取项目信息
      this.projectInfo = this.parseUrl();
      
      if (!this.projectInfo) {
        this.showError('无效的 URL 格式。请使用: /-/ide/project/:owner/:repo/edit/:ref');
        return;
      }
      
      this.showLoading('正在检查服务器配置...');
      
      // 检查 WebIDE 是否启用
      const webideEnabled = await this.checkWebIDEEnabled();
      if (!webideEnabled) {
        this.showWebIDEDisabled();
        return;
      }
      
      this.showLoading('正在检查认证状态...');
      
      // 检查是否已认证
      this.accessToken = this.getStoredToken();
      
      if (!this.accessToken) {
        // 开始 OAuth 流程
        this.startOAuthFlow();
        return;
      }
      
      // 验证 token
      const valid = await this.validateToken();
      if (!valid) {
        this.clearToken();
        this.startOAuthFlow();
        return;
      }
      
      // 初始化 API 客户端
      this.apiClient = new GitFoxApiClient({
        baseUrl: window.location.origin,
        accessToken: this.accessToken,
      });
      
      // 加载 WebIDE
      await this.loadWebIDE();
      
    } catch (error) {
      console.error('WebIDE 初始化失败:', error);
      this.showError('WebIDE 加载失败');
    }
  }

  /**
   * 解析 URL 获取项目信息
   * URL 格式: /-/ide/project/:owner/:repo/edit/:ref/-/:path
   */
  private parseUrl(): ProjectInfo | null {
    const pathname = window.location.pathname;
    
    // 移除 /-/ide 前缀
    const path = pathname.replace(/^\/-\/ide\/?/, '');
    
    // 匹配: project/:owner/:repo/edit/:ref/-/:path
    const match = path.match(/^project\/([^\/]+)\/([^\/]+)\/edit\/([^\/]+)\/-\/(.*)$/);
    
    if (match) {
      return {
        owner: decodeURIComponent(match[1]),
        repo: decodeURIComponent(match[2]),
        ref: decodeURIComponent(match[3]),
        filePath: match[4] ? decodeURIComponent(match[4]) : undefined,
      };
    }
    
    // 也匹配没有文件路径的情况: project/:owner/:repo/edit/:ref
    const matchNoPath = path.match(/^project\/([^\/]+)\/([^\/]+)\/edit\/([^\/]+)\/?$/);
    
    if (matchNoPath) {
      return {
        owner: decodeURIComponent(matchNoPath[1]),
        repo: decodeURIComponent(matchNoPath[2]),
        ref: decodeURIComponent(matchNoPath[3]),
      };
    }
    
    return null;
  }

  private getStoredToken(): string | null {
    return sessionStorage.getItem('webide_access_token');
  }

  private storeToken(token: string) {
    sessionStorage.setItem('webide_access_token', token);
  }

  private clearToken() {
    sessionStorage.removeItem('webide_access_token');
  }

  /**
   * 检查服务器端 WebIDE 是否启用
   */
  private async checkWebIDEEnabled(): Promise<boolean> {
    try {
      const response = await fetch('/api/v1/config');
      if (!response.ok) return false;
      
      const config = await response.json();
      return config.webide_enabled === true;
    } catch {
      return false;
    }
  }

  private async validateToken(): Promise<boolean> {
    if (!this.accessToken) return false;
    
    try {
      // 使用 OIDC UserInfo 端点验证 OAuth access token
      const response = await fetch('/oauth/userinfo', {
        headers: {
          'Authorization': `Bearer ${this.accessToken}`,
        },
      });
      return response.ok;
    } catch {
      return false;
    }
  }

  private startOAuthFlow() {
    this.showLoading('请稍等...');
    
    // 保存当前 URL 用于 OAuth 回调后恢复
    sessionStorage.setItem('webide_return_url', window.location.href);
    
    // 开始标准 OAuth2 PKCE 流程
    this.oauthClient.startAuthorization();
  }

  private async handleOAuthCallback() {
    this.showLoading('正在完成登录...');
    
    try {
      const tokenResponse = await this.oauthClient.handleCallback(window.location.href);
      this.storeToken(tokenResponse.access_token);
      
      // 返回原始 URL
      const returnUrl = sessionStorage.getItem('webide_return_url');
      sessionStorage.removeItem('webide_return_url');
      
      if (returnUrl) {
        window.location.href = returnUrl;
      } else {
        // 如果没有返回 URL，回到 IDE 首页
        window.location.href = '/-/ide/';
      }
    } catch (err) {
      console.error('OAuth 回调处理失败:', err);
      this.showError(`登录失败: ${err instanceof Error ? err.message : '未知错误'}`);
    }
  }

  private async loadWebIDE() {
    if (!this.projectInfo) return;
    
    this.showLoading(`正在加载 ${this.projectInfo.owner}/${this.projectInfo.repo}...`);
    
    // 加载扩展配置
    const extensionConfig = await this.loadExtensionConfig();
    if (extensionConfig?.enabled) {
      this.configureExtensionGallery(extensionConfig);
    }
    
    // 加载 VS Code workbench
    await this.loadWorkbench();
    
    // 初始化文件系统
    await this.initializeFileSystem();
    
    // 如果有指定文件，打开它
    if (this.projectInfo.filePath) {
      this.openFile(this.projectInfo.filePath);
    }
    
    // 隐藏加载器
    this.hideLoader();
  }

  private async loadExtensionConfig(): Promise<ExtensionConfig | null> {
    try {
      const response = await fetch('/api/v1/admin/system-configs', {
        headers: {
          'Authorization': `Bearer ${this.accessToken}`,
        },
      });
      
      if (!response.ok) return null;
      
      const configs = await response.json();
      const enabled = configs.find((c: any) => c.key === 'vscode_extensions_enabled')?.value === 'true';
      
      if (!enabled) return null;
      
      return {
        enabled: true,
        serviceUrl: configs.find((c: any) => c.key === 'vscode_marketplace_service_url')?.value,
        itemUrl: configs.find((c: any) => c.key === 'vscode_marketplace_item_url')?.value,
        resourceUrl: configs.find((c: any) => c.key === 'vscode_marketplace_resource_url')?.value,
      };
    } catch {
      return null;
    }
  }

  private configureExtensionGallery(config: ExtensionConfig) {
    (window as any).__GITFOX_EXTENSION_GALLERY__ = {
      serviceUrl: config.serviceUrl || 'https://open-vsx.org/vscode/gallery',
      itemUrl: config.itemUrl || 'https://open-vsx.org/vscode/item',
      resourceUrlTemplate: config.resourceUrl || 
        'https://open-vsx.org/vscode/unpkg/{publisher}/{name}/{version}/{path}'
    };
  }

  private async loadWorkbench(): Promise<void> {
    // Fetch workbench.html 并替换模板变量
    const response = await fetch('/-/ide/vscode/out/vs/code/browser/workbench/workbench.html');
    let html = await response.text();
    
    const baseUrl = '/-/ide/vscode';
    
    // 使用 OIDC UserInfo 端点获取用户信息（OAuth access token）
    const userInfoResponse = await fetch('/oauth/userinfo', {
      headers: {
        'Authorization': `Bearer ${this.accessToken}`,
      },
    });
    
    if (!userInfoResponse.ok) {
      throw new Error('Failed to fetch user info');
    }
    
    const userInfo = await userInfoResponse.json();
    
    const config = {
      folderUri: { 
        scheme: 'gitfox', 
        authority: this.projectInfo!.owner, 
        path: `/${this.projectInfo!.repo}` 
      },
      workspaceProvider: {
        payload: btoa(JSON.stringify({
          owner: this.projectInfo!.owner,
          repo: this.projectInfo!.repo,
          ref: this.projectInfo!.ref,
        }))
      },
      // 加载 GitFox 内置扩展
      additionalBuiltinExtensions: [{
        scheme: 'http',
        path: '/-/ide/extensions/gitfox-provider'
      }],
      productConfiguration: {
        extensionsGallery: (window as any).__GITFOX_EXTENSION_GALLERY__,
        nameShort: 'GitFox IDE',
        nameLong: 'GitFox Web IDE',
      }
    };
    
    // 在 window 上暴露配置给扩展
    (window as any).__GITFOX_CONFIG__ = {
      accessToken: this.accessToken,
      projectInfo: this.projectInfo,
      userInfo: {
        id: parseInt(userInfo.sub),
        username: userInfo.preferred_username,
        name: userInfo.name,
        email: userInfo.email,
      },
      apiClient: this.apiClient,
    };
    
    // 替换模板变量
    html = html.replace(/\{\{WORKBENCH_WEB_BASE_URL\}\}/g, baseUrl);
    html = html.replace(/\{\{WORKBENCH_WEB_CONFIGURATION\}\}/g, JSON.stringify(config).replace(/"/g, '&quot;'));
    html = html.replace(/\{\{WORKBENCH_AUTH_SESSION\}\}/g, '');
    html = html.replace(/\{\{WORKBENCH_NLS_FALLBACK_URL\}\}/g, `${baseUrl}/out/nls.messages.js`);
    html = html.replace(/\{\{WORKBENCH_NLS_URL\}\}/g, `${baseUrl}/out/nls.messages.zh-cn.js`);
    
    // 替换整个文档
    document.open();
    document.write(html);
    document.close();
  }

  private async initializeFileSystem(): Promise<void> {
    if (!this.projectInfo || !this.accessToken) return;
    
    // 初始化虚拟文件系统
    // 从 GitFox API 获取仓库树
    const { owner, repo, ref } = this.projectInfo;
    
    // 文件系统提供者将被挂载并供 VS Code 使用
    console.log(`初始化文件系统: ${owner}/${repo}@${ref}`);
  }

  private openFile(path: string) {
    // 在 VS Code 编辑器中打开文件
    console.log(`打开文件: ${path}`);
  }

  private showLoading(message: string) {
    const loader = document.getElementById('loader');
    if (loader) {
      loader.innerHTML = `
        <div class="loader-spinner"></div>
        <div class="loader-text">${message}</div>
      `;
      loader.style.display = 'flex';
    }
  }

  private hideLoader() {
    const loader = document.getElementById('loader');
    if (loader) {
      loader.style.display = 'none';
    }
  }

  private showError(message: string) {
    const loader = document.getElementById('loader');
    if (loader) {
      loader.innerHTML = `
        <div class="loader-error">
          <svg viewBox="0 0 24 24" width="48" height="48">
            <path fill="currentColor" d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>
          </svg>
          <div>${message}</div>
          <button onclick="location.reload()">重试</button>
        </div>
      `;
    }
  }

  /**
   * 显示 WebIDE 未启用的提示页面
   */
  private showWebIDEDisabled() {
    const loader = document.getElementById('loader');
    if (loader) {
      loader.innerHTML = `
        <div class="loader-error">
          <svg viewBox="0 0 24 24" width="48" height="48">
            <path fill="currentColor" d="M13,13H11V7H13M13,17H11V15H13M12,2A10,10 0 0,0 2,12A10,10 0 0,0 12,22A10,10 0 0,0 22,12A10,10 0 0,0 12,2Z"/>
          </svg>
          <div style="font-size: 1.2em; margin-bottom: 0.5em;">WebIDE 未启用</div>
          <div style="color: #666; margin-bottom: 1em;">此功能需要管理员在系统设置中启用</div>
          <button onclick="window.location.href='/'" style="margin-right: 0.5em;">返回首页</button>
          <button onclick="window.history.back()">返回上一页</button>
        </div>
      `;
    }
  }

  // 公开 API 供文件系统和扩展使用
  public getAccessToken(): string | null {
    return this.accessToken;
  }

  public getProjectInfo(): ProjectInfo | null {
    return this.projectInfo;
  }

  public getApiClient(): GitFoxApiClient | null {
    return this.apiClient;
  }
}

// 全局实例
(window as any).__GITFOX_WEBIDE__ = new WebIDEApp();

export {};
