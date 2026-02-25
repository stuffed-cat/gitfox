/**
 * GitFox VS Code Extension
 * 
 * 提供完整的源代码管理功能:
 * - gitfox:// FileSystemProvider - 远程文件读写
 * - SourceControl Provider - 变更跟踪、暂存、提交
 * - AuthenticationProvider - OAuth2 认证
 * 
 * 核心策略：
 * 1. 文件通过 API 按需加载，缓存在内存中
 * 2. 本地修改跟踪在 pendingChanges 中
 * 3. 提交通过 batch_commit API 一次性推送到服务器
 */

import * as vscode from 'vscode';

// ==================== 类型定义 ====================

interface GitFoxConfig {
  accessToken: string;
  apiBaseUrl?: string;
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
  encoding: 'base64' | 'text';
  content: string;
}

interface FileChange {
  path: string;
  action: 'create' | 'update' | 'delete';
  originalContent?: Uint8Array;  // 原始内容 (用于 diff)
  localContent?: Uint8Array;     // 修改后的内容
}

interface BatchCommitRequest {
  branch: string;
  commit_message: string;
  actions: Array<{
    action: string;
    file_path: string;
    content?: string;
  }>;
}

interface CommitResponse {
  sha: string;
}

// ==================== 扩展入口 ====================

let extensionInstance: GitFoxExtension | null = null;

export function activate(context: vscode.ExtensionContext): Promise<void> {
  return activateInternal(context);
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

  extensionInstance = new GitFoxExtension(config, context);
  await extensionInstance.initialize();
  
  console.log('[GitFox] Extension activated successfully');
}

export function deactivate() {
  extensionInstance?.dispose();
  extensionInstance = null;
  console.log('[GitFox] Extension deactivated');
}

// ==================== GitFox Extension 主类 ====================

class GitFoxExtension {
  private fsProvider: GitFoxFileSystemProvider;
  private scmProvider: GitFoxSourceControlProvider;
  private authProvider: GitFoxAuthProvider;
  private disposables: vscode.Disposable[] = [];

  constructor(
    private config: GitFoxConfig,
    private context: vscode.ExtensionContext
  ) {
    this.fsProvider = new GitFoxFileSystemProvider(config);
    this.scmProvider = new GitFoxSourceControlProvider(config, this.fsProvider);
    this.authProvider = new GitFoxAuthProvider(config);
  }

  async initialize() {
    // 1. 注册认证提供者
    this.disposables.push(
      vscode.authentication.registerAuthenticationProvider(
        'gitfox',
        'GitFox',
        this.authProvider,
        { supportsMultipleAccounts: false }
      )
    );
    console.log('[GitFox] Authentication provider registered');

    // 2. 注册文件系统提供者
    this.disposables.push(
      vscode.workspace.registerFileSystemProvider('gitfox', this.fsProvider, {
        isCaseSensitive: true,
        isReadonly: false,
      })
    );
    console.log('[GitFox] File system provider registered');

    // 3. 注册 SCM 提供者
    this.scmProvider.register();
    this.disposables.push(this.scmProvider);
    console.log('[GitFox] Source control provider registered');

    // 4. 注册命令
    this.registerCommands();

    // 5. 监听文件变更
    this.disposables.push(
      vscode.workspace.onDidSaveTextDocument(doc => this.onDocumentSaved(doc))
    );

    // 6. 打开工作区
    const folderUri = vscode.Uri.parse(
      `gitfox://${this.config.projectInfo.owner}/${this.config.projectInfo.repo}`
    );
    
    const existingFolders = vscode.workspace.workspaceFolders || [];
    const alreadyOpen = existingFolders.some(f => f.uri.toString() === folderUri.toString());
    if (!alreadyOpen) {
      vscode.workspace.updateWorkspaceFolders(existingFolders.length, 0, { uri: folderUri });
    }
  }

