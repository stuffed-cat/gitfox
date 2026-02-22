/**
 * GitFox API Client for WebIDE
 * 
 * Provides typed access to GitFox REST API endpoints needed by WebIDE.
 */

export interface GitFoxApiConfig {
  baseUrl: string;
  accessToken: string;
}

export interface TreeEntry {
  id: string;
  name: string;
  type: 'blob' | 'tree';
  path: string;
  mode: string;
}

export interface FileContent {
  file_name: string;
  file_path: string;
  size: number;
  encoding: 'base64' | 'text';
  content: string;
  content_sha256: string;
  ref: string;
  blob_id: string;
  commit_id: string;
  last_commit_id: string;
}

export interface Branch {
  name: string;
  commit: {
    id: string;
    short_id: string;
    title: string;
  };
  protected: boolean;
  default: boolean;
}

export interface Commit {
  id: string;
  short_id: string;
  title: string;
  message: string;
  author_name: string;
  author_email: string;
  authored_date: string;
  committer_name: string;
  committer_email: string;
  committed_date: string;
}

export interface CreateCommitAction {
  action: 'create' | 'update' | 'delete' | 'move';
  file_path: string;
  content?: string;
  encoding?: 'base64' | 'text';
  previous_path?: string;
}

export interface CreateCommitRequest {
  branch: string;
  commit_message: string;
  actions: CreateCommitAction[];
  start_branch?: string;
}

export class GitFoxApiClient {
  private baseUrl: string;
  private accessToken: string;

  constructor(config: GitFoxApiConfig) {
    this.baseUrl = config.baseUrl.replace(/\/$/, '');
    this.accessToken = config.accessToken;
  }

  private async fetch<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseUrl}/api/v1${endpoint}`;
    
    const response = await fetch(url, {
      ...options,
      headers: {
        'Authorization': `Bearer ${this.accessToken}`,
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`API Error ${response.status}: ${error}`);
    }

    return response.json();
  }

  // Repository tree
  async getTree(
    projectPath: string,
    ref: string = 'main',
    path: string = '',
    recursive: boolean = false
  ): Promise<TreeEntry[]> {
    const params = new URLSearchParams({
      ref,
      ...(path && { path }),
      ...(recursive && { recursive: 'true' }),
    });
    
    return this.fetch<TreeEntry[]>(
      `/projects/${encodeURIComponent(projectPath)}/repository/tree?${params}`
    );
  }

  // File content
  async getFileContent(
    projectPath: string,
    filePath: string,
    ref: string = 'main'
  ): Promise<FileContent> {
    const params = new URLSearchParams({ ref });
    
    return this.fetch<FileContent>(
      `/projects/${encodeURIComponent(projectPath)}/repository/files/${encodeURIComponent(filePath)}?${params}`
    );
  }

  // Raw file content (for binary files)
  async getFileRaw(
    projectPath: string,
    filePath: string,
    ref: string = 'main'
  ): Promise<Blob> {
    const params = new URLSearchParams({ ref });
    const url = `${this.baseUrl}/api/v1/projects/${encodeURIComponent(projectPath)}/repository/files/${encodeURIComponent(filePath)}/raw?${params}`;
    
    const response = await fetch(url, {
      headers: {
        'Authorization': `Bearer ${this.accessToken}`,
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch file: ${response.status}`);
    }

    return response.blob();
  }

  // Branches
  async getBranches(projectPath: string): Promise<Branch[]> {
    return this.fetch<Branch[]>(
      `/projects/${encodeURIComponent(projectPath)}/repository/branches`
    );
  }

  async getBranch(projectPath: string, branch: string): Promise<Branch> {
    return this.fetch<Branch>(
      `/projects/${encodeURIComponent(projectPath)}/repository/branches/${encodeURIComponent(branch)}`
    );
  }

  // Commits
  async createCommit(
    projectPath: string,
    request: CreateCommitRequest
  ): Promise<Commit> {
    return this.fetch<Commit>(
      `/projects/${encodeURIComponent(projectPath)}/repository/commits`,
      {
        method: 'POST',
        body: JSON.stringify(request),
      }
    );
  }

  // User info
  async getCurrentUser(): Promise<{ id: number; username: string; name: string; email: string }> {
    return this.fetch('/auth/me');
  }
}

export function createApiClient(config: GitFoxApiConfig): GitFoxApiClient {
  return new GitFoxApiClient(config);
}

export default GitFoxApiClient;
