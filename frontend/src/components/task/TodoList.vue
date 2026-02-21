<script setup lang="ts">
import type { components } from '@/api/schema'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import { ref } from 'vue'

type TodoResponse = components['schemas']['TodoResponse']

defineProps<{
  todos: TodoResponse[]
  loading: boolean
}>()

const emit = defineEmits<{
  toggle: [todoId: string, currentStatus: string]
  delete: [todoId: string]
  create: [title: string, description: string]
}>()

const newTitle = ref('')
const newDescription = ref('')

function handleCreate() {
  if (!newTitle.value.trim()) return
  emit('create', newTitle.value.trim(), newDescription.value.trim())
  newTitle.value = ''
  newDescription.value = ''
}
</script>

<template>
  <div>
    <form class="create-form" @submit.prevent="handleCreate">
      <BaseInput v-model="newTitle" label="Title" placeholder="New todo" />
      <BaseInput v-model="newDescription" label="Description" placeholder="Optional description" />
      <BaseButton :disabled="loading">Add Todo</BaseButton>
    </form>

    <ul class="todo-list">
      <li v-for="todo in todos" :key="todo.id" class="todo-item">
        <div class="todo-left">
          <input
            type="checkbox"
            :checked="todo.status === 'completed'"
            @change="emit('toggle', todo.id, todo.status)"
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
        <BaseButton variant="danger" :disabled="loading" @click="emit('delete', todo.id)">
          Delete
        </BaseButton>
      </li>
    </ul>
    <p v-if="todos.length === 0 && !loading" class="empty-text">No todos yet.</p>
  </div>
</template>

<style scoped>
.create-form {
  display: flex;
  gap: 0.75rem;
  align-items: flex-end;
  margin-bottom: 1rem;
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

.empty-text {
  color: #9ca3af;
  font-size: 0.875rem;
}
</style>
