import { useRouter } from 'vue-router'
import { startAuthentication } from '@simplewebauthn/browser'
import { useAuthStore } from '@/stores/auth'
import client from '@/api/client'

export function useAuth() {
  const store = useAuthStore()
  const router = useRouter()

  function checkAuth(): boolean {
    return store.isAuthenticated
  }

  async function requestMagicLink(email: string): Promise<void> {
    await client.POST('/auth/magic-link/request', {
      body: { email },
    })
  }

  async function verifyMagicLink(token: string): Promise<boolean> {
    const { data } = await client.GET('/auth/magic-link/verify', {
      params: { query: { token } },
    })

    if (data) {
      store.setTokens(data.accessToken)
      await store.fetchCurrentUser()
      return true
    }
    return false
  }

  async function loginWithPasskey(): Promise<boolean> {
    const { data: beginData } = await client.POST('/auth/passkey/login/begin')

    if (!beginData) return false

    const credential = await startAuthentication({
      optionsJSON: beginData as Parameters<typeof startAuthentication>[0]['optionsJSON'],
    })

    const { data: completeData } = await client.POST('/auth/passkey/login/complete', {
      body: credential,
    })

    if (completeData) {
      store.setTokens(completeData.accessToken)
      await store.fetchCurrentUser()
      return true
    }
    return false
  }

  async function logout(): Promise<void> {
    await store.logout()
    await router.push({ name: 'login' })
  }

  async function refreshToken(): Promise<boolean> {
    return store.refreshToken()
  }

  async function tryAutoLogin(): Promise<boolean> {
    return store.refreshToken()
  }

  return {
    isAuthenticated: store.isAuthenticated,
    currentUser: store.currentUser,
    checkAuth,
    requestMagicLink,
    verifyMagicLink,
    loginWithPasskey,
    logout,
    refreshToken,
    tryAutoLogin,
  }
}
