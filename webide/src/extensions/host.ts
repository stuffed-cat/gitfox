/**
 * Extension Host
 * 
 * Manages the execution environment for extensions.
 * Provides the VSCode API subset that extensions can use.
 */

import * as monaco from 'monaco-editor'

// Type definitions (put first to avoid forward reference issues)
interface Disposable {
  dispose(): void
}

interface TextEditor {
  document: TextDocument
  selection: Selection
  selections: Selection[]
}

interface TextDocument {
  uri: Uri
  fileName: string
  languageId: string
  lineCount: number
  getText(range?: Range): string
  lineAt(line: number): TextLine
  positionAt(offset: number): Position
  offsetAt(position: Position): number
}

interface TextLine {
  lineNumber: number
  text: string
  range: Range
  firstNonWhitespaceCharacterIndex: number
  isEmptyOrWhitespace: boolean
}

interface TextDocumentChangeEvent {
  document: TextDocument
  contentChanges: TextDocumentContentChangeEvent[]
}

interface TextDocumentContentChangeEvent {
  range: Range
  text: string
}

interface WorkspaceFolder {
  uri: Uri
  name: string
  index: number
}

interface Diagnostic {
  range: Range
  message: string
  severity: number
  source?: string
}

interface Extension {
  id: string
  isActive: boolean
  exports: any
}

type DocumentSelector = string | { language?: string; scheme?: string; pattern?: string }

interface CompletionItemProvider {
  provideCompletionItems(document: TextDocument, position: Position, token: any, context: any): any
}

interface HoverProvider {
  provideHover(document: TextDocument, position: Position, token: any): any
}

interface DefinitionProvider {
  provideDefinition(document: TextDocument, position: Position, token: any): any
}

// Core classes - defined before vscode namespace
export class Position {
  constructor(public line: number, public character: number) {}
  
  isEqual(other: Position): boolean {
    return this.line === other.line && this.character === other.character
  }
  
  isBefore(other: Position): boolean {
    return this.line < other.line || (this.line === other.line && this.character < other.character)
  }
}

export class Range {
  constructor(
    public start: Position,
    public end: Position
  ) {}
  
  static create(startLine: number, startChar: number, endLine: number, endChar: number): Range {
    return new Range(new Position(startLine, startChar), new Position(endLine, endChar))
  }
  
  contains(position: Position): boolean {
    return !position.isBefore(this.start) && position.isBefore(this.end)
  }
}

export class Selection extends Range {
  constructor(
    public anchor: Position,
    public active: Position
  ) {
    super(anchor.isBefore(active) ? anchor : active, anchor.isBefore(active) ? active : anchor)
  }
  
  get isReversed(): boolean {
    return this.anchor.isBefore(this.active)
  }
}

export class TextEdit {
  constructor(public range: Range, public newText: string) {}
  
  static insert(position: Position, newText: string): TextEdit {
    return new TextEdit(new Range(position, position), newText)
  }
  
  static delete(range: Range): TextEdit {
    return new TextEdit(range, '')
  }
  
  static replace(range: Range, newText: string): TextEdit {
    return new TextEdit(range, newText)
  }
}

export class Uri {
  constructor(
    public scheme: string,
    public authority: string,
    public path: string,
    public query: string,
    public fragment: string
  ) {}
  
  static parse(value: string): Uri {
    const url = new URL(value)
    return new Uri(url.protocol.slice(0, -1), url.host, url.pathname, url.search.slice(1), url.hash.slice(1))
  }
  
  static file(path: string): Uri {
    return new Uri('file', '', path, '', '')
  }
  
  toString(): string {
    return `${this.scheme}://${this.authority}${this.path}${this.query ? '?' + this.query : ''}${this.fragment ? '#' + this.fragment : ''}`
  }
  
  get fsPath(): string {
    return this.path
  }
}

// Event emitter - defined before usage
class EventEmitter<T> {
  private listeners: ((e: T) => void)[] = []
  
