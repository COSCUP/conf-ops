<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useMemoryStore } from '@/stores/memory'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import BaseInput from '@/components/base/BaseInput.vue'

const route = useRoute()
const store = useMemoryStore()

const orgId = route.params.orgId as string

const showCreateForm = ref(false)
const newTitle = ref('')
const newContent = ref('')
const editingDocumentId = ref<string | null>(null)
const editTitle = ref('')
const editContent = ref('')
const expandedVersionsId = ref<string | null>(null)
const previewMode = ref<'edit' | 'preview'>('edit')
const editPreviewMode = ref<'edit' | 'preview'>('edit')

onMounted(async () => {
  await store.listLibraryDocuments('organization', orgId)
})

async function handleCreate() {
  if (!newTitle.value.trim() || !newContent.value.trim()) return
  const result = await store.createLibraryDocument({
    title: newTitle.value.trim(),
    content: newContent.value.trim(),
    scopeType: 'organization',
    scopeId: orgId,
  })
  if (result) {
    newTitle.value = ''
    newContent.value = ''
    showCreateForm.value = false
  }
}

function startEdit(documentId: string, title: string, content: string) {
  editingDocumentId.value = documentId
  editTitle.value = title
  editContent.value = content
}

function cancelEdit() {
  editingDocumentId.value = null
  editTitle.value = ''
  editContent.value = ''
}

async function handleUpdate(documentId: string) {
  if (!editContent.value.trim()) return
  const result = await store.updateLibraryDocument(documentId, {
    title: editTitle.value.trim() || null,
    content: editContent.value.trim(),
  })
  if (result) {
    editingDocumentId.value = null
  }
}

async function handleDelete(documentId: string) {
  await store.deleteLibraryDocument(documentId)
  if (expandedVersionsId.value === documentId) {
    expandedVersionsId.value = null
  }
}

async function toggleVersions(documentId: string) {
  if (expandedVersionsId.value === documentId) {
    expandedVersionsId.value = null
    return
  }
  expandedVersionsId.value = documentId
  await store.listLibraryDocumentVersions(documentId)
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString()
}

function truncate(text: string, length = 200): string {
  return text.length > length ? text.slice(0, length) + '...' : text
}

function renderMarkdownPreview(text: string): string {
  return text
    .split('\n')
    .map((line) => {
      // Escape HTML entities
      const escaped = line
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
      // Headers
      if (escaped.startsWith('### ')) return `<h4>${escaped.slice(4)}</h4>`
      if (escaped.startsWith('## ')) return `<h3>${escaped.slice(3)}</h3>`
      if (escaped.startsWith('# ')) return `<h2>${escaped.slice(2)}</h2>`
      // List items
      if (escaped.startsWith('- ')) return `<li>${escaped.slice(2)}</li>`
      // Empty line → paragraph break
      if (escaped.trim() === '') return '<br>'
      return `<p>${escaped}</p>`
    })
    .join('\n')
}

const newContentPreview = computed(() => renderMarkdownPreview(newContent.value))
const editContentPreview = computed(() => renderMarkdownPreview(editContent.value))
</script>

<template>
  <div class="library-document-view">
    <h1>Library Documents</h1>

    <p v-if="store.error" class="error-message">{{ store.error }}</p>

    <div class="view-actions">
      <BaseButton @click="showCreateForm = !showCreateForm">
        {{ showCreateForm ? 'Cancel' : 'New Document' }}
      </BaseButton>
    </div>

    <BaseCard v-if="showCreateForm" title="Create Library Document">
      <form class="form-vertical" @submit.prevent="handleCreate">
        <BaseInput v-model="newTitle" label="Title" placeholder="Document title" />
        <div class="form-field">
          <label class="form-label">Content (Markdown)</label>
          <div class="editor-tabs">
            <button
              type="button"
              :class="['tab-btn', { active: previewMode === 'edit' }]"
              @click="previewMode = 'edit'"
            >
              Edit
            </button>
            <button
              type="button"
              :class="['tab-btn', { active: previewMode === 'preview' }]"
              @click="previewMode = 'preview'"
            >
              Preview
            </button>
          </div>
          <textarea
            v-if="previewMode === 'edit'"
            v-model="newContent"
            class="form-textarea form-textarea--md"
            placeholder="Document content (Markdown supported)..."
            rows="8"
          />
          <div
            v-else
            class="markdown-preview"
            v-html="newContentPreview"
          />
        </div>
        <BaseButton :disabled="store.loading">Create</BaseButton>
      </form>
    </BaseCard>

    <div v-if="store.libraryDocuments.length > 0" class="document-list">
      <div
        v-for="doc in store.libraryDocuments"
        :key="doc.id"
        class="document-item"
      >
        <template v-if="editingDocumentId === doc.id">
          <BaseCard :title="'Editing: ' + doc.title">
            <form class="form-vertical" @submit.prevent="handleUpdate(doc.id)">
              <BaseInput v-model="editTitle" label="Title" />
              <div class="form-field">
                <label class="form-label">Content (Markdown)</label>
                <div class="editor-tabs">
                  <button
                    type="button"
                    :class="['tab-btn', { active: editPreviewMode === 'edit' }]"
                    @click="editPreviewMode = 'edit'"
                  >
                    Edit
                  </button>
                  <button
                    type="button"
                    :class="['tab-btn', { active: editPreviewMode === 'preview' }]"
                    @click="editPreviewMode = 'preview'"
                  >
                    Preview
                  </button>
                </div>
                <textarea
                  v-if="editPreviewMode === 'edit'"
                  v-model="editContent"
                  class="form-textarea form-textarea--md"
                  rows="8"
                />
                <div
                  v-else
                  class="markdown-preview"
                  v-html="editContentPreview"
                />
              </div>
              <div class="form-actions">
                <BaseButton :disabled="store.loading">Save</BaseButton>
                <BaseButton variant="secondary" @click="cancelEdit">Cancel</BaseButton>
              </div>
            </form>
          </BaseCard>
        </template>
        <template v-else>
          <div class="document-header">
            <div class="document-info">
              <h2 class="document-title">{{ doc.title }}</h2>
              <p class="document-preview">{{ truncate(doc.content) }}</p>
              <span class="document-date">Updated {{ formatDate(doc.updatedAt) }}</span>
            </div>
            <div class="document-actions">
              <BaseButton
                variant="secondary"
                :disabled="store.loading"
                @click="startEdit(doc.id, doc.title, doc.content)"
              >
                Edit
              </BaseButton>
              <BaseButton
                variant="secondary"
                :disabled="store.loading"
                @click="toggleVersions(doc.id)"
              >
                {{ expandedVersionsId === doc.id ? 'Hide Versions' : 'Versions' }}
              </BaseButton>
              <BaseButton
                variant="danger"
                :disabled="store.loading"
                @click="handleDelete(doc.id)"
              >
                Delete
              </BaseButton>
            </div>
          </div>

          <div v-if="expandedVersionsId === doc.id" class="versions-panel">
            <h4 class="versions-title">Version History</h4>
            <div
              v-if="store.libraryDocumentVersions.length > 0"
              class="versions-list"
            >
              <div
                v-for="version in store.libraryDocumentVersions"
                :key="version.id"
                class="version-item"
              >
                <span class="version-date">{{ formatDate(version.createdAt) }}</span>
                <span v-if="version.title" class="version-title-badge">{{ version.title }}</span>
                <p class="version-preview">{{ truncate(version.content, 100) }}</p>
              </div>
            </div>
            <p v-else class="empty-text">No versions available.</p>
          </div>
        </template>
      </div>
    </div>
    <p v-else-if="!store.loading" class="empty-text">No library documents yet.</p>
  </div>
