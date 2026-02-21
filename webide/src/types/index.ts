// File tree types
export interface FileTreeNode {
  name: string
  path: string
  type: 'file' | 'directory'
  children?: FileTreeNode[]
  size?: number
  mode?: string
}

// Open file in editor
export interface OpenFile {
  path: string
  name: string
  content: string
  originalContent: string
  language: string
  modified: boolean
}

// Extension system types
export interface Extension {
  id: string
  name: string
  version: string
  description: string
  publisher: string
  enabled: boolean
  categories: string[]
  activationEvents: string[]
  contributes?: ExtensionContributes
}

export interface ExtensionContributes {
  themes?: ContributedTheme[]
  languages?: ContributedLanguage[]
  commands?: ContributedCommand[]
  keybindings?: ContributedKeybinding[]
  snippets?: ContributedSnippet[]
}

export interface ContributedTheme {
  label: string
  uiTheme: 'vs' | 'vs-dark' | 'hc-black'
  path: string
}

export interface ContributedLanguage {
  id: string
  aliases?: string[]
  extensions?: string[]
  filenames?: string[]
  configuration?: string
}

export interface ContributedCommand {
  command: string
  title: string
  category?: string
  icon?: string
}

export interface ContributedKeybinding {
  command: string
  key: string
  mac?: string
  when?: string
}

export interface ContributedSnippet {
  language: string
  path: string
}

// User extension settings (stored per user)
export interface UserExtensionSettings {
  userId: string
  installedExtensions: string[]
  extensionSettings: Record<string, Record<string, unknown>>
}

// Commit types
export interface CommitRequest {
  branch: string
  message: string
  files: FileChange[]
}

export interface FileChange {
  path: string
  action: 'create' | 'update' | 'delete'
  content?: string
}

// API Response types
export interface Repository {
  id: number
  name: string
  fullPath: string
  description?: string
  defaultBranch: string
  visibility: string
}

export interface Branch {
  name: string
  commit: string
  protected: boolean
}

// Panel types for IDE layout
export interface Panel {
  id: string
  title: string
  icon: string
  component: string
  position: 'left' | 'bottom' | 'right'
  visible: boolean
  size: number
}

// Command palette
export interface Command {
  id: string
  label: string
  description?: string
  keybinding?: string
  category?: string
  action: () => void | Promise<void>
}

// Search result
export interface SearchResult {
  path: string
  line: number
  column: number
  preview: string
  matchStart: number
  matchEnd: number
}
