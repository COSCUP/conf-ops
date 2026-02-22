<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import type { Suggestion } from '@/stores/suggestion'
import { useSuggestionStore } from '@/stores/suggestion'

const props = defineProps<{
  suggestion: Suggestion
  show: boolean
  projectId: string
  taskId: string
}>()

const emit = defineEmits<{
  confirm: [modifiedParameters: unknown]
  close: []
}>()

const store = useSuggestionStore()

const parametersJson = ref('')
const jsonError = ref<string | null>(null)
const placeholderPreviews = ref<Record<string, string>>({})
const previewLoading = ref(false)

const placeholderPattern = /\{\{(profile|data)\.\w+\}\}/g

const detectedPlaceholders = computed(() => {
  const matches = parametersJson.value.match(placeholderPattern)
  return matches ? [...new Set(matches)] : []
})

watch(
  () => props.show,
  (visible) => {
    if (visible) {
      try {
        parametersJson.value = JSON.stringify(props.suggestion.parameters, null, 2)
      } catch {
        parametersJson.value = ''
      }
      jsonError.value = null
      placeholderPreviews.value = {}
    }
  },
  { immediate: true },
)

async function loadPreview() {
  if (detectedPlaceholders.value.length === 0) return

  previewLoading.value = true
  try {
    const testText = detectedPlaceholders.value.join(' | ')
    const result = await store.resolvePlaceholders(props.projectId, props.taskId, testText)
    if (result) {
      const resolved = result.text.split(' | ')
      const previews: Record<string, string> = {}
      for (let i = 0; i < detectedPlaceholders.value.length; i++) {
        const placeholder = detectedPlaceholders.value[i]
        if (placeholder !== undefined) {
          const resolvedValue = resolved[i]
          previews[placeholder] = resolvedValue !== undefined ? resolvedValue.trim() : placeholder
        }
      }
      placeholderPreviews.value = previews
    }
  } finally {
    previewLoading.value = false
  }
}

watch(detectedPlaceholders, (placeholders) => {
  if (placeholders.length > 0) {
    void loadPreview()
  } else {
    placeholderPreviews.value = {}
  }
})

function handleConfirm() {
  jsonError.value = null
  try {
    const parsed: unknown = JSON.parse(parametersJson.value)
    emit('confirm', parsed)
  } catch {
    jsonError.value = 'Invalid JSON. Please check the syntax and try again.'
  }
}

function handleClose() {
  emit('close')
}

function handleOverlayClick(event: MouseEvent) {
  if (event.target === event.currentTarget) {
    handleClose()
  }
}
</script>

<template>
  <div v-if="show" class="dialog-overlay" @click="handleOverlayClick">
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title">
      <div class="dialog-header">
        <h3 id="dialog-title" class="dialog-title">Modify Parameters</h3>
        <span class="dialog-tool">{{ suggestion.tool }}</span>
      </div>

      <p class="dialog-description">
        Edit the parameters below and confirm to accept with modifications.
      </p>

      <div class="dialog-body">
        <label for="parameters-editor" class="editor-label">Parameters (JSON)</label>
        <textarea
          id="parameters-editor"
          v-model="parametersJson"
          class="parameters-editor"
          rows="12"
          spellcheck="false"
        />
        <p v-if="jsonError" class="json-error">{{ jsonError }}</p>

        <div v-if="detectedPlaceholders.length > 0" class="placeholder-preview">
          <span class="preview-label">Placeholder Preview</span>
          <div v-if="previewLoading" class="preview-loading">Resolving placeholders...</div>
          <table v-else-if="Object.keys(placeholderPreviews).length > 0" class="preview-table">
            <thead>
              <tr>
                <th>Placeholder</th>
                <th>Resolved Value</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="ph in detectedPlaceholders" :key="ph">
                <td class="preview-placeholder">{{ ph }}</td>
                <td class="preview-value">{{ placeholderPreviews[ph] ?? '(unresolved)' }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div class="dialog-footer">
        <button type="button" class="btn btn--secondary" @click="handleClose">Cancel</button>
        <button type="button" class="btn btn--primary" @click="handleConfirm">Confirm</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 50;
}

.dialog {
  background: #ffffff;
  border-radius: 0.5rem;
  box-shadow:
    0 20px 25px -5px rgba(0, 0, 0, 0.1),
    0 10px 10px -5px rgba(0, 0, 0, 0.04);
  width: 90%;
  max-width: 36rem;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 1.25rem;
  border-bottom: 1px solid #e5e7eb;
}

.dialog-title {
  font-size: 1rem;
  font-weight: 600;
  color: #111827;
  margin: 0;
}

.dialog-tool {
  font-size: 0.75rem;
  font-weight: 600;
  color: #065f46;
  background: #d1fae5;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
}

.dialog-description {
  font-size: 0.875rem;
  color: #6b7280;
  margin: 0;
  padding: 0.75rem 1.25rem 0;
}

.dialog-body {
  flex: 1;
  overflow-y: auto;
  padding: 0.75rem 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.editor-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: #374151;
}

.parameters-editor {
  width: 100%;
  font-family: ui-monospace, 'Cascadia Code', 'Source Code Pro', Menlo, Consolas, 'DejaVu Sans Mono', monospace;
  font-size: 0.8125rem;
  color: #1f2937;
  background: #f9fafb;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  padding: 0.5rem;
  resize: vertical;
  box-sizing: border-box;
}

.parameters-editor:focus {
  outline: 2px solid #3b82f6;
  outline-offset: -1px;
}

.json-error {
  font-size: 0.75rem;
  color: #dc2626;
  margin: 0;
}

.placeholder-preview {
  margin-top: 0.5rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  padding: 0.5rem;
  background: #fefce8;
}

.preview-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: #92400e;
  display: block;
  margin-bottom: 0.375rem;
}

.preview-loading {
  font-size: 0.75rem;
  color: #6b7280;
  font-style: italic;
}

.preview-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.75rem;
}

.preview-table th {
  text-align: left;
  font-weight: 600;
  color: #374151;
  padding: 0.25rem 0.5rem;
  border-bottom: 1px solid #e5e7eb;
}

.preview-table td {
  padding: 0.25rem 0.5rem;
  border-bottom: 1px solid #f3f4f6;
}

.preview-placeholder {
  font-family: ui-monospace, 'Cascadia Code', 'Source Code Pro', Menlo, Consolas, 'DejaVu Sans Mono', monospace;
  color: #7c3aed;
}

.preview-value {
  color: #059669;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  padding: 0.75rem 1.25rem;
  border-top: 1px solid #e5e7eb;
}

.btn {
  font-size: 0.875rem;
  font-weight: 500;
  padding: 0.5rem 1rem;
  border-radius: 0.375rem;
  border: 1px solid transparent;
  cursor: pointer;
  transition: opacity 0.15s;
}

.btn:hover {
  opacity: 0.85;
}

.btn--primary {
  background: #059669;
  color: #ffffff;
}

.btn--secondary {
  background: #ffffff;
  color: #374151;
  border-color: #d1d5db;
}
</style>