  private registerCommands() {
    // 提交
    this.disposables.push(
      vscode.commands.registerCommand('gitfox.commit', () => this.scmProvider.commit())
    );
    
    // 刷新
    this.disposables.push(
      vscode.commands.registerCommand('gitfox.refresh', () => this.scmProvider.refresh())
    );
    
    // 丢弃所有更改
    this.disposables.push(
      vscode.commands.registerCommand('gitfox.discardAll', () => this.scmProvider.discardAll())
    );
    
    // 暂存所有
    this.disposables.push(
      vscode.commands.registerCommand('gitfox.stageAll', () => this.scmProvider.stageAll())
    );
    
    // 取消暂存所有
    this.disposables.push(
      vscode.commands.registerCommand('gitfox.unstageAll', () => this.scmProvider.unstageAll())
    );
    
    // 打开文件
    this.disposables.push(
      vscode.commands.registerCommand('gitfox.openFile', (resource: vscode.SourceControlResourceState) => {
        if (resource?.resourceUri) {
          vscode.commands.executeCommand('vscode.open', resource.resourceUri);
        }
      })
    );
    
    // 暂存单个文件
    this.disposables.push(
      vscode.commands.registerCommand('gitfox.stageChange', (resource: vscode.SourceControlResourceState) => {
        if (resource?.resourceUri) {
          this.scmProvider.stage(resource.resourceUri);
        }
      })
    );
    
    // 取消暂存单个文件
    this.disposables.push(
      vscode.commands.registerCommand('gitfox.unstageChange', (resource: vscode.SourceControlResourceState) => {
        if (resource?.resourceUri) {
          this.scmProvider.unstage(resource.resourceUri);
        }
      })
    );
    
    // 丢弃单个更改
    this.disposables.push(
      vscode.commands.registerCommand('gitfox.discardChange', (resource: vscode.SourceControlResourceState) => {
        if (resource?.resourceUri) {
          this.scmProvider.discard(resource.resourceUri);
        }
      })
    );
  }

  private onDocumentSaved(doc: vscode.TextDocument) {
    if (doc.uri.scheme === 'gitfox') {
      // 文件已保存到 FileSystemProvider，SCM 会自动检测到变更
      this.scmProvider.notifyFileChanged(doc.uri);
    }
  }

  dispose() {
    this.disposables.forEach(d => d.dispose());
    this.disposables = [];
  }
}

// ==================== Source Control Provider ====================

class GitFoxSourceControlProvider implements vscode.Disposable {
  private scm: vscode.SourceControl;
  private changesGroup: vscode.SourceControlResourceGroup;
  private stagedGroup: vscode.SourceControlResourceGroup;
  
  /** 所有待处理的更改 (path -> change) */
  private changes = new Map<string, FileChange>();
  /** 已暂存的文件路径 */
  private stagedPaths = new Set<string>();
  
  private disposables: vscode.Disposable[] = [];

  constructor(
    private config: GitFoxConfig,
    private fsProvider: GitFoxFileSystemProvider
  ) {
    const rootUri = vscode.Uri.parse(
      `gitfox://${config.projectInfo.owner}/${config.projectInfo.repo}`
    );
    
    this.scm = vscode.scm.createSourceControl('gitfox', 'GitFox', rootUri);
    this.scm.inputBox.placeholder = `提交到 ${config.projectInfo.ref} 分支的消息`;
    this.scm.acceptInputCommand = {
      command: 'gitfox.commit',
      title: 'Commit'
    };
    
    // 创建资源组
    this.changesGroup = this.scm.createResourceGroup('changes', 'Changes');
    this.stagedGroup = this.scm.createResourceGroup('staged', 'Staged Changes');
    
    this.changesGroup.hideWhenEmpty = true;
    this.stagedGroup.hideWhenEmpty = true;
  }

  register() {
    // 监听 FileSystemProvider 的变更
    this.disposables.push(
      this.fsProvider.onDidChangeFile(events => this.onFileSystemChange(events))
    );
  }

  private onFileSystemChange(events: readonly vscode.FileChangeEvent[]) {
    for (const event of events) {
      const path = this.getFilePath(event.uri);
      
      switch (event.type) {
        case vscode.FileChangeType.Created:
          this.trackChange(path, 'create');
          break;
        case vscode.FileChangeType.Changed:
          // 只有在不是新建文件时才标记为 update
          if (!this.changes.has(path) || this.changes.get(path)?.action !== 'create') {
            this.trackChange(path, 'update');
          }
          break;
        case vscode.FileChangeType.Deleted:
          this.trackChange(path, 'delete');
          break;
      }
    }
    this.updateResourceGroups();
  }

