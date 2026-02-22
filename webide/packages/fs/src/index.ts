/**
 * GitFox WebIDE Virtual File System
 * 
 * Provides a VS Code compatible file system that:
 * - Fetches files from GitFox repository on demand
 * - Tracks local changes in memory
 * - Supports dirty state detection for commit workflow
 */

import type { GitFoxApiClient } from '@gitfox/api-client';

export interface FileSystemEntry {
  type: 'file' | 'directory';
  path: string;
  name: string;
  localContent?: string;  // Modified content (not yet committed)
  remoteContent?: string; // Original content from server
  modified: boolean;
  children?: Map<string, FileSystemEntry>;
}

export interface FileChange {
  path: string;
  action: 'create' | 'modify' | 'delete';
  content?: string;
}

export interface FileSystemProviderOptions {
  projectPath: string;
  ref: string;
  apiClient: GitFoxApiClient;
}

export class GitFoxFileSystem {
  private projectPath: string;
  private ref: string;
  private apiClient: GitFoxApiClient;
  private root: FileSystemEntry;
  private changes: Map<string, FileChange> = new Map();
  private loadedPaths: Set<string> = new Set();

  constructor(options: FileSystemProviderOptions) {
    this.projectPath = options.projectPath;
    this.ref = options.ref;
    this.apiClient = options.apiClient;
    this.root = {
      type: 'directory',
      path: '/',
      name: '',
      modified: false,
      children: new Map(),
    };
  }

  /**
   * Initialize file system by loading root tree
   */
  async initialize(): Promise<void> {
    await this.loadDirectory('/');
  }

