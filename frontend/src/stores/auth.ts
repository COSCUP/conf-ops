import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type AccountResponse = components['schemas']['AccountResponse']

export const useAuthStore = defineStore('auth', () => {
  const accessToken = ref<string | null>(null)
  const currentUser = ref<AccountResponse | null>(null)
  const isAuthenticated = computed(() => accessToken.value !== null)

  function setTokens(token: string) {
    accessToken.value = token
  }

  function clearAuth() {
    accessToken.value = null
    currentUser.value = null
  }

  async function fetchCurrentUser() {
    if (!accessToken.value) return

    const { data } = await client.GET('/api/v1/accounts/me')

    if (data) {
      currentUser.value = data
    }
  }

  async function refreshToken(): Promise<boolean> {
    try {
      const { data } = await client.POST('/api/v1/auth/refresh')

      if (data) {
        accessToken.value = data.accessToken
        await fetchCurrentUser()
        return true
      }
      return false
    } catch {
      clearAuth()
      return false
    }
  }

  async function logout() {
    try {
      if (accessToken.value) {
        await client.POST('/api/v1/auth/logout')
      }
    } finally {
      clearAuth()
    }
  }

  return {
    accessToken,
    currentUser,
    isAuthenticated,
    setTokens,
    clearAuth,
    fetchCurrentUser,
    refreshToken,
    logout,
  }
})
