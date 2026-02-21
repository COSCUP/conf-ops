<script setup lang="ts">
import { ref, computed } from 'vue'
import BaseButton from '@/components/base/BaseButton.vue'
import FilePreview from '@/components/file/FilePreview.vue'
import { useFileUpload, validateFile } from '@/composables/useFileUpload'
import type { FileUploadResult } from '@/composables/useFileUpload'

export interface MentionMember {
  id: string
  displayName: string
}

export interface MentionPayload {
  type: 'member'
  id: string
}

const props = defineProps<{
  disabled: boolean
  members?: MentionMember[]
  scopeType?: string
  scopeId?: string
}>()

const emit = defineEmits<{
  send: [text: string, mentions: MentionPayload[], attachments: string[]]
  typing: [isTyping: boolean]
}>()

const text = ref('')
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const showDropdown = ref(false)
const mentionQuery = ref('')
const mentionStartIndex = ref(-1)
const selectedMentions = ref<MentionPayload[]>([])

// Attachment state
const attachments = ref<FileUploadResult[]>([])
const fileInputRef = ref<HTMLInputElement | null>(null)
const { uploading, progress, error: uploadError, uploadFile, deleteFile } = useFileUpload()

let typingTimeout: ReturnType<typeof setTimeout> | null = null

const filteredMembers = computed(() => {
  if (!props.members || !showDropdown.value) return []
  const query = mentionQuery.value.toLowerCase()
  return props.members.filter((m) => m.displayName.toLowerCase().includes(query))
})

const canAttach = computed(() => Boolean(props.scopeType && props.scopeId))

function detectMention() {
  const el = textareaRef.value
  if (!el) return

  const cursorPos = el.selectionStart ?? 0
  const textBefore = text.value.slice(0, cursorPos)
  const atIndex = textBefore.lastIndexOf('@')

  if (atIndex === -1) {
    closeMentionDropdown()
    return
  }

  const textAfterAt = textBefore.slice(atIndex + 1)
  // Only trigger if no space between @ and cursor
  if (/\s/.test(textAfterAt)) {
    closeMentionDropdown()
    return
  }

  mentionStartIndex.value = atIndex
  mentionQuery.value = textAfterAt
  showDropdown.value = true
}

function closeMentionDropdown() {
  showDropdown.value = false
  mentionQuery.value = ''
  mentionStartIndex.value = -1
}

function selectMention(member: MentionMember) {
  const start = mentionStartIndex.value
  if (start === -1) return

  const el = textareaRef.value
  const cursorPos = el?.selectionStart ?? text.value.length
  const before = text.value.slice(0, start)
  const after = text.value.slice(cursorPos)
  text.value = `${before}@${member.displayName} ${after}`

  // Track the mention
  if (!selectedMentions.value.some((m) => m.id === member.id)) {
    selectedMentions.value.push({ type: 'member', id: member.id })
  }

  closeMentionDropdown()

  // Restore focus and place cursor after the inserted mention
  const newCursor = before.length + member.displayName.length + 2 // '@' + name + ' '
  if (el) {
    el.focus()
    requestAnimationFrame(() => {
      el.setSelectionRange(newCursor, newCursor)
    })
  }
}

function handleInput() {
  detectMention()
  // Clean up mentions that were removed from the text
  selectedMentions.value = selectedMentions.value.filter((mention) => {
    const member = props.members?.find((m) => m.id === mention.id)
    if (!member) return false
    return text.value.includes(`@${member.displayName}`)
  })

  emit('typing', true)
  if (typingTimeout) clearTimeout(typingTimeout)
  typingTimeout = setTimeout(() => {
    emit('typing', false)
  }, 2000)
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter' && !event.shiftKey && !showDropdown.value) {
    event.preventDefault()
    handleSend()
  }
  if (event.key === 'Escape' && showDropdown.value) {
    closeMentionDropdown()
  }
}

function handleSend() {
  const trimmed = text.value.trim()
  if (!trimmed && attachments.value.length === 0) return
  const attachmentIds = attachments.value.map((f) => f.id)
  emit('send', trimmed, [...selectedMentions.value], attachmentIds)
  text.value = ''
  selectedMentions.value = []
  attachments.value = []
  closeMentionDropdown()
  emit('typing', false)
  if (typingTimeout) {
    clearTimeout(typingTimeout)
    typingTimeout = null
  }
}

// Attachment handlers
function openAttachmentPicker() {
  fileInputRef.value?.click()
}

