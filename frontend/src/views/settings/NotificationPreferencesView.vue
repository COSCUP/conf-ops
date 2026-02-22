<script setup lang="ts">
import { ref, onMounted } from 'vue'
import client from '@/api/client'

interface ChannelPreference {
  enabled: boolean
  categories: Record<string, boolean>
}

interface Preferences {
  channels: {
    email: ChannelPreference
    webPush: ChannelPreference
    inApp: ChannelPreference
  }
}

const loading = ref(false)
const saving = ref(false)
const error = ref<string | null>(null)
const successMessage = ref<string | null>(null)

const channelNames = ['inApp', 'webPush', 'email'] as const
const categoryNames = ['mention', 'task_update', 'reminder', 'system', 'collaboration'] as const

const channelLabels: Record<string, string> = {
  inApp: 'In-App',
  webPush: 'Web Push',
  email: 'Email',
}

const categoryLabels: Record<string, string> = {
  mention: 'Mentions',
  task_update: 'Task Updates',
  reminder: 'Reminders',
  system: 'System',
  collaboration: 'Collaboration',
}

function defaultPreferences(): Preferences {
  return {
    channels: {
      email: { enabled: true, categories: {} },
      webPush: { enabled: true, categories: {} },
      inApp: { enabled: true, categories: {} },
    },
  }
}

const preferences = ref<Preferences>(defaultPreferences())

onMounted(async () => {
  await loadPreferences()
})

async function loadPreferences() {
  loading.value = true
  error.value = null
  try {
    const { data } = await client.GET('/api/v1/notifications/preferences' as never)
    const result = data as { preferences: Preferences } | undefined
    if (result?.preferences?.channels) {
      preferences.value = result.preferences
    }
  } catch {
    error.value = 'Failed to load notification preferences.'
  } finally {
    loading.value = false
  }
}

async function savePreferences() {
  saving.value = true
  error.value = null
  successMessage.value = null
  try {
    await client.PUT('/api/v1/notifications/preferences' as never, {
      body: { preferences: preferences.value },
    } as never)
    successMessage.value = 'Preferences saved successfully.'
    setTimeout(() => {
      successMessage.value = null
    }, 3000)
  } catch {
    error.value = 'Failed to save notification preferences.'
  } finally {
    saving.value = false
  }
}

function isCategoryEnabled(channel: string, category: string): boolean {
  const ch = preferences.value.channels[channel as keyof Preferences['channels']]
  if (!ch) return true
  if (!ch.enabled) return false
  const val = ch.categories[category]
  return val !== false
}

function toggleCategory(channel: string, category: string) {
  const ch = preferences.value.channels[channel as keyof Preferences['channels']]
  if (!ch) return
  const current = ch.categories[category]
  ch.categories[category] = current === false
}
</script>

<template>
  <div class="notification-preferences">
    <div class="header">
      <h2>Notification Preferences</h2>
    </div>

    <div v-if="error" class="error-banner">{{ error }}</div>
    <div v-if="successMessage" class="success-banner">{{ successMessage }}</div>

    <div v-if="loading" class="loading">Loading preferences...</div>

    <div v-else class="preferences-grid">
      <table class="prefs-table">
        <thead>
          <tr>
            <th>Category</th>
            <th v-for="channel in channelNames" :key="channel">
              {{ channelLabels[channel] }}
            </th>
          </tr>
        </thead>
        <tbody>
          <!-- Channel master toggles -->
          <tr class="master-row">
            <td class="category-label"><strong>All notifications</strong></td>
            <td v-for="channel in channelNames" :key="channel">
              <label class="toggle-label">
                <input
                  v-model="preferences.channels[channel].enabled"
                  type="checkbox"
                  class="toggle-input"
                />
                <span class="toggle-text">
                  {{ preferences.channels[channel].enabled ? 'On' : 'Off' }}
                </span>
              </label>
            </td>
          </tr>
          <!-- Per-category toggles -->
          <tr v-for="category in categoryNames" :key="category">
            <td class="category-label">{{ categoryLabels[category] }}</td>
            <td v-for="channel in channelNames" :key="channel">
              <label class="toggle-label">
                <input
                  type="checkbox"
                  class="toggle-input"
                  :checked="isCategoryEnabled(channel, category)"
                  :disabled="!preferences.channels[channel].enabled"
                  @change="toggleCategory(channel, category)"
                />
              </label>
            </td>
          </tr>
        </tbody>
      </table>

      <div class="save-actions">
        <button
          type="button"
          class="btn btn-primary"
          :disabled="saving"
          @click="savePreferences"
        >
          {{ saving ? 'Saving...' : 'Save preferences' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.notification-preferences {
  padding: 1rem;
}

.header {
  margin-bottom: 1.5rem;
}

.header h2 {
  margin: 0;
}

.error-banner {
  background: #fef2f2;
  color: #dc2626;
  padding: 0.75rem;
  border-radius: 0.375rem;
  margin-bottom: 1rem;
}

.success-banner {
  background: #f0fdf4;
  color: #166534;
  padding: 0.75rem;
  border-radius: 0.375rem;
  margin-bottom: 1rem;
}

.loading {
  color: #6b7280;
  padding: 2rem 0;
  text-align: center;
}

.preferences-grid {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.prefs-table {
  width: 100%;
  border-collapse: collapse;
}

.prefs-table th,
.prefs-table td {
  padding: 0.625rem 0.75rem;
  text-align: center;
  border-bottom: 1px solid #e5e7eb;
}

.prefs-table th {
  font-size: 0.875rem;
  font-weight: 600;
  color: #374151;
  background: #f9fafb;
}

.category-label {
  text-align: left;
  font-size: 0.875rem;
  color: #1f2937;
}

.master-row {
  background: #f9fafb;
}

.toggle-label {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  cursor: pointer;
}

.toggle-input {
  cursor: pointer;
}

.toggle-input:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.toggle-text {
  font-size: 0.75rem;
  color: #6b7280;
}

.save-actions {
  display: flex;
  justify-content: flex-end;
}

.btn {
  padding: 0.5rem 1rem;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  cursor: pointer;
  border: 1px solid transparent;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: #2563eb;
  color: white;
}
</style>
