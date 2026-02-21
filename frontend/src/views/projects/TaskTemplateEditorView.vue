<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTaskTemplateStore } from '@/stores/taskTemplate'
import { useMemberTagStore } from '@/stores/memberTag'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import DataSchemaEditor from '@/components/data-schema/DataSchemaEditor.vue'
import type { components } from '@/api/schema'

type DataSchemaField = components['schemas']['DataSchemaField']

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

// Tags
const linkTagId = ref('')

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

const dragIndex = ref<number | null>(null)

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

function onDragStart(index: number) {
  dragIndex.value = index
}

function onDragOver(event: DragEvent) {
  event.preventDefault()
}

async function onDrop(targetIndex: number) {
  const from = dragIndex.value
  dragIndex.value = null
  if (from === null || from === targetIndex) return

  const items = [...store.todoTemplates]
  const [moved] = items.splice(from, 1)
  if (!moved) return
  items.splice(targetIndex, 0, moved)

  const orders = items.map((item, i) => ({ id: item.id, sortOrder: i }))
  await store.reorderTodoTemplates(projectId, templateId, orders)
}

function onDragEnd() {
  dragIndex.value = null
}

// ── Data Schemas ────────────────────────────────────────────

async function handleCreateSchema(name: string, fields: DataSchemaField[]) {
  await store.createDataSchema(projectId, templateId, name, fields)
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
        <li
          v-for="(todo, index) in store.todoTemplates"
          :key="todo.id"
          class="item-row"
          :class="{ 'drag-over': dragIndex !== null && dragIndex !== index }"
          draggable="true"
          @dragstart="onDragStart(index)"
          @dragover="onDragOver"
          @drop="onDrop(index)"
          @dragend="onDragEnd"
        >
          <div class="item-info">
            <span class="drag-handle" aria-label="Drag to reorder">&#x2630;</span>
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
      <DataSchemaEditor
        :schemas="store.dataSchemas"
        :loading="store.loading"
        @create="handleCreateSchema"
        @delete="handleDeleteSchema"
      />
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

.item-row[draggable='true'] {
  cursor: grab;
}

.item-row[draggable='true']:active {
  cursor: grabbing;
}

.item-row.drag-over {
  border-color: #3b82f6;
  border-style: dashed;
}

.drag-handle {
  cursor: grab;
  color: #9ca3af;
  margin-right: 0.5rem;
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
