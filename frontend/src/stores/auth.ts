import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '@/api'
import type { User, LoginRequest, RegisterRequest } from '@/types'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const token = ref<string | null>(localStorage.getItem('token'))

  const isAuthenticated = computed(() => !!token.value)
  const isAdmin = computed(() => user.value?.role?.toLowerCase() === 'admin')

  async function login(credentials: LoginRequest) {
    const response = await api.auth.login(credentials)
    token.value = response.token
    user.value = response.user
    localStorage.setItem('token', response.token)
    api.setAuthToken(response.token)
  }

  async function register(data: RegisterRequest) {
    await api.auth.register(data)
  }

  async function fetchCurrentUser() {
    if (token.value) {
      api.setAuthToken(token.value)
      try {
        user.value = await api.auth.me()
      } catch {
        logout()
      }
    }
  }

  function logout() {
    user.value = null
    token.value = null
    localStorage.removeItem('token')
    api.setAuthToken(null)
  }

  function setToken(newToken: string) {
    token.value = newToken
    localStorage.setItem('token', newToken)
    api.setAuthToken(newToken)
  }

  // Initialize
  if (token.value) {
    api.setAuthToken(token.value)
    fetchCurrentUser()
  }

  return {
    user,
    token,
    isAuthenticated,
    isAdmin,
    login,
    register,
    fetchCurrentUser,
    logout,
    setToken
  }
})
