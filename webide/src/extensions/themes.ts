/**
 * Theme Manager
 * 
 * Manages Monaco editor themes and supports loading VSCode themes
 */

import * as monaco from 'monaco-editor'

export interface ThemeDefinition {
  name: string
  type: 'dark' | 'light'
  colors: Record<string, string>
  tokenColors: TokenColor[]
}

interface TokenColor {
  scope: string | string[]
  settings: {
    foreground?: string
    background?: string
    fontStyle?: string
  }
}

// GitFox Dark Theme (Catppuccin Mocha based)
export const gitfoxDarkTheme: monaco.editor.IStandaloneThemeData = {
  base: 'vs-dark',
  inherit: true,
  rules: [
    // Comments
    { token: 'comment', foreground: '6c7086', fontStyle: 'italic' },
    { token: 'comment.line', foreground: '6c7086', fontStyle: 'italic' },
    { token: 'comment.block', foreground: '6c7086', fontStyle: 'italic' },
    
    // Keywords
    { token: 'keyword', foreground: 'cba6f7' },
    { token: 'keyword.control', foreground: 'cba6f7' },
    { token: 'keyword.operator', foreground: '89dceb' },
    
    // Storage/Types
    { token: 'storage', foreground: 'cba6f7' },
    { token: 'storage.type', foreground: 'f9e2af' },
    { token: 'type', foreground: 'f9e2af' },
    { token: 'type.identifier', foreground: 'f9e2af' },
    
    // Functions
    { token: 'entity.name.function', foreground: '89b4fa' },
    { token: 'support.function', foreground: '89b4fa' },
    { token: 'function', foreground: '89b4fa' },
    
    // Variables
    { token: 'variable', foreground: 'cdd6f4' },
    { token: 'variable.parameter', foreground: 'fab387' },
    { token: 'variable.other', foreground: 'cdd6f4' },
    
    // Strings
    { token: 'string', foreground: 'a6e3a1' },
    { token: 'string.quoted', foreground: 'a6e3a1' },
    { token: 'string.template', foreground: 'a6e3a1' },
    
    // Numbers
    { token: 'constant.numeric', foreground: 'fab387' },
    { token: 'number', foreground: 'fab387' },
    
    // Constants
    { token: 'constant', foreground: 'fab387' },
    { token: 'constant.language', foreground: 'fab387' },
    { token: 'constant.character', foreground: 'f38ba8' },
    
    // Punctuation
    { token: 'punctuation', foreground: '9399b2' },
    { token: 'delimiter', foreground: '9399b2' },
    
    // Tags (HTML/XML)
    { token: 'tag', foreground: 'cba6f7' },
    { token: 'tag.attribute.name', foreground: 'f9e2af' },
    { token: 'tag.attribute.value', foreground: 'a6e3a1' },
    
    // Markup
    { token: 'markup.heading', foreground: 'f38ba8', fontStyle: 'bold' },
    { token: 'markup.bold', fontStyle: 'bold' },
    { token: 'markup.italic', fontStyle: 'italic' },
    { token: 'markup.underline', fontStyle: 'underline' },
    
    // JSON
    { token: 'string.key.json', foreground: '89b4fa' },
    { token: 'string.value.json', foreground: 'a6e3a1' },
    
    // CSS
    { token: 'attribute.name.css', foreground: 'f9e2af' },
    { token: 'attribute.value.css', foreground: 'a6e3a1' },
    { token: 'tag.css', foreground: 'cba6f7' },
    
    // Regex
    { token: 'regexp', foreground: 'f38ba8' },
    
    // Invalid
    { token: 'invalid', foreground: 'f38ba8', fontStyle: 'strikethrough' }
  ],
  colors: {
    // Editor
    'editor.background': '#1e1e2e',
    'editor.foreground': '#cdd6f4',
    'editor.lineHighlightBackground': '#313244',
    'editor.selectionBackground': '#45475a',
    'editor.inactiveSelectionBackground': '#313244',
    'editor.findMatchBackground': '#f9e2af40',
    'editor.findMatchHighlightBackground': '#f9e2af20',
    'editorCursor.foreground': '#f5e0dc',
    'editorWhitespace.foreground': '#45475a',
    
    // Line numbers
    'editorLineNumber.foreground': '#6c7086',
    'editorLineNumber.activeForeground': '#cdd6f4',
    
    // Gutter
    'editorGutter.background': '#1e1e2e',
    'editorGutter.addedBackground': '#a6e3a1',
    'editorGutter.modifiedBackground': '#f9e2af',
    'editorGutter.deletedBackground': '#f38ba8',
    
    // Minimap
    'minimap.background': '#181825',
    'minimapSlider.background': '#45475a40',
    'minimapSlider.hoverBackground': '#45475a60',
    'minimapSlider.activeBackground': '#45475a80',
    
    // Editor widgets
    'editorWidget.background': '#181825',
    'editorWidget.border': '#313244',
    'editorSuggestWidget.background': '#181825',
    'editorSuggestWidget.border': '#313244',
    'editorSuggestWidget.foreground': '#cdd6f4',
    'editorSuggestWidget.selectedBackground': '#45475a',
    
    // Input
    'input.background': '#313244',
    'input.border': '#45475a',
    'input.foreground': '#cdd6f4',
    'input.placeholderForeground': '#6c7086',
    
    // Scrollbar
    'scrollbar.shadow': '#11111b',
    'scrollbarSlider.background': '#45475a80',
    'scrollbarSlider.hoverBackground': '#45475aa0',
    'scrollbarSlider.activeBackground': '#45475ac0',
    
    // Bracket matching
    'editorBracketMatch.background': '#45475a',
    'editorBracketMatch.border': '#89b4fa',
    
    // Indent guides
    'editorIndentGuide.background': '#313244',
    'editorIndentGuide.activeBackground': '#45475a',
    
    // Ruler
    'editorRuler.foreground': '#313244',
    
    // Overview ruler (scrollbar decorations)
    'editorOverviewRuler.border': '#313244',
    'editorOverviewRuler.errorForeground': '#f38ba8',
    'editorOverviewRuler.warningForeground': '#f9e2af',
    'editorOverviewRuler.infoForeground': '#89b4fa'
  }
}

