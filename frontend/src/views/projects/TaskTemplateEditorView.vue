<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTaskTemplateStore } from '@/stores/taskTemplate'
import { useMemberTagStore } from '@/stores/memberTag'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import type { components } from '@/api/schema'

type DataSchemaField = components['schemas']['DataSchemaField']
type FieldType = components['schemas']['FieldType']

const route = useRoute()
const router = useRouter()
const store = useTaskTemplateStore()
const tagStore = useMemberTagStore()

const projectId = route.params.projectId as string
const templateId = route.params.templateId as string

// Edit template
const editName = ref('')
const editDescription = ref('')

// Todo templates
const newTodoName = ref('')
const newTodoDescription = ref('')

// Data schemas
const newSchemaName = ref('')
const newSchemaFields = ref<DataSchemaField[]>([])

// Tags
const linkTagId = ref('')

const fieldTypes: FieldType[] = [
  'single_line_text',
  'multi_line_text',
  'number',
  'date',
  'email',
  'url',
  'select',
  'boolean',
  'image',
  'file',
]

onMounted(async () => {
  await Promise.all([
    store.getTemplate(projectId, templateId),
    store.fetchTodoTemplates(projectId, templateId),
    store.fetchDataSchemas(projectId, templateId),
    store.fetchLinkedTags(projectId, templateId),
    tagStore.fetchTags(projectId),
  ])
  if (store.currentTemplate) {
    editName.value = store.currentTemplate.name
    editDescription.value = store.currentTemplate.description ?? ''
  }
})

function goBack() {
  router.push({ name: 'project-task-templates', params: { projectId } })
}

// ── Template ────────────────────────────────────────────────

async function handleUpdateTemplate() {
  await store.updateTemplate(projectId, templateId, {
    name: editName.value,
    description: editDescription.value || null,
  })
}

// ── Todo Templates ──────────────────────────────────────────

async function handleCreateTodo() {
  if (!newTodoName.value.trim()) return
  const sortOrder = store.todoTemplates.length
  const result = await store.createTodoTemplate(
    projectId,
    templateId,
    newTodoName.value.trim(),
    sortOrder,
    undefined,
    newTodoDescription.value.trim() || undefined,
  )
  if (result) {
    newTodoName.value = ''
    newTodoDescription.value = ''
  }
}

async function handleDeleteTodo(todoTemplateId: string) {
  await store.deleteTodoTemplate(projectId, templateId, todoTemplateId)
}

// ── Data Schemas ────────────────────────────────────────────

function addField() {
  newSchemaFields.value.push({
    key: '',
    label: '',
    description: '',
    type: 'single_line_text',
    required: false,
    constraints: null,
  })
}

function removeField(index: number) {
  newSchemaFields.value.splice(index, 1)
}

async function handleCreateSchema() {
  if (!newSchemaName.value.trim() || newSchemaFields.value.length === 0) return
  const result = await store.createDataSchema(
    projectId,
    templateId,
    newSchemaName.value.trim(),
    newSchemaFields.value,
  )
  if (result) {
    newSchemaName.value = ''
    newSchemaFields.value = []
  }
}

async function handleDeleteSchema(schemaId: string) {
  await store.deleteDataSchema(projectId, templateId, schemaId)
}

// ── Tags ────────────────────────────────────────────────────

async function handleLinkTag() {
  if (!linkTagId.value) return
  await store.linkTag(projectId, templateId, linkTagId.value)
  linkTagId.value = ''
}

async function handleUnlinkTag(memberTagId: string) {
  await store.unlinkTag(projectId, templateId, memberTagId)
}

function getTagName(memberTagId: string): string {
  const tag = tagStore.tags.find((t) => t.id === memberTagId)
  return tag?.name ?? memberTagId
}

function availableTags() {
  const linkedIds = new Set(store.linkedTags.map((t) => t.memberTagId))
  return tagStore.tags.filter((t) => !linkedIds.has(t.id))
}
</script>