  /**
   * Load directory contents from GitFox API
   */
  async loadDirectory(path: string): Promise<FileSystemEntry[]> {
    if (this.loadedPaths.has(path)) {
      return this.getDirectoryContents(path);
    }

    const normalizedPath = path === '/' ? '' : path.replace(/^\//, '');
    
    try {
      const entries = await this.apiClient.getTree(
        this.projectPath,
        this.ref,
        normalizedPath
      );

      const parent = this.getEntry(path) || this.root;
      if (parent.type !== 'directory') return [];

      for (const entry of entries) {
        const fsEntry: FileSystemEntry = {
          type: entry.type === 'tree' ? 'directory' : 'file',
          path: '/' + entry.path,
          name: entry.name,
          modified: false,
          ...(entry.type === 'tree' && { children: new Map() }),
        };
        parent.children!.set(entry.name, fsEntry);
      }

      this.loadedPaths.add(path);
      return this.getDirectoryContents(path);
    } catch (error) {
      console.error(`Failed to load directory ${path}:`, error);
      return [];
    }
  }

  /**
   * Get directory contents (already loaded)
   */
  getDirectoryContents(path: string): FileSystemEntry[] {
    const entry = this.getEntry(path);
    if (!entry || entry.type !== 'directory' || !entry.children) {
      return [];
    }
    return Array.from(entry.children.values());
  }

  /**
   * Get entry by path
   */
  getEntry(path: string): FileSystemEntry | null {
    if (path === '/' || path === '') return this.root;

    const parts = path.replace(/^\//, '').split('/');
    let current = this.root;

    for (const part of parts) {
      if (!current.children?.has(part)) {
        return null;
      }
      current = current.children.get(part)!;
    }

    return current;
  }

  /**
   * Read file content
   */
  async readFile(path: string): Promise<string> {
    const entry = this.getEntry(path);
    
    // Return local changes if available
    if (entry?.localContent !== undefined) {
      return entry.localContent;
    }

    // Fetch from server
    try {
      const content = await this.apiClient.getFileContent(
        this.projectPath,
        path.replace(/^\//, ''),
        this.ref
      );

      // Decode base64 content
      const decoded = content.encoding === 'base64'
        ? atob(content.content)
        : content.content;

      // Cache remote content
      if (entry) {
        entry.remoteContent = decoded;
      }

      return decoded;
    } catch (error) {
      console.error(`Failed to read file ${path}:`, error);
      throw error;
    }
  }

  /**
   * Write file content (stores locally, doesn't commit)
   */
  writeFile(path: string, content: string): void {
    let entry = this.getEntry(path);
    
    if (!entry) {
      // Create new file entry
      entry = this.createFileEntry(path);
      this.changes.set(path, {
        path,
        action: 'create',
        content,
      });
    } else {
      // Modify existing file
      const action = entry.remoteContent === undefined ? 'create' : 'modify';
      this.changes.set(path, {
        path,
        action,
        content,
      });
    }

    entry.localContent = content;
    entry.modified = content !== entry.remoteContent;
  }

  /**
   * Delete file
   */
  deleteFile(path: string): void {
    const entry = this.getEntry(path);
    if (!entry) return;

    // If file was created locally and not committed, just remove it
    const existingChange = this.changes.get(path);
    if (existingChange?.action === 'create') {
      this.changes.delete(path);
    } else {
      this.changes.set(path, {
        path,
        action: 'delete',
      });
    }

    // Remove from parent
    const parentPath = path.substring(0, path.lastIndexOf('/')) || '/';
    const parent = this.getEntry(parentPath);
    if (parent?.children) {
      parent.children.delete(entry.name);
    }
  }

  /**
   * Create directory
   */
  createDirectory(path: string): void {
    const parts = path.replace(/^\//, '').split('/');
    let current = this.root;

    for (const part of parts) {
      if (!current.children!.has(part)) {
        const newDir: FileSystemEntry = {
          type: 'directory',
          path: current.path === '/' ? `/${part}` : `${current.path}/${part}`,
          name: part,
          modified: false,
          children: new Map(),
        };
        current.children!.set(part, newDir);
      }
      current = current.children!.get(part)!;
    }
  }

  /**
   * Create file entry in tree structure
   */
  private createFileEntry(path: string): FileSystemEntry {
    const parts = path.replace(/^\//, '').split('/');
    const fileName = parts.pop()!;
    const dirPath = parts.length > 0 ? '/' + parts.join('/') : '/';

    // Ensure directory exists
    this.createDirectory(dirPath);

    const parent = this.getEntry(dirPath)!;
    const entry: FileSystemEntry = {
      type: 'file',
      path,
      name: fileName,
      modified: true,
    };

    parent.children!.set(fileName, entry);
    return entry;
  }

  /**
   * Get all changes
   */
  getChanges(): FileChange[] {
    return Array.from(this.changes.values());
  }

  /**
   * Check if there are any uncommitted changes
   */
  hasChanges(): boolean {
    return this.changes.size > 0;
  }

  /**
   * Clear changes (after successful commit)
   */
  clearChanges(): void {
    // Update entries to reflect committed state
    for (const change of this.changes.values()) {
      const entry = this.getEntry(change.path);
      if (entry && change.action !== 'delete') {
        entry.remoteContent = entry.localContent;
        entry.modified = false;
      }
    }
    this.changes.clear();
  }

  /**
   * Discard all local changes
   */
  discardChanges(): void {
    for (const change of this.changes.values()) {
      const entry = this.getEntry(change.path);
      if (!entry) continue;

      if (change.action === 'create') {
        // Remove created files
        const parentPath = change.path.substring(0, change.path.lastIndexOf('/')) || '/';
        const parent = this.getEntry(parentPath);
        if (parent?.children) {
          parent.children.delete(entry.name);
        }
      } else {
        // Restore original content
        entry.localContent = entry.remoteContent;
        entry.modified = false;
      }
    }
    this.changes.clear();
  }

  /**
   * Check if a specific file is modified
   */
  isModified(path: string): boolean {
    return this.changes.has(path);
  }

  /**
   * Get file stat
   */
  stat(path: string): { type: 'file' | 'directory'; size: number; modified: boolean } | null {
    const entry = this.getEntry(path);
    if (!entry) return null;

    return {
      type: entry.type,
      size: entry.localContent?.length ?? entry.remoteContent?.length ?? 0,
      modified: entry.modified,
    };
  }
}

export function createFileSystem(options: FileSystemProviderOptions): GitFoxFileSystem {
  return new GitFoxFileSystem(options);
}

export default GitFoxFileSystem;
