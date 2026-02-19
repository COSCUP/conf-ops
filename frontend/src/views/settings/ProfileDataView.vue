<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAccount } from '@/composables/useAccount'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const { profile, loading, error, fetchProfile, updateProfile } = useAccount()

const bio = ref('')
const saved = ref(false)

onMounted(async () => {
  await fetchProfile()
  bio.value = profile.value.bio ?? ''
})

async function handleSave() {
  saved.value = false
  await updateProfile({ ...profile.value, bio: bio.value })
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
      <form class="profile-form" @submit.prevent="handleSave">
        <BaseInput v-model="bio" label="Bio" placeholder="Tell us about yourself" />
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
</style>