  notifyFileChanged(uri: vscode.Uri) {
    const path = this.getFilePath(uri);
    
    // 检查文件是否真的有变更
    const localContent = this.fsProvider.getFileCache(path);
    const originalContent = this.fsProvider.getOriginalContent(path);
    
    if (localContent && originalContent) {
      // 比较内容
      if (this.arraysEqual(localContent, originalContent)) {
        // 没有变更，移除跟踪
        this.changes.delete(path);
        this.stagedPaths.delete(path);
      } else {
        // 有变更
        this.trackChange(path, 'update', originalContent, localContent);
      }
    } else if (localContent && !originalContent) {
      // 新文件
      this.trackChange(path, 'create', undefined, localContent);
    }
    
    this.updateResourceGroups();
  }

  private trackChange(
    path: string, 
    action: 'create' | 'update' | 'delete',
    originalContent?: Uint8Array,
    localContent?: Uint8Array
  ) {
    const existing = this.changes.get(path);
    
    // 如果删除一个新建的文件，直接移除记录
    if (action === 'delete' && existing?.action === 'create') {
      this.changes.delete(path);
      this.stagedPaths.delete(path);
      return;
    }
    
    this.changes.set(path, {
      path,
      action,
      originalContent: originalContent ?? existing?.originalContent,
      localContent: localContent ?? existing?.localContent,
    });
  }

  private updateResourceGroups() {
    const changedResources: vscode.SourceControlResourceState[] = [];
    const stagedResources: vscode.SourceControlResourceState[] = [];
    
    for (const [path, change] of this.changes) {
      const uri = this.pathToUri(path);
      const resource = this.createResourceState(uri, change);
      
      if (this.stagedPaths.has(path)) {
        stagedResources.push(resource);
      } else {
        changedResources.push(resource);
      }
    }
    
    this.changesGroup.resourceStates = changedResources;
    this.stagedGroup.resourceStates = stagedResources;
    
    // 更新徽章计数
    this.scm.count = this.changes.size;
  }

  private createResourceState(
    uri: vscode.Uri, 
    change: FileChange
  ): vscode.SourceControlResourceState {
    let decorations: vscode.SourceControlResourceDecorations;
    
    switch (change.action) {
      case 'create':
        decorations = {
          strikeThrough: false,
          tooltip: 'New File',
          iconPath: new vscode.ThemeIcon('add', new vscode.ThemeColor('gitDecoration.addedResourceForeground')),
        };
        break;
      case 'delete':
        decorations = {
          strikeThrough: true,
          tooltip: 'Deleted',
          iconPath: new vscode.ThemeIcon('trash', new vscode.ThemeColor('gitDecoration.deletedResourceForeground')),
        };
        break;
      default:
        decorations = {
          strikeThrough: false,
          tooltip: 'Modified',
          iconPath: new vscode.ThemeIcon('edit', new vscode.ThemeColor('gitDecoration.modifiedResourceForeground')),
        };
    }
    
    return {
      resourceUri: uri,
      decorations,
      command: change.action !== 'delete' ? {
        command: 'vscode.open',
        title: 'Open',
        arguments: [uri]
      } : undefined,
    };
  }

  // ==================== SCM 操作 ====================

  async commit() {
    const message = this.scm.inputBox.value?.trim();
    if (!message) {
      vscode.window.showWarningMessage('请输入提交消息');
      return;
    }
    
    // 获取要提交的更改 (优先暂存区，否则全部)
    const pathsToCommit = this.stagedPaths.size > 0 
      ? [...this.stagedPaths] 
      : [...this.changes.keys()];
    
    if (pathsToCommit.length === 0) {
      vscode.window.showInformationMessage('没有要提交的更改');
      return;
    }
    
    // 构建提交请求
    const actions: BatchCommitRequest['actions'] = [];
    
    for (const path of pathsToCommit) {
      const change = this.changes.get(path);
      if (!change) continue;
      
      if (change.action === 'delete') {
        actions.push({
          action: 'delete',
          file_path: path,
        });
      } else {
        const content = this.fsProvider.getFileCache(path);
        if (content) {
          // 将 Uint8Array 转换为字符串
          const textContent = new TextDecoder().decode(content);
          actions.push({
            action: change.action,
            file_path: path,
            content: textContent,
          });
        }
      }
    }
    
    if (actions.length === 0) {
      vscode.window.showWarningMessage('没有有效的更改可提交');
      return;
    }
    
    // 发送提交请求
    try {
      await vscode.window.withProgress(
        {
          location: vscode.ProgressLocation.SourceControl,
          title: '正在提交更改...',
        },
        async () => {
          const { projectInfo, accessToken } = this.config;
          const base = this.config.apiBaseUrl || 
            (typeof self !== 'undefined' && (self as any).location?.origin) ||
            (typeof location !== 'undefined' ? location.origin : '');
          
          const response = await fetch(
            `${base}/api/v1/projects/${projectInfo.owner}/${projectInfo.repo}/repository/commits/batch`,
            {
              method: 'POST',
              headers: {
                'Authorization': `Bearer ${accessToken}`,
                'Content-Type': 'application/json',
              },
              body: JSON.stringify({
                branch: projectInfo.ref,
                commit_message: message,
                actions,
              }),
            }
          );
          
          if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`提交失败: ${errorText}`);
          }
          
          const result: CommitResponse = await response.json();
          return result;
        }
      );
      
