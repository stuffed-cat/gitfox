/**
 * GitFox VS Code Extension
 * 注册 gitfox:// FileSystemProvider 和 GitFox AuthenticationProvider
 */

import * as vscode from 'vscode';

interface GitFoxConfig {
  accessToken: string;
  projectInfo: {
    owner: string;
    repo: string;
    ref: string;
  };
  userInfo: {
    id: number;
    username: string;
    name: string;
    email: string;
  };
}

interface TreeEntry {
  name: string;
  type: 'blob' | 'tree';
}

interface FileContent {
  encoding: 'base64' | 'text';
  content: string;
}

export function activate(context: vscode.ExtensionContext) {
  void activateInternal(context);
}

async function activateInternal(context: vscode.ExtensionContext): Promise<void> {
  console.log('[GitFox] Extension activating...');

  const config = await resolveConfig();
  if (!config) {
    console.error('[GitFox] Config not found from runtime globals/storage/url');
    return;
  }

  console.log('[GitFox] Config loaded:', {
    user: config.userInfo.username,
    project: `${config.projectInfo.owner}/${config.projectInfo.repo}`,
    ref: config.projectInfo.ref
  });

  // 1. 注册认证提供者
  const authProvider = new GitFoxAuthProvider(config);
  context.subscriptions.push(
    vscode.authentication.registerAuthenticationProvider(
      'gitfox',
      'GitFox',
      authProvider,
      { supportsMultipleAccounts: false }
    )
  );
  console.log('[GitFox] Authentication provider registered');

  // 2. 注册文件系统提供者
  const fsProvider = new GitFoxFileSystemProvider(config);
  context.subscriptions.push(
    vscode.workspace.registerFileSystemProvider('gitfox', fsProvider, {
      isCaseSensitive: true,
      isReadonly: false,
    })
  );
  console.log('[GitFox] File system provider registered for gitfox://');

  // 在扩展激活后，打开 gitfox:// 工作区
  const folderUri = vscode.Uri.parse(`gitfox://${config.projectInfo.owner}/${config.projectInfo.repo}`);
  console.log('[GitFox] Opening workspace:', folderUri.toString());
  
  // 使用 updateWorkspaceFolders 添加工作区
  vscode.workspace.updateWorkspaceFolders(0, 0, { uri: folderUri });

  console.log('[GitFox] Extension activated successfully');
}

export function deactivate() {
  console.log('[GitFox] Extension deactivated');
}

function parseProjectInfoFromUrl(url: string): GitFoxConfig['projectInfo'] | null {
  try {
    const parsed = new URL(url);
    const path = parsed.pathname.replace(/^\/-\/ide\/?/, '');
    const match = path.match(/^project\/([^\/]+)\/([^\/]+)\/edit\/([^\/]+)(?:\/|$)/);

    if (!match) {
      return null;
    }

    return {
      owner: decodeURIComponent(match[1]),
      repo: decodeURIComponent(match[2]),
      ref: decodeURIComponent(match[3]),
    };
  } catch {
    return null;
  }
}

function loadConfigFromStorage(): Partial<GitFoxConfig> | null {
  const key = 'gitfox.webide.config';

  try {
    if (typeof localStorage !== 'undefined') {
      const raw = localStorage.getItem(key);
      if (raw) {
        return JSON.parse(raw) as Partial<GitFoxConfig>;
      }
    }
  } catch {
  }

  try {
    if (typeof sessionStorage !== 'undefined') {
      const raw = sessionStorage.getItem(key);
      if (raw) {
        return JSON.parse(raw) as Partial<GitFoxConfig>;
      }
    }
  } catch {
  }

  return null;
}

async function loadConfigFromIndexedDb(): Promise<Partial<GitFoxConfig> | null> {
  try {
    if (typeof indexedDB === 'undefined') {
      return null;
    }

    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('gitfox-webide', 1);
      request.onupgradeneeded = () => {
        const database = request.result;
        if (!database.objectStoreNames.contains('runtime')) {
          database.createObjectStore('runtime');
        }
      };
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });

    const value = await new Promise<unknown>((resolve, reject) => {
      const tx = db.transaction('runtime', 'readonly');
      const store = tx.objectStore('runtime');
      const getRequest = store.get('gitfox.config');
      getRequest.onsuccess = () => resolve(getRequest.result);
      getRequest.onerror = () => reject(getRequest.error);
      tx.onabort = () => reject(tx.error);
    });

    db.close();

    if (value && typeof value === 'object') {
      return value as Partial<GitFoxConfig>;
    }
  } catch {
  }

  return null;
}

