<script setup lang="ts">
import { onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useContactStore } from '@/stores/contact'
import BaseCard from '@/components/base/BaseCard.vue'

const route = useRoute()
const store = useContactStore()

const projectId = route.params.projectId as string

onMounted(async () => {
  await store.fetchContacts(projectId)
})
</script>

<template>
  <div class="project-contacts-view">
    <h1>Project Contacts</h1>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <BaseCard title="Contacts">
      <ul class="contact-list">
        <li v-for="contact in store.contacts" :key="contact.id" class="contact-item">
          <div class="contact-info">
            <strong>{{ contact.name }}</strong>
            <span class="contact-email">{{ contact.email }}</span>
          </div>
        </li>
      </ul>
      <p v-if="store.contacts.length === 0 && !store.loading" class="empty-text">
        No contacts associated with this project.
      </p>
    </BaseCard>
  </div>
</template>

<style scoped>
.project-contacts-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 700px;
}

.contact-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.contact-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.contact-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.contact-email {
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
