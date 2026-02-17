import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '@/api'
import type { User, LoginRequest, RegisterRequest, TwoFactorRequiredResponse, VerifyTwoFactorRequest } from '@/types'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const token = ref<string | null>(localStorage.getItem('token'))
  const twoFactorRequired = ref<TwoFactorRequiredResponse | null>(null)

  const isAuthenticated = computed(() => !!token.value)
  const isAdmin = computed(() => user.value?.role?.toLowerCase() === 'admin')

  async function login(credentials: LoginRequest) {
    const response = await api.auth.login(credentials)
    
    // Check if 2FA is required
    if ('requires_two_factor' in response && response.requires_two_factor) {
      twoFactorRequired.value = response as TwoFactorRequiredResponse
      return response as TwoFactorRequiredResponse
    }
    
    // Regular login without 2FA - type assertion since we checked above
    const loginResponse = response as { token: string; user: User }
    token.value = loginResponse.token
    user.value = loginResponse.user
    localStorage.setItem('token', loginResponse.token)
    api.setAuthToken(loginResponse.token)
    return response
  }

  async function verifyTwoFactor(request: VerifyTwoFactorRequest) {
    const response = await api.auth.verifyTwoFactor(request)
    token.value = response.token
    user.value = response.user
    localStorage.setItem('token', response.token)
    api.setAuthToken(response.token)
    twoFactorRequired.value = null
    return response
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
    twoFactorRequired,
    isAuthenticated,
    isAdmin,
    login,
    verifyTwoFactor,
    register,
    fetchCurrentUser,
    logout,
    setToken
  }
})
