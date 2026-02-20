<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useMyTodosStore } from '@/stores/myTodos'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import type { components } from '@/api/schema'

type TodoStatus = components['schemas']['TodoStatus']

const router = useRouter()
const store = useMyTodosStore()

const filterStatus = ref<TodoStatus | ''>('')

const statusOptions: TodoStatus[] = ['open', 'completed']

onMounted(async () => {
  await store.fetchMyTodos()
})

async function handleFilter() {
  await store.fetchMyTodos(filterStatus.value || undefined)
}

function goToTask(projectId: string, taskId: string) {
  router.push({ name: 'task-detail', params: { projectId, taskId } })
}

// Group items by project
function groupedByProject() {
  const groups: Record<string, { projectName: string; items: typeof store.items }> = {}
  for (const item of store.items) {
    let group = groups[item.projectId]
    if (!group) {
      group = { projectName: item.projectName, items: [] }
      groups[item.projectId] = group
    }
    group.items.push(item)
  }
  return Object.entries(groups)
}
</script>

<template>
  <div class="my-todos-view">
    <h1>My Todos</h1>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

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
        <BaseButton variant="secondary" :disabled="store.loading">Filter</BaseButton>
      </form>
    </BaseCard>

    <!-- Grouped Todos -->
    <template v-if="groupedByProject().length > 0">
      <BaseCard v-for="[projectId, group] in groupedByProject()" :key="projectId" :title="group.projectName">
        <ul class="todo-list">
          <li
            v-for="item in group.items"
            :key="item.id"
            class="todo-item"
            @click="goToTask(item.projectId, item.taskId)"
          >
            <div class="todo-info">
              <div class="todo-header">
                <strong>{{ item.title }}</strong>
                <span :class="['status-badge', `status-${item.status}`]">{{ item.status }}</span>
              </div>
              <span v-if="item.description" class="todo-sub">{{ item.description }}</span>
              <span class="todo-meta">
                Task: {{ item.taskName }}
                <template v-if="item.dueDate"> · Due {{ new Date(item.dueDate).toLocaleDateString() }}</template>
              </span>
            </div>
          </li>
        </ul>
      </BaseCard>
    </template>

    <BaseCard v-if="store.items.length === 0 && !store.loading" title="No Todos">
      <p class="empty-text">You have no assigned todos.</p>
    </BaseCard>
  </div>
</template>

<style scoped>
.my-todos-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 700px;
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

.todo-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.todo-item {
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  cursor: pointer;
}

.todo-item:hover {
  background: #f9fafb;
}

.todo-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.todo-header {
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

.status-open {
  background: #fef3c7;
  color: #92400e;
}

.status-completed {
  background: #d1fae5;
  color: #065f46;
}

.todo-sub {
  font-size: 0.75rem;
  color: #6b7280;
}

.todo-meta {
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
