<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import {
  useMemoryStore,
  SCOPE_LABELS,
  SCOPE_ORDER,
  type TaskMemoryContext,
} from '@/stores/memory'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import BaseInput from '@/components/base/BaseInput.vue'

const props = defineProps<{
  scopeType?: string
  scopeId?: string
  taskContext?: TaskMemoryContext
}>()

const store = useMemoryStore()
const showAddForm = ref(false)
const newContent = ref('')
const newSource = ref('')
const addScopeType = ref('')
const editingMemoryId = ref<string | null>(null)
const editContent = ref('')
const collapsedScopes = ref<Set<string>>(new Set())

const isGroupedMode = computed(() => !!props.taskContext)

const orderedScopes = computed(() => {
  if (!isGroupedMode.value) return []
  return SCOPE_ORDER.filter((scope) => {
    const items = store.groupedMemories[scope]
    return items && items.length > 0
  })
})

const emptyScopes = computed(() => {
  if (!isGroupedMode.value) return []
  return SCOPE_ORDER.filter((scope) => {
    const items = store.groupedMemories[scope]
    return !items || items.length === 0
  })
})

onMounted(async () => {
  if (props.taskContext) {
    await store.loadGroupedMemories(props.taskContext)
  } else if (props.scopeType && props.scopeId) {
    await store.listMemories(props.scopeType, props.scopeId)
  }
})

watch(
  () => props.taskContext,
  async (ctx) => {
    if (ctx) {
      await store.loadGroupedMemories(ctx)
    }
  },
  { deep: true },
)

function toggleScope(scope: string) {
  if (collapsedScopes.value.has(scope)) {
    collapsedScopes.value.delete(scope)
  } else {
    collapsedScopes.value.add(scope)
  }
}

function getScopeLabel(scope: string): string {
  return SCOPE_LABELS[scope] ?? scope
}

async function handleCreate() {
  if (!newContent.value.trim() || !newSource.value.trim()) return

  const scopeType = isGroupedMode.value ? addScopeType.value : props.scopeType
  const scopeId = isGroupedMode.value ? getScopeIdForType(addScopeType.value) : props.scopeId
  if (!scopeType || !scopeId) return

  const result = await store.createMemory({
    content: newContent.value.trim(),
    source: newSource.value.trim(),
    scopeType,
    scopeId,
  })
  if (result) {
    newContent.value = ''
    newSource.value = ''
    addScopeType.value = ''
    showAddForm.value = false
    if (props.taskContext) {
      await store.loadGroupedMemories(props.taskContext)
    }
  }
}

function getScopeIdForType(scopeType: string): string | undefined {
  const ctx = props.taskContext
  if (!ctx) return undefined
  switch (scopeType) {
    case 'account':
      return ctx.accountId
    case 'organization':
      return ctx.organizationId
    case 'project':
      return ctx.projectId
    case 'member_tag':
      return ctx.memberTagId
    case 'task_template':
      return ctx.taskTemplateId
    case 'task':
      return ctx.taskId
    default:
      return undefined
  }
}

function availableAddScopes(): { value: string; label: string }[] {
  if (!props.taskContext) return []
  return SCOPE_ORDER.filter((s) => getScopeIdForType(s) !== undefined).map((s) => ({
    value: s,
    label: getScopeLabel(s),
  }))
}

function startEdit(memoryId: string, content: string) {
  editingMemoryId.value = memoryId
  editContent.value = content
}

function cancelEdit() {
  editingMemoryId.value = null
  editContent.value = ''
}

async function handleUpdate(memoryId: string) {
  if (!editContent.value.trim()) return
  const result = await store.updateMemory(memoryId, { content: editContent.value.trim() })
  if (result) {
    editingMemoryId.value = null
    editContent.value = ''
    if (props.taskContext) {
      await store.loadGroupedMemories(props.taskContext)
    }
  }
}