async function handleAttachmentInput(event: Event) {
  const input = event.target as HTMLInputElement
  const files = input.files
  if (!files || !props.scopeType || !props.scopeId) return

  for (const file of Array.from(files)) {
    const validationError = validateFile(file)
    if (validationError) {
      continue
    }
    const result = await uploadFile(file, props.scopeType, props.scopeId)
    if (result) {
      attachments.value.push(result)
    }
  }
  // Reset input
  if (fileInputRef.value) {
    fileInputRef.value.value = ''
  }
}

async function removeAttachment(fileId: string) {
  const success = await deleteFile(fileId)
  if (success) {
    attachments.value = attachments.value.filter((f) => f.id !== fileId)
  }
}

</script>

<template>
  <form class="message-input" @submit.prevent="handleSend">
    <!-- Attachment preview bar -->
    <div v-if="attachments.length > 0" class="attachment-bar">
      <div v-for="file in attachments" :key="file.id" class="attachment-item">
        <FilePreview
          :file-id="file.id"
          :filename="file.filename"
          :mime-type="file.mimeType"
          :size="file.size"
        />
        <button type="button" class="attachment-remove" @click="removeAttachment(file.id)">
          &times;
        </button>
      </div>
    </div>

    <!-- Upload progress -->
    <div v-if="uploading" class="upload-progress">
      <div class="upload-progress-bar" :style="{ width: `${progress}%` }" />
    </div>

    <p v-if="uploadError" class="upload-error">{{ uploadError }}</p>

    <div class="input-row">
      <!-- Attachment button -->
      <button
        v-if="canAttach"
        type="button"
        class="attach-button"
        title="Attach file"
        :disabled="disabled"
        @click="openAttachmentPicker"
      >
        &#128206;
      </button>
      <input
        ref="fileInputRef"
        type="file"
        class="hidden-input"
        multiple
        @change="handleAttachmentInput"
      />

      <div class="textarea-wrapper">
        <textarea
          ref="textareaRef"
          v-model="text"
          class="message-textarea"
          placeholder="Type a message... (use @ to mention)"
          rows="2"
          :disabled="disabled"
          @input="handleInput"
          @keydown="handleKeydown"
        />
        <ul
          v-if="showDropdown && filteredMembers.length > 0"
          class="mention-dropdown"
          role="listbox"
        >
          <li
            v-for="member in filteredMembers"
            :key="member.id"
            class="mention-item"
            role="option"
            @mousedown.prevent="selectMention(member)"
          >
            {{ member.displayName }}
          </li>
        </ul>
      </div>
      <BaseButton :disabled="disabled || (!text.trim() && attachments.length === 0)">
        Send
      </BaseButton>
    </div>
  </form>
</template>

<style scoped>
.message-input {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.input-row {
  display: flex;
  gap: 0.5rem;
  align-items: flex-end;
}

.textarea-wrapper {
  flex: 1;
  position: relative;
}

.message-textarea {
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  font-family: inherit;
  resize: none;
  outline: none;
  box-sizing: border-box;
}

.message-textarea:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 1px #3b82f6;
}

.message-textarea:disabled {
  background: #f3f4f6;
}

.mention-dropdown {
  position: absolute;
  bottom: calc(100% + 4px);
  left: 0;
  right: 0;
  background: #ffffff;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);
  list-style: none;
  margin: 0;
  padding: 0.25rem 0;
  max-height: 200px;
  overflow-y: auto;
  z-index: 10;
}

.mention-item {
  padding: 0.375rem 0.75rem;
  font-size: 0.875rem;
  cursor: pointer;
  color: #111827;
}

.mention-item:hover {
  background: #f3f4f6;
}

.attach-button {
  background: none;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  padding: 0.375rem 0.5rem;
  cursor: pointer;
  font-size: 1.125rem;
  line-height: 1;
  color: #6b7280;
  transition: border-color 0.15s;
}

.attach-button:hover {
  border-color: #3b82f6;
  color: #3b82f6;
}

.attach-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.hidden-input {
  display: none;
}

.attachment-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
  padding: 0.25rem 0;
}

.attachment-item {
  display: inline-flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.25rem;
  padding: 0.25rem;
  background: #f3f4f6;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  position: relative;
}

.attachment-remove {
  background: none;
  border: none;
  color: #6b7280;
  cursor: pointer;
  font-size: 0.875rem;
  line-height: 1;
  padding: 0;
}

.attachment-remove:hover {
  color: #ef4444;
}

.upload-progress {
  height: 3px;
  background: #e5e7eb;
  border-radius: 2px;
  overflow: hidden;
}

.upload-progress-bar {
  height: 100%;
  background: #3b82f6;
  transition: width 0.2s;
}

.upload-error {
  color: #ef4444;
  font-size: 0.75rem;
  margin: 0;
}
</style>
