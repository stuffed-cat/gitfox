import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export type ThemeType = 'dark' | 'light'

export const useThemeStore = defineStore('theme', () => {
  // Load saved theme or default to dark (GitLab style)
  const savedTheme = localStorage.getItem('gitfox-ide-theme') as ThemeType | null
  const theme = ref<ThemeType>(savedTheme || 'dark')

  // Watch for theme changes and persist
  watch(theme, (newTheme) => {
    localStorage.setItem('gitfox-ide-theme', newTheme)
    document.documentElement.setAttribute('data-theme', newTheme)
  }, { immediate: true })

  function toggleTheme() {
    theme.value = theme.value === 'dark' ? 'light' : 'dark'
  }

  function setTheme(newTheme: ThemeType) {
    theme.value = newTheme
  }

  return {
    theme,
    toggleTheme,
    setTheme
  }
})
