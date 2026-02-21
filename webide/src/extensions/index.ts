/**
 * GitFox WebIDE Extension System
 * 
 * This module provides a VSCode-compatible extension API that allows
 * loading and running VSCode extensions in the browser environment.
 * 
 * Key features:
 * - Extension lifecycle management (activate/deactivate)
 * - User-level extension isolation
 * - Subset of VSCode API implementation
 * - Extension marketplace integration
 */

import type { Extension, ExtensionContributes } from '@/types'
import { ExtensionHost } from './host'
import { ExtensionStorage } from './storage'

export interface ExtensionManifest {
  name: string
  displayName: string
  description: string
  version: string
  publisher: string
  engines: {
    vscode: string
  }
  categories: string[]
  activationEvents: string[]
  main?: string
  browser?: string
  contributes?: ExtensionContributes
}

export interface ExtensionContext {
  extensionPath: string
  extensionUri: string
  globalState: ExtensionStorage
  workspaceState: ExtensionStorage
  subscriptions: { dispose(): void }[]
}

export interface ExtensionModule {
  activate(context: ExtensionContext): void | Promise<void>
  deactivate?(): void | Promise<void>
}

export class ExtensionManager {
  private extensions = new Map<string, LoadedExtension>()
  private _host: ExtensionHost
  private userId: string

  constructor(userId: string) {
    this.userId = userId
    this._host = new ExtensionHost()
  }

  /**
   * Load extension manifest from URL
   */
  async loadManifest(url: string): Promise<ExtensionManifest> {
    const response = await fetch(`${url}/package.json`)
    return response.json()
  }

  /**
   * Install an extension
   */
  async install(extensionId: string, manifestUrl: string): Promise<void> {
    const manifest = await this.loadManifest(manifestUrl)
    
    // Store extension metadata
    const installedExtensions = await this.getInstalledExtensions()
    installedExtensions[extensionId] = {
      id: extensionId,
      manifest,
      manifestUrl,
      enabled: true,
      installedAt: Date.now()
    }
    
    await this.saveInstalledExtensions(installedExtensions)
    
    // Activate if should be activated
    if (this.shouldActivate(manifest)) {
      await this.activate(extensionId)
    }
  }

  /**
   * Uninstall an extension
   */
  async uninstall(extensionId: string): Promise<void> {
    // Deactivate first
    await this.deactivate(extensionId)
    
    // Remove from storage
    const installedExtensions = await this.getInstalledExtensions()
    delete installedExtensions[extensionId]
    await this.saveInstalledExtensions(installedExtensions)
  }

  /**
   * Activate an extension
   */
  async activate(extensionId: string): Promise<void> {
    const installed = await this.getInstalledExtensions()
    const extInfo = installed[extensionId]
    
    if (!extInfo) {
      throw new Error(`Extension ${extensionId} not installed`)
    }

    if (this.extensions.has(extensionId)) {
      return // Already activated
    }

    try {
      // Load extension module
      const moduleUrl = extInfo.manifest.browser || extInfo.manifest.main
      if (!moduleUrl) {
        // Extension without code (just contributes)
        this.extensions.set(extensionId, {
          id: extensionId,
          manifest: extInfo.manifest,
          context: this.createContext(extensionId, extInfo.manifestUrl)
        })
        return
      }

      const module = await this.loadExtensionModule(extInfo.manifestUrl, moduleUrl)
      const context = this.createContext(extensionId, extInfo.manifestUrl)
      
      // Activate
      await module.activate(context)
      
      this.extensions.set(extensionId, {
        id: extensionId,
        manifest: extInfo.manifest,
        module,
        context
      })

      console.log(`Extension ${extensionId} activated`)
    } catch (error) {
      console.error(`Failed to activate extension ${extensionId}:`, error)
      throw error
    }
  }

  /**
   * Deactivate an extension
   */
  async deactivate(extensionId: string): Promise<void> {
    const extension = this.extensions.get(extensionId)
    if (!extension) return

    try {
      // Call deactivate if exists
      if (extension.module?.deactivate) {
        await extension.module.deactivate()
      }

      // Dispose subscriptions
      for (const sub of extension.context.subscriptions) {
        sub.dispose()
      }

      this.extensions.delete(extensionId)
      console.log(`Extension ${extensionId} deactivated`)
    } catch (error) {
      console.error(`Failed to deactivate extension ${extensionId}:`, error)
    }
  }

  /**
   * Get all installed extensions for current user
   */
  async getInstalledExtensions(): Promise<Record<string, InstalledExtension>> {
    const key = `gitfox-extensions-${this.userId}`
    const stored = localStorage.getItem(key)
    return stored ? JSON.parse(stored) : {}
  }

  /**
   * Save installed extensions
   */
  private async saveInstalledExtensions(extensions: Record<string, InstalledExtension>): Promise<void> {
    const key = `gitfox-extensions-${this.userId}`
    localStorage.setItem(key, JSON.stringify(extensions))
  }

  /**
   * Check if extension should be activated based on activation events
   */
  private shouldActivate(manifest: ExtensionManifest): boolean {
    if (!manifest.activationEvents) return false
    return manifest.activationEvents.includes('*') ||
           manifest.activationEvents.includes('onStartup')
  }

  /**
   * Load extension module dynamically
   */
  private async loadExtensionModule(baseUrl: string, modulePath: string): Promise<ExtensionModule> {
    const url = `${baseUrl}/${modulePath}`
    const module = await import(/* @vite-ignore */ url)
    return module
  }

  /**
   * Create extension context
   */
  private createContext(extensionId: string, extensionPath: string): ExtensionContext {
    return {
      extensionPath,
      extensionUri: extensionPath,
      globalState: new ExtensionStorage(`${this.userId}-${extensionId}-global`),
      workspaceState: new ExtensionStorage(`${this.userId}-${extensionId}-workspace`),
      subscriptions: []
    }
  }

  /**
   * Get extension contributions (themes, languages, etc.)
   */
  getContributions(type: keyof ExtensionContributes): any[] {
    const contributions: any[] = []
    
    for (const extension of this.extensions.values()) {
      const contrib = extension.manifest.contributes?.[type]
      if (contrib) {
        contributions.push(...contrib)
      }
    }
    
    return contributions
  }

  /**
   * Get the VSCode API for extensions
   */
  getAPI() {
    return this._host.getAPI()
  }

  /**
   * Get all active extensions
   */
  getActiveExtensions(): Extension[] {
    return Array.from(this.extensions.values()).map(ext => ({
      id: ext.id,
      name: ext.manifest.displayName || ext.manifest.name,
      version: ext.manifest.version,
      description: ext.manifest.description,
      publisher: ext.manifest.publisher,
      enabled: true,
      categories: ext.manifest.categories || [],
      activationEvents: ext.manifest.activationEvents || [],
      contributes: ext.manifest.contributes
    }))
  }
}

interface LoadedExtension {
  id: string
  manifest: ExtensionManifest
  module?: ExtensionModule
  context: ExtensionContext
}

interface InstalledExtension {
  id: string
  manifest: ExtensionManifest
  manifestUrl: string
  enabled: boolean
  installedAt: number
}

// Singleton instance
let extensionManager: ExtensionManager | null = null

export function getExtensionManager(userId: string): ExtensionManager {
  if (!extensionManager || extensionManager['userId'] !== userId) {
    extensionManager = new ExtensionManager(userId)
  }
  return extensionManager
}
