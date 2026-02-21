import { ref } from 'vue'
import { useAuthStore } from '@/stores/auth'

const ALLOWED_IMAGE_TYPES = [
  'image/jpeg',
  'image/png',
  'image/gif',
  'image/webp',
  'image/svg+xml',
]

const ALLOWED_DOCUMENT_TYPES = [
  'application/pdf',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  'application/vnd.ms-excel',
  'application/msword',
]

const ALLOWED_TEXT_TYPES = ['text/plain', 'text/csv']

const ALL_ALLOWED_TYPES = [...ALLOWED_IMAGE_TYPES, ...ALLOWED_DOCUMENT_TYPES, ...ALLOWED_TEXT_TYPES]

const MAX_IMAGE_SIZE = 10 * 1024 * 1024 // 10 MB
const MAX_DOCUMENT_SIZE = 50 * 1024 * 1024 // 50 MB
const MAX_OTHER_SIZE = 20 * 1024 * 1024 // 20 MB

export interface FileUploadResult {
  id: string
  filename: string
  mimeType: string
  size: number
  createdAt: string
}

export interface FileValidationError {
  type: 'mime' | 'size'
  message: string
}

function getMaxSize(mimeType: string): number {
  if (mimeType.startsWith('image/')) return MAX_IMAGE_SIZE
  if (mimeType.startsWith('application/')) return MAX_DOCUMENT_SIZE
  return MAX_OTHER_SIZE
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

export function validateFile(file: File): FileValidationError | null {
  // Check MIME type
  const isAllowed =
    file.type.startsWith('image/') ||
    file.type.startsWith('text/') ||
    ALL_ALLOWED_TYPES.includes(file.type)

  if (!isAllowed) {
    return {
      type: 'mime',
      message: `Unsupported file type: ${file.type || 'unknown'}`,
    }
  }

  // Check size
  const maxSize = getMaxSize(file.type)
  if (file.size > maxSize) {
    return {
      type: 'size',
      message: `File too large: ${formatSize(file.size)} (max ${formatSize(maxSize)})`,
    }
  }

  return null
}

export function useFileUpload() {
  const uploading = ref(false)
  const progress = ref(0)
  const error = ref<string | null>(null)

  async function uploadFile(
    file: File,
    scopeType: string,
    scopeId: string,
  ): Promise<FileUploadResult | null> {
    const validationError = validateFile(file)
    if (validationError) {
      error.value = validationError.message
      return null
    }

    const authStore = useAuthStore()
    const token = authStore.accessToken
    if (!token) {
      error.value = 'Not authenticated'
      return null
    }

    uploading.value = true
    progress.value = 0
    error.value = null

    const formData = new FormData()
    formData.append('file', file)
    formData.append('scopeType', scopeType)
    formData.append('scopeId', scopeId)

    const baseUrl = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080'

    return new Promise<FileUploadResult | null>((resolve) => {
      const xhr = new XMLHttpRequest()

      xhr.upload.addEventListener('progress', (e) => {
        if (e.lengthComputable) {
          progress.value = Math.round((e.loaded / e.total) * 100)
        }
      })

      xhr.addEventListener('load', () => {
        uploading.value = false
        if (xhr.status === 201) {
          try {
            const result = JSON.parse(xhr.responseText) as FileUploadResult
            progress.value = 100
            resolve(result)
          } catch {
            error.value = 'Invalid server response'
            resolve(null)
          }
        } else {
          try {
            const errBody = JSON.parse(xhr.responseText) as { detail?: string }
            error.value = errBody.detail ?? `Upload failed (${xhr.status})`
          } catch {
            error.value = `Upload failed (${xhr.status})`
          }
          resolve(null)
        }
      })

      xhr.addEventListener('error', () => {
        uploading.value = false
        error.value = 'Network error during upload'
        resolve(null)
      })

      xhr.addEventListener('abort', () => {
        uploading.value = false
        error.value = 'Upload cancelled'
        resolve(null)
      })

      xhr.open('POST', `${baseUrl}/api/v1/files/upload`)
      xhr.setRequestHeader('Authorization', `Bearer ${token}`)
      xhr.send(formData)
    })
  }

  async function deleteFile(fileId: string): Promise<boolean> {
    const authStore = useAuthStore()
    const token = authStore.accessToken
    if (!token) return false

    const baseUrl = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080'

    try {
      const resp = await fetch(`${baseUrl}/api/v1/files/${fileId}`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${token}` },
      })
      return resp.status === 204
    } catch {
      return false
    }
  }

  return {
    uploading,
    progress,
    error,
    uploadFile,
    deleteFile,
    validateFile,
  }
}
