<script setup lang="ts">
import { computed, watch } from 'vue'

const props = defineProps<{
  schema: Record<string, unknown>
  modelValue: Record<string, unknown>
}>()

const emit = defineEmits<{
  'update:modelValue': [value: Record<string, unknown>]
}>()

interface PropertyDef {
  type?: string
  description?: string
  enum?: string[]
  format?: string
  items?: Record<string, unknown>
}

const properties = computed(() => {
  const schemaProps = (props.schema.properties ?? {}) as Record<string, PropertyDef>
  const requiredFields = (props.schema.required ?? []) as string[]
  return Object.entries(schemaProps).map(([key, def]) => ({
    key,
    type: def.type ?? 'string',
    description: def.description ?? '',
    enumValues: def.enum,
    format: def.format,
    required: requiredFields.includes(key),
  }))
})

function updateField(key: string, value: unknown) {
  emit('update:modelValue', { ...props.modelValue, [key]: value })
}

function getFieldValue(key: string): unknown {
  return props.modelValue[key] ?? ''
}

// Initialize empty fields when schema changes
watch(
  () => props.schema,
  () => {
    const currentValue = { ...props.modelValue }
    let changed = false
    for (const prop of properties.value) {
      if (!(prop.key in currentValue)) {
        currentValue[prop.key] = ''
        changed = true
      }
    }
    if (changed) {
      emit('update:modelValue', currentValue)
    }
  },
  { immediate: true },
)
</script>

<template>
  <div class="tool-parameter-form">
    <div v-for="prop in properties" :key="prop.key" class="form-field">
      <label :for="`field-${prop.key}`" class="field-label">
        {{ prop.key }}
        <span v-if="prop.required" class="required-marker">*</span>
      </label>
      <p v-if="prop.description" class="field-description">{{ prop.description }}</p>

      <select
        v-if="prop.enumValues"
        :id="`field-${prop.key}`"
        :value="getFieldValue(prop.key) as string"
        class="field-input"
        @change="updateField(prop.key, ($event.target as HTMLSelectElement).value)"
      >
        <option value="">-- Select --</option>
        <option v-for="val in prop.enumValues" :key="val" :value="val">{{ val }}</option>
      </select>

      <textarea
        v-else-if="prop.type === 'string' && prop.key.toLowerCase().includes('body')"
        :id="`field-${prop.key}`"
        :value="getFieldValue(prop.key) as string"
        class="field-input field-textarea"
        rows="4"
        @input="updateField(prop.key, ($event.target as HTMLTextAreaElement).value)"
      />

      <input
        v-else
        :id="`field-${prop.key}`"
        :type="prop.type === 'number' ? 'number' : 'text'"
        :value="getFieldValue(prop.key) as string"
        class="field-input"
        :placeholder="prop.format ? `Format: ${prop.format}` : ''"
        @input="updateField(prop.key, ($event.target as HTMLInputElement).value)"
      />
    </div>
    <p v-if="properties.length === 0" class="no-params">No parameters required.</p>
  </div>
</template>

<style scoped>
.tool-parameter-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.field-label {
  font-weight: 600;
  font-size: 0.875rem;
}

.required-marker {
  color: #dc2626;
  margin-left: 0.125rem;
}

.field-description {
  font-size: 0.75rem;
  color: #6b7280;
  margin: 0;
}

.field-input {
  padding: 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
}

.field-textarea {
  resize: vertical;
  font-family: inherit;
}

.no-params {
  color: #6b7280;
  font-style: italic;
}
</style>