  event = (listener: (e: T) => void): Disposable => {
    this.listeners.push(listener)
    return {
      dispose: () => {
        const index = this.listeners.indexOf(listener)
        if (index !== -1) this.listeners.splice(index, 1)
      }
    }
  }
  
  fire(data: T): void {
    this.listeners.forEach(l => l(data))
  }
}

// Output channel
class OutputChannel {
  constructor(public name: string) {}
  
  append(value: string): void {
    console.log(`[${this.name}]`, value)
  }
  
  appendLine(value: string): void {
    console.log(`[${this.name}]`, value)
  }
  
  clear(): void {}
  show(): void {}
  hide(): void {}
  dispose(): void {}
}

// Terminal
class Terminal {
  constructor(public name?: string) {}
  
  sendText(text: string): void {
    console.log(`[Terminal ${this.name}]`, text)
  }
  
  show(): void {}
  hide(): void {}
  dispose(): void {}
}

// Workspace configuration
class WorkspaceConfiguration {
  constructor(private _section?: string) {}
  
  get<T>(key: string, defaultValue?: T): T | undefined {
    const fullKey = this._section ? `${this._section}.${key}` : key
    const stored = localStorage.getItem(`gitfox-config-${fullKey}`)
    return stored ? JSON.parse(stored) : defaultValue
  }
  
  has(key: string): boolean {
    const fullKey = this._section ? `${this._section}.${key}` : key
    return localStorage.getItem(`gitfox-config-${fullKey}`) !== null
  }
  
  update(key: string, value: any): Promise<void> {
    const fullKey = this._section ? `${this._section}.${key}` : key
    localStorage.setItem(`gitfox-config-${fullKey}`, JSON.stringify(value))
    return Promise.resolve()
  }
}

// Diagnostic collection
class DiagnosticCollection {
  private diagnostics = new Map<string, Diagnostic[]>()
  
  constructor(public name?: string) {}
  
  set(uri: string, diagnostics: Diagnostic[]): void {
    this.diagnostics.set(uri, diagnostics)
  }
  
  delete(uri: string): void {
    this.diagnostics.delete(uri)
  }
  
  clear(): void {
    this.diagnostics.clear()
  }
  
  forEach(callback: (uri: string, diagnostics: Diagnostic[]) => void): void {
    this.diagnostics.forEach((diags, uri) => callback(uri, diags))
  }
  
  dispose(): void {
    this.clear()
  }
}

// Helper functions for Monaco conversion
function selectorToMonaco(selector: DocumentSelector): string {
  if (typeof selector === 'string') return selector
  return selector.language || '*'
}

function monacoModelToDocument(model: monaco.editor.ITextModel): TextDocument {
  return {
    uri: Uri.parse(model.uri.toString()),
    fileName: model.uri.path,
    languageId: model.getLanguageId(),
    lineCount: model.getLineCount(),
    getText: (range?: Range) => {
      if (!range) return model.getValue()
      return model.getValueInRange({
        startLineNumber: range.start.line + 1,
        startColumn: range.start.character + 1,
        endLineNumber: range.end.line + 1,
        endColumn: range.end.character + 1
      })
    },
    lineAt: (line: number) => {
      const text = model.getLineContent(line + 1)
      return {
        lineNumber: line,
        text,
        range: Range.create(line, 0, line, text.length),
        firstNonWhitespaceCharacterIndex: text.search(/\S/),
        isEmptyOrWhitespace: text.trim().length === 0
      }
    },
    positionAt: (offset: number) => {
      const pos = model.getPositionAt(offset)
      return new Position(pos.lineNumber - 1, pos.column - 1)
    },
    offsetAt: (position: Position) => {
      return model.getOffsetAt({ lineNumber: position.line + 1, column: position.character + 1 })
    }
  }
}

function itemToMonaco(item: any): monaco.languages.CompletionItem {
  return {
    label: item.label,
    kind: item.kind,
    insertText: item.insertText || item.label,
    documentation: item.documentation,
    detail: item.detail,
    range: undefined as any
  }
}

