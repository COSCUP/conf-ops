<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useMemberStore } from '@/stores/member'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const route = useRoute()
const store = useMemberStore()

const projectId = route.params.projectId as string
const inviteAccountId = ref('')
const inviteRole = ref<'owner' | 'tag_admin' | 'member'>('member')

onMounted(async () => {
  await store.fetchMembers(projectId)
})

async function handleInvite() {
  if (!inviteAccountId.value.trim()) return
  const result = await store.inviteMember(
    projectId,
    inviteAccountId.value.trim(),
    inviteRole.value,
  )
  if (result) {
    inviteAccountId.value = ''
  }
}

async function handleUpdateRole(memberId: string, newRole: string) {
  await store.updateRole(projectId, memberId, newRole as 'owner' | 'tag_admin' | 'member')
}

async function handleRemove(memberId: string) {
  await store.removeMember(projectId, memberId)
}
</script>

<template>
  <div class="project-members-view">
    <h1>Project Members</h1>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <BaseCard title="Invite Member">
      <form class="invite-form" @submit.prevent="handleInvite">
        <BaseInput
          v-model="inviteAccountId"
          label="Account ID"
          placeholder="Enter account ID"
        />
        <select v-model="inviteRole" class="role-select">
          <option value="member">Member</option>
          <option value="tag_admin">Tag Admin</option>
          <option value="owner">Owner</option>
        </select>
        <BaseButton :disabled="store.loading">Invite</BaseButton>
      </form>
    </BaseCard>

    <BaseCard title="Members">
      <ul class="member-list">
        <li v-for="member in store.members" :key="member.id" class="member-item">
          <div class="member-info">
            <strong>{{ member.name }}</strong>
            <span class="member-email">{{ member.email }}</span>
          </div>
          <div class="member-actions">
            <span class="role-badge" :class="`role-${member.role}`">{{ member.role }}</span>
            <select
              :value="member.role"
              class="role-select"
              @change="handleUpdateRole(member.id, ($event.target as HTMLSelectElement).value)"
            >
              <option value="member">Member</option>
              <option value="tag_admin">Tag Admin</option>
              <option value="owner">Owner</option>
            </select>
            <BaseButton
              variant="danger"
              :disabled="store.loading"
              @click="handleRemove(member.id)"
            >
              Remove
            </BaseButton>
          </div>
        </li>
      </ul>
      <p v-if="store.members.length === 0 && !store.loading" class="empty-text">
        No members yet.
      </p>
    </BaseCard>
  </div>
</template>

<style scoped>
.project-members-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 700px;
}

.invite-form {
  display: flex;
  gap: 0.75rem;
  align-items: flex-end;
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

.role-badge {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
  font-size: 0.75rem;
  font-weight: 500;
}

.role-owner { background: #fef3c7; color: #92400e; }
.role-tag_admin { background: #dbeafe; color: #1e40af; }
.role-member { background: #f3f4f6; color: #6b7280; }

.empty-text {
  color: #9ca3af;
  font-size: 0.875rem;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
}
</style>
