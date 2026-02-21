/**
 * Extension Storage
 * 
 * Provides persistent storage for extensions, isolated per user.
 * Implements a subset of the VSCode Memento interface.
 */

export class ExtensionStorage {
  private storageKey: string
  private cache: Map<string, any> = new Map()

  constructor(storageKey: string) {
    this.storageKey = `gitfox-ext-storage-${storageKey}`
    this.load()
  }

  /**
   * Load data from localStorage
   */
  private load(): void {
    try {
      const stored = localStorage.getItem(this.storageKey)
      if (stored) {
        const data = JSON.parse(stored)
        this.cache = new Map(Object.entries(data))
      }
    } catch (error) {
      console.error('Failed to load extension storage:', error)
    }
  }

  /**
   * Save data to localStorage
   */
  private save(): void {
    try {
      const data = Object.fromEntries(this.cache)
      localStorage.setItem(this.storageKey, JSON.stringify(data))
    } catch (error) {
      console.error('Failed to save extension storage:', error)
    }
  }

  /**
   * Get a value from storage
   */
  get<T>(key: string): T | undefined
  get<T>(key: string, defaultValue: T): T
  get<T>(key: string, defaultValue?: T): T | undefined {
    const value = this.cache.get(key)
    return value !== undefined ? value : defaultValue
  }

  /**
   * Store a value
   */
  async update(key: string, value: any): Promise<void> {
    if (value === undefined) {
      this.cache.delete(key)
    } else {
      this.cache.set(key, value)
    }
    this.save()
  }

  /**
   * Get all keys
   */
  keys(): readonly string[] {
    return Array.from(this.cache.keys())
  }

  /**
   * Clear all data
   */
  clear(): void {
    this.cache.clear()
    localStorage.removeItem(this.storageKey)
  }
}
