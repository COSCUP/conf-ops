<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import client from '@/api/client'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import BaseInput from '@/components/base/BaseInput.vue'

const route = useRoute()
const projectId = route.params.projectId as string

interface ApiKeyEntry {
  id: string
  name: string
  permissions: Record<string, unknown>
  lastUsedAt: string | null
  createdAt: string
}

const apiKeys = ref<ApiKeyEntry[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const showCreate = ref(false)
const newKeyName = ref('')
const newKeyScopes = ref<string[]>([])
const createdKeyValue = ref<string | null>(null)

const availableScopes = [
  'read:task_templates',
  'read:tasks',
  'read:data',
  'write:data',
]

async function fetchApiKeys() {
  loading.value = true
  error.value = null
  const { data, error: err } = await client.GET('/api/v1/projects/{projectId}/api-keys', {
    params: { path: { projectId } },
  })
  loading.value = false
  if (err) {
    error.value = 'Failed to load API keys'
    return
  }
  if (data) {
    apiKeys.value = (data.data ?? []) as ApiKeyEntry[]
  }
}

async function handleCreate() {
  if (!newKeyName.value) return
  createdKeyValue.value = null
  const { data, error: err } = await client.POST('/api/v1/projects/{projectId}/api-keys', {
    params: { path: { projectId } },
    body: {
      name: newKeyName.value,
      permissions: { scopes: newKeyScopes.value },
    },
  })
  if (err) {
    error.value = 'Failed to create API key'
    return
  }
  if (data) {
    createdKeyValue.value = (data as Record<string, unknown>).apiKey as string
  }
  newKeyName.value = ''
  newKeyScopes.value = []
  await fetchApiKeys()
}

async function handleRevoke(keyId: string) {
  await client.DELETE('/api/v1/projects/{projectId}/api-keys/{keyId}', {
    params: { path: { projectId, keyId } },
  })
  await fetchApiKeys()
}

function toggleScope(scope: string) {
  const idx = newKeyScopes.value.indexOf(scope)
  if (idx >= 0) {
    newKeyScopes.value.splice(idx, 1)
  } else {
    newKeyScopes.value.push(scope)
  }
}

function formatDate(iso: string | null): string {
  if (!iso) return 'Never'
  return new Date(iso).toLocaleString()
}

onMounted(fetchApiKeys)
</script>

<template>
  <div class="api-key-management-view">
    <h1>API Key Management</h1>

    <p v-if="error" class="error-message">{{ error }}</p>
    <p v-if="loading">Loading...</p>

    <BaseButton @click="showCreate = !showCreate">
      {{ showCreate ? 'Cancel' : 'Create API Key' }}
    </BaseButton>

    <BaseCard v-if="showCreate" title="New API Key" class="create-form">
      <BaseInput v-model="newKeyName" label="Name" placeholder="API key name" />
      <div class="scopes">
        <label>Scopes:</label>
        <div v-for="scope in availableScopes" :key="scope" class="scope-checkbox">
          <input
            type="checkbox"
            :checked="newKeyScopes.includes(scope)"
            @change="toggleScope(scope)"
          />
          <span>{{ scope }}</span>
        </div>
      </div>
      <BaseButton @click="handleCreate">Create</BaseButton>
    </BaseCard>

    <BaseCard v-if="createdKeyValue" title="API Key Created" class="key-display">
      <p class="key-warning">Copy this key now. It will not be shown again.</p>
      <code class="key-value">{{ createdKeyValue }}</code>
      <BaseButton @click="createdKeyValue = null">Dismiss</BaseButton>
    </BaseCard>

    <div class="key-list">
      <BaseCard
        v-for="key in apiKeys"
        :key="key.id"
        :title="key.name"
      >
        <p><strong>Created:</strong> {{ formatDate(key.createdAt) }}</p>
        <p><strong>Last used:</strong> {{ formatDate(key.lastUsedAt) }}</p>
        <p><strong>Scopes:</strong> {{ ((key.permissions as Record<string, unknown>)?.scopes as string[] ?? []).join(', ') }}</p>
        <BaseButton @click="handleRevoke(key.id)">Revoke</BaseButton>
      </BaseCard>
    </div>
  </div>
</template>

<style scoped>
.api-key-management-view {
  padding: 1rem;
}
.create-form {
  margin: 1rem 0;
}
.scopes {
  margin: 0.5rem 0;
}
.scope-checkbox {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  margin: 0.25rem 0;
}
.key-display {
  margin: 1rem 0;
  background: #fff3cd;
}
.key-warning {
  color: #856404;
  font-weight: bold;
}
.key-value {
  display: block;
  padding: 0.5rem;
  background: #f5f5f5;
  border-radius: 4px;
  word-break: break-all;
  margin: 0.5rem 0;
}
.key-list {
  margin-top: 1rem;
}
.error-message {
  color: red;
}
</style>
