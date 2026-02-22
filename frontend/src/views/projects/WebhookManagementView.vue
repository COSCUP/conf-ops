<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import client from '@/api/client'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import BaseInput from '@/components/base/BaseInput.vue'

const route = useRoute()
const projectId = route.params.projectId as string

interface Webhook {
  id: string
  name: string
  url: string
  hasSecret: boolean
  eventTypes: string[]
  enabled: boolean
  createdAt: string
}

const webhooks = ref<Webhook[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const showCreate = ref(false)
const newName = ref('')
const newUrl = ref('')
const newSecret = ref('')
const newEventTypes = ref<string[]>([])

const availableEventTypes = [
  'task.created', 'task.updated', 'task.completed', 'task.deleted',
  'todo.created', 'todo.completed', 'todo.updated',
  'data_entry.created', 'data_entry.updated', 'data_entry.deleted',
  'message.created', 'member.added', 'member.removed',
]

async function fetchWebhooks() {
  loading.value = true
  error.value = null
  const { data, error: err } = await client.GET('/api/v1/projects/{projectId}/webhooks', {
    params: { path: { projectId } },
  })
  loading.value = false
  if (err) {
    error.value = 'Failed to load webhooks'
    return
  }
  if (data) {
    webhooks.value = (data.data ?? []) as Webhook[]
  }
}

async function handleCreate() {
  if (!newName.value || !newUrl.value || newEventTypes.value.length === 0) return
  const { error: err } = await client.POST('/api/v1/projects/{projectId}/webhooks', {
    params: { path: { projectId } },
    body: {
      name: newName.value,
      url: newUrl.value,
      secret: newSecret.value || null,
      eventTypes: newEventTypes.value,
    },
  })
  if (err) {
    error.value = 'Failed to create webhook'
    return
  }
  showCreate.value = false
  newName.value = ''
  newUrl.value = ''
  newSecret.value = ''
  newEventTypes.value = []
  await fetchWebhooks()
}

async function handleDelete(webhookId: string) {
  await client.DELETE('/api/v1/projects/{projectId}/webhooks/{webhookId}', {
    params: { path: { projectId, webhookId } },
  })
  await fetchWebhooks()
}

async function handleToggle(webhook: Webhook) {
  await client.PUT('/api/v1/projects/{projectId}/webhooks/{webhookId}', {
    params: { path: { projectId, webhookId: webhook.id } },
    body: { enabled: !webhook.enabled },
  })
  await fetchWebhooks()
}

async function handleTest(webhookId: string) {
  const { error: err } = await client.POST('/api/v1/projects/{projectId}/webhooks/{webhookId}/test', {
    params: { path: { projectId, webhookId } },
  })
  if (err) {
    error.value = 'Test event failed'
  }
}

function toggleEventType(eventType: string) {
  const idx = newEventTypes.value.indexOf(eventType)
  if (idx >= 0) {
    newEventTypes.value.splice(idx, 1)
  } else {
    newEventTypes.value.push(eventType)
  }
}

onMounted(fetchWebhooks)
</script>

<template>
  <div class="webhook-management-view">
    <h1>Webhook Management</h1>

    <p v-if="error" class="error-message">{{ error }}</p>
    <p v-if="loading">Loading...</p>

    <BaseButton @click="showCreate = !showCreate">
      {{ showCreate ? 'Cancel' : 'Add Webhook' }}
    </BaseButton>

    <BaseCard v-if="showCreate" title="New Webhook" class="create-form">
      <BaseInput v-model="newName" label="Name" placeholder="Webhook name" />
      <BaseInput v-model="newUrl" label="URL" placeholder="https://example.com/webhook" />
      <BaseInput v-model="newSecret" label="Secret (optional)" placeholder="HMAC secret" />
      <div class="event-types">
        <label>Event Types:</label>
        <div v-for="et in availableEventTypes" :key="et" class="event-type-checkbox">
          <input
            type="checkbox"
            :checked="newEventTypes.includes(et)"
            @change="toggleEventType(et)"
          />
          <span>{{ et }}</span>
        </div>
      </div>
      <BaseButton @click="handleCreate">Create</BaseButton>
    </BaseCard>

    <div class="webhook-list">
      <BaseCard
        v-for="webhook in webhooks"
        :key="webhook.id"
        :title="webhook.name"
      >
        <p><strong>URL:</strong> {{ webhook.url }}</p>
        <p><strong>Events:</strong> {{ Array.isArray(webhook.eventTypes) ? webhook.eventTypes.join(', ') : '' }}</p>
        <p><strong>Enabled:</strong> {{ webhook.enabled ? 'Yes' : 'No' }}</p>
        <p><strong>Secret:</strong> {{ webhook.hasSecret ? 'Set' : 'Not set' }}</p>
        <div class="webhook-actions">
          <BaseButton @click="handleToggle(webhook)">
            {{ webhook.enabled ? 'Disable' : 'Enable' }}
          </BaseButton>
          <BaseButton @click="handleTest(webhook.id)">Test</BaseButton>
          <BaseButton @click="handleDelete(webhook.id)">Delete</BaseButton>
        </div>
      </BaseCard>
    </div>
  </div>
</template>

<style scoped>
.webhook-management-view {
  padding: 1rem;
}
.create-form {
  margin: 1rem 0;
}
.event-types {
  margin: 0.5rem 0;
}
.event-type-checkbox {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  margin: 0.25rem 0;
}
.webhook-list {
  margin-top: 1rem;
}
.webhook-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}
.error-message {
  color: red;
}
</style>
