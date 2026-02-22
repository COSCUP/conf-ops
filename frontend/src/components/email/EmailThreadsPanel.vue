<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useEmailThreadStore } from '@/stores/emailThread'
import EmailMessageItem from './EmailMessageItem.vue'
import EmailComposer from './EmailComposer.vue'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const props = defineProps<{
  projectId: string
  taskId: string
}>()

const store = useEmailThreadStore()
const expandedThreadId = ref<string | null>(null)
const showNewThread = ref(false)
const newSubject = ref('')
const newParticipants = ref('')

onMounted(async () => {
  await store.fetchThreads(props.projectId, props.taskId)
})

async function handleCreateThread() {
  const participants = newParticipants.value
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0)
  if (!newSubject.value.trim() || participants.length === 0) return

  await store.createThread(props.projectId, props.taskId, newSubject.value, participants)
  newSubject.value = ''
  newParticipants.value = ''
  showNewThread.value = false
}

async function toggleThread(threadId: string) {
  if (expandedThreadId.value === threadId) {
    expandedThreadId.value = null
    return
  }
  expandedThreadId.value = threadId
  await store.fetchMessages(props.projectId, props.taskId, threadId)
}

async function handleDeleteThread(threadId: string) {
  await store.deleteThread(props.projectId, props.taskId, threadId)
  if (expandedThreadId.value === threadId) {
    expandedThreadId.value = null
  }
}

async function handleSendEmail(
  threadId: string,
  payload: {
    toAddresses: string[]
    ccAddresses: string[]
    subject: string
    htmlBody: string
  },
) {
  await store.sendEmail(props.projectId, props.taskId, threadId, payload)
}

function formatDate(iso: string | null | undefined): string {
  if (!iso) return 'N/A'
  return new Date(iso).toLocaleString()
}
</script>

<template>
  <div class="email-threads-panel">
    <div class="panel-header">
      <h3>Email Threads</h3>
      <BaseButton variant="secondary" @click="showNewThread = !showNewThread">
        {{ showNewThread ? 'Cancel' : 'New Thread' }}
      </BaseButton>
    </div>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <!-- New Thread Form -->
    <BaseCard v-if="showNewThread" title="Create Thread">
      <form class="form-vertical" @submit.prevent="handleCreateThread">
        <BaseInput v-model="newSubject" label="Subject" placeholder="Thread subject" />
        <BaseInput
          v-model="newParticipants"
          label="Participants"
          placeholder="email@example.com, ..."
        />
        <BaseButton :disabled="store.loading">Create</BaseButton>
      </form>
    </BaseCard>

    <!-- Thread List -->
    <div v-if="store.threads.length > 0" class="thread-list">
      <div v-for="thread in store.threads" :key="thread.id" class="thread-item">
        <div class="thread-header" @click="toggleThread(thread.id)">
          <span class="thread-subject">{{ thread.subject }}</span>
          <span class="thread-meta">{{ formatDate(thread.lastMessageAt) }}</span>
          <BaseButton
            variant="danger"
            @click.stop="handleDeleteThread(thread.id)"
          >
            Delete
          </BaseButton>
        </div>

        <!-- Expanded Thread Messages -->
        <div v-if="expandedThreadId === thread.id" class="thread-messages">
          <div v-if="store.currentMessages.length > 0" class="messages-list">
            <EmailMessageItem
              v-for="msg in store.currentMessages"
              :key="msg.id"
              :message="msg"
            />
          </div>
          <p v-else class="empty-text">No messages in this thread yet.</p>

          <EmailComposer
            :default-subject="`Re: ${thread.subject}`"
            :disabled="store.loading"
            @send="(payload) => handleSendEmail(thread.id, payload)"
          />
        </div>
      </div>
    </div>
    <p v-else-if="!store.loading" class="empty-text">No email threads yet.</p>
  </div>
</template>

<style scoped>
.email-threads-panel {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.panel-header h3 {
  margin: 0;
  font-size: 1rem;
}

.form-vertical {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  max-width: 400px;
}

.thread-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.thread-item {
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  overflow: hidden;
}

.thread-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem;
  cursor: pointer;
  background: #f9fafb;
}

.thread-header:hover {
  background: #f3f4f6;
}

.thread-subject {
  font-weight: 600;
  font-size: 0.875rem;
  color: #1f2937;
  flex: 1;
}

.thread-meta {
  font-size: 0.75rem;
  color: #9ca3af;
}

.thread-messages {
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  border-top: 1px solid #e5e7eb;
}

.messages-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-height: 300px;
  overflow-y: auto;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
  font-size: 0.875rem;
}

.empty-text {
  color: #9ca3af;
  font-size: 0.875rem;
  text-align: center;
  padding: 1rem 0;
}
</style>
