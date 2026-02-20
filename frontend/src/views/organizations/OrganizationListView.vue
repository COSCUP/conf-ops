<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useOrganizationStore } from '@/stores/organization'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const router = useRouter()
const store = useOrganizationStore()

const showCreateForm = ref(false)
const newOrgName = ref('')
const newOrgDescription = ref('')

onMounted(async () => {
  await store.fetchMyOrganizations()
})

async function handleCreate() {
  if (!newOrgName.value.trim()) return
  const org = await store.createOrganization(
    newOrgName.value.trim(),
    newOrgDescription.value.trim() || undefined,
  )
  if (org) {
    newOrgName.value = ''
    newOrgDescription.value = ''
    showCreateForm.value = false
  }
}

function goToSettings(orgId: string) {
  router.push({ name: 'org-settings', params: { orgId } })
}

function goToProjects(orgId: string) {
  router.push({ name: 'org-projects', params: { orgId } })
}
</script>

<template>
  <div class="org-list-view">
    <div class="view-header">
      <h1>Organizations</h1>
      <BaseButton v-if="!showCreateForm" @click="showCreateForm = true">
        New Organization
      </BaseButton>
    </div>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <BaseCard v-if="showCreateForm" title="Create Organization">
      <form class="create-form" @submit.prevent="handleCreate">
        <BaseInput v-model="newOrgName" label="Name" placeholder="Organization name" />
        <BaseInput
          v-model="newOrgDescription"
          label="Description"
          placeholder="Optional description"
        />
        <div class="form-actions">
          <BaseButton :disabled="store.loading">Create</BaseButton>
          <BaseButton variant="secondary" @click="showCreateForm = false">Cancel</BaseButton>
        </div>
      </form>
    </BaseCard>

    <div v-if="store.organizations.length === 0 && !store.loading" class="empty-state">
      <p>You don't belong to any organizations yet.</p>
    </div>

    <div class="org-grid">
      <BaseCard
        v-for="org in store.organizations"
        :key="org.id"
        :title="org.name"
      >
        <p v-if="org.description" class="org-description">{{ org.description }}</p>
        <p class="org-role">Role: {{ org.role }}</p>
        <div class="org-actions">
          <BaseButton variant="secondary" @click="goToProjects(org.id)">Projects</BaseButton>
          <BaseButton variant="secondary" @click="goToSettings(org.id)">Settings</BaseButton>
        </div>
      </BaseCard>
    </div>
  </div>
</template>

<style scoped>
.org-list-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.view-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.create-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.form-actions {
  display: flex;
  gap: 0.75rem;
}

.empty-state {
  color: #9ca3af;
  text-align: center;
  padding: 2rem;
}

.org-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 1rem;
}

.org-description {
  color: #6b7280;
  font-size: 0.875rem;
  margin: 0 0 0.5rem;
}

.org-role {
  font-size: 0.75rem;
  color: #9ca3af;
  margin: 0 0 0.75rem;
}

.org-actions {
  display: flex;
  gap: 0.5rem;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
}
</style>
