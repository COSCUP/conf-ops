<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useOrganizationStore } from '@/stores/organization'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const route = useRoute()
const router = useRouter()
const store = useOrganizationStore()

const orgId = route.params.orgId as string
const editName = ref('')
const editDescription = ref('')
const inviteEmail = ref('')
const inviteRole = ref('org_member')

onMounted(async () => {
  await Promise.all([store.fetchOrganization(orgId), store.fetchMembers(orgId)])
  if (store.currentOrganization) {
    editName.value = store.currentOrganization.name
    editDescription.value = store.currentOrganization.description ?? ''
  }
})

async function handleUpdateOrg() {
  await store.updateOrganization(orgId, {
    name: editName.value,
    description: editDescription.value || null,
  })
}

async function handleDeleteOrg() {
  await store.deleteOrganization(orgId)
  if (!store.error) {
    await router.push({ name: 'organizations' })
  }
}

async function handleInvite() {
  if (!inviteEmail.value.trim()) return
  const result = await store.inviteMember(orgId, inviteEmail.value.trim(), inviteRole.value)
  if (result) {
    inviteEmail.value = ''
  }
}

async function handleRemoveMember(memberId: string) {
  await store.removeMember(orgId, memberId)
}

async function handleUpdateRole(memberId: string, newRole: string) {
  await store.updateMemberRole(orgId, memberId, newRole)
}
</script>

<template>
  <div class="org-settings-view">
    <h1>Organization Settings</h1>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <BaseCard title="General">
      <form class="settings-form" @submit.prevent="handleUpdateOrg">
        <BaseInput v-model="editName" label="Name" />
        <BaseInput v-model="editDescription" label="Description" />
        <BaseButton :disabled="store.loading">Save</BaseButton>
      </form>
    </BaseCard>

    <BaseCard title="Members">
      <form class="invite-form" @submit.prevent="handleInvite">
        <BaseInput v-model="inviteEmail" label="Email" type="email" placeholder="user@example.com" />
        <select v-model="inviteRole" class="role-select">
          <option value="org_member">Member</option>
          <option value="org_admin">Admin</option>
          <option value="org_owner">Owner</option>
        </select>
        <BaseButton :disabled="store.loading">Invite</BaseButton>
      </form>

      <ul class="member-list">
        <li v-for="member in store.members" :key="member.id" class="member-item">
          <div class="member-info">
            <strong>{{ member.name }}</strong>
            <span class="member-email">{{ member.email }}</span>
          </div>
          <div class="member-actions">
            <select
              :value="member.role"
              class="role-select"
              @change="handleUpdateRole(member.id, ($event.target as HTMLSelectElement).value)"
            >
              <option value="org_member">Member</option>
              <option value="org_admin">Admin</option>
              <option value="org_owner">Owner</option>
            </select>
            <BaseButton
              variant="danger"
              :disabled="store.loading"
              @click="handleRemoveMember(member.id)"
            >
              Remove
            </BaseButton>
          </div>
        </li>
      </ul>
    </BaseCard>

    <BaseCard title="Danger Zone">
      <p class="danger-text">Deleting this organization is permanent and cannot be undone.</p>
      <BaseButton variant="danger" :disabled="store.loading" @click="handleDeleteOrg">
        Delete Organization
      </BaseButton>
    </BaseCard>
  </div>
</template>

<style scoped>
.org-settings-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 700px;
}

.settings-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.invite-form {
  display: flex;
  gap: 0.75rem;
  align-items: flex-end;
  margin-bottom: 1rem;
}

.role-select {
  padding: 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
}

.member-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.member-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.member-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.member-email {
  font-size: 0.75rem;
  color: #9ca3af;
}

.member-actions {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.danger-text {
  color: #ef4444;
  margin: 0 0 1rem;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
}
</style>
