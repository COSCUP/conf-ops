<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import ToolParameterForm from './ToolParameterForm.vue'
import type { ToolSummary, ExecuteToolResult } from '@/stores/toolConfig'
import { useToolConfigStore } from '@/stores/toolConfig'

const props = defineProps<{
  projectId: string
  taskId: string
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
  executed: [result: ExecuteToolResult]
}>()

const store = useToolConfigStore()

const selectedToolName = ref('')
const parameters = ref<Record<string, unknown>>({})
const executing = ref(false)
const result = ref<ExecuteToolResult | null>(null)
const executionError = ref<string | null>(null)

const selectedTool = computed<ToolSummary | undefined>(() =>
  store.tools.find((t) => t.name === selectedToolName.value),
)

const selectedToolSchema = computed(() => {
  // For now, use a generic schema. In production this would come from getToolDetails.
  return {
    type: 'object',
    properties: {},
  } as Record<string, unknown>
})

watch(
  () => props.visible,
  async (visible) => {
    if (visible) {
      await store.listTools(props.projectId)
      selectedToolName.value = ''
      parameters.value = {}
      result.value = null
      executionError.value = null
    }
  },
)

watch(selectedToolName, () => {
  parameters.value = {}
  result.value = null
  executionError.value = null
})

async function handleExecute() {
  if (!selectedToolName.value) return

  executing.value = true
  executionError.value = null
  result.value = null

  const execResult = await store.executeTool(props.projectId, selectedToolName.value, {
    taskId: props.taskId,
    parameters: parameters.value,
  })

  executing.value = false

  if (execResult) {
    result.value = execResult
    emit('executed', execResult)
  } else {
    executionError.value = store.error ?? 'Execution failed'
  }
}

function formatResult(val: unknown): string {
  try {
    return JSON.stringify(val, null, 2)
  } catch {
    return String(val)
  }
}
</script>

<template>
  <div v-if="visible" class="dialog-overlay" @click.self="emit('close')">
    <div class="dialog-content">
      <div class="dialog-header">
        <h2>Execute Tool</h2>
        <button class="close-button" @click="emit('close')">X</button>
      </div>

      <div class="dialog-body">
        <div class="form-field">
          <label for="tool-select">Select Tool</label>
          <select id="tool-select" v-model="selectedToolName" class="tool-select">
            <option value="">-- Choose a tool --</option>
            <option v-for="tool in store.tools" :key="tool.name" :value="tool.name">
              {{ tool.displayName ?? tool.name }} ({{ tool.category }})
            </option>
          </select>
        </div>

        <div v-if="selectedTool" class="tool-info">
          <p class="tool-description">{{ selectedTool.description }}</p>
          <span v-if="selectedTool.requiresConfirmation" class="confirmation-badge">
            Requires confirmation
          </span>
        </div>

        <ToolParameterForm
          v-if="selectedToolName"
          v-model="parameters"
          :schema="selectedToolSchema"
        />

        <div v-if="executionError" class="error-message">
          {{ executionError }}
        </div>

        <div v-if="result" class="result-container">
          <h3>Result</h3>
          <pre class="result-pre">{{ formatResult(result.result) }}</pre>
          <p class="result-meta">
            {{ result.success ? 'Success' : 'Error' }} - {{ result.durationMs }}ms
          </p>
        </div>
      </div>

      <div class="dialog-footer">
        <button class="btn btn-secondary" @click="emit('close')">Cancel</button>
        <button
          class="btn btn-primary"
          :disabled="!selectedToolName || executing"
          @click="handleExecute"
        >
          {{ executing ? 'Executing...' : 'Execute' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog-content {
  background: white;
  border-radius: 0.5rem;
  width: 100%;
  max-width: 600px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 1.5rem;
  border-bottom: 1px solid #e5e7eb;
}

.dialog-header h2 {
  margin: 0;
  font-size: 1.25rem;
}

.close-button {
  background: none;
  border: none;
  font-size: 1.25rem;
  cursor: pointer;
  color: #6b7280;
}

.dialog-body {
  padding: 1.5rem;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1rem;
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

.tool-select {
  padding: 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
}

.tool-info {
  background: #f9fafb;
  padding: 0.75rem;
  border-radius: 0.375rem;
}

.tool-description {
  margin: 0 0 0.5rem;
  font-size: 0.875rem;
  color: #374151;
}

.confirmation-badge {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  background: #fef3c7;
  color: #92400e;
  font-size: 0.75rem;
  border-radius: 0.25rem;
}

.error-message {
  background: #fef2f2;
  color: #dc2626;
  padding: 0.75rem;
  border-radius: 0.375rem;
  font-size: 0.875rem;
}

.result-container {
  background: #f0fdf4;
  padding: 0.75rem;
  border-radius: 0.375rem;
}

.result-container h3 {
  margin: 0 0 0.5rem;
  font-size: 0.875rem;
}

.result-pre {
  background: #ecfdf5;
  padding: 0.5rem;
  border-radius: 0.25rem;
  font-size: 0.75rem;
  overflow-x: auto;
  max-height: 200px;
}

.result-meta {
  margin: 0.5rem 0 0;
  font-size: 0.75rem;
  color: #6b7280;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  padding: 1rem 1.5rem;
  border-top: 1px solid #e5e7eb;
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
</style>
