<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTaskStore } from '@/stores/task'
import { useTodoStore } from '@/stores/todo'
import { useMemberStore } from '@/stores/member'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import TodoList from '@/components/task/TodoList.vue'
import DataEntryForm from '@/components/data-schema/DataEntryForm.vue'
import ConversationPanel from '@/components/conversation/ConversationPanel.vue'
import EmailThreadsPanel from '@/components/email/EmailThreadsPanel.vue'
import client from '@/api/client'
import type { components } from '@/api/schema'

type TaskStatus = components['schemas']['TaskStatus']
type DataEntryResponse = components['schemas']['DataEntryResponse']
type DataSchemaResponse = components['schemas']['DataSchemaResponse']

const route = useRoute()
const router = useRouter()
const taskStore = useTaskStore()
const todoStore = useTodoStore()
const memberStore = useMemberStore()

const projectId = route.params.projectId as string
const taskId = route.params.taskId as string

// Edit task
const editName = ref('')
const editDescription = ref('')

// Data entries and schemas
const dataEntries = ref<DataEntryResponse[]>([])
const dataSchemas = ref<DataSchemaResponse[]>([])

const statusOptions: TaskStatus[] = ['pending', 'in_progress', 'completed', 'cancelled']

onMounted(async () => {
  await Promise.all([
    taskStore.getTask(projectId, taskId),
    todoStore.fetchTodos(projectId, taskId),
    memberStore.fetchMembers(projectId),
    fetchDataEntries(),
  ])
  if (taskStore.currentTask) {
    editName.value = taskStore.currentTask.name
    editDescription.value = taskStore.currentTask.description ?? ''
    await fetchDataSchemas(taskStore.currentTask.taskTemplateId)
  }
})

function goBack() {
  router.push({ name: 'project-tasks', params: { projectId } })
}

async function handleUpdateTask() {
  await taskStore.updateTask(projectId, taskId, {
    name: editName.value,
    description: editDescription.value || null,
  })
}

async function handleStatusChange(status: TaskStatus) {
  await taskStore.updateTaskStatus(projectId, taskId, status)
}

async function handleDeleteTask() {
  await taskStore.deleteTask(projectId, taskId)
  router.push({ name: 'project-tasks', params: { projectId } })
}

// ── Todos ───────────────────────────────────────────────────

async function handleCreateTodo(title: string, description: string) {
  await todoStore.createTodo(projectId, taskId, title, description || undefined)
}

async function handleToggleTodo(todoId: string, currentStatus: string) {
  const newStatus = currentStatus === 'open' ? 'completed' : 'open'
  await todoStore.updateTodoStatus(projectId, taskId, todoId, newStatus)
}

async function handleDeleteTodo(todoId: string) {
  await todoStore.deleteTodo(projectId, taskId, todoId)
}

// ── Data Entries ────────────────────────────────────────────

async function fetchDataEntries() {
  try {
    const { data } = await client.GET(
      '/api/v1/projects/{projectId}/tasks/{taskId}/data-entries',
      { params: { path: { projectId, taskId } } },
    )
    if (data) {
      dataEntries.value = data.entries
    }
  } catch {
    // Silently handle — not critical
  }
}

async function fetchDataSchemas(taskTemplateId: string) {
  try {
    const { data } = await client.GET(
      '/api/v1/projects/{projectId}/task-templates/{templateId}/data-schemas',
      { params: { path: { projectId, templateId: taskTemplateId } } },
    )
    if (data) {
      dataSchemas.value = data.dataSchemas
    }
  } catch {
    // Silently handle — schemas may not be available
  }
}

async function handleSaveEntry(schemaId: string, values: Record<string, unknown>) {
  try {
    const { data } = await client.PUT(
      '/api/v1/projects/{projectId}/tasks/{taskId}/data-entries/{schemaId}',
      {
        params: { path: { projectId, taskId, schemaId } },
        body: { values },
      },
    )
    if (data) {
      // Update the local entry
      const idx = dataEntries.value.findIndex((e) => e.dataSchemaId === schemaId)
      if (idx >= 0) {
        dataEntries.value[idx] = data
      } else {
        dataEntries.value.push(data)
      }
    }
  } catch {
    // Error handling can be improved
  }
}
</script>

<template>
  <div class="task-detail-view">
    <div class="header">
      <BaseButton variant="secondary" @click="goBack">Back</BaseButton>
      <h1>Task Detail</h1>
    </div>

    <p v-if="taskStore.error" class="error-message">{{ taskStore.error }}</p>
    <p v-if="todoStore.error" class="error-message">{{ todoStore.error }}</p>

    <!-- Task Info -->
    <BaseCard v-if="taskStore.currentTask" title="Task Info">
      <form class="form-vertical" @submit.prevent="handleUpdateTask">
        <BaseInput v-model="editName" label="Name" placeholder="Task name" />
        <BaseInput v-model="editDescription" label="Description" placeholder="Description" />
        <BaseButton :disabled="taskStore.loading">Save</BaseButton>
      </form>

      <div class="status-section">
        <span class="field-label">Status:</span>
        <span :class="['status-badge', `status-${taskStore.currentTask.status}`]">
          {{ taskStore.currentTask.status }}
        </span>
      </div>

      <div class="status-actions">
        <BaseButton
          v-for="s in statusOptions"
          :key="s"
          variant="secondary"
          :disabled="taskStore.loading || taskStore.currentTask.status === s"
          @click="handleStatusChange(s)"
        >
          {{ s }}
        </BaseButton>
      </div>

      <div class="danger-zone">
        <BaseButton variant="danger" :disabled="taskStore.loading" @click="handleDeleteTask">
          Delete Task
        </BaseButton>
      </div>
    </BaseCard>

    <!-- Todos -->
    <BaseCard title="Todos">
      <TodoList
        :todos="todoStore.todos"
        :loading="todoStore.loading"
        @toggle="handleToggleTodo"
        @delete="handleDeleteTodo"
        @create="handleCreateTodo"
      />
    </BaseCard>

    <!-- Data Entries -->
    <BaseCard title="Data Entries">
      <DataEntryForm :schemas="dataSchemas" :entries="dataEntries" @save="handleSaveEntry" />
    </BaseCard>

    <!-- Email Threads -->
    <BaseCard title="Email">
      <EmailThreadsPanel :project-id="projectId" :task-id="taskId" />
    </BaseCard>

    <!-- Conversation -->
    <BaseCard title="Conversation">
      <ConversationPanel :project-id="projectId" :task-id="taskId" />
    </BaseCard>
  </div>
</template>

<style scoped>
.task-detail-view {
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
  margin-bottom: 1rem;
}

.status-section {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.status-actions {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.danger-zone {
  padding-top: 1rem;
  border-top: 1px solid #fecaca;
}

.field-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: #374151;
}

.status-badge {
  font-size: 0.75rem;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
  font-weight: 500;
}

.status-pending {
  background: #fef3c7;
  color: #92400e;
}

.status-in_progress {
  background: #dbeafe;
  color: #1e40af;
}

.status-completed {
  background: #d1fae5;
  color: #065f46;
}

.status-cancelled {
  background: #f3f4f6;
  color: #6b7280;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
}
</style>
