<script setup lang="ts">
import { ref, computed } from 'vue'
import { useFileUpload, validateFile } from '@/composables/useFileUpload'
import type { FileUploadResult } from '@/composables/useFileUpload'
import BaseButton from '@/components/base/BaseButton.vue'

const props = defineProps<{
  scopeType: string
  scopeId: string
  accept?: string
  multiple?: boolean
}>()

const emit = defineEmits<{
  uploaded: [file: FileUploadResult]
  removed: [fileId: string]
}>()

const { uploading, progress, error, uploadFile, deleteFile } = useFileUpload()
const uploadedFiles = ref<FileUploadResult[]>([])
const dragActive = ref(false)
const fileInputRef = ref<HTMLInputElement | null>(null)

const hasFiles = computed(() => uploadedFiles.value.length > 0)

function openFilePicker() {
  fileInputRef.value?.click()
}

async function handleFiles(files: FileList | null) {
  if (!files) return
  for (const file of Array.from(files)) {
    const validationError = validateFile(file)
    if (validationError) {
      error.value = validationError.message
      continue
    }
    const result = await uploadFile(file, props.scopeType, props.scopeId)
    if (result) {
      uploadedFiles.value.push(result)
      emit('uploaded', result)
    }
  }
  // Reset input so the same file can be selected again
  if (fileInputRef.value) {
    fileInputRef.value.value = ''
  }
}

function handleFileInput(event: Event) {
  const input = event.target as HTMLInputElement
  void handleFiles(input.files)
}

function handleDragOver(event: DragEvent) {
  event.preventDefault()
  dragActive.value = true
}

function handleDragLeave() {
  dragActive.value = false
}

function handleDrop(event: DragEvent) {
  event.preventDefault()
  dragActive.value = false
  void handleFiles(event.dataTransfer?.files ?? null)
}

async function removeFile(fileId: string) {
  const success = await deleteFile(fileId)
  if (success) {
    uploadedFiles.value = uploadedFiles.value.filter((f) => f.id !== fileId)
    emit('removed', fileId)
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<template>
  <div class="file-uploader">
    <div
      class="drop-zone"
      :class="{ 'drop-zone--active': dragActive }"
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDrop"
      @click="openFilePicker"
    >
      <input
        ref="fileInputRef"
        type="file"
        class="file-input"
        :accept="accept"
        :multiple="multiple"
        @change="handleFileInput"
      />
      <div class="drop-zone-content">
        <span class="drop-zone-icon">&#128206;</span>
        <span class="drop-zone-text">Drop files here or click to browse</span>
      </div>
    </div>

    <div v-if="uploading" class="progress-bar-container">
      <div class="progress-bar" :style="{ width: `${progress}%` }" />
      <span class="progress-text">{{ progress }}%</span>
    </div>

    <p v-if="error" class="upload-error">{{ error }}</p>

    <ul v-if="hasFiles" class="file-list">
      <li v-for="file in uploadedFiles" :key="file.id" class="file-item">
        <span class="file-name">{{ file.filename }}</span>
        <span class="file-size">{{ formatSize(file.size) }}</span>
        <BaseButton variant="danger" @click="removeFile(file.id)">Remove</BaseButton>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.file-uploader {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.drop-zone {
  border: 2px dashed #d1d5db;
  border-radius: 0.5rem;
  padding: 1.5rem;
  text-align: center;
  cursor: pointer;
  transition: border-color 0.2s, background-color 0.2s;
  position: relative;
}

.drop-zone:hover,
.drop-zone--active {
  border-color: #3b82f6;
  background-color: #eff6ff;
}

.file-input {
  position: absolute;
  inset: 0;
  opacity: 0;
  cursor: pointer;
}

.drop-zone-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.25rem;
  pointer-events: none;
}

.drop-zone-icon {
  font-size: 1.5rem;
}

.drop-zone-text {
  font-size: 0.875rem;
  color: #6b7280;
}

.progress-bar-container {
  position: relative;
  height: 1.25rem;
  background: #e5e7eb;
  border-radius: 0.375rem;
  overflow: hidden;
}

.progress-bar {
  height: 100%;
  background: #3b82f6;
  transition: width 0.2s;
}

.progress-text {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
  font-weight: 500;
  color: #1f2937;
}

.upload-error {
  color: #ef4444;
  font-size: 0.8125rem;
  margin: 0;
}

.file-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.5rem;
  background: #f9fafb;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  font-size: 0.8125rem;
}

.file-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #1f2937;
}

.file-size {
  color: #6b7280;
  white-space: nowrap;
}
</style>
