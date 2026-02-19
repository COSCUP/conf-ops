<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuth } from '@/composables/useAuth'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'

const router = useRouter()
const { requestMagicLink, loginWithPasskey } = useAuth()

const email = ref('')
const loading = ref(false)
const magicLinkSent = ref(false)
const error = ref('')

async function handleMagicLink() {
  if (!email.value) {
    error.value = 'Please enter your email address.'
    return
  }

  loading.value = true
  error.value = ''

  try {
    await requestMagicLink(email.value)
    magicLinkSent.value = true
  } catch {
    error.value = 'Failed to send magic link. Please try again.'
  } finally {
    loading.value = false
  }
}

async function handlePasskeyLogin() {
  loading.value = true
  error.value = ''

  try {
    const success = await loginWithPasskey()
    if (success) {
      await router.push({ name: 'dashboard' })
    } else {
      error.value = 'Passkey login failed. Please try again.'
    }
  } catch {
    error.value = 'Passkey login failed or was cancelled.'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="login-view">
    <template v-if="magicLinkSent">
      <div class="magic-link-sent">
        <h2>Check your email</h2>
        <p>We sent a login link to <strong>{{ email }}</strong>.</p>
        <p>Click the link in the email to sign in.</p>
        <BaseButton variant="secondary" @click="magicLinkSent = false">
          Back to login
        </BaseButton>
      </div>
    </template>

    <template v-else>
      <h2>Sign in</h2>

      <p v-if="error" class="error-message">{{ error }}</p>

      <form class="login-form" @submit.prevent="handleMagicLink">
        <BaseInput
          v-model="email"
          label="Email"
          type="email"
          placeholder="you@example.com"
        />
        <BaseButton :disabled="loading" @click="handleMagicLink">
          {{ loading ? 'Sending...' : 'Send magic link' }}
        </BaseButton>
      </form>

      <div class="divider">
        <span>or</span>
      </div>

      <BaseButton
        variant="secondary"
        :disabled="loading"
        @click="handlePasskeyLogin"
      >
        Sign in with Passkey
      </BaseButton>
    </template>
  </div>
</template>

<style scoped>
.login-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.divider {
  display: flex;
  align-items: center;
  gap: 1rem;
  color: #9ca3af;
  font-size: 0.875rem;
}

.divider::before,
.divider::after {
  content: '';
  flex: 1;
  border-bottom: 1px solid #e5e7eb;
}

.error-message {
  color: #ef4444;
  font-size: 0.875rem;
  margin: 0;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
}

.magic-link-sent {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  text-align: center;
}

.magic-link-sent p {
  margin: 0;
  color: #6b7280;
}
</style>