      // 提交成功，清理状态
      for (const path of pathsToCommit) {
        // 更新原始内容为当前内容
        const content = this.fsProvider.getFileCache(path);
        if (content) {
          this.fsProvider.setOriginalContent(path, content);
        }
        this.changes.delete(path);
        this.stagedPaths.delete(path);
      }
      
      this.scm.inputBox.value = '';
      this.updateResourceGroups();
      vscode.window.showInformationMessage(`成功提交 ${actions.length} 个文件变更`);
      
    } catch (error) {
      vscode.window.showErrorMessage(`${error}`);
    }
  }

  refresh() {
    // 清除缓存并重新检测变更
    this.fsProvider.clearCache();
    this.changes.clear();
    this.stagedPaths.clear();
    this.updateResourceGroups();
    vscode.window.showInformationMessage('已刷新');
  }

  stageAll() {
    for (const path of this.changes.keys()) {
      this.stagedPaths.add(path);
    }
    this.updateResourceGroups();
  }

  unstageAll() {
    this.stagedPaths.clear();
    this.updateResourceGroups();
  }

  stage(uri: vscode.Uri) {
    const path = this.getFilePath(uri);
    this.stagedPaths.add(path);
    this.updateResourceGroups();
  }

  unstage(uri: vscode.Uri) {
    const path = this.getFilePath(uri);
    this.stagedPaths.delete(path);
    this.updateResourceGroups();
  }

  async discard(uri: vscode.Uri) {
    const path = this.getFilePath(uri);
    const change = this.changes.get(path);
    
    if (!change) return;
    
    const confirm = await vscode.window.showWarningMessage(
      `确定要丢弃对 "${path}" 的更改吗？`,
      { modal: true },
      '丢弃'
    );
    
    if (confirm !== '丢弃') return;
    
    if (change.action === 'create') {
      // 删除新建的文件
      this.fsProvider.deleteFromCache(path);
    } else if (change.originalContent) {
      // 恢复原始内容
      this.fsProvider.restoreContent(path, change.originalContent);
    }
    
    this.changes.delete(path);
    this.stagedPaths.delete(path);
    this.updateResourceGroups();
  }

  async discardAll() {
    if (this.changes.size === 0) return;
    
    const confirm = await vscode.window.showWarningMessage(
      `确定要丢弃所有 ${this.changes.size} 个更改吗？`,
      { modal: true },
      '丢弃全部'
    );
    
    if (confirm !== '丢弃全部') return;
    
    for (const [path, change] of this.changes) {
      if (change.action === 'create') {
        this.fsProvider.deleteFromCache(path);
      } else if (change.originalContent) {
        this.fsProvider.restoreContent(path, change.originalContent);
      }
    }
    
    this.changes.clear();
    this.stagedPaths.clear();
    this.updateResourceGroups();
  }

  // ==================== 工具方法 ====================

  private getFilePath(uri: vscode.Uri): string {
    const repo = this.config.projectInfo.repo;
    const prefix = `/${repo}/`;
    const rootPath = `/${repo}`;

    if (uri.path === rootPath || uri.path === rootPath + '/' || uri.path === '/' || uri.path === '') {
      return '';
    } else if (uri.path.startsWith(prefix)) {
      return uri.path.slice(prefix.length);
    } else {
      return uri.path.replace(/^\//, '');
    }
  }

  private pathToUri(path: string): vscode.Uri {
    const { owner, repo } = this.config.projectInfo;
    return vscode.Uri.parse(`gitfox://${owner}/${repo}/${path}`);
  }

  private arraysEqual(a: Uint8Array, b: Uint8Array): boolean {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) return false;
    }
    return true;
  }

  dispose() {
    this.disposables.forEach(d => d.dispose());
    this.scm.dispose();
  }
}

// ==================== File System Provider ====================

