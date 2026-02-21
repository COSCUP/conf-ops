<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { components } from '@/api/schema'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseButton from '@/components/base/BaseButton.vue'
import FileUploader from '@/components/file/FileUploader.vue'
import FilePreview from '@/components/file/FilePreview.vue'
import type { FileUploadResult } from '@/composables/useFileUpload'

type DataSchemaResponse = components['schemas']['DataSchemaResponse']
type DataSchemaField = components['schemas']['DataSchemaField']
type DataEntryResponse = components['schemas']['DataEntryResponse']

const props = defineProps<{
  schemas: DataSchemaResponse[]
  entries: DataEntryResponse[]
  scopeType?: string
  scopeId?: string
}>()

const emit = defineEmits<{
  save: [schemaId: string, values: Record<string, unknown>]
}>()

// Form state: schemaId -> fieldKey -> value
const formData = ref<Record<string, Record<string, unknown>>>({})
const savingSchemaId = ref<string | null>(null)

// Initialize form data from entries
function initFormData() {
  const data: Record<string, Record<string, unknown>> = {}
  for (const schema of props.schemas) {
    const entry = props.entries.find((e) => e.dataSchemaId === schema.id)
    const values = (entry?.values as Record<string, unknown>) ?? {}
    const fields: Record<string, unknown> = {}
    const schemaFields = schema.fields ?? []
    for (const field of schemaFields) {
      const val = values[field.key]
      fields[field.key] = val ?? getDefaultValue(field)
    }
    data[schema.id] = fields
  }
  formData.value = data
}

watch(
  () => [props.schemas, props.entries],
  () => initFormData(),
  { immediate: true, deep: true },
)

function getDefaultValue(field: DataSchemaField): unknown {
  switch (field.type) {
    case 'boolean':
      return false
    case 'number':
      return null
    default:
      return ''
  }
}

function getFieldValue(schemaId: string, key: string): unknown {
  return formData.value[schemaId]?.[key] ?? ''
}

function setFieldValue(schemaId: string, key: string, value: unknown) {
  if (!formData.value[schemaId]) {
    formData.value[schemaId] = {}
  }
  formData.value[schemaId]![key] = value
}

function handleSubmit(schemaId: string) {
  const values = formData.value[schemaId]
  if (!values) return

  // Filter out empty/null values for submission
  const filtered: Record<string, unknown> = {}
  for (const [key, val] of Object.entries(values)) {
    if (val !== '' && val !== null && val !== undefined) {
      filtered[key] = val
    }
  }

  savingSchemaId.value = schemaId
  emit('save', schemaId, filtered)
  // Reset saving state after a tick (parent should handle async)
  setTimeout(() => {
    savingSchemaId.value = null
  }, 500)
}

function handleFileUploaded(schemaId: string, fieldKey: string, file: FileUploadResult) {
  setFieldValue(schemaId, fieldKey, file.id)
}

function handleFileRemoved(schemaId: string, fieldKey: string) {
  setFieldValue(schemaId, fieldKey, '')
}

const hasSchemas = computed(() => props.schemas.length > 0)
</script>

