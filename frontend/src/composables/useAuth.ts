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
    await client.POST('/api/v1/auth/magic-link/request' as never, {
      body: { email },
    } as never)
  }

  async function verifyMagicLink(token: string): Promise<boolean> {
    const { data } = await client.POST('/api/v1/auth/magic-link/verify' as never, {
      body: { token },
    } as never)

    if (data) {
      const tokenData = data as { access_token: string }
      store.setTokens(tokenData.access_token)
      await store.fetchCurrentUser()
      return true
    }
    return false
  }

  async function loginWithPasskey(): Promise<boolean> {
    const { data: beginData } = await client.POST(
      '/api/v1/auth/passkeys/login/begin' as never,
    )

    if (!beginData) return false

    const begin = beginData as { challenge_id: string; options: unknown }

    const credential = await startAuthentication({
      optionsJSON: begin.options as Parameters<typeof startAuthentication>[0]['optionsJSON'],
    })

    const { data: completeData } = await client.POST(
      '/api/v1/auth/passkeys/login/complete' as never,
      {
        body: {
          challenge_id: begin.challenge_id,
          credential,
        },
      } as never,
    )

    if (completeData) {
      const tokenData = completeData as { access_token: string }
      store.setTokens(tokenData.access_token)
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