class GitFoxFileSystemProvider implements vscode.FileSystemProvider {
  private _emitter = new vscode.EventEmitter<vscode.FileChangeEvent[]>();
  readonly onDidChangeFile = this._emitter.event;

  private fileCache = new Map<string, Uint8Array>();
  private dirCache = new Map<string, [string, vscode.FileType][]>();
  private typeCache = new Map<string, vscode.FileType>();
  
  /** 原始内容 (从服务器获取的版本，用于 diff 比较) */
  private originalContent = new Map<string, Uint8Array>();

  constructor(private config: GitFoxConfig) {}

  // ==================== 缓存访问方法 (供 SCM Provider 使用) ====================

  getFileCache(path: string): Uint8Array | undefined {
    return this.fileCache.get(path);
  }

  getOriginalContent(path: string): Uint8Array | undefined {
    return this.originalContent.get(path);
  }

  setOriginalContent(path: string, content: Uint8Array) {
    this.originalContent.set(path, content);
  }

  deleteFromCache(path: string) {
    this.fileCache.delete(path);
    this.originalContent.delete(path);
  }

  restoreContent(path: string, content: Uint8Array) {
    this.fileCache.set(path, content);
    // 触发文件内容变更
    const uri = this.pathToUri(path);
    this._emitter.fire([{ type: vscode.FileChangeType.Changed, uri }]);
  }

  clearCache() {
    this.fileCache.clear();
    this.dirCache.clear();
    this.typeCache.clear();
    this.originalContent.clear();
  }

  private pathToUri(path: string): vscode.Uri {
    const { owner, repo } = this.config.projectInfo;
    return vscode.Uri.parse(`gitfox://${owner}/${repo}/${path}`);
  }

  // ==================== FileSystemProvider 实现 ====================