<template>
  <div class="data-entry-form">
    <div v-if="!hasSchemas" class="empty-text">No data schemas defined for this template.</div>

    <div v-for="schema in schemas" :key="schema.id" class="schema-section">
      <h3 class="schema-title">{{ schema.name }}</h3>
      <form class="schema-form" @submit.prevent="handleSubmit(schema.id)">
        <div v-for="field in schema.fields" :key="field.key" class="field-group">
          <label class="field-label">
            {{ field.label }}
            <span v-if="field.required" class="required-mark">*</span>
          </label>
          <p v-if="field.description" class="field-description">{{ field.description }}</p>

          <!-- single_line_text -->
          <BaseInput
            v-if="field.type === 'single_line_text'"
            :model-value="String(getFieldValue(schema.id, field.key) ?? '')"
            :placeholder="field.label"
            @update:model-value="setFieldValue(schema.id, field.key, $event)"
          />

          <!-- multi_line_text -->
          <textarea
            v-else-if="field.type === 'multi_line_text'"
            class="textarea-field"
            :value="String(getFieldValue(schema.id, field.key) ?? '')"
            :placeholder="field.label"
            rows="3"
            @input="
              setFieldValue(
                schema.id,
                field.key,
                ($event.target as HTMLTextAreaElement).value,
              )
            "
          />

          <!-- number -->
          <input
            v-else-if="field.type === 'number'"
            type="number"
            class="input-field"
            :value="getFieldValue(schema.id, field.key)"
            :min="field.constraints?.min ?? undefined"
            :max="field.constraints?.max ?? undefined"
            :step="field.constraints?.decimal ? '0.01' : '1'"
            :placeholder="field.label"
            @input="
              setFieldValue(
                schema.id,
                field.key,
                ($event.target as HTMLInputElement).value
                  ? Number(($event.target as HTMLInputElement).value)
                  : null,
              )
            "
          />

          <!-- date -->
          <input
            v-else-if="field.type === 'date'"
            type="date"
            class="input-field"
            :value="String(getFieldValue(schema.id, field.key) ?? '')"
            @input="
              setFieldValue(
                schema.id,
                field.key,
                ($event.target as HTMLInputElement).value,
              )
            "
          />

          <!-- email -->
          <BaseInput
            v-else-if="field.type === 'email'"
            type="email"
            :model-value="String(getFieldValue(schema.id, field.key) ?? '')"
            :placeholder="field.label"
            @update:model-value="setFieldValue(schema.id, field.key, $event)"
          />

          <!-- url -->
          <BaseInput
            v-else-if="field.type === 'url'"
            type="url"
            :model-value="String(getFieldValue(schema.id, field.key) ?? '')"
            :placeholder="field.label"
            @update:model-value="setFieldValue(schema.id, field.key, $event)"
          />

          <!-- select -->
          <select
            v-else-if="field.type === 'select'"
            class="select-field"
            :value="String(getFieldValue(schema.id, field.key) ?? '')"
            @change="
              setFieldValue(
                schema.id,
                field.key,
                ($event.target as HTMLSelectElement).value,
              )
            "
          >
            <option value="">-- Select --</option>
            <option
              v-for="option in field.constraints?.options ?? []"
              :key="option"
              :value="option"
            >
              {{ option }}
            </option>
          </select>

          <!-- boolean -->
          <label v-else-if="field.type === 'boolean'" class="checkbox-label">
            <input
              type="checkbox"
              :checked="Boolean(getFieldValue(schema.id, field.key))"
              @change="
                setFieldValue(
                  schema.id,
                  field.key,
                  ($event.target as HTMLInputElement).checked,
                )
              "
            />
            {{ field.label }}
          </label>

          <!-- image field -->
          <template v-else-if="field.type === 'image'">
            <FilePreview
              v-if="getFieldValue(schema.id, field.key)"
              :file-id="String(getFieldValue(schema.id, field.key))"
              :filename="field.label"
              mime-type="image/*"
              :size="0"
            />
            <FileUploader
              v-if="scopeType && scopeId"
              :scope-type="scopeType"
              :scope-id="scopeId"
              accept="image/*"
              @uploaded="handleFileUploaded(schema.id, field.key, $event)"
              @removed="handleFileRemoved(schema.id, field.key)"
            />
          </template>

          <!-- file field -->
          <template v-else-if="field.type === 'file'">
            <FilePreview
              v-if="getFieldValue(schema.id, field.key)"
              :file-id="String(getFieldValue(schema.id, field.key))"
              :filename="field.label"
              mime-type="application/octet-stream"
              :size="0"
            />
            <FileUploader
              v-if="scopeType && scopeId"
              :scope-type="scopeType"
              :scope-id="scopeId"
              @uploaded="handleFileUploaded(schema.id, field.key, $event)"
              @removed="handleFileRemoved(schema.id, field.key)"
            />
          </template>
        </div>

        <BaseButton type="submit" :disabled="savingSchemaId === schema.id">
          {{ savingSchemaId === schema.id ? 'Saving...' : 'Save' }}
        </BaseButton>
      </form>
    </div>
  </div>
</template>

<style scoped>
.data-entry-form {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.schema-section {
  border: 1px solid #e5e7eb;
  border-radius: 0.5rem;
  padding: 1rem;
}

.schema-title {
  font-size: 1rem;
  font-weight: 600;
  color: #1f2937;
  margin: 0 0 1rem;
}

.schema-form {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  max-width: 500px;
}

.field-group {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.field-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: #374151;
}

.required-mark {
  color: #ef4444;
  margin-left: 0.125rem;
}

.field-description {
  font-size: 0.75rem;
  color: #6b7280;
  margin: 0;
}

.input-field,
.textarea-field,
.select-field {
  padding: 0.5rem 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  outline: none;
  transition: border-color 0.15s;
  font-family: inherit;
}

.input-field:focus,
.textarea-field:focus,
.select-field:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.875rem;
  color: #374151;
  cursor: pointer;
}

.empty-text {
  color: #9ca3af;
  font-size: 0.875rem;
}
</style>
