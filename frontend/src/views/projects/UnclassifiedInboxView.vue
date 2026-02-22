<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useUnassignedInboxStore } from '@/stores/unassignedInbox'
import { useTaskStore } from '@/stores/task'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const route = useRoute()
const inboxStore = useUnassignedInboxStore()
const taskStore = useTaskStore()

const projectId = route.params.projectId as string

const assigningEmailId = ref<string | null>(null)
const selectedTaskId = ref('')

onMounted(async () => {
  await Promise.all([
    inboxStore.fetchEmails(projectId),
    taskStore.fetchTasks(projectId),
  ])
})

function startAssign(emailId: string) {
  assigningEmailId.value = emailId
  selectedTaskId.value = ''
}

function cancelAssign() {
  assigningEmailId.value = null
  selectedTaskId.value = ''
}

async function handleAssign(emailId: string) {
  if (!selectedTaskId.value) return
  await inboxStore.assignEmail(projectId, emailId, selectedTaskId.value)
  assigningEmailId.value = null
  selectedTaskId.value = ''
}

async function loadMore() {
  if (inboxStore.pagination.nextCursor) {
    await inboxStore.fetchEmails(projectId, inboxStore.pagination.nextCursor)
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString()
}
</script>

<template>
  <div class="inbox-view">
    <h1>Unclassified Inbox</h1>
    <p class="subtitle">Emails that could not be automatically assigned to a task.</p>

    <p v-if="inboxStore.error" class="error-message">{{ inboxStore.error }}</p>

    <div v-if="inboxStore.emails.length > 0" class="email-list">
      <BaseCard v-for="email in inboxStore.emails" :key="email.id" :title="email.subject">
        <div class="email-info">
          <div class="email-from">
            <strong>From:</strong>
            {{ email.fromName ? `${email.fromName} <${email.fromAddress}>` : email.fromAddress }}
          </div>
          <div class="email-date">{{ formatDate(email.receivedAt) }}</div>
          <div v-if="email.snippet" class="email-snippet">{{ email.snippet }}</div>
          <div v-if="email.hasAttachments" class="has-attachments">Has attachments</div>
        </div>

        <!-- Assign UI -->
        <div v-if="assigningEmailId === email.id" class="assign-form">
          <select v-model="selectedTaskId" class="task-select">
            <option value="" disabled>Select a task...</option>
            <option v-for="task in taskStore.tasks" :key="task.id" :value="task.id">
              {{ task.name }}
            </option>
          </select>
          <div class="assign-actions">
            <BaseButton :disabled="!selectedTaskId || inboxStore.loading" @click="handleAssign(email.id)">
              Assign
            </BaseButton>
            <BaseButton variant="secondary" @click="cancelAssign">Cancel</BaseButton>
          </div>
        </div>
        <BaseButton v-else variant="secondary" @click="startAssign(email.id)">
          Assign to Task
        </BaseButton>
      </BaseCard>
    </div>
    <p v-else-if="!inboxStore.loading" class="empty-text">No unclassified emails.</p>

    <BaseButton
      v-if="inboxStore.pagination.hasMore"
      variant="secondary"
      :disabled="inboxStore.loading"
      @click="loadMore"
    >
      Load More
    </BaseButton>
  </div>
</template>

<style scoped>
.inbox-view {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 800px;
}

.subtitle {
  color: #6b7280;
  font-size: 0.875rem;
  margin-top: -0.5rem;
}

.email-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.email-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin-bottom: 0.75rem;
}

.email-from {
  font-size: 0.875rem;
  color: #374151;
}

.email-date {
  font-size: 0.75rem;
  color: #9ca3af;
}

.email-snippet {
  font-size: 0.875rem;
  color: #6b7280;
  font-style: italic;
}

.has-attachments {
  font-size: 0.75rem;
  color: #3b82f6;
}

.assign-form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.task-select {
  padding: 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
}

.assign-actions {
  display: flex;
  gap: 0.5rem;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
}

.empty-text {
  color: #9ca3af;
  font-size: 0.875rem;
  text-align: center;
  padding: 2rem 0;
}
</style>
