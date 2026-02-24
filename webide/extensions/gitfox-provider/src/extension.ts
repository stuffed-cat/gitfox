/**
 * GitFox VS Code Extension
 * 注册 gitfox:// FileSystemProvider 和 GitFox AuthenticationProvider
 */

import * as vscode from 'vscode';

interface GitFoxConfig {
  accessToken: string;
  /** API 基础 URL（含 origin），如 http://localhost:8080 */
  apiBaseUrl: string;
  /** 页面完整 URL，由 main.ts 在主线程传入（扩展在 Web Worker 里无法直接读取页面 URL） */
  pageUrl?: string;
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
  path: string;
  entry_type: 'File' | 'Directory' | 'Submodule' | 'Symlink';
  size: number | null;
  mode: number;
}

interface FileContent {
  path: string;
  content: string;
  encoding: string;
  is_binary: boolean;
  size: number;
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

  // 解析初始文件路径：必须用 main.ts 传入的页面 URL，
  // 因为扩展运行在 Web Worker 里，Worker 的 location.href 是 worker 脚本 URL，不是页面 URL
  const pageUrl = config.pageUrl ?? '';
  const initialFilePath = pageUrl ? parseInitialFilePathFromUrl(pageUrl) : null;
  console.log('[GitFox] Page URL:', pageUrl);
  console.log('[GitFox] Initial file path:', initialFilePath);

  // 工作区文件夹已由 main.ts 在 workbench config 中通过 folderUri 设置，
  // 此处不再调用 updateWorkspaceFolders，避免引入多根工作区或刷新循环。
  const { owner, repo } = config.projectInfo;
  const folderUri = vscode.Uri.parse(`gitfox://${owner}/${repo}`);
  console.log('[GitFox] Workspace folder (from main.ts config):', folderUri.toString());

