<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuth } from '@/composables/useAuth'

const route = useRoute()
const router = useRouter()
const { verifyMagicLink } = useAuth()

const status = ref<'verifying' | 'success' | 'error'>('verifying')
const errorMessage = ref('')

onMounted(async () => {
  const token = route.query.token as string | undefined

  if (!token) {
    status.value = 'error'
    errorMessage.value = 'Invalid or missing token.'
    return
  }

  try {
    const success = await verifyMagicLink(token)
    if (success) {
      status.value = 'success'
      await router.push({ name: 'dashboard' })
    } else {
      status.value = 'error'
      errorMessage.value = 'The link is invalid or has expired.'
    }
  } catch {
    status.value = 'error'
    errorMessage.value = 'Verification failed. Please request a new link.'
  }
})
</script>

<template>
  <div class="verify-view">
    <template v-if="status === 'verifying'">
      <h2>Verifying...</h2>
      <p>Please wait while we verify your login link.</p>
    </template>

    <template v-else-if="status === 'error'">
      <h2>Verification failed</h2>
      <p class="error-message">{{ errorMessage }}</p>
      <RouterLink to="/login">Back to login</RouterLink>
    </template>
  </div>
</template>

<style scoped>
.verify-view {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
  text-align: center;
}

.error-message {
  color: #ef4444;
}
</style>
