<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAccount } from '@/composables/useAccount'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const { profileData, profileSchema, loading, error, fetchProfile, updateProfile } = useAccount()

const saved = ref(false)

onMounted(async () => {
  await fetchProfile()
})

async function handleSave() {
  saved.value = false
  await updateProfile({ ...profileData.value })
  if (!error.value) {
    saved.value = true
  }
}
</script>

<template>
  <div class="profile-view">
    <h1>Profile</h1>

    <p v-if="error" class="error-message">{{ error }}</p>

    <BaseCard title="Profile Information">
      <template v-if="profileSchema.length === 0">
        <p class="empty-schema">No profile fields have been configured yet.</p>
      </template>

      <form v-else class="profile-form" @submit.prevent="handleSave">
        <div v-for="field in profileSchema" :key="field.key" class="profile-field">
          <template v-if="field.type === 'checkbox'">
            <label class="checkbox-label">
              <input
                type="checkbox"
                :checked="Boolean(profileData[field.key])"
                @change="
                  profileData[field.key] = ($event.target as HTMLInputElement).checked
                "
              />
              {{ field.label }}
            </label>
            <p v-if="field.description" class="field-description">{{ field.description }}</p>
          </template>
          <template v-else>
            <BaseInput
              :model-value="String(profileData[field.key] ?? '')"
              :label="field.label"
              :type="field.type === 'number' ? 'number' : field.type"
              :placeholder="field.description"
              @update:model-value="profileData[field.key] = $event"
            />
          </template>
        </div>

        <div class="form-actions">
          <BaseButton :disabled="loading">
            {{ loading ? 'Saving...' : 'Save' }}
          </BaseButton>
          <span v-if="saved" class="success-text">Saved!</span>
        </div>
      </form>
    </BaseCard>
  </div>
</template>

<style scoped>
.profile-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 600px;
}

.profile-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.profile-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
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

.empty-schema {
  color: #9ca3af;
  font-size: 0.875rem;
  margin: 0;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.field-description {
  font-size: 0.75rem;
  color: #9ca3af;
  margin: 0;
}
</style>
