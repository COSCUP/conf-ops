<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useMemberTagStore } from '@/stores/memberTag'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseCard from '@/components/base/BaseCard.vue'

const route = useRoute()
const store = useMemberTagStore()

const projectId = route.params.projectId as string
const newTagName = ref('')
const newTagDescription = ref('')
const selectedTagId = ref<string | null>(null)
const assignMemberId = ref('')
const assignContactId = ref('')
const editingTagId = ref<string | null>(null)
const editTagName = ref('')
const editTagDescription = ref('')

onMounted(async () => {
  await store.fetchTags(projectId)
})

async function handleCreateTag() {
  if (!newTagName.value.trim()) return
  const result = await store.createTag(
    projectId,
    newTagName.value.trim(),
    newTagDescription.value.trim() || undefined,
  )
  if (result) {
    newTagName.value = ''
    newTagDescription.value = ''
  }
}

function startEditTag(tag: { id: string; name: string; description: string | null }) {
  editingTagId.value = tag.id
  editTagName.value = tag.name
  editTagDescription.value = tag.description ?? ''
}

function cancelEditTag() {
  editingTagId.value = null
}

async function handleUpdateTag(tagId: string) {
  await store.updateTag(projectId, tagId, {
    name: editTagName.value,
    description: editTagDescription.value || null,
  })
  editingTagId.value = null
}

async function handleDeleteTag(tagId: string) {
  await store.deleteTag(projectId, tagId)
  if (selectedTagId.value === tagId) {
    selectedTagId.value = null
  }
}

async function handleSelectTag(tagId: string) {
  selectedTagId.value = tagId
  await store.getTagDetail(projectId, tagId)
}

async function handleAssign() {
  if (!selectedTagId.value) return
  if (!assignMemberId.value.trim() && !assignContactId.value.trim()) return
  const result = await store.assignTag(
    projectId,
    selectedTagId.value,
    assignMemberId.value.trim() || undefined,
    assignContactId.value.trim() || undefined,
  )
  if (result) {
    assignMemberId.value = ''
    assignContactId.value = ''
  }
}

async function handleRemoveAssignment(tagId: string, assignmentId: string) {
  await store.removeAssignment(projectId, tagId, assignmentId)
}
</script>

<template>
  <div class="member-tags-view">
    <h1>Member Tags</h1>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <BaseCard title="Create Tag">
      <form class="create-form" @submit.prevent="handleCreateTag">
        <BaseInput v-model="newTagName" label="Name" placeholder="Tag name" />
        <BaseInput v-model="newTagDescription" label="Description" placeholder="Optional description" />
        <BaseButton :disabled="store.loading">Create</BaseButton>
      </form>
    </BaseCard>

    <BaseCard title="Tags">
      <ul class="tag-list">
        <li v-for="tag in store.tags" :key="tag.id" class="tag-item">
          <template v-if="editingTagId === tag.id">
            <form class="edit-form" @submit.prevent="handleUpdateTag(tag.id)">
              <BaseInput v-model="editTagName" label="Name" />
              <BaseInput v-model="editTagDescription" label="Description" />
              <div class="edit-actions">
                <BaseButton :disabled="store.loading">Save</BaseButton>
                <BaseButton variant="secondary" @click="cancelEditTag">Cancel</BaseButton>
              </div>
            </form>
          </template>
          <template v-else>
            <div
              class="tag-info"
              :class="{ selected: selectedTagId === tag.id }"
              @click="handleSelectTag(tag.id)"
            >
              <strong>{{ tag.name }}</strong>
              <span v-if="tag.description" class="tag-description">{{ tag.description }}</span>
              <span class="tag-counts">
                {{ tag.memberCount }} members, {{ tag.contactCount }} contacts
              </span>
            </div>
            <div class="tag-actions">
              <BaseButton
                variant="secondary"
                :disabled="store.loading"
                @click="startEditTag(tag)"
              >
                Edit
              </BaseButton>
              <BaseButton
                variant="danger"
                :disabled="store.loading"
                @click="handleDeleteTag(tag.id)"
              >
                Delete
              </BaseButton>
            </div>
          </template>
        </li>
      </ul>
      <p v-if="store.tags.length === 0 && !store.loading" class="empty-text">
        No tags yet.
      </p>
    </BaseCard>

    <BaseCard v-if="selectedTagId && store.currentTagDetail" title="Tag Detail">
      <h3>{{ store.currentTagDetail.tag.name }}</h3>

      <form class="assign-form" @submit.prevent="handleAssign">
        <BaseInput v-model="assignMemberId" label="Member ID" placeholder="Member ID (optional)" />
        <BaseInput v-model="assignContactId" label="Contact ID" placeholder="Contact ID (optional)" />
        <BaseButton :disabled="store.loading">Assign</BaseButton>
      </form>

      <div v-if="store.currentTagDetail.members.length > 0" class="assigned-section">
        <h4>Assigned Members</h4>
        <ul class="assigned-list">
          <li v-for="m in store.currentTagDetail.members" :key="m.assignmentId" class="assigned-item">
            <div class="assigned-info">
              <strong>{{ m.name }}</strong>
              <span class="assigned-email">{{ m.email }}</span>
            </div>
            <BaseButton
              variant="danger"
              :disabled="store.loading"
              @click="handleRemoveAssignment(selectedTagId!, m.assignmentId)"
            >
              Remove
            </BaseButton>
          </li>
        </ul>
      </div>

      <div v-if="store.currentTagDetail.contacts.length > 0" class="assigned-section">
        <h4>Assigned Contacts</h4>
        <ul class="assigned-list">
          <li v-for="c in store.currentTagDetail.contacts" :key="c.assignmentId" class="assigned-item">
            <div class="assigned-info">
              <strong>{{ c.name }}</strong>
              <span class="assigned-email">{{ c.email }}</span>
            </div>
            <BaseButton
              variant="danger"
              :disabled="store.loading"
              @click="handleRemoveAssignment(selectedTagId!, c.assignmentId)"
            >
              Remove
            </BaseButton>
          </li>
        </ul>
      </div>
    </BaseCard>
  </div>
</template>

<style scoped>
.member-tags-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 700px;
}

.create-form,
.assign-form {
  display: flex;
  gap: 0.75rem;
  align-items: flex-end;
  margin-bottom: 1rem;
}

.tag-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.tag-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.tag-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  cursor: pointer;
}

.tag-info.selected {
  color: #2563eb;
}

.tag-description {
  font-size: 0.75rem;
  color: #6b7280;
}

.tag-counts {
  font-size: 0.75rem;
  color: #9ca3af;
}

.tag-actions {
  display: flex;
  gap: 0.5rem;
  align-items: center;
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

.assigned-section {
  margin-top: 1rem;
}

.assigned-list {
  list-style: none;
  padding: 0;
  margin: 0.5rem 0 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.assigned-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.assigned-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.assigned-email {
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