function hoverToMonaco(hover: any): monaco.languages.Hover {
  return {
    contents: Array.isArray(hover.contents) 
      ? hover.contents.map((c: any) => ({ value: typeof c === 'string' ? c : c.value }))
      : [{ value: typeof hover.contents === 'string' ? hover.contents : hover.contents.value }]
  }
}

function locationToMonaco(location: any): monaco.languages.Location {
  return {
    uri: monaco.Uri.parse(location.uri.toString()),
    range: {
      startLineNumber: location.range.start.line + 1,
      startColumn: location.range.start.character + 1,
      endLineNumber: location.range.end.line + 1,
      endColumn: location.range.end.character + 1
    }
  }
}

// Now create the vscode namespace with all dependencies defined
const windowActiveTextEditorEmitter = new EventEmitter<TextEditor | undefined>()
const windowVisibleTextEditorsEmitter = new EventEmitter<TextEditor[]>()
const workspaceOpenDocumentEmitter = new EventEmitter<TextDocument>()
const workspaceCloseDocumentEmitter = new EventEmitter<TextDocument>()
const workspaceChangeDocumentEmitter = new EventEmitter<TextDocumentChangeEvent>()
const workspaceSaveDocumentEmitter = new EventEmitter<TextDocument>()

/**
 * VSCode API namespace implementation
 * This is a subset of the VSCode API that we support in the browser
 */