<template>
  <div class="template-editor-view">
    <div class="header">
      <BaseButton variant="secondary" @click="goBack">Back</BaseButton>
      <h1>Edit Template</h1>
    </div>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <!-- Template Info -->
    <BaseCard title="Template Info">
      <form class="form-vertical" @submit.prevent="handleUpdateTemplate">
        <BaseInput v-model="editName" label="Name" placeholder="Template name" />
        <BaseInput v-model="editDescription" label="Description" placeholder="Description" />
        <BaseButton :disabled="store.loading">Save</BaseButton>
      </form>
    </BaseCard>

    <!-- Todo Templates -->
    <BaseCard title="Todo Templates">
      <form class="create-form" @submit.prevent="handleCreateTodo">
        <BaseInput v-model="newTodoName" label="Name" placeholder="Todo name" />
        <BaseInput
          v-model="newTodoDescription"
          label="Description"
          placeholder="Optional description"
        />
        <BaseButton :disabled="store.loading">Add</BaseButton>
      </form>

      <ul class="item-list">
        <li v-for="todo in store.todoTemplates" :key="todo.id" class="item-row">
          <div class="item-info">
            <strong>{{ todo.name }}</strong>
            <span v-if="todo.description" class="item-sub">{{ todo.description }}</span>
            <span class="item-sub">Sort: {{ todo.sortOrder }}</span>
          </div>
          <BaseButton
            variant="danger"
            :disabled="store.loading"
            @click="handleDeleteTodo(todo.id)"
          >
            Delete
          </BaseButton>
        </li>
      </ul>
      <p v-if="store.todoTemplates.length === 0 && !store.loading" class="empty-text">
        No todo templates yet.
      </p>
    </BaseCard>

    <!-- Data Schemas -->
    <BaseCard title="Data Schemas">
      <div class="schema-create">
        <BaseInput v-model="newSchemaName" label="Schema Name" placeholder="Schema name" />

        <div v-for="(field, idx) in newSchemaFields" :key="idx" class="field-row">
          <BaseInput v-model="field.key" label="Key" placeholder="field_key" />
          <BaseInput v-model="field.label" label="Label" placeholder="Field Label" />
          <BaseInput v-model="field.description" label="Description" placeholder="Description" />
          <div class="field-select">
            <label class="field-label">Type</label>
            <select v-model="field.type">
              <option v-for="ft in fieldTypes" :key="ft" :value="ft">{{ ft }}</option>
            </select>
          </div>
          <label class="checkbox-label">
            <input v-model="field.required" type="checkbox" />
            Required
          </label>
          <BaseButton variant="danger" @click="removeField(idx)">Remove</BaseButton>
        </div>

        <div class="schema-actions">
          <BaseButton variant="secondary" @click="addField">Add Field</BaseButton>
          <BaseButton :disabled="store.loading" @click="handleCreateSchema">
            Create Schema
          </BaseButton>
        </div>
      </div>

      <ul class="item-list">
        <li v-for="schema in store.dataSchemas" :key="schema.id" class="item-row">
          <div class="item-info">
            <strong>{{ schema.name }}</strong>
            <span class="item-sub">{{ schema.fields.length }} fields</span>
            <ul class="field-summary">
              <li v-for="field in schema.fields" :key="field.key" class="field-summary-item">
                {{ field.label }} ({{ field.type }}{{ field.required ? ', required' : '' }})
              </li>
            </ul>
          </div>
          <BaseButton
            variant="danger"
            :disabled="store.loading"
            @click="handleDeleteSchema(schema.id)"
          >
            Delete
          </BaseButton>
        </li>
      </ul>
      <p v-if="store.dataSchemas.length === 0 && !store.loading" class="empty-text">
        No data schemas yet.
      </p>
    </BaseCard>

    <!-- Linked Tags -->
    <BaseCard title="Linked Tags">
      <form class="create-form" @submit.prevent="handleLinkTag">
        <div class="field-select">
          <label class="field-label">Tag</label>
          <select v-model="linkTagId">
            <option value="" disabled>Select a tag</option>
            <option v-for="tag in availableTags()" :key="tag.id" :value="tag.id">
              {{ tag.name }}
            </option>
          </select>
        </div>
        <BaseButton :disabled="store.loading || !linkTagId">Link</BaseButton>
      </form>

      <ul class="item-list">
        <li v-for="link in store.linkedTags" :key="link.id" class="item-row">
          <div class="item-info">
            <strong>{{ getTagName(link.memberTagId) }}</strong>
          </div>
          <BaseButton
            variant="danger"
            :disabled="store.loading"
            @click="handleUnlinkTag(link.memberTagId)"
          >
            Unlink
          </BaseButton>
        </li>
      </ul>
      <p v-if="store.linkedTags.length === 0 && !store.loading" class="empty-text">
        No linked tags yet.
      </p>
    </BaseCard>
  </div>
</template>

<style scoped>
.template-editor-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 800px;
}

.header {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.form-vertical {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  max-width: 400px;
}

.create-form {
  display: flex;
  gap: 0.75rem;
  align-items: flex-end;
  margin-bottom: 1rem;
}

.item-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.item-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.item-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.item-sub {
  font-size: 0.75rem;
  color: #6b7280;
}

.schema-create {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.field-row {
  display: flex;
  gap: 0.5rem;
  align-items: flex-end;
  flex-wrap: wrap;
  padding: 0.5rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.field-select {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.field-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: #374151;
}

.field-select select {
  padding: 0.375rem 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.875rem;
  white-space: nowrap;
}

.schema-actions {
  display: flex;
  gap: 0.5rem;
}

.field-summary {
  list-style: none;
  padding: 0;
  margin: 0.25rem 0 0;
}

.field-summary-item {
  font-size: 0.75rem;
  color: #9ca3af;
}

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
