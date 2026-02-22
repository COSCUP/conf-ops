<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useToolConfigStore } from '@/stores/toolConfig'
import type { CreateToolConfigPayload, UpdateToolConfigPayload } from '@/stores/toolConfig'

const props = defineProps<{
  projectId: string
}>()

const store = useToolConfigStore()

const showCreateForm = ref(false)
const editingConfigId = ref<string | null>(null)

// Create form state
const newConfig = ref<CreateToolConfigPayload>({
  toolType: 'builtin',
  toolName: '',
  displayName: '',
  description: '',
  enabled: false,
  config: {},
})

// Edit form state
const editConfig = ref<UpdateToolConfigPayload>({})

onMounted(async () => {
  await Promise.all([store.listTools(props.projectId), store.listToolConfigs(props.projectId)])
})

function resetCreateForm() {
  newConfig.value = {
    toolType: 'builtin',
    toolName: '',
    displayName: '',
    description: '',
    enabled: false,
    config: {},
  }
  showCreateForm.value = false
}

async function handleCreate() {
  const result = await store.createToolConfig(props.projectId, newConfig.value)
  if (result) {
    resetCreateForm()
  }
}

function startEdit(configId: string) {
  const config = store.toolConfigs.find((c) => c.id === configId)
  if (!config) return

  editingConfigId.value = configId
  const update: UpdateToolConfigPayload = {
    enabled: config.enabled,
    config: config.config,
  }
  if (config.displayName !== undefined) {
    update.displayName = config.displayName
  }
  if (config.description !== undefined) {
    update.description = config.description
  }
  editConfig.value = update
}

async function handleUpdate() {
  if (!editingConfigId.value) return

  const result = await store.updateToolConfig(
    props.projectId,
    editingConfigId.value,
    editConfig.value,
  )
  if (result) {
    editingConfigId.value = null
    editConfig.value = {}
  }
}

async function handleDelete(configId: string) {
  await store.deleteToolConfig(props.projectId, configId)
}

async function handleToggle(configId: string, enabled: boolean) {
  await store.updateToolConfig(props.projectId, configId, { enabled })
}

function formatConfig(config: Record<string, unknown>): string {
  try {
    return JSON.stringify(config, null, 2)
  } catch {
    return '{}'
  }
}
</script>

<template>
  <div class="tool-config-view">
    <div class="header">
      <h2>Tool Configuration</h2>
      <button class="btn btn-primary" @click="showCreateForm = !showCreateForm">
        {{ showCreateForm ? 'Cancel' : 'Add Tool Config' }}
      </button>
    </div>

    <div v-if="store.error" class="error-banner">{{ store.error }}</div>

    <!-- Create form -->
    <div v-if="showCreateForm" class="create-form">
      <h3>New Tool Configuration</h3>
      <div class="form-grid">
        <div class="form-field">
          <label for="new-tool-name">Tool Name</label>
          <input
            id="new-tool-name"
            v-model="newConfig.toolName"
            type="text"
            placeholder="e.g., smtp/sendEmail"
          />
        </div>
        <div class="form-field">
          <label for="new-tool-type">Tool Type</label>
          <select id="new-tool-type" v-model="newConfig.toolType">
            <option value="builtin">Builtin</option>
            <option value="external">External</option>
          </select>
        </div>
        <div class="form-field">
          <label for="new-display-name">Display Name</label>
          <input id="new-display-name" v-model="newConfig.displayName" type="text" />
        </div>
        <div class="form-field">
          <label for="new-description">Description</label>
          <input id="new-description" v-model="newConfig.description" type="text" />
        </div>
        <div class="form-field">
          <label>
            <input v-model="newConfig.enabled" type="checkbox" /> Enabled
          </label>
        </div>
      </div>
      <button class="btn btn-primary" :disabled="!newConfig.toolName" @click="handleCreate">
        Create
      </button>
    </div>

    <!-- Available tools -->
    <div class="section">
      <h3>Available Tools ({{ store.tools.length }})</h3>
      <div v-if="store.loading" class="loading">Loading...</div>
      <table v-else-if="store.tools.length > 0" class="tool-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Category</th>
            <th>Description</th>
            <th>Confirmation</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="tool in store.tools" :key="tool.name">
            <td>{{ tool.displayName ?? tool.name }}</td>
            <td>
              <span :class="['category-badge', `category-${tool.category}`]">
                {{ tool.category }}
              </span>
            </td>
            <td>{{ tool.description }}</td>
            <td>{{ tool.requiresConfirmation ? 'Yes' : 'No' }}</td>
          </tr>
        </tbody>
      </table>
      <p v-else class="empty">No tools available.</p>
    </div>

    <!-- Tool configurations -->
    <div class="section">
      <h3>Project Tool Configs ({{ store.toolConfigs.length }})</h3>
      <div v-if="store.toolConfigs.length > 0" class="config-list">
        <div v-for="config in store.toolConfigs" :key="config.id" class="config-card">
          <div v-if="editingConfigId === config.id" class="edit-form">
            <div class="form-field">
              <label>Display Name</label>
              <input v-model="editConfig.displayName" type="text" />
            </div>
            <div class="form-field">
              <label>
                <input v-model="editConfig.enabled" type="checkbox" /> Enabled
              </label>
            </div>
            <div class="form-actions">
              <button class="btn btn-primary" @click="handleUpdate">Save</button>
              <button class="btn btn-secondary" @click="editingConfigId = null">Cancel</button>
            </div>
          </div>
          <div v-else class="config-info">
            <div class="config-header">
              <strong>{{ config.displayName ?? config.toolName }}</strong>
              <span :class="['status-badge', config.enabled ? 'enabled' : 'disabled']">
                {{ config.enabled ? 'Enabled' : 'Disabled' }}
              </span>
            </div>
            <p class="config-meta">{{ config.toolType }} | {{ config.toolName }}</p>
            <pre v-if="Object.keys(config.config).length > 0" class="config-json">{{
              formatConfig(config.config)
            }}</pre>
            <div class="config-actions">
              <button class="btn btn-sm" @click="handleToggle(config.id, !config.enabled)">
                {{ config.enabled ? 'Disable' : 'Enable' }}
              </button>
              <button class="btn btn-sm" @click="startEdit(config.id)">Edit</button>
              <button class="btn btn-sm btn-danger" @click="handleDelete(config.id)">Delete</button>
            </div>
          </div>
        </div>
      </div>
      <p v-else class="empty">No tool configurations yet.</p>
    </div>
  </div>
