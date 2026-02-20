<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const route = useRoute()
const router = useRouter()
const store = useProjectStore()

const projectId = route.params.projectId as string
const editName = ref('')
const editDescription = ref('')

onMounted(async () => {
  await store.fetchProject(projectId)
  if (store.currentProject) {
    editName.value = store.currentProject.name
    editDescription.value = store.currentProject.description ?? ''
  }
})

async function handleUpdate() {
  await store.updateProject(projectId, {
    name: editName.value,
    description: editDescription.value || null,
  })
}

async function handleStatusChange(newStatus: string) {
  await store.updateProjectStatus(projectId, newStatus)
}

async function handleDelete() {
  await store.deleteProject(projectId)
  if (!store.error && store.currentProject === null) {
    await router.push({ name: 'organizations' })
  }
}
</script>

<template>
  <div class="project-settings-view">
    <h1>Project Settings</h1>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <BaseCard title="General">
      <form class="settings-form" @submit.prevent="handleUpdate">
        <BaseInput v-model="editName" label="Name" />
        <BaseInput v-model="editDescription" label="Description" />
        <BaseButton :disabled="store.loading">Save</BaseButton>
      </form>
    </BaseCard>

    <BaseCard v-if="store.currentProject" title="Status">
      <p class="current-status">
        Current:
        <span class="project-status" :class="`status-${store.currentProject.status}`">
          {{ store.currentProject.status }}
        </span>
      </p>
      <div class="status-actions">
        <BaseButton
          v-if="store.currentProject.status === 'preparing'"
          variant="secondary"
          :disabled="store.loading"
          @click="handleStatusChange('active')"
        >
          Activate
        </BaseButton>
        <BaseButton
          v-if="store.currentProject.status === 'active'"
          variant="secondary"
          :disabled="store.loading"
          @click="handleStatusChange('completed')"
        >
          Complete
        </BaseButton>
        <BaseButton
          v-if="store.currentProject.status !== 'archived'"
          variant="secondary"
          :disabled="store.loading"
          @click="handleStatusChange('archived')"
        >
          Archive
        </BaseButton>
      </div>
    </BaseCard>

    <BaseCard title="Danger Zone">
      <p class="danger-text">Deleting this project is permanent and cannot be undone.</p>
      <BaseButton variant="danger" :disabled="store.loading" @click="handleDelete">
        Delete Project
      </BaseButton>
    </BaseCard>
  </div>
</template>

<style scoped>
.project-settings-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 700px;
}

.settings-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.current-status {
  margin: 0 0 0.75rem;
}

.project-status {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
  font-size: 0.75rem;
  font-weight: 500;
}

.status-preparing { background: #fef3c7; color: #92400e; }
.status-active { background: #d1fae5; color: #065f46; }
.status-completed { background: #dbeafe; color: #1e40af; }
.status-archived { background: #f3f4f6; color: #6b7280; }

.status-actions {
  display: flex;
  gap: 0.5rem;
}

.danger-text {
  color: #ef4444;
  margin: 0 0 1rem;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
}
</style>
