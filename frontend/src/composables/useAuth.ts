import { ref } from 'vue'

const isAuthenticated = ref(false)

export function useAuth() {
  function checkAuth(): boolean {
    // Skeleton: will be implemented in Phase 1
    return isAuthenticated.value
  }

  function setAuthenticated(value: boolean) {
    isAuthenticated.value = value
  }

  return {
    isAuthenticated,
    checkAuth,
    setAuthenticated,
  }
}