// GitFox Light Theme
export const gitfoxLightTheme: monaco.editor.IStandaloneThemeData = {
  base: 'vs',
  inherit: true,
  rules: [
    { token: 'comment', foreground: '8c8fa1', fontStyle: 'italic' },
    { token: 'keyword', foreground: '8839ef' },
    { token: 'storage.type', foreground: 'df8e1d' },
    { token: 'entity.name.function', foreground: '1e66f5' },
    { token: 'string', foreground: '40a02b' },
    { token: 'number', foreground: 'fe640b' },
    { token: 'constant', foreground: 'fe640b' }
  ],
  colors: {
    'editor.background': '#eff1f5',
    'editor.foreground': '#4c4f69',
    'editor.lineHighlightBackground': '#e6e9ef',
    'editor.selectionBackground': '#acb0be',
    'editorCursor.foreground': '#dc8a78',
    'editorLineNumber.foreground': '#8c8fa1',
    'editorLineNumber.activeForeground': '#4c4f69'
  }
}

export class ThemeManager {
  private registeredThemes = new Map<string, monaco.editor.IStandaloneThemeData>()
  private currentTheme = 'gitfox-dark'

  constructor() {
    // Register built-in themes
    this.registerTheme('gitfox-dark', gitfoxDarkTheme)
    this.registerTheme('gitfox-light', gitfoxLightTheme)
  }

  /**
   * Register a theme with Monaco
   */
  registerTheme(name: string, theme: monaco.editor.IStandaloneThemeData): void {
    monaco.editor.defineTheme(name, theme)
    this.registeredThemes.set(name, theme)
  }

  /**
   * Convert VSCode theme to Monaco format
   */
  convertVSCodeTheme(theme: ThemeDefinition): monaco.editor.IStandaloneThemeData {
    const rules: monaco.editor.ITokenThemeRule[] = []
    
    for (const tokenColor of theme.tokenColors) {
      const scopes = Array.isArray(tokenColor.scope) ? tokenColor.scope : [tokenColor.scope]
      
      for (const scope of scopes) {
        const rule: monaco.editor.ITokenThemeRule = { token: scope }
        
        if (tokenColor.settings.foreground) {
          rule.foreground = tokenColor.settings.foreground.replace('#', '')
        }
        if (tokenColor.settings.background) {
          rule.background = tokenColor.settings.background.replace('#', '')
        }
        if (tokenColor.settings.fontStyle) {
          rule.fontStyle = tokenColor.settings.fontStyle
        }
        
        rules.push(rule)
      }
    }

    return {
      base: theme.type === 'dark' ? 'vs-dark' : 'vs',
      inherit: true,
      rules,
      colors: theme.colors
    }
  }

  /**
   * Load a theme from URL (VSCode theme format)
   */
  async loadThemeFromUrl(name: string, url: string): Promise<void> {
    const response = await fetch(url)
    const themeData: ThemeDefinition = await response.json()
    
    const monacoTheme = this.convertVSCodeTheme(themeData)
    this.registerTheme(name, monacoTheme)
  }

  /**
   * Apply a theme
   */
  applyTheme(name: string): void {
    if (!this.registeredThemes.has(name)) {
      console.warn(`Theme ${name} not found, using gitfox-dark`)
      name = 'gitfox-dark'
    }
    
    monaco.editor.setTheme(name)
    this.currentTheme = name
    
    // Update CSS variables based on theme
    const theme = this.registeredThemes.get(name)
    if (theme?.colors) {
      const root = document.documentElement
      
      if (theme.colors['editor.background']) {
        root.style.setProperty('--monaco-editor-bg', theme.colors['editor.background'])
      }
      if (theme.colors['editor.foreground']) {
        root.style.setProperty('--monaco-editor-fg', theme.colors['editor.foreground'])
      }
    }
  }

  /**
   * Get current theme name
   */
  getCurrentTheme(): string {
    return this.currentTheme
  }

  /**
   * Get all registered theme names
   */
  getThemes(): string[] {
    return Array.from(this.registeredThemes.keys())
  }

  /**
   * Check if a theme is dark
   */
  isDarkTheme(name: string): boolean {
    const theme = this.registeredThemes.get(name)
    return theme?.base === 'vs-dark'
  }
}

// Singleton instance
let themeManager: ThemeManager | null = null

export function getThemeManager(): ThemeManager {
  if (!themeManager) {
    themeManager = new ThemeManager()
  }
  return themeManager
}