</template>

<style scoped>
.library-document-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-width: 800px;
}

.view-actions {
  display: flex;
  gap: 0.75rem;
}

.form-vertical {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.form-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: #374151;
}

.form-textarea {
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  resize: vertical;
  font-family: inherit;
  box-sizing: border-box;
}

.form-textarea--md {
  font-family: 'SF Mono', 'Fira Code', 'Fira Mono', monospace;
  font-size: 0.8125rem;
}

.form-textarea:focus {
  outline: none;
  border-color: #6366f1;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}

.editor-tabs {
  display: flex;
  gap: 0;
  margin-bottom: 0.25rem;
}

.tab-btn {
  padding: 0.25rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 500;
  border: 1px solid #d1d5db;
  background: #f9fafb;
  color: #6b7280;
  cursor: pointer;
}

.tab-btn:first-child {
  border-radius: 0.25rem 0 0 0.25rem;
}

.tab-btn:last-child {
  border-radius: 0 0.25rem 0.25rem 0;
  border-left: none;
}

.tab-btn.active {
  background: #ffffff;
  color: #1f2937;
  border-color: #6366f1;
}

.markdown-preview {
  min-height: 12rem;
  padding: 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  background: #ffffff;
  overflow-y: auto;
}

.markdown-preview :deep(h2) {
  font-size: 1.25rem;
  font-weight: 700;
  margin: 0.5rem 0 0.25rem;
}

.markdown-preview :deep(h3) {
  font-size: 1.1rem;
  font-weight: 600;
  margin: 0.5rem 0 0.25rem;
}

.markdown-preview :deep(h4) {
  font-size: 1rem;
  font-weight: 600;
  margin: 0.5rem 0 0.25rem;
}

.markdown-preview :deep(p) {
  margin: 0.25rem 0;
}

.markdown-preview :deep(li) {
  margin-left: 1.5rem;
  list-style: disc;
}

.form-actions {
  display: flex;
  gap: 0.5rem;
}

.document-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.document-item {
  border: 1px solid #e5e7eb;
  border-radius: 0.5rem;
  overflow: hidden;
}

.document-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  padding: 1rem;
  background: #ffffff;
}

.document-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  flex: 1;
  min-width: 0;
}

.document-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  color: #1f2937;
}

.document-preview {
  margin: 0;
  font-size: 0.875rem;
  color: #6b7280;
  word-break: break-word;
}

.document-date {
  font-size: 0.75rem;
  color: #9ca3af;
}

.document-actions {
  display: flex;
  gap: 0.5rem;
  flex-shrink: 0;
}

.versions-panel {
  border-top: 1px solid #e5e7eb;
  padding: 0.75rem 1rem;
  background: #f9fafb;
}

.versions-title {
  margin: 0 0 0.5rem;
  font-size: 0.875rem;
  font-weight: 600;
  color: #374151;
}

.versions-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.version-item {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  padding: 0.5rem;
  background: #ffffff;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
}

.version-date {
  font-size: 0.75rem;
  color: #9ca3af;
}

.version-title-badge {
  font-size: 0.75rem;
  font-weight: 500;
  color: #6366f1;
}

.version-preview {
  margin: 0;
  font-size: 0.75rem;
  color: #6b7280;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
  font-size: 0.875rem;
}

.empty-text {
  color: #9ca3af;
  font-size: 0.875rem;
  text-align: center;
  padding: 1rem 0;
}
</style>
