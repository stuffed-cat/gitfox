/**
 * GitFox WebIDE - Main Entry Point
 * 
 * Provides the start() API for embedding VS Code WebIDE in GitFox.
 * The WebIDE runs in a sandboxed iframe for security isolation.
 */

export interface WebIDEConfig {
  /** GitFox instance URL (e.g., https://gitfox.example.com) */
  gitfoxUrl: string;
  
  /** Project path (e.g., namespace/project) */
  projectPath: string;
  
  /** Git ref to open (branch, tag, or commit) */
  ref?: string;
  
  /** File path to open initially */
  filePath?: string;
  
  /** OAuth access token (required for API access) */
  accessToken: string;
  
  /** Container element to mount WebIDE */
  container: HTMLElement;
  
  /** Theme: 'dark' | 'light' | 'auto' */
  theme?: 'dark' | 'light' | 'auto';
  
  /** Extension marketplace configuration (from admin settings) */
  extensions?: {
    enabled: boolean;
    serviceUrl?: string;
    itemUrl?: string;
    resourceUrl?: string;
  };
  
  /** Callback when user saves changes */
  onSave?: (changes: FileChange[]) => Promise<void>;
  
  /** Callback when user wants to commit */
  onCommit?: (message: string, changes: FileChange[]) => Promise<void>;
  
  /** Callback when WebIDE is closed */
  onClose?: () => void;
}

export interface FileChange {
  path: string;
  content: string;
  action: 'create' | 'modify' | 'delete';
}

export interface WebIDEInstance {
  /** Dispose the WebIDE instance */
  dispose(): void;
  
  /** Get current file changes */
  getChanges(): FileChange[];
  
  /** Open a specific file */
  openFile(path: string): void;
  
  /** Send message to WebIDE iframe */
  postMessage(type: string, payload: unknown): void;
}

/**
 * Start GitFox WebIDE
 * 
 * Creates a sandboxed iframe containing VS Code Workbench and
 * establishes communication with the main application.
 */
export function start(config: WebIDEConfig): WebIDEInstance {
  const {
    gitfoxUrl,
    projectPath,
    ref = 'main',
    filePath,
    accessToken,
    container,
    theme = 'auto',
    extensions,
    onSave,
    onCommit,
    onClose,
  } = config;

  // Create sandboxed iframe
  const iframe = document.createElement('iframe');
  iframe.className = 'gitfox-webide-frame';
  iframe.style.cssText = 'width: 100%; height: 100%; border: none;';
  iframe.sandbox.add(
    'allow-scripts',
    'allow-same-origin',
    'allow-forms',
    'allow-popups',
    'allow-modals'
  );
  
  // Build workbench URL with configuration
  const workbenchUrl = new URL('/ide/workbench', gitfoxUrl);
  workbenchUrl.searchParams.set('project', projectPath);
  workbenchUrl.searchParams.set('ref', ref);
  if (filePath) {
    workbenchUrl.searchParams.set('file', filePath);
  }
  workbenchUrl.searchParams.set('theme', theme);
  
  iframe.src = workbenchUrl.toString();
  
  // File changes tracking
  const changes: FileChange[] = [];
  
  // Message handler for iframe communication
  const handleMessage = (event: MessageEvent) => {
    if (event.source !== iframe.contentWindow) return;
    
    const { type, payload } = event.data || {};
    
    switch (type) {
      case 'webide:ready':
        // WebIDE is ready, send initial configuration
        iframe.contentWindow?.postMessage({
          type: 'webide:init',
          payload: {
            accessToken,
            projectPath,
            ref,
            filePath,
            extensions,
          }
        }, '*');
        break;
        
      case 'webide:file-change':
        // Track file changes
        const existingIndex = changes.findIndex(c => c.path === payload.path);
        if (existingIndex >= 0) {
          changes[existingIndex] = payload;
        } else {
          changes.push(payload);
        }
        break;
        
      case 'webide:save':
        onSave?.(payload.changes);
        break;
        
      case 'webide:commit':
        onCommit?.(payload.message, payload.changes);
        break;
        
      case 'webide:close':
        onClose?.();
        break;
    }
  };
  
  window.addEventListener('message', handleMessage);
  container.appendChild(iframe);
  
  return {
    dispose() {
      window.removeEventListener('message', handleMessage);
      iframe.remove();
    },
    
    getChanges() {
      return [...changes];
    },
    
    openFile(path: string) {
      iframe.contentWindow?.postMessage({
        type: 'webide:open-file',
        payload: { path }
      }, '*');
    },
    
    postMessage(type: string, payload: unknown) {
      iframe.contentWindow?.postMessage({ type, payload }, '*');
    }
  };
}

export default { start };
