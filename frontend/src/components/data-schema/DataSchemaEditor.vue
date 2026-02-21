<script setup lang="ts">
import type { components } from '@/api/schema'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import { ref } from 'vue'

type DataSchemaResponse = components['schemas']['DataSchemaResponse']
type DataSchemaField = components['schemas']['DataSchemaField']
type FieldType = components['schemas']['FieldType']

defineProps<{
  schemas: DataSchemaResponse[]
  loading: boolean
}>()

const emit = defineEmits<{
  create: [name: string, fields: DataSchemaField[]]
  delete: [schemaId: string]
}>()

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

const newSchemaName = ref('')
const newSchemaFields = ref<DataSchemaField[]>([])

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

function handleCreate() {
  if (!newSchemaName.value.trim() || newSchemaFields.value.length === 0) return
  emit('create', newSchemaName.value.trim(), newSchemaFields.value)
  newSchemaName.value = ''
  newSchemaFields.value = []
}
</script>

<template>
  <div>
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
        <BaseButton :disabled="loading" @click="handleCreate">Create Schema</BaseButton>
      </div>
    </div>

    <ul class="item-list">
      <li v-for="schema in schemas" :key="schema.id" class="item-row">
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
          :disabled="loading"
          @click="emit('delete', schema.id)"
        >
          Delete
        </BaseButton>
      </li>
    </ul>
    <p v-if="schemas.length === 0 && !loading" class="empty-text">No data schemas yet.</p>
  </div>
</template>

<style scoped>
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
</style>
