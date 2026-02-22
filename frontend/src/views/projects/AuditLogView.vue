<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import client from '@/api/client'
import BaseCard from '@/components/base/BaseCard.vue'
import BaseButton from '@/components/base/BaseButton.vue'

const route = useRoute()
const projectId = route.params.projectId as string

interface AuditLogEntry {
  id: string
  actorType: string
  actorId: string | null
  action: string
  resourceType: string
  resourceId: string
  contextType: string | null
  contextId: string | null
  details: Record<string, unknown>
  createdAt: string
}

const logs = ref<AuditLogEntry[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const nextCursor = ref<string | null>(null)
const expandedId = ref<string | null>(null)
const filterAction = ref('')
const filterActorType = ref('')

async function fetchLogs(cursor?: string) {
  loading.value = true
  error.value = null
  const { data, error: err } = await client.GET('/api/v1/projects/{projectId}/audit-logs', {
    params: {
      path: { projectId },
      query: {
        ...(cursor ? { cursor } : {}),
        limit: 50,
        ...(filterActorType.value ? { actorType: filterActorType.value } : {}),
        ...(filterAction.value ? { action: filterAction.value } : {}),
      },
    },
  })
  loading.value = false
  if (err) {
    error.value = 'Failed to load audit logs'
    return
  }
  if (data) {
    if (cursor) {
      logs.value.push(...((data.data ?? []) as AuditLogEntry[]))
    } else {
      logs.value = (data.data ?? []) as AuditLogEntry[]
    }
    nextCursor.value = (data.nextCursor as string) ?? null
  }
}

function toggleExpand(id: string) {
  expandedId.value = expandedId.value === id ? null : id
}

function handleFilter() {
  nextCursor.value = null
  fetchLogs()
}

function loadMore() {
  if (nextCursor.value) {
    fetchLogs(nextCursor.value)
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString()
}

onMounted(() => fetchLogs())
</script>

<template>
  <div class="audit-log-view">
    <h1>Audit Logs</h1>

    <p v-if="error" class="error-message">{{ error }}</p>

    <div class="filters">
      <label>
        Actor Type:
        <select v-model="filterActorType">
          <option value="">All</option>
          <option value="account">Account</option>
          <option value="system">System</option>
          <option value="ai">AI</option>
          <option value="api_key">API Key</option>
        </select>
      </label>
      <label>
        Action:
        <input v-model="filterAction" type="text" placeholder="e.g. task.create" />
      </label>
      <BaseButton @click="handleFilter">Filter</BaseButton>
    </div>

    <p v-if="loading">Loading...</p>

    <div class="log-list">
      <BaseCard
        v-for="log in logs"
        :key="log.id"
        :title="`${log.action} on ${log.resourceType}`"
        @click="toggleExpand(log.id)"
      >
        <p><strong>Actor:</strong> {{ log.actorType }}{{ log.actorId ? ` (${log.actorId})` : '' }}</p>
        <p><strong>Resource:</strong> {{ log.resourceType }} / {{ log.resourceId }}</p>
        <p><strong>Time:</strong> {{ formatDate(log.createdAt) }}</p>
        <div v-if="expandedId === log.id" class="details">
          <pre>{{ JSON.stringify(log.details, null, 2) }}</pre>
        </div>
      </BaseCard>
    </div>

    <BaseButton v-if="nextCursor" @click="loadMore">Load More</BaseButton>
  </div>
</template>

<style scoped>
.audit-log-view {
  padding: 1rem;
}
.filters {
  display: flex;
  gap: 1rem;
  margin-bottom: 1rem;
  align-items: end;
}
.filters label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.log-list {
  margin-top: 1rem;
}
.details {
  margin-top: 0.5rem;
  background: #f5f5f5;
  padding: 0.5rem;
  border-radius: 4px;
  overflow-x: auto;
}
.details pre {
  margin: 0;
  font-size: 0.85rem;
}
.error-message {
  color: red;
}
</style>