</template>

<style scoped>
.tool-config-view {
  padding: 1rem;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
}

.header h2 {
  margin: 0;
}

.error-banner {
  background: #fef2f2;
  color: #dc2626;
  padding: 0.75rem;
  border-radius: 0.375rem;
  margin-bottom: 1rem;
}

.create-form {
  background: #f9fafb;
  padding: 1rem;
  border-radius: 0.375rem;
  margin-bottom: 1.5rem;
}

.create-form h3 {
  margin: 0 0 1rem;
}

.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.form-field label {
  font-weight: 600;
  font-size: 0.875rem;
}

.form-field input[type='text'],
.form-field select {
  padding: 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
}

.section {
  margin-bottom: 2rem;
}

.section h3 {
  margin: 0 0 0.75rem;
}

.loading {
  color: #6b7280;
}

.empty {
  color: #6b7280;
  font-style: italic;
}

.tool-table {
  width: 100%;
  border-collapse: collapse;
}

.tool-table th,
.tool-table td {
  padding: 0.5rem 0.75rem;
  text-align: left;
  border-bottom: 1px solid #e5e7eb;
}

.tool-table th {
  font-weight: 600;
  font-size: 0.875rem;
  color: #6b7280;
}

.category-badge {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  border-radius: 0.25rem;
  font-size: 0.75rem;
  font-weight: 500;
}

.category-core {
  background: #dbeafe;
  color: #1e40af;
}

.category-configurable {
  background: #fef3c7;
  color: #92400e;
}

.category-external {
  background: #e0e7ff;
  color: #3730a3;
}

.config-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.config-card {
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  padding: 1rem;
}

.config-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.25rem;
}

.status-badge {
  padding: 0.125rem 0.5rem;
  border-radius: 0.25rem;
  font-size: 0.75rem;
}

.status-badge.enabled {
  background: #dcfce7;
  color: #166534;
}

.status-badge.disabled {
  background: #f3f4f6;
  color: #6b7280;
}

.config-meta {
  margin: 0.25rem 0;
  font-size: 0.875rem;
  color: #6b7280;
}

.config-json {
  background: #f9fafb;
  padding: 0.5rem;
  border-radius: 0.25rem;
  font-size: 0.75rem;
  overflow-x: auto;
  max-height: 100px;
}

.config-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.edit-form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.form-actions {
  display: flex;
  gap: 0.5rem;
}

.btn {
  padding: 0.5rem 1rem;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  cursor: pointer;
  border: 1px solid transparent;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: #2563eb;
  color: white;
}

.btn-secondary {
  background: white;
  border-color: #d1d5db;
  color: #374151;
}

.btn-sm {
  padding: 0.25rem 0.5rem;
  font-size: 0.75rem;
  background: white;
  border-color: #d1d5db;
  color: #374151;
}

.btn-danger {
  color: #dc2626;
  border-color: #fecaca;
}
</style>