export const vscode = {
  // Window API
  window: {
    showInformationMessage(message: string, ...items: string[]): Promise<string | undefined> {
      console.info('[Extension]', message)
      return Promise.resolve(items[0])
    },

    showWarningMessage(message: string, ...items: string[]): Promise<string | undefined> {
      console.warn('[Extension]', message)
      return Promise.resolve(items[0])
    },

    showErrorMessage(message: string, ...items: string[]): Promise<string | undefined> {
      console.error('[Extension]', message)
      return Promise.resolve(items[0])
    },

    showQuickPick(items: string[], _options?: { placeHolder?: string }): Promise<string | undefined> {
      return Promise.resolve(items[0])
    },

    showInputBox(_options?: { prompt?: string; value?: string }): Promise<string | undefined> {
      return Promise.resolve(prompt(_options?.prompt || '') || undefined)
    },

    createOutputChannel(name: string): OutputChannel {
      return new OutputChannel(name)
    },

    createTerminal(name?: string): Terminal {
      return new Terminal(name)
    },

    activeTextEditor: undefined as TextEditor | undefined,
    visibleTextEditors: [] as TextEditor[],
    onDidChangeActiveTextEditor: windowActiveTextEditorEmitter.event,
    onDidChangeVisibleTextEditors: windowVisibleTextEditorsEmitter.event
  },

  // Workspace API
  workspace: {
    workspaceFolders: [] as WorkspaceFolder[],
    name: undefined as string | undefined,
    
    getConfiguration(section?: string): WorkspaceConfiguration {
      return new WorkspaceConfiguration(section)
    },

    openTextDocument(_uri: string): Promise<TextDocument> {
      return Promise.resolve({} as TextDocument)
    },

    onDidOpenTextDocument: workspaceOpenDocumentEmitter.event,
    onDidCloseTextDocument: workspaceCloseDocumentEmitter.event,
    onDidChangeTextDocument: workspaceChangeDocumentEmitter.event,
    onDidSaveTextDocument: workspaceSaveDocumentEmitter.event
  },

  // Languages API
  languages: {
    registerCompletionItemProvider(
      selector: DocumentSelector,
      provider: CompletionItemProvider,
      ..._triggerCharacters: string[]
    ): Disposable {
      const disposable = monaco.languages.registerCompletionItemProvider(
        selectorToMonaco(selector),
        {
          provideCompletionItems: async (model, position) => {
            const document = monacoModelToDocument(model)
            const monacoPosition = new Position(position.lineNumber - 1, position.column - 1)
            const result = await provider.provideCompletionItems(
              document,
              monacoPosition,
              {} as any,
              {} as any
            )
            return result ? { suggestions: (result as any[]).map(itemToMonaco) } : undefined
          }
        }
      )
      return { dispose: () => disposable.dispose() }
    },

    registerHoverProvider(
      selector: DocumentSelector,
      provider: HoverProvider
    ): Disposable {
      const disposable = monaco.languages.registerHoverProvider(
        selectorToMonaco(selector),
        {
          provideHover: async (model, position) => {
            const document = monacoModelToDocument(model)
            const monacoPosition = new Position(position.lineNumber - 1, position.column - 1)
            const result = await provider.provideHover(document, monacoPosition, {} as any)
            return result ? hoverToMonaco(result) : undefined
          }
        }
      )
      return { dispose: () => disposable.dispose() }
    },

    registerDefinitionProvider(
      selector: DocumentSelector,
      provider: DefinitionProvider
    ): Disposable {
      const disposable = monaco.languages.registerDefinitionProvider(
        selectorToMonaco(selector),
        {
          provideDefinition: async (model, position) => {
            const document = monacoModelToDocument(model)
            const monacoPosition = new Position(position.lineNumber - 1, position.column - 1)
            const result = await provider.provideDefinition(document, monacoPosition, {} as any)
            return result ? locationToMonaco(result) : undefined
          }
        }
      )
      return { dispose: () => disposable.dispose() }
    },

    getDiagnostics(): [string, Diagnostic[]][] {
      return []
    },

    createDiagnosticCollection(name?: string): DiagnosticCollection {
      return new DiagnosticCollection(name)
    }
  },

  // Commands API
  commands: {
    _commands: new Map<string, (...args: any[]) => any>(),

    registerCommand(command: string, callback: (...args: any[]) => any): Disposable {
      this._commands.set(command, callback)
      return {
        dispose: () => this._commands.delete(command)
      }
    },

    executeCommand<T>(command: string, ...args: any[]): Promise<T | undefined> {
      const handler = this._commands.get(command)
      if (handler) {
        return Promise.resolve(handler(...args))
      }
      return Promise.resolve(undefined)
    },

    getCommands(_filterInternal?: boolean): Promise<string[]> {
      return Promise.resolve(Array.from(this._commands.keys()))
    }
  },

  // Extensions API
  extensions: {
    all: [] as Extension[],
    
    getExtension(extensionId: string): Extension | undefined {
      return this.all.find(e => e.id === extensionId)
    }
  },

  // Types
  Position,
  Range,
  Selection,
  TextEdit,
  Uri,
  DiagnosticSeverity: {
    Error: 0,
    Warning: 1,
    Information: 2,
    Hint: 3
  },
  CompletionItemKind: {
    Text: 0,
    Method: 1,
    Function: 2,
    Constructor: 3,
    Field: 4,
    Variable: 5,
    Class: 6,
    Interface: 7,
    Module: 8,
    Property: 9,
    Unit: 10,
    Value: 11,
    Enum: 12,
    Keyword: 13,
    Snippet: 14,
    Color: 15,
    File: 16,
    Reference: 17,
    Folder: 18
  }
}

/**
 * Extension Host class
 * Manages the sandboxed environment for extension execution
 */
export class ExtensionHost {
  private api = vscode

  /**
   * Get the VSCode API for extensions
   */
  getAPI() {
    return this.api
  }

  /**
   * Execute extension code in a sandboxed environment
   */
  async executeInSandbox(code: string, _context: any): Promise<any> {
    const sandbox = {
      vscode: this.api,
      console,
      setTimeout,
      setInterval,
      clearTimeout,
      clearInterval,
      Promise,
      fetch
    }

    const fn = new Function(...Object.keys(sandbox), code)
    return fn(...Object.values(sandbox))
  }
}

export { vscode as default }
