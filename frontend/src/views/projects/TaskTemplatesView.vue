<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTaskTemplateStore } from '@/stores/taskTemplate'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const route = useRoute()
const router = useRouter()
const store = useTaskTemplateStore()

const projectId = route.params.projectId as string
const newName = ref('')
const newDescription = ref('')

onMounted(async () => {
  await store.fetchTemplates(projectId)
})

async function handleCreate() {
  if (!newName.value.trim()) return
  const result = await store.createTemplate(
    projectId,
    newName.value.trim(),
    newDescription.value.trim() || undefined,
  )
  if (result) {
    newName.value = ''
    newDescription.value = ''
  }
}

function handleEdit(templateId: string) {
  router.push({
    name: 'task-template-editor',
    params: { projectId, templateId },
  })
}

async function handleDelete(templateId: string) {
  await store.deleteTemplate(projectId, templateId)
}
</script>

<template>
  <div class="task-templates-view">
    <h1>Task Templates</h1>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <BaseCard title="Create Template">
      <form class="create-form" @submit.prevent="handleCreate">
        <BaseInput v-model="newName" label="Name" placeholder="Template name" />
        <BaseInput
          v-model="newDescription"
          label="Description"
          placeholder="Optional description"
        />
        <BaseButton :disabled="store.loading">Create</BaseButton>
      </form>
    </BaseCard>

    <BaseCard title="Templates">
      <ul class="template-list">
        <li v-for="tmpl in store.templates" :key="tmpl.id" class="template-item">
          <div class="template-info" @click="handleEdit(tmpl.id)">
            <strong>{{ tmpl.name }}</strong>
            <span v-if="tmpl.description" class="template-description">
              {{ tmpl.description }}
            </span>
            <span class="template-meta">
              Created {{ new Date(tmpl.createdAt).toLocaleDateString() }}
            </span>
          </div>
          <div class="template-actions">
            <BaseButton variant="secondary" :disabled="store.loading" @click="handleEdit(tmpl.id)">
              Edit
            </BaseButton>
            <BaseButton variant="danger" :disabled="store.loading" @click="handleDelete(tmpl.id)">
              Delete
            </BaseButton>
          </div>
        </li>
      </ul>
      <p v-if="store.templates.length === 0 && !store.loading" class="empty-text">
        No task templates yet.
      </p>
    </BaseCard>
  </div>
</template>

<style scoped>
.task-templates-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 700px;
}

.create-form {
  display: flex;
  gap: 0.75rem;
  align-items: flex-end;
  margin-bottom: 1rem;
}

.template-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.template-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.template-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  cursor: pointer;
}

.template-description {
  font-size: 0.75rem;
  color: #6b7280;
}

.template-meta {
  font-size: 0.75rem;
  color: #9ca3af;
}

.template-actions {
  display: flex;
  gap: 0.5rem;
  align-items: center;
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
