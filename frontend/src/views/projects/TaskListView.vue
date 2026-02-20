<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTaskStore } from '@/stores/task'
import { useTaskTemplateStore } from '@/stores/taskTemplate'
import { useMemberTagStore } from '@/stores/memberTag'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import type { components } from '@/api/schema'

type TaskStatus = components['schemas']['TaskStatus']

const route = useRoute()
const router = useRouter()
const taskStore = useTaskStore()
const templateStore = useTaskTemplateStore()
const tagStore = useMemberTagStore()

const projectId = route.params.projectId as string

// Filters
const filterStatus = ref<TaskStatus | ''>('')
const filterTagId = ref('')

// Create task
const newTaskName = ref('')
const newTaskDescription = ref('')
const selectedTemplateId = ref('')
const selectedOwnerTagId = ref('')

const statusOptions: TaskStatus[] = ['pending', 'in_progress', 'completed', 'cancelled']

onMounted(async () => {
  await Promise.all([
    taskStore.fetchTasks(projectId),
    templateStore.fetchTemplates(projectId),
    tagStore.fetchTags(projectId),
  ])
})

async function handleFilter() {
  await taskStore.fetchTasks(
    projectId,
    filterStatus.value || undefined,
    filterTagId.value || undefined,
  )
}

async function handleCreate() {
  if (!newTaskName.value.trim() || !selectedTemplateId.value || !selectedOwnerTagId.value) return
  const result = await taskStore.createTask(
    projectId,
    selectedTemplateId.value,
    selectedOwnerTagId.value,
    newTaskName.value.trim(),
    newTaskDescription.value.trim() || undefined,
  )
  if (result) {
    newTaskName.value = ''
    newTaskDescription.value = ''
    selectedTemplateId.value = ''
    selectedOwnerTagId.value = ''
  }
}

function handleViewTask(taskId: string) {
  router.push({ name: 'task-detail', params: { projectId, taskId } })
}

function statusClass(status: string): string {
  return `status-${status}`
}
</script>

<template>
  <div class="task-list-view">
    <h1>Tasks</h1>

    <p v-if="taskStore.error" class="error-message">{{ taskStore.error }}</p>

    <!-- Create Task -->
    <BaseCard title="Create Task">
      <form class="create-form" @submit.prevent="handleCreate">
        <BaseInput v-model="newTaskName" label="Name" placeholder="Task name" />
        <BaseInput
          v-model="newTaskDescription"
          label="Description"
          placeholder="Optional description"
        />
        <div class="field-select">
          <label class="field-label">Template</label>
          <select v-model="selectedTemplateId">
            <option value="" disabled>Select template</option>
            <option v-for="t in templateStore.templates" :key="t.id" :value="t.id">
              {{ t.name }}
            </option>
          </select>
        </div>
        <div class="field-select">
          <label class="field-label">Owner Tag</label>
          <select v-model="selectedOwnerTagId">
            <option value="" disabled>Select tag</option>
            <option v-for="tag in tagStore.tags" :key="tag.id" :value="tag.id">
              {{ tag.name }}
            </option>
          </select>
        </div>
        <BaseButton :disabled="taskStore.loading">Create</BaseButton>
      </form>
    </BaseCard>

    <!-- Filters -->
    <BaseCard title="Filters">
      <form class="filter-form" @submit.prevent="handleFilter">
        <div class="field-select">
          <label class="field-label">Status</label>
          <select v-model="filterStatus">
            <option value="">All</option>
            <option v-for="s in statusOptions" :key="s" :value="s">{{ s }}</option>
          </select>
        </div>
        <div class="field-select">
          <label class="field-label">Tag</label>
          <select v-model="filterTagId">
            <option value="">All</option>
            <option v-for="tag in tagStore.tags" :key="tag.id" :value="tag.id">
              {{ tag.name }}
            </option>
          </select>
        </div>
        <BaseButton variant="secondary" :disabled="taskStore.loading">Filter</BaseButton>
      </form>
    </BaseCard>

    <!-- Task List -->
    <BaseCard title="Task List">
      <ul class="task-list">
        <li
          v-for="task in taskStore.tasks"
          :key="task.id"
          class="task-item"
          @click="handleViewTask(task.id)"
        >
          <div class="task-info">
            <div class="task-header">
              <strong>{{ task.name }}</strong>
              <span :class="['status-badge', statusClass(task.status)]">{{ task.status }}</span>
            </div>
            <span v-if="task.description" class="task-description">{{ task.description }}</span>
            <span class="task-meta">
              Created {{ new Date(task.createdAt).toLocaleDateString() }}
            </span>
          </div>
        </li>
      </ul>
      <p v-if="taskStore.tasks.length === 0 && !taskStore.loading" class="empty-text">
        No tasks yet.
      </p>
    </BaseCard>
  </div>
</template>

<style scoped>
.task-list-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 700px;
}

.create-form {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  max-width: 400px;
}

.filter-form {
  display: flex;
  gap: 0.75rem;
  align-items: flex-end;
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

.task-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.task-item {
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  cursor: pointer;
}

.task-item:hover {
  background: #f9fafb;
}

.task-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.task-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
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

.task-description {
  font-size: 0.75rem;
  color: #6b7280;
}

.task-meta {
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
