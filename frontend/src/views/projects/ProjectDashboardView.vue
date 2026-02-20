<script setup lang="ts">
import { onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const route = useRoute()
const router = useRouter()
const store = useProjectStore()

const projectId = route.params.projectId as string

onMounted(async () => {
  await store.fetchProject(projectId)
})

function goToSettings() {
  router.push({ name: 'project-settings', params: { projectId } })
}
</script>

<template>
  <div class="project-dashboard">
    <template v-if="store.currentProject">
      <div class="view-header">
        <h1>{{ store.currentProject.name }}</h1>
        <BaseButton variant="secondary" @click="goToSettings">Settings</BaseButton>
      </div>

      <p v-if="store.currentProject.description" class="project-description">
        {{ store.currentProject.description }}
      </p>

      <BaseCard title="Status">
        <span class="project-status" :class="`status-${store.currentProject.status}`">
          {{ store.currentProject.status }}
        </span>
      </BaseCard>

      <BaseCard title="Overview">
        <p class="placeholder-text">Project dashboard content will be added in future phases.</p>
      </BaseCard>
    </template>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>
  </div>
</template>

<style scoped>
.project-dashboard {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.view-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.project-description {
  color: #6b7280;
  margin: 0;
}

.project-status {
  display: inline-block;
  padding: 0.25rem 0.75rem;
  border-radius: 9999px;
  font-size: 0.875rem;
  font-weight: 500;
}

.status-preparing { background: #fef3c7; color: #92400e; }
.status-active { background: #d1fae5; color: #065f46; }
.status-completed { background: #dbeafe; color: #1e40af; }
.status-archived { background: #f3f4f6; color: #6b7280; }

.placeholder-text {
  color: #9ca3af;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
}
</style>
