<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useContactStore } from '@/stores/contact'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const route = useRoute()
const store = useContactStore()

const orgId = route.params.orgId as string
const searchQuery = ref('')
const newName = ref('')
const newEmail = ref('')
const mergeTargetId = ref('')
const mergeSourceIds = ref('')
const editingContactId = ref<string | null>(null)
const editName = ref('')
const editEmail = ref('')

onMounted(async () => {
  await store.fetchContacts(orgId)
})

async function handleSearch() {
  await store.fetchContacts(orgId, searchQuery.value || undefined)
}

async function handleCreate() {
  if (!newName.value.trim() || !newEmail.value.trim()) return
  const result = await store.createContact(orgId, newName.value.trim(), newEmail.value.trim())
  if (result) {
    newName.value = ''
    newEmail.value = ''
  }
}

function startEdit(contact: { id: string; name: string; email: string }) {
  editingContactId.value = contact.id
  editName.value = contact.name
  editEmail.value = contact.email
}

function cancelEdit() {
  editingContactId.value = null
}

async function handleUpdate(contactId: string) {
  await store.updateContact(orgId, contactId, {
    name: editName.value,
    email: editEmail.value,
  })
  editingContactId.value = null
}

async function handleDelete(contactId: string) {
  await store.deleteContact(orgId, contactId)
}

async function handleMerge() {
  if (!mergeTargetId.value.trim() || !mergeSourceIds.value.trim()) return
  const sourceIds = mergeSourceIds.value.split(',').map((s) => s.trim()).filter(Boolean)
  const result = await store.mergeContacts(orgId, sourceIds, mergeTargetId.value.trim())
  if (result) {
    mergeTargetId.value = ''
    mergeSourceIds.value = ''
  }
}
</script>

<template>
  <div class="contacts-view">
    <h1>Contacts</h1>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <BaseCard title="Search">
      <form class="search-form" @submit.prevent="handleSearch">
        <BaseInput v-model="searchQuery" label="Search" placeholder="Search by name or email" />
        <BaseButton :disabled="store.loading">Search</BaseButton>
      </form>
    </BaseCard>

    <BaseCard title="Create Contact">
      <form class="create-form" @submit.prevent="handleCreate">
        <BaseInput v-model="newName" label="Name" placeholder="Contact name" />
        <BaseInput v-model="newEmail" label="Email" type="email" placeholder="contact@example.com" />
        <BaseButton :disabled="store.loading">Create</BaseButton>
      </form>
    </BaseCard>

    <BaseCard title="Contact List">
      <ul class="contact-list">
        <li v-for="contact in store.contacts" :key="contact.id" class="contact-item">
          <template v-if="editingContactId === contact.id">
            <form class="edit-form" @submit.prevent="handleUpdate(contact.id)">
              <BaseInput v-model="editName" label="Name" />
              <BaseInput v-model="editEmail" label="Email" type="email" />
              <div class="edit-actions">
                <BaseButton :disabled="store.loading">Save</BaseButton>
                <BaseButton variant="secondary" @click="cancelEdit">Cancel</BaseButton>
              </div>
            </form>
          </template>
          <template v-else>
            <div class="contact-info">
              <strong>{{ contact.name }}</strong>
              <span class="contact-email">{{ contact.email }}</span>
              <span v-if="contact.mergedIntoId" class="merged-badge">Merged</span>
            </div>
            <div class="contact-actions">
              <BaseButton
                variant="secondary"
                :disabled="store.loading"
                @click="startEdit(contact)"
              >
                Edit
              </BaseButton>
              <BaseButton
                variant="danger"
                :disabled="store.loading"
                @click="handleDelete(contact.id)"
              >
                Delete
              </BaseButton>
            </div>
          </template>
        </li>
      </ul>
      <p v-if="store.contacts.length === 0 && !store.loading" class="empty-text">
        No contacts yet.
      </p>
    </BaseCard>

    <BaseCard title="Merge Contacts">
      <form class="merge-form" @submit.prevent="handleMerge">
        <BaseInput
          v-model="mergeSourceIds"
          label="Source IDs (comma-separated)"
          placeholder="id1, id2, ..."
        />
        <BaseInput v-model="mergeTargetId" label="Target ID" placeholder="Target contact ID" />
        <BaseButton :disabled="store.loading">Merge</BaseButton>
      </form>
    </BaseCard>
  </div>
</template>

<style scoped>
.contacts-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 700px;
}

.search-form,
.create-form,
.merge-form {
  display: flex;
  gap: 0.75rem;
  align-items: flex-end;
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

.contact-actions {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.merged-badge {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
  font-size: 0.625rem;
  font-weight: 500;
  background: #fef3c7;
  color: #92400e;
  width: fit-content;
}

.edit-form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: 100%;
}

.edit-actions {
  display: flex;
  gap: 0.5rem;
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
