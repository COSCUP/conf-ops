<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTaskStore } from '@/stores/task'
import { useTodoStore } from '@/stores/todo'
import { useMemberStore } from '@/stores/member'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import client from '@/api/client'
import type { components } from '@/api/schema'

type TaskStatus = components['schemas']['TaskStatus']
type DataEntryResponse = components['schemas']['DataEntryResponse']

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

// Todo creation
const newTodoTitle = ref('')
const newTodoDescription = ref('')

// Data entries
const dataEntries = ref<DataEntryResponse[]>([])

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

async function handleCreateTodo() {
  if (!newTodoTitle.value.trim()) return
  const result = await todoStore.createTodo(
    projectId,
    taskId,
    newTodoTitle.value.trim(),
    newTodoDescription.value.trim() || undefined,
  )
  if (result) {
    newTodoTitle.value = ''
    newTodoDescription.value = ''
  }
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
      <form class="create-form" @submit.prevent="handleCreateTodo">
        <BaseInput v-model="newTodoTitle" label="Title" placeholder="New todo" />
        <BaseInput
          v-model="newTodoDescription"
          label="Description"
          placeholder="Optional description"
        />
        <BaseButton :disabled="todoStore.loading">Add Todo</BaseButton>
      </form>

      <ul class="todo-list">
        <li v-for="todo in todoStore.todos" :key="todo.id" class="todo-item">
          <div class="todo-left">
            <input
              type="checkbox"
              :checked="todo.status === 'completed'"
              @change="handleToggleTodo(todo.id, todo.status)"
            />
            <div class="todo-info">
              <span :class="{ 'todo-completed': todo.status === 'completed' }">
                {{ todo.title }}
              </span>
              <span v-if="todo.description" class="todo-sub">{{ todo.description }}</span>
              <span class="todo-sub">
                {{ todo.todoType }}
                <template v-if="todo.dueDate"> · Due {{ todo.dueDate }}</template>
              </span>
            </div>
          </div>
          <BaseButton
            variant="danger"
            :disabled="todoStore.loading"
            @click="handleDeleteTodo(todo.id)"
          >
            Delete
          </BaseButton>
        </li>
      </ul>
      <p v-if="todoStore.todos.length === 0 && !todoStore.loading" class="empty-text">
        No todos yet.
      </p>
    </BaseCard>

    <!-- Data Entries -->
    <BaseCard title="Data Entries">
      <ul class="entry-list">
        <li v-for="entry in dataEntries" :key="entry.id" class="entry-item">
          <div class="entry-info">
            <span class="entry-schema">Schema: {{ entry.dataSchemaId }}</span>
            <pre class="entry-values">{{ JSON.stringify(entry.values, null, 2) }}</pre>
          </div>
        </li>
      </ul>
      <p v-if="dataEntries.length === 0" class="empty-text">No data entries yet.</p>
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

.create-form {
  display: flex;
  gap: 0.75rem;
  align-items: flex-end;
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

.todo-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.todo-item {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 0.5rem 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.todo-left {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
}

.todo-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.todo-completed {
  text-decoration: line-through;
  color: #9ca3af;
}

.todo-sub {
  font-size: 0.75rem;
  color: #6b7280;
}

.entry-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.entry-item {
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.entry-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.entry-schema {
  font-size: 0.75rem;
  color: #6b7280;
  font-weight: 600;
}

.entry-values {
  font-size: 0.75rem;
  background: #f9fafb;
  padding: 0.5rem;
  border-radius: 0.25rem;
  overflow-x: auto;
  margin: 0;
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