function getStoredAccessToken(): string | null {
  try {
    if (typeof sessionStorage !== 'undefined') {
      const token = sessionStorage.getItem('webide_access_token');
      if (token) {
        return token;
      }
    }
  } catch {
  }

  const storedConfig = loadConfigFromStorage();
  if (storedConfig?.accessToken) {
    return storedConfig.accessToken;
  }

  return null;
}

async function fetchUserInfo(accessToken: string): Promise<GitFoxConfig['userInfo'] | null> {
  try {
    const response = await fetch('/oauth/userinfo', {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });

    if (!response.ok) {
      return null;
    }

    const userInfo = await response.json();
    return {
      id: parseInt(userInfo.sub, 10),
      username: userInfo.preferred_username,
      name: userInfo.name,
      email: userInfo.email,
    };
  } catch {
    return null;
  }
}

async function resolveConfig(): Promise<GitFoxConfig | null> {
  const runtimeConfig = (globalThis as any).__GITFOX_CONFIG__ as GitFoxConfig | undefined;
  if (runtimeConfig?.accessToken && runtimeConfig?.projectInfo) {
    return runtimeConfig;
  }

  const maxAttempts = 5;
  const delayMs = 200;

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const indexedDbConfig = await loadConfigFromIndexedDb();
      const storageConfig = loadConfigFromStorage();
      const storedConfig = indexedDbConfig || storageConfig;

      const accessToken = getStoredAccessToken() || storedConfig?.accessToken || null;
      if (!accessToken) {
        throw new Error('no accessToken in storage');
      }

      const projectInfo =
        storedConfig?.projectInfo ||
        (typeof location !== 'undefined' ? parseProjectInfoFromUrl(location.href) : null) ||
        (typeof document !== 'undefined' && document.referrer ? parseProjectInfoFromUrl(document.referrer) : null);

      if (!projectInfo) {
        throw new Error('no projectInfo from storage/url');
      }

      const userInfo = (storedConfig?.userInfo as GitFoxConfig['userInfo'] | undefined) || (await fetchUserInfo(accessToken));
      if (!userInfo) {
        throw new Error('userInfo fetch failed');
      }

      return {
        accessToken,
        projectInfo,
        userInfo,
      };
    } catch (err) {
      console.error(`[GitFox] Config resolve attempt ${attempt}/${maxAttempts} failed:`, err);
      if (attempt < maxAttempts) {
        await new Promise((r) => setTimeout(r, delayMs));
      }
    }
  }

  return null;
}

/**
 * GitFox 认证提供者
 */
class GitFoxAuthProvider implements vscode.AuthenticationProvider {
  private _onDidChangeSessions = new vscode.EventEmitter<vscode.AuthenticationProviderAuthenticationSessionsChangeEvent>();
  readonly onDidChangeSessions = this._onDidChangeSessions.event;

  private _session: vscode.AuthenticationSession;

  constructor(private config: GitFoxConfig) {
    this._session = {
      id: `gitfox-${config.userInfo.id}`,
      accessToken: config.accessToken,
      account: {
        id: config.userInfo.id.toString(),
        label: config.userInfo.username,
      },
      scopes: ['api', 'read_user', 'read_repository', 'write_repository'],
    };
  }

  async getSessions(scopes?: string[]): Promise<vscode.AuthenticationSession[]> {
    // 简单实现：总是返回当前会话
    return [this._session];
  }

  async createSession(scopes: string[]): Promise<vscode.AuthenticationSession> {
    return this._session;
  }

  async removeSession(sessionId: string): Promise<void> {
    // 不支持移除会话
    throw new Error('Session removal not supported in GitFox WebIDE');
  }
}

/**
 * GitFox 文件系统提供者
 */
class GitFoxFileSystemProvider implements vscode.FileSystemProvider {
  private _emitter = new vscode.EventEmitter<vscode.FileChangeEvent[]>();
  readonly onDidChangeFile = this._emitter.event;

  private fileCache = new Map<string, Uint8Array>();
  private dirCache = new Map<string, [string, vscode.FileType][]>();

  constructor(private config: GitFoxConfig) {}

