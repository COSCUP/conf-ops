<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAuthStore } from '@/stores/auth'

const props = defineProps<{
  fileId: string
  filename: string
  mimeType: string
  size: number
}>()

const showModal = ref(false)
const authStore = useAuthStore()

const baseUrl = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080'

const downloadUrl = computed(() => `${baseUrl}/api/v1/files/${props.fileId}`)

const isImage = computed(() => props.mimeType.startsWith('image/'))
const isPdf = computed(() => props.mimeType === 'application/pdf')

const authHeaders = computed(() => ({
  Authorization: `Bearer ${authStore.accessToken ?? ''}`,
}))

// For images, we build a blob URL with auth
const imageUrl = ref<string | null>(null)

async function loadImage() {
  if (!isImage.value || imageUrl.value) return
  try {
    const resp = await fetch(downloadUrl.value, { headers: authHeaders.value })
    if (resp.ok) {
      const blob = await resp.blob()
      imageUrl.value = URL.createObjectURL(blob)
    }
  } catch {
    // Failed to load image, show fallback
  }
}

// For PDF, we build a blob URL with auth for embedded preview
const pdfUrl = ref<string | null>(null)

async function loadPdf() {
  if (!isPdf.value || pdfUrl.value) return
  try {
    const resp = await fetch(downloadUrl.value, { headers: authHeaders.value })
    if (resp.ok) {
      const blob = await resp.blob()
      pdfUrl.value = URL.createObjectURL(blob)
    }
  } catch {
    // Failed to load PDF, fall back to download only
  }
}

if (isImage.value) {
  void loadImage()
}

if (isPdf.value) {
  void loadPdf()
}

function openModal() {
  showModal.value = true
}

function closeModal() {
  showModal.value = false
}

async function handleDownload() {
  try {
    const resp = await fetch(downloadUrl.value, { headers: authHeaders.value })
    if (resp.ok) {
      const blob = await resp.blob()
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = props.filename
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
    }
  } catch {
    // Download failed silently
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<template>
  <div class="file-preview">
    <!-- Image preview -->
    <div v-if="isImage" class="image-preview" @click="openModal">
      <img v-if="imageUrl" :src="imageUrl" :alt="filename" class="thumbnail" />
      <span v-else class="thumbnail-placeholder">Loading...</span>
    </div>

    <!-- PDF preview -->
    <div v-else-if="isPdf" class="pdf-preview">
      <iframe v-if="pdfUrl" :src="pdfUrl" class="pdf-embed" :title="filename" />
      <div class="pdf-meta">
        <span class="file-icon">&#128196;</span>
        <span class="file-name">{{ filename }}</span>
        <span class="file-size">{{ formatSize(size) }}</span>
        <button class="download-link" @click="handleDownload">Download</button>
      </div>
    </div>

    <!-- Other files -->
    <div v-else class="generic-preview">
      <span class="file-icon">&#128196;</span>
      <span class="file-name">{{ filename }}</span>
      <span class="file-size">{{ formatSize(size) }}</span>
      <button class="download-link" @click="handleDownload">Download</button>
    </div>

    <!-- Image modal overlay -->
    <Teleport to="body">
      <div v-if="showModal && imageUrl" class="modal-overlay" @click="closeModal">
        <div class="modal-content" @click.stop>
          <button class="modal-close" @click="closeModal">&times;</button>
          <img :src="imageUrl" :alt="filename" class="modal-image" />
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.file-preview {
  display: inline-flex;
}

.image-preview {
  cursor: pointer;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  overflow: hidden;
  display: inline-block;
}

.thumbnail {
  display: block;
  max-width: 120px;
  max-height: 120px;
  object-fit: cover;
}

.thumbnail-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 120px;
  height: 80px;
  background: #f3f4f6;
  color: #9ca3af;
  font-size: 0.75rem;
}

.pdf-preview {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  overflow: hidden;
  font-size: 0.8125rem;
  background: #f9fafb;
}

.pdf-embed {
  width: 100%;
  height: 400px;
  border: none;
  display: block;
}

.pdf-meta {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.5rem;
}

.generic-preview {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.5rem;
  background: #f9fafb;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  font-size: 0.8125rem;
}

.file-icon {
  font-size: 1.25rem;
}

.file-name {
  color: #1f2937;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-size {
  color: #6b7280;
  white-space: nowrap;
}

.download-link {
  background: none;
  border: none;
  color: #3b82f6;
  cursor: pointer;
  font-size: 0.8125rem;
  padding: 0;
  text-decoration: underline;
}

.download-link:hover {
  color: #2563eb;
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  position: relative;
  max-width: 90vw;
  max-height: 90vh;
}

.modal-close {
  position: absolute;
  top: -2rem;
  right: -0.5rem;
  background: none;
  border: none;
  color: #ffffff;
  font-size: 1.5rem;
  cursor: pointer;
  line-height: 1;
}

.modal-image {
  max-width: 90vw;
  max-height: 90vh;
  object-fit: contain;
  border-radius: 0.25rem;
}
</style>
