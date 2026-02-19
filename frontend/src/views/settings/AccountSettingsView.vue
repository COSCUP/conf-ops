<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useAccount } from '@/composables/useAccount'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const authStore = useAuthStore()
const {
  notificationPreferences,
  passkeys,
  loading,
  error,
  updateDisplayName,
  fetchNotificationPreferences,
  updateNotificationPreferences,
  fetchPasskeys,
  deletePasskey,
} = useAccount()

const displayName = ref('')
const displayNameSaved = ref(false)

onMounted(async () => {
  displayName.value = authStore.currentUser?.display_name ?? ''
  await Promise.all([fetchNotificationPreferences(), fetchPasskeys()])
})

async function handleUpdateDisplayName() {
  displayNameSaved.value = false
  await updateDisplayName(displayName.value)
  if (!error.value) {
    displayNameSaved.value = true
  }
}

async function handleToggleEmailNotifications() {
  await updateNotificationPreferences({
    ...notificationPreferences.value,
    email_notifications: !notificationPreferences.value.email_notifications,
  })
}

async function handleDeletePasskey(id: string) {
  await deletePasskey(id)
}
</script>

<template>
  <div class="settings-view">
    <h1>Account Settings</h1>

    <p v-if="error" class="error-message">{{ error }}</p>

    <BaseCard title="Display Name">
      <form class="settings-form" @submit.prevent="handleUpdateDisplayName">
        <BaseInput v-model="displayName" label="Display Name" />
        <div class="form-actions">
          <BaseButton :disabled="loading">Save</BaseButton>
          <span v-if="displayNameSaved" class="success-text">Saved!</span>
        </div>
      </form>
    </BaseCard>

    <BaseCard title="Passkeys">
      <div v-if="passkeys.length === 0" class="empty-state">
        No passkeys registered.
      </div>
      <ul v-else class="passkey-list">
        <li v-for="passkey in passkeys" :key="passkey.id" class="passkey-item">
          <div class="passkey-info">
            <strong>{{ passkey.name }}</strong>
            <span class="passkey-date">Added {{ passkey.created_at }}</span>
          </div>
          <BaseButton
            variant="danger"
            :disabled="loading"
            @click="handleDeletePasskey(passkey.id)"
          >
            Delete
          </BaseButton>
        </li>
      </ul>
    </BaseCard>

    <BaseCard title="Notification Preferences">
      <div class="notification-toggle">
        <label>
          <input
            type="checkbox"
            :checked="notificationPreferences.email_notifications"
            :disabled="loading"
            @change="handleToggleEmailNotifications"
          />
          Email notifications
        </label>
      </div>
    </BaseCard>
  </div>
</template>

<style scoped>
.settings-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 600px;
}

.settings-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.form-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.success-text {
  color: #16a34a;
  font-size: 0.875rem;
}

.error-message {
  color: #ef4444;
  font-size: 0.875rem;
  margin: 0;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
}

.passkey-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.passkey-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.passkey-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.passkey-date {
  font-size: 0.75rem;
  color: #9ca3af;
}

.empty-state {
  color: #9ca3af;
  font-size: 0.875rem;
}

.notification-toggle label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}
</style>