  private async apiFetch<T>(endpoint: string): Promise<T> {
    const response = await fetch(`/api/v1${endpoint}`, {
      headers: {
        Authorization: `Bearer ${this.config.accessToken}`,
        'Content-Type': 'application/json',
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`API Error ${response.status}: ${errorText}`);
    }

    return response.json();
  }

  watch(): vscode.Disposable {
    return new vscode.Disposable(() => {});
  }

  async stat(uri: vscode.Uri): Promise<vscode.FileStat> {
    console.log('[GitFox] stat:', uri.toString());
    
    const path = uri.path;
    if (path === '/' || path === '') {
      return {
        type: vscode.FileType.Directory,
        ctime: 0,
        mtime: 0,
        size: 0,
      };
    }

    // 检查缓存
    if (this.fileCache.has(path)) {
      return {
        type: vscode.FileType.File,
        ctime: 0,
        mtime: Date.now(),
        size: this.fileCache.get(path)!.byteLength,
      };
    }

    // TODO: 从 API 获取文件信息
    return {
      type: vscode.FileType.File,
      ctime: 0,
      mtime: 0,
      size: 0,
    };
  }

  async readDirectory(uri: vscode.Uri): Promise<[string, vscode.FileType][]> {
    console.log('[GitFox] readDirectory:', uri.toString());
    
    const path = uri.path === '/' ? '' : uri.path.replace(/^\//, '');
    
    // 检查缓存
    if (this.dirCache.has(path)) {
      return this.dirCache.get(path)!;
    }

    // 从 API 获取目录内容
    try {
      const { projectInfo } = this.config;
      const projectPath = `${projectInfo.owner}/${projectInfo.repo}`;

      const params = new URLSearchParams({
        ref: projectInfo.ref,
        ...(path && { path }),
      });

      const entries = await this.apiFetch<TreeEntry[]>(
        `/projects/${encodeURIComponent(projectPath)}/repository/tree?${params.toString()}`
      );

      const result: [string, vscode.FileType][] = entries.map((entry: any) => [
        entry.name,
        entry.type === 'tree' ? vscode.FileType.Directory : vscode.FileType.File
      ]);

      this.dirCache.set(path, result);
      console.log(`[GitFox] readDirectory result: ${result.length} entries`);
      return result;
    } catch (error) {
      console.error('[GitFox] readDirectory error:', error);
      throw vscode.FileSystemError.FileNotFound(uri);
    }
  }

  async readFile(uri: vscode.Uri): Promise<Uint8Array> {
    console.log('[GitFox] readFile:', uri.toString());
    
    const path = uri.path.replace(/^\//, '');

    // 检查缓存
    if (this.fileCache.has(path)) {
      return this.fileCache.get(path)!;
    }

    // 从 API 获取文件内容
    try {
      const { projectInfo } = this.config;
      const projectPath = `${projectInfo.owner}/${projectInfo.repo}`;

      const params = new URLSearchParams({ ref: projectInfo.ref });
      const fileContent = await this.apiFetch<FileContent>(
        `/projects/${encodeURIComponent(projectPath)}/repository/files/${encodeURIComponent(path)}?${params.toString()}`
      );

      // 解码内容
      const content = fileContent.encoding === 'base64'
        ? Uint8Array.from(atob(fileContent.content), c => c.charCodeAt(0))
        : new TextEncoder().encode(fileContent.content);

      this.fileCache.set(path, content);
      console.log(`[GitFox] readFile success: ${content.byteLength} bytes`);
      return content;
    } catch (error) {
      console.error('[GitFox] readFile error:', error);
      throw vscode.FileSystemError.FileNotFound(uri);
    }
  }

  async writeFile(
    uri: vscode.Uri,
    content: Uint8Array,
    options: { create: boolean; overwrite: boolean }
  ): Promise<void> {
    console.log('[GitFox] writeFile:', uri.toString());
    
    const path = uri.path.replace(/^\//, '');
    this.fileCache.set(path, content);

    this._emitter.fire([{
      type: vscode.FileChangeType.Changed,
      uri
    }]);

    // TODO: 标记为待提交的更改
  }

  async delete(uri: vscode.Uri): Promise<void> {
    console.log('[GitFox] delete:', uri.toString());
    
    const path = uri.path.replace(/^\//, '');
    this.fileCache.delete(path);

    this._emitter.fire([{
      type: vscode.FileChangeType.Deleted,
      uri
    }]);
  }

  async rename(oldUri: vscode.Uri, newUri: vscode.Uri): Promise<void> {
    console.log('[GitFox] rename:', oldUri.toString(), '->', newUri.toString());
    
    const oldPath = oldUri.path.replace(/^\//, '');
    const newPath = newUri.path.replace(/^\//, '');
    
    const content = this.fileCache.get(oldPath);
    if (content) {
      this.fileCache.set(newPath, content);
      this.fileCache.delete(oldPath);
    }

    this._emitter.fire([
      { type: vscode.FileChangeType.Deleted, uri: oldUri },
      { type: vscode.FileChangeType.Created, uri: newUri }
    ]);
  }

  async createDirectory(uri: vscode.Uri): Promise<void> {
    console.log('[GitFox] createDirectory:', uri.toString());
    // 目录通过写入文件隐式创建
  }
}