  private getFilePath(uri: vscode.Uri): string {
    const repo = this.config.projectInfo.repo;
    const prefix = `/${repo}/`;
    const rootPath = `/${repo}`;

    if (uri.path === rootPath || uri.path === rootPath + '/' || uri.path === '/' || uri.path === '') {
      return '';
    } else if (uri.path.startsWith(prefix)) {
      return uri.path.slice(prefix.length);
    } else {
      return uri.path.replace(/^\//, '');
    }
  }

  private async apiFetch<T>(endpoint: string): Promise<T> {
    const base = this.config.apiBaseUrl ||
      (typeof self !== 'undefined' && (self as any).location?.origin) ||
      (typeof location !== 'undefined' ? location.origin : '');
    const response = await fetch(`${base}/api/v1${endpoint}`, {
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

    const filePath = this.getFilePath(uri);

    if (filePath === '') {
      return { type: vscode.FileType.Directory, ctime: 0, mtime: 0, size: 0 };
    }

    if (this.fileCache.has(filePath)) {
      return {
        type: vscode.FileType.File,
        ctime: 0,
        mtime: Date.now(),
        size: this.fileCache.get(filePath)!.byteLength,
      };
    }

    if (this.dirCache.has(filePath)) {
      return { type: vscode.FileType.Directory, ctime: 0, mtime: 0, size: 0 };
    }

    const lastSlash = filePath.lastIndexOf('/');
    const parentPath = lastSlash >= 0 ? filePath.slice(0, lastSlash) : '';
    const entryName = lastSlash >= 0 ? filePath.slice(lastSlash + 1) : filePath;

    if (!this.dirCache.has(parentPath)) {
      try {
        const { projectInfo } = this.config;
        const apiBase = `/projects/${projectInfo.owner}/${projectInfo.repo}`;
        const params = new URLSearchParams({
          ref: projectInfo.ref,
          ...(parentPath && { path: parentPath }),
        });
        const entries = await this.apiFetch<TreeEntry[]>(
          `${apiBase}/repository/tree?${params.toString()}`
        );
        const result: [string, vscode.FileType][] = entries.map((e: any) => [
          e.name,
          e.entry_type === 'Directory' ? vscode.FileType.Directory : vscode.FileType.File,
        ]);
        this.dirCache.set(parentPath, result);
        this.typeCache.set(parentPath.length ? parentPath : '__root__', vscode.FileType.Directory);
        const prefix = parentPath ? `${parentPath}/` : '';
        for (const [n, t] of result) {
          this.typeCache.set(`${prefix}${n}`, t);
        }
      } catch {
        // ignore
      }
    }

    const parentEntries = this.dirCache.get(parentPath);
    if (parentEntries) {
      const entry = parentEntries.find(([n]) => n === entryName);
      if (!entry) {
        throw vscode.FileSystemError.FileNotFound(uri);
      }
      const [, type] = entry;
      if (type === vscode.FileType.Directory) {
        return { type: vscode.FileType.Directory, ctime: 0, mtime: 0, size: 0 };
      }
      try {
        const { projectInfo } = this.config;
        const params = new URLSearchParams({ ref: projectInfo.ref });
        const apiBase = `/projects/${projectInfo.owner}/${projectInfo.repo}`;
        const fileContent = await this.apiFetch<FileContent>(
          `${apiBase}/repository/files/${encodeURIComponent(filePath)}?${params.toString()}`
        );
        const content = fileContent.encoding === 'base64'
          ? Uint8Array.from(atob(fileContent.content), c => c.charCodeAt(0))
          : new TextEncoder().encode(fileContent.content);
        this.fileCache.set(filePath, content);
        this.originalContent.set(filePath, content);  // 保存原始内容
        return { type: vscode.FileType.File, ctime: 0, mtime: 0, size: content.byteLength };
      } catch {
        return { type: vscode.FileType.File, ctime: 0, mtime: 0, size: 0 };
      }
    }

    const cachedType = this.typeCache.get(filePath);
    if (cachedType === vscode.FileType.Directory) {
      return { type: vscode.FileType.Directory, ctime: 0, mtime: 0, size: 0 };
    }

    throw vscode.FileSystemError.FileNotFound(uri);
  }

  async readDirectory(uri: vscode.Uri): Promise<[string, vscode.FileType][]> {
    console.log('[GitFox] readDirectory:', uri.toString());

    const filePath = this.getFilePath(uri);

    if (this.dirCache.has(filePath)) {
      return this.dirCache.get(filePath)!;
    }

    try {
      const { projectInfo } = this.config;
      const apiBase = `/projects/${projectInfo.owner}/${projectInfo.repo}`;

      const params = new URLSearchParams({
        ref: projectInfo.ref,
        ...(filePath && { path: filePath }),
      });

      const entries = await this.apiFetch<TreeEntry[]>(
        `${apiBase}/repository/tree?${params.toString()}`
      );

      const result: [string, vscode.FileType][] = entries.map((entry: any) => [
        entry.name,
        entry.entry_type === 'Directory' ? vscode.FileType.Directory : vscode.FileType.File
      ]);

      this.dirCache.set(filePath, result);
      if (filePath) {
        this.typeCache.set(filePath, vscode.FileType.Directory);
      }
      const prefix = filePath ? `${filePath}/` : '';
      for (const [name, type] of result) {
        this.typeCache.set(`${prefix}${name}`, type);
      }
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

    if (this.fileCache.has(filePath)) {
      return this.fileCache.get(filePath)!;
    }

    try {
      const { projectInfo } = this.config;
      const apiBase = `/projects/${projectInfo.owner}/${projectInfo.repo}`;

      const params = new URLSearchParams({ ref: projectInfo.ref });
      const fileContent = await this.apiFetch<FileContent>(
        `${apiBase}/repository/files/${encodeURIComponent(filePath)}?${params.toString()}`
      );

      const content = fileContent.encoding === 'base64'
        ? Uint8Array.from(atob(fileContent.content), c => c.charCodeAt(0))
        : new TextEncoder().encode(fileContent.content);

      this.fileCache.set(filePath, content);
      this.originalContent.set(filePath, content);  // 保存原始内容
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
    const isNew = !this.fileCache.has(filePath) && !this.originalContent.has(filePath);
    
    this.fileCache.set(filePath, content);

    this._emitter.fire([{
      type: isNew ? vscode.FileChangeType.Created : vscode.FileChangeType.Changed,
      uri
    }]);
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

// ==================== Authentication Provider ====================

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
    return [this._session];
  }

  async createSession(scopes: string[]): Promise<vscode.AuthenticationSession> {
    return this._session;
  }

  async removeSession(sessionId: string): Promise<void> {
    throw new Error('Session removal not supported in GitFox WebIDE');
  }
}

// ==================== 配置解析 ====================

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
  } catch {}

  try {
    if (typeof sessionStorage !== 'undefined') {
      const raw = sessionStorage.getItem(key);
      if (raw) {
        return JSON.parse(raw) as Partial<GitFoxConfig>;
      }
    }
  } catch {}

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
  } catch {}

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
  } catch {}

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
