<script setup lang="ts">
import { ref } from 'vue'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseInput from '@/components/base/BaseInput.vue'

defineProps<{
  disabled?: boolean
  defaultSubject?: string
}>()

const emit = defineEmits<{
  send: [payload: { toAddresses: string[]; ccAddresses: string[]; subject: string; htmlBody: string }]
}>()

const toField = ref('')
const ccField = ref('')
const subjectField = ref('')
const bodyField = ref('')

function handleSend() {
  const toAddresses = toField.value
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0)
  const ccAddresses = ccField.value
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0)

  if (toAddresses.length === 0 || bodyField.value.trim().length === 0) return

  emit('send', {
    toAddresses,
    ccAddresses,
    subject: subjectField.value,
    htmlBody: bodyField.value,
  })

  toField.value = ''
  ccField.value = ''
  subjectField.value = ''
  bodyField.value = ''
}
</script>

<template>
  <form class="email-composer" @submit.prevent="handleSend">
    <BaseInput v-model="toField" label="To" placeholder="email@example.com, ..." />
    <BaseInput v-model="ccField" label="CC" placeholder="Optional CC addresses" />
    <BaseInput
      v-model="subjectField"
      label="Subject"
      :placeholder="defaultSubject ?? 'Email subject'"
    />
    <div class="body-field">
      <label class="field-label">Body</label>
      <textarea
        v-model="bodyField"
        class="body-textarea"
        rows="4"
        placeholder="Write your email..."
      />
    </div>
    <BaseButton :disabled="disabled" type="submit">Send</BaseButton>
  </form>
</template>

<style scoped>
.email-composer {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.body-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.field-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: #374151;
}

.body-textarea {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-family: inherit;
  font-size: 0.875rem;
  resize: vertical;
}

.body-textarea:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgb(59 130 246 / 20%);
}
</style>
