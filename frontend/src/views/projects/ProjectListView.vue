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

const orgId = route.params.orgId as string
const showCreateForm = ref(false)
const newProjectName = ref('')
const newProjectDescription = ref('')

const showCopyForm = ref(false)
const copySourceId = ref('')
const copyProjectName = ref('')
const copyProjectDescription = ref('')

onMounted(async () => {
  await store.fetchProjects(orgId)
})

function openCreateForm() {
  showCopyForm.value = false
  showCreateForm.value = true
}

function openCopyForm() {
  showCreateForm.value = false
  showCopyForm.value = true
}

async function handleCreate() {
  if (!newProjectName.value.trim()) return
  const project = await store.createProject(
    orgId,
    newProjectName.value.trim(),
    newProjectDescription.value.trim() || undefined,
  )
  if (project) {
    newProjectName.value = ''
    newProjectDescription.value = ''
    showCreateForm.value = false
    await store.fetchProjects(orgId)
  }
}

async function handleCopy() {
  if (!copySourceId.value || !copyProjectName.value.trim()) return
  const project = await store.copyProject(
    orgId,
    copySourceId.value,
    copyProjectName.value.trim(),
    copyProjectDescription.value.trim() || undefined,
  )
  if (project) {
    copySourceId.value = ''
    copyProjectName.value = ''
    copyProjectDescription.value = ''
    showCopyForm.value = false
    await store.fetchProjects(orgId)
  }
}

function goToProject(projectId: string) {
  router.push({ name: 'project-dashboard', params: { projectId } })
}
</script>

<template>
  <div class="project-list-view">
    <div class="view-header">
      <h1>Projects</h1>
      <div v-if="!showCreateForm && !showCopyForm" class="header-actions">
        <BaseButton @click="openCreateForm">
          New Project
        </BaseButton>
        <BaseButton variant="secondary" @click="openCopyForm">
          Copy Project
        </BaseButton>
      </div>
    </div>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <BaseCard v-if="showCreateForm" title="Create Project">
      <form class="create-form" @submit.prevent="handleCreate">
        <BaseInput v-model="newProjectName" label="Name" placeholder="Project name" />
        <BaseInput
          v-model="newProjectDescription"
          label="Description"
          placeholder="Optional description"
        />
        <div class="form-actions">
          <BaseButton :disabled="store.loading">Create</BaseButton>
          <BaseButton variant="secondary" @click="showCreateForm = false">Cancel</BaseButton>
        </div>
      </form>
    </BaseCard>

    <BaseCard v-if="showCopyForm" title="Copy Project">
      <form class="copy-form" @submit.prevent="handleCopy">
        <label class="select-label">
          Source Project
          <select v-model="copySourceId" class="source-select">
            <option value="" disabled>Select a project</option>
            <option v-for="project in store.projects" :key="project.id" :value="project.id">
              {{ project.name }}
            </option>
          </select>
        </label>
        <BaseInput v-model="copyProjectName" label="New Project Name" placeholder="Project name" />
        <BaseInput
          v-model="copyProjectDescription"
          label="Description"
          placeholder="Optional description"
        />
        <div class="form-actions">
          <BaseButton :disabled="store.loading || !copySourceId">Copy</BaseButton>
          <BaseButton variant="secondary" @click="showCopyForm = false">Cancel</BaseButton>
        </div>
      </form>
    </BaseCard>

    <div v-if="store.projects.length === 0 && !store.loading" class="empty-state">
      <p>No projects yet. Create your first project to get started.</p>
    </div>

    <div class="project-grid">
      <BaseCard
        v-for="project in store.projects"
        :key="project.id"
        :title="project.name"
      >
        <p v-if="project.description" class="project-description">{{ project.description }}</p>
        <span class="project-status" :class="`status-${project.status}`">{{ project.status }}</span>
        <div class="project-actions">
          <BaseButton variant="secondary" @click="goToProject(project.id)">Open</BaseButton>
        </div>
      </BaseCard>
    </div>
  </div>
</template>

<style scoped>
.project-list-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.view-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-actions {
  display: flex;
  gap: 0.5rem;
}

.create-form,
.copy-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.form-actions {
  display: flex;
  gap: 0.75rem;
}

.select-label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.875rem;
  font-weight: 500;
  color: #374151;
}

.source-select {
  padding: 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
}

.empty-state {
  color: #9ca3af;
  text-align: center;
  padding: 2rem;
}

.project-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 1rem;
}

.project-description {
  color: #6b7280;
  font-size: 0.875rem;
  margin: 0 0 0.5rem;
}

.project-status {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
  font-size: 0.75rem;
  font-weight: 500;
  margin-bottom: 0.75rem;
}

.status-preparing { background: #fef3c7; color: #92400e; }
.status-active { background: #d1fae5; color: #065f46; }
.status-completed { background: #dbeafe; color: #1e40af; }
.status-archived { background: #f3f4f6; color: #6b7280; }

.project-actions {
  display: flex;
  gap: 0.5rem;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
}
</style>