  // 等待 FileSystemProvider 就绪（最多 8 秒），然后打开初始文件
  void waitForFsReady(folderUri).then(ready => {
    if (!ready) {
      console.warn('[GitFox] FS provider not ready in time');
      return;
    }
    void vscode.commands.executeCommand('workbench.view.explorer');
    if (initialFilePath) {
      const fileUri = vscode.Uri.parse(`gitfox://${owner}/${repo}/${initialFilePath}`);
      console.log('[GitFox] Opening initial file:', fileUri.toString());
      void vscode.window.showTextDocument(fileUri, { preview: false }).then(
        () => console.log('[GitFox] Initial file opened'),
        err => console.error('[GitFox] Failed to open initial file:', err),
      );
    }
  });

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

/**
 * 从 WebIDE URL 中提取初始要打开的文件路径
 * URL 格式: /-/ide/project/{owner}/{repo}/edit/{ref}/-/{filepath}
 */
function parseInitialFilePathFromUrl(url: string): string | null {
  try {
    const parsed = new URL(url);
    const rawPath = parsed.pathname.replace(/^\/-\/ide\/?/, '');
    // 匹配 edit/{ref}/-/{filepath}
    const match = rawPath.match(/^project\/[^\/]+\/[^\/]+\/edit\/[^\/]+\/-\/(.+)$/);
    if (!match) {
      return null;
    }
    const filePath = decodeURIComponent(match[1]).replace(/\/+$/, '');
    return filePath || null;
  } catch {
    return null;
  }
}

/**
 * 轮询直到 gitfox FileSystemProvider 能够响应 stat 请求（最多等 maxMs 毫秒）
 */
async function waitForFsReady(uri: vscode.Uri, maxMs = 8000): Promise<boolean> {
  const start = Date.now();
  while (Date.now() - start < maxMs) {
    try {
      await vscode.workspace.fs.stat(uri);
      return true;
    } catch {
      await new Promise<void>(r => setTimeout(r, 300));
    }
  }
  return false;
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

async function fetchUserInfo(accessToken: string, baseUrl: string): Promise<GitFoxConfig['userInfo'] | null> {
  try {
    const response = await fetch(`${baseUrl}/oauth/userinfo`, {
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
    // 如果没有 apiBaseUrl，尝试从 location 推断
    if (!runtimeConfig.apiBaseUrl) {
      runtimeConfig.apiBaseUrl = typeof location !== 'undefined' ? location.origin : '';
    }
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

      // apiBaseUrl: 优先从存储的配置读取，否则从 location.origin 推断
      const apiBaseUrl =
        (storedConfig as any)?.apiBaseUrl ||
        (typeof location !== 'undefined' ? location.origin : '') ||
        '';

      const userInfo = (storedConfig?.userInfo as GitFoxConfig['userInfo'] | undefined) || (await fetchUserInfo(accessToken, apiBaseUrl));
      if (!userInfo) {
        throw new Error('userInfo fetch failed');
      }

      const pageUrl = (storedConfig as any)?.pageUrl as string | undefined;

      return {
        accessToken,
        apiBaseUrl,
        pageUrl,
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
  /** 正在进行中的目录请求，用于合并并发请求，避免重复 API 调用 */
  private dirFetchInFlight = new Map<string, Promise<[string, vscode.FileType][]>>();
  /** 已确认不存在的路径（负缓存），避免对 404 路径重复请求 */
  private notFoundCache = new Set<string>();

  constructor(private config: GitFoxConfig) {}

  /**
   * 从 URI 提取相对于 repo 根目录的文件路径。
   *
   * URI 格式: gitfox://{owner}/{repo}/{...path}
   *   uri.authority = owner
   *   uri.path = '/{repo}/{...path}'
   *
   * 映射规则（全部映射到 repo 内的相对路径）：
   *   path = '' | '/'            → '' （repo 根）
   *   path = '/{repo}'           → '' （repo 根）
   *   path = '/{repo}/{file}'    → '{file}'
   *   path = '/{anything}'      → '{anything}' （fallback，去掉 '/')
   */
  private getFilePath(uri: vscode.Uri): string {
    const repo = this.config.projectInfo.repo;
    const prefix = `/${repo}/`;
    const rootPath = `/${repo}`;

    if (uri.path === '' || uri.path === '/' || uri.path === rootPath || uri.path === rootPath + '/') {
      return '';
    } else if (uri.path.startsWith(prefix)) {
      return uri.path.slice(prefix.length);
    } else {
      return uri.path.replace(/^\//, '');
    }
  }

  private async apiFetch<T>(endpoint: string): Promise<T> {
    const url = `${this.config.apiBaseUrl}/api/v1${endpoint}`;
    console.log('[GitFox] apiFetch:', url);
    const response = await fetch(url, {
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

  /**
   * 从 API 获取目录内容并写入缓存，返回条目列表
   * 已缓存直接返回；并发请求同一路径时共享同一个 Promise，避免重复 API 调用
   */
  private async fetchDirectory(dirPath: string): Promise<[string, vscode.FileType][]> {
    if (this.dirCache.has(dirPath)) {
      return this.dirCache.get(dirPath)!;
    }

    // 负缓存：已知不存在的路径直接抛错
    if (this.notFoundCache.has(dirPath)) {
      throw new Error(`Not found (cached): ${dirPath}`);
    }

    // 如果已有进行中的请求，复用同一个 Promise
    if (this.dirFetchInFlight.has(dirPath)) {
      return this.dirFetchInFlight.get(dirPath)!;
    }

    const fetchPromise = (async (): Promise<[string, vscode.FileType][]> => {
      try {
        const { projectInfo } = this.config;
        const apiBase = `/projects/${projectInfo.owner}/${projectInfo.repo}`;
        const params = new URLSearchParams({
          ref_name: projectInfo.ref,
          ...(dirPath && { path: dirPath }),
        });

        const entries = await this.apiFetch<TreeEntry[]>(
          `${apiBase}/repository/tree?${params.toString()}`
        );

        const result: [string, vscode.FileType][] = entries.map((entry: TreeEntry) => [
          entry.name,
          entry.entry_type === 'Directory' || entry.entry_type === 'Submodule'
            ? vscode.FileType.Directory
            : vscode.FileType.File,
        ]);

        this.dirCache.set(dirPath, result);
        console.log(`[GitFox] fetchDirectory(${dirPath || '/'}) → ${result.length} entries`);
        return result;
      } catch (err) {
        // 记录到负缓存，避免重复请求不存在的路径
        this.notFoundCache.add(dirPath);
        throw err;
      } finally {
        this.dirFetchInFlight.delete(dirPath);
      }
    })();

    this.dirFetchInFlight.set(dirPath, fetchPromise);
    return fetchPromise;
  }

  async stat(uri: vscode.Uri): Promise<vscode.FileStat> {
    console.log('[GitFox] stat:', uri.toString());

    const filePath = this.getFilePath(uri);

    // repo 根目录（包括虚拟父根 '/' 以及 '/{repo}'）始终是 Directory
    if (filePath === '') {
      return { type: vscode.FileType.Directory, ctime: 0, mtime: 0, size: 0 };
    }

    // 检查文件缓存
    if (this.fileCache.has(filePath)) {
      return {
        type: vscode.FileType.File,
        ctime: 0,
        mtime: Date.now(),
        size: this.fileCache.get(filePath)!.byteLength,
      };
    }

    // 检查目录缓存（说明这个路径本身是个已知目录）
    if (this.dirCache.has(filePath)) {
      return { type: vscode.FileType.Directory, ctime: 0, mtime: 0, size: 0 };
    }

    // 加载父目录（可能触发 API 请求），然后从条目中查类型
    const lastSlash = filePath.lastIndexOf('/');
    const parentPath = lastSlash >= 0 ? filePath.substring(0, lastSlash) : '';
    const baseName = lastSlash >= 0 ? filePath.substring(lastSlash + 1) : filePath;

    try {
      const parentEntries = await this.fetchDirectory(parentPath);
      const entry = parentEntries.find(([name]) => name === baseName);
      if (entry) {
        return { type: entry[1], ctime: 0, mtime: 0, size: 0 };
      }
      // 父目录中没有这个条目 → 确实不存在
      throw vscode.FileSystemError.FileNotFound(uri);
    } catch (err) {
      if (err instanceof vscode.FileSystemError) {
        throw err;
      }
      // 父目录加载失败（网络错误等），返回 File 作为降级，让 readFile 自行决定
      console.warn('[GitFox] stat: parent dir fetch failed, defaulting to File:', err);
      return { type: vscode.FileType.File, ctime: 0, mtime: 0, size: 0 };
    }
  }

  async readDirectory(uri: vscode.Uri): Promise<[string, vscode.FileType][]> {
    console.log('[GitFox] readDirectory:', uri.toString());

    const filePath = this.getFilePath(uri);

    try {
      const result = await this.fetchDirectory(filePath);
      console.log(`[GitFox] readDirectory result: ${result.length} entries`);
      return result;
    } catch (error) {
      console.error('[GitFox] readDirectory error:', error);
      throw vscode.FileSystemError.FileNotFound(uri);
    }
  }

  async readFile(uri: vscode.Uri): Promise<Uint8Array> {
    console.log('[GitFox] readFile:', uri.toString());

    const filePath = this.getFilePath(uri);

    // 检查缓存（stat 预缓存或上次读取的内容）
    if (this.fileCache.has(filePath)) {
      return this.fileCache.get(filePath)!;
    }

    // 从 API 获取文件内容
    try {
      const { projectInfo } = this.config;
      const apiBase = `/projects/${projectInfo.owner}/${projectInfo.repo}`;

      const params = new URLSearchParams({ ref_name: projectInfo.ref });
      const fileContent = await this.apiFetch<FileContent>(
        `${apiBase}/repository/files/${filePath}?${params.toString()}`
      );

      // 解码内容
      const content = fileContent.encoding === 'base64'
        ? Uint8Array.from(atob(fileContent.content), c => c.charCodeAt(0))
        : new TextEncoder().encode(fileContent.content);

      this.fileCache.set(filePath, content);
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

    const filePath = this.getFilePath(uri);
    this.fileCache.set(filePath, content);

    this._emitter.fire([{
      type: vscode.FileChangeType.Changed,
      uri
    }]);

    // TODO: 标记为待提交的更改
  }

  async delete(uri: vscode.Uri): Promise<void> {
    console.log('[GitFox] delete:', uri.toString());

    const filePath = this.getFilePath(uri);
    this.fileCache.delete(filePath);

    this._emitter.fire([{
      type: vscode.FileChangeType.Deleted,
      uri
    }]);
  }

  async rename(oldUri: vscode.Uri, newUri: vscode.Uri): Promise<void> {
    console.log('[GitFox] rename:', oldUri.toString(), '->', newUri.toString());

    const oldPath = this.getFilePath(oldUri);
    const newPath = this.getFilePath(newUri);

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