async function handleDelete(memoryId: string) {
  await store.deleteMemory(memoryId)
  if (props.taskContext) {
    await store.loadGroupedMemories(props.taskContext)
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString()
}

function truncate(text: string, length = 120): string {
  return text.length > length ? text.slice(0, length) + '...' : text
}
</script>

<template>
  <div class="memory-panel">
    <div class="panel-header">
      <h3>Memories</h3>
      <BaseButton variant="secondary" @click="showAddForm = !showAddForm">
        {{ showAddForm ? 'Cancel' : 'Add Memory' }}
      </BaseButton>
    </div>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <BaseCard v-if="showAddForm" title="Add Memory">
      <form class="form-vertical" @submit.prevent="handleCreate">
        <div v-if="isGroupedMode" class="form-field">
          <label class="form-label">Scope</label>
          <select v-model="addScopeType" class="form-select">
            <option value="" disabled>Select scope...</option>
            <option v-for="s in availableAddScopes()" :key="s.value" :value="s.value">
              {{ s.label }}
            </option>
          </select>
        </div>
        <div class="form-field">
          <label class="form-label">Content</label>
          <textarea
            v-model="newContent"
            class="form-textarea"
            placeholder="Memory content..."
            rows="3"
          />
        </div>
        <BaseInput v-model="newSource" label="Source" placeholder="e.g. manual, ai, import" />
        <BaseButton :disabled="store.loading">Save</BaseButton>
      </form>
    </BaseCard>

    <!-- Grouped mode: six-level display -->
    <template v-if="isGroupedMode">
      <div v-for="scope in orderedScopes" :key="scope" class="scope-group">
        <button class="scope-header" @click="toggleScope(scope)">
          <span class="scope-toggle">{{ collapsedScopes.has(scope) ? '>' : 'v' }}</span>
          <span class="scope-label">{{ getScopeLabel(scope) }}</span>
          <span class="scope-count">({{ store.groupedMemories[scope]?.length ?? 0 }})</span>
        </button>
        <div v-if="!collapsedScopes.has(scope)" class="memory-list">
          <div
            v-for="memory in store.groupedMemories[scope]"
            :key="memory.id"
            class="memory-item"
          >
            <template v-if="editingMemoryId === memory.id">
              <form class="form-vertical" @submit.prevent="handleUpdate(memory.id)">
                <div class="form-field">
                  <label class="form-label">Content</label>
                  <textarea v-model="editContent" class="form-textarea" rows="3" />
                </div>
                <div class="item-actions">
                  <BaseButton :disabled="store.loading">Save</BaseButton>
                  <BaseButton variant="secondary" @click="cancelEdit">Cancel</BaseButton>
                </div>
              </form>
            </template>
            <template v-else>
              <div class="memory-content">
                <p class="memory-text">{{ truncate(memory.content) }}</p>
                <div class="memory-meta">
                  <span class="memory-source">{{ memory.source }}</span>
                  <span class="memory-date">{{ formatDate(memory.createdAt) }}</span>
                </div>
              </div>
              <div class="item-actions">
                <BaseButton
                  variant="secondary"
                  :disabled="store.loading"
                  @click="startEdit(memory.id, memory.content)"
                >
                  Edit
                </BaseButton>
                <BaseButton
                  variant="danger"
                  :disabled="store.loading"
                  @click="handleDelete(memory.id)"
                >
                  Delete
                </BaseButton>
              </div>
            </template>
          </div>
        </div>
      </div>
      <div v-if="emptyScopes.length > 0" class="empty-scopes">
        <p class="empty-scopes-label">Empty scopes: {{ emptyScopes.map(getScopeLabel).join(', ') }}</p>
      </div>
      <p v-if="orderedScopes.length === 0 && !store.loading" class="empty-text">
        No memories yet.
      </p>
    </template>

    <!-- Single-scope mode (original behavior) -->
    <template v-else>
      <div v-if="store.memories.length > 0" class="memory-list">
        <div v-for="memory in store.memories" :key="memory.id" class="memory-item">
          <template v-if="editingMemoryId === memory.id">
            <form class="form-vertical" @submit.prevent="handleUpdate(memory.id)">
              <div class="form-field">
                <label class="form-label">Content</label>
                <textarea v-model="editContent" class="form-textarea" rows="3" />
              </div>
              <div class="item-actions">
                <BaseButton :disabled="store.loading">Save</BaseButton>
                <BaseButton variant="secondary" @click="cancelEdit">Cancel</BaseButton>
              </div>
            </form>
          </template>
          <template v-else>
            <div class="memory-content">
              <p class="memory-text">{{ truncate(memory.content) }}</p>
              <div class="memory-meta">
                <span class="memory-source">{{ memory.source }}</span>
                <span class="memory-date">{{ formatDate(memory.createdAt) }}</span>
              </div>
            </div>
            <div class="item-actions">
              <BaseButton
                variant="secondary"
                :disabled="store.loading"
                @click="startEdit(memory.id, memory.content)"
              >
                Edit
              </BaseButton>
              <BaseButton
                variant="danger"
                :disabled="store.loading"
                @click="handleDelete(memory.id)"
              >
                Delete
              </BaseButton>
            </div>
          </template>
        </div>
      </div>
      <p v-else-if="!store.loading" class="empty-text">No memories yet.</p>
    </template>
  </div>
</template>

<style scoped>
.memory-panel {
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
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.form-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: #374151;
}

.form-textarea {
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  resize: vertical;
  font-family: inherit;
  box-sizing: border-box;
}

.form-textarea:focus {
  outline: none;
  border-color: #6366f1;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}

.form-select {
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  font-family: inherit;
  background: #ffffff;
  box-sizing: border-box;
}

.form-select:focus {
  outline: none;
  border-color: #6366f1;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}

.scope-group {
  border: 1px solid #e5e7eb;
  border-radius: 0.5rem;
  overflow: hidden;
}

.scope-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.5rem 0.75rem;
  background: #f9fafb;
  border: none;
  cursor: pointer;
  font-size: 0.875rem;
  font-weight: 600;
  color: #374151;
  text-align: left;
}

.scope-header:hover {
  background: #f3f4f6;
}

.scope-toggle {
  font-family: monospace;
  font-size: 0.75rem;
  color: #9ca3af;
  width: 1rem;
  text-align: center;
}

.scope-label {
  flex: 1;
}

.scope-count {
  font-weight: 400;
  color: #6b7280;
  font-size: 0.75rem;
}

.memory-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.5rem;
}

.memory-item {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  background: #ffffff;
}

.memory-content {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  flex: 1;
  min-width: 0;
}

.memory-text {
  margin: 0;
  font-size: 0.875rem;
  color: #1f2937;
  word-break: break-word;
}

.memory-meta {
  display: flex;
  gap: 0.75rem;
  align-items: center;
}

.memory-source {
  font-size: 0.75rem;
  font-weight: 500;
  color: #6366f1;
  background: #eef2ff;
  padding: 0.125rem 0.375rem;
  border-radius: 9999px;
}

.memory-date {
  font-size: 0.75rem;
  color: #9ca3af;
}

.item-actions {
  display: flex;
  gap: 0.5rem;
  flex-shrink: 0;
}

.empty-scopes {
  padding: 0.25rem 0;
}

.empty-scopes-label {
  font-size: 0.75rem;
  color: #9ca3af;
  margin: 0;
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
