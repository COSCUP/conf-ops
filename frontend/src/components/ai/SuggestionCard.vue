<script setup lang="ts">
import { ref } from 'vue'
import type { Suggestion, SuggestionDecision } from '@/stores/suggestion'
import SuggestionModifyDialog from './SuggestionModifyDialog.vue'

const props = defineProps<{
  suggestion: Suggestion
  groupId: string
  messageId: string
  projectId: string
  taskId: string
}>()

const emit = defineEmits<{
  decide: [
    payload: {
      groupId: string
      suggestionId: string
      decision: SuggestionDecision
      messageId: string
      modifiedParameters?: unknown
      additionalInstructions?: string
    },
  ]
}>()

const reasoningExpanded = ref(false)
const contextExpanded = ref(false)
const showModifyDialog = ref(false)
const showReSuggestInput = ref(false)
const reSuggestInstructions = ref('')

function toggleReasoning() {
  reasoningExpanded.value = !reasoningExpanded.value
}

function formatParameters(params: unknown): string {
  try {
    return JSON.stringify(params, null, 2)
  } catch {
    return String(params)
  }
}

function formatDecision(decision: SuggestionDecision): string {
  switch (decision) {
    case 'accept':
      return 'Accepted'
    case 'modify_and_accept':
      return 'Modified & Accepted'
    case 'reject':
      return 'Rejected'
    case 're_suggest':
      return 'Re-suggested'
    default:
      return 'Pending'
  }
}

function getDecisionClass(decision: SuggestionDecision): string {
  switch (decision) {
    case 'accept':
      return 'badge--accept'
    case 'modify_and_accept':
      return 'badge--modify'
    case 'reject':
      return 'badge--reject'
    case 're_suggest':
      return 'badge--resuggest'
    default:
      return 'badge--pending'
  }
}

function formatTime(iso: string): string {
  const d = new Date(iso)
  return d.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function handleAccept() {
  emit('decide', {
    groupId: props.groupId,
    suggestionId: props.suggestion.id,
    decision: 'accept',
    messageId: props.messageId,
  })
}

function handleReject() {
  emit('decide', {
    groupId: props.groupId,
    suggestionId: props.suggestion.id,
    decision: 'reject',
    messageId: props.messageId,
  })
}

function toggleContext() {
  contextExpanded.value = !contextExpanded.value
}

function handleReSuggest() {
  if (!showReSuggestInput.value) {
    showReSuggestInput.value = true
    return
  }
  const payload: {
    groupId: string
    suggestionId: string
    decision: SuggestionDecision
    messageId: string
    additionalInstructions?: string
  } = {
    groupId: props.groupId,
    suggestionId: props.suggestion.id,
    decision: 're_suggest',
    messageId: props.messageId,
  }
  if (reSuggestInstructions.value) {
    payload.additionalInstructions = reSuggestInstructions.value
  }
  emit('decide', payload)
  showReSuggestInput.value = false
  reSuggestInstructions.value = ''
}

function handleModifyConfirm(modifiedParameters: unknown) {
  showModifyDialog.value = false
  emit('decide', {
    groupId: props.groupId,
    suggestionId: props.suggestion.id,
    decision: 'modify_and_accept',
    messageId: props.messageId,
    modifiedParameters,
  })
}
</script>

<template>
  <div class="suggestion-card">
    <div class="suggestion-header">
      <span class="suggestion-tool">{{ suggestion.tool }}</span>
      <span
        v-if="suggestion.decision !== 'pending'"
        :class="['decision-badge', getDecisionClass(suggestion.decision)]"
      >
        {{ formatDecision(suggestion.decision) }}
      </span>
    </div>

    <p class="suggestion-summary">{{ suggestion.summary }}</p>

    <div class="suggestion-parameters">
      <span class="parameters-label">Parameters:</span>
      <pre class="parameters-preview">{{ formatParameters(suggestion.parameters) }}</pre>
    </div>

    <div class="suggestion-reasoning">
      <button
        type="button"
        class="reasoning-toggle"
        @click="toggleReasoning"
      >
        {{ reasoningExpanded ? 'Hide' : 'Show' }} AI reasoning
      </button>
      <div v-if="reasoningExpanded" class="reasoning-content">
        {{ suggestion.reasoning }}
      </div>
    </div>

    <div v-if="suggestion.contextUsed && suggestion.contextUsed.length > 0" class="suggestion-context">
      <button
        type="button"
        class="reasoning-toggle"
        @click="toggleContext"
      >
        {{ contextExpanded ? 'Hide' : 'Show' }} context used ({{ suggestion.contextUsed.length }})
      </button>
      <div v-if="contextExpanded" class="context-list">
        <div
          v-for="(ctx, idx) in suggestion.contextUsed"
          :key="idx"
          class="context-item"
        >
          <span class="context-scope">{{ ctx.scopeType }}</span>
          <span class="context-content">{{ ctx.content }}</span>
        </div>
      </div>
    </div>

    <div
      v-if="suggestion.decision !== 'pending'"
      class="decided-info"
    >
      <span v-if="suggestion.decidedAt" class="decided-at">
        Decided at {{ formatTime(suggestion.decidedAt) }}
      </span>
    </div>

    <div v-else class="suggestion-actions">
      <button
        type="button"
        class="action-btn action-btn--accept"
        @click="handleAccept"
      >
        Accept
      </button>
      <button
        type="button"
        class="action-btn action-btn--modify"
        @click="showModifyDialog = true"
      >
        Modify
      </button>
      <button
        type="button"
        class="action-btn action-btn--reject"
        @click="handleReject"
      >
        Reject
      </button>
      <button
        type="button"
        class="action-btn action-btn--resuggest"
        @click="handleReSuggest"
      >
        {{ showReSuggestInput ? 'Send' : 'Re-suggest' }}
      </button>
    </div>

    <div v-if="showReSuggestInput" class="resuggest-input">
      <textarea
        v-model="reSuggestInstructions"
        class="resuggest-textarea"
        placeholder="Provide instructions for the AI to re-suggest..."
        rows="2"
      />
      <button
        type="button"
        class="resuggest-cancel"
        @click="showReSuggestInput = false; reSuggestInstructions = ''"
      >
        Cancel
      </button>
    </div>

    <SuggestionModifyDialog
      v-if="showModifyDialog"
      :suggestion="suggestion"
      :show="showModifyDialog"
      :project-id="props.projectId"
      :task-id="props.taskId"
      @confirm="handleModifyConfirm"
      @close="showModifyDialog = false"
    />
  </div>
</template>

<style scoped>
.suggestion-card {
  border: 1px solid #d1fae5;
  border-radius: 0.5rem;
  background: #f0fdf4;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.suggestion-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.suggestion-tool {
  font-size: 0.75rem;
  font-weight: 700;
  color: #065f46;
  background: #d1fae5;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
}

.decision-badge {
  font-size: 0.6875rem;
  font-weight: 600;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
}

.badge--accept {
  background: #d1fae5;
  color: #065f46;
}

.badge--modify {
  background: #dbeafe;
  color: #1e40af;
}

.badge--reject {
  background: #fee2e2;
  color: #991b1b;
}

.badge--resuggest {
  background: #fef3c7;
  color: #92400e;
}

.badge--pending {
  background: #f3f4f6;
  color: #6b7280;
}

.suggestion-summary {
  font-size: 0.875rem;
  color: #1f2937;
  margin: 0;
}

.suggestion-parameters {
  background: #ffffff;
  border: 1px solid #d1fae5;
  border-radius: 0.375rem;
  padding: 0.5rem;
}

.parameters-label {
  font-size: 0.6875rem;
  font-weight: 600;
  color: #6b7280;
  display: block;
  margin-bottom: 0.25rem;
}

.parameters-preview {
  font-size: 0.75rem;
  color: #374151;
  white-space: pre-wrap;
  word-break: break-word;
  margin: 0;
  max-height: 8rem;
  overflow-y: auto;
}

.suggestion-reasoning {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.reasoning-toggle {
  font-size: 0.75rem;
  color: #059669;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
  text-decoration: underline;
  text-align: left;
}

.reasoning-content {
  font-size: 0.8125rem;
  color: #374151;
  padding: 0.5rem;
  background: #ecfdf5;
  border-radius: 0.25rem;
  white-space: pre-wrap;
}

.decided-info {
  font-size: 0.6875rem;
  color: #6b7280;
}

.suggestion-actions {
  display: flex;
  gap: 0.375rem;
  flex-wrap: wrap;
}

.action-btn {
  font-size: 0.75rem;
  font-weight: 500;
  padding: 0.25rem 0.75rem;
  border-radius: 0.25rem;
  border: 1px solid transparent;
  cursor: pointer;
  transition: opacity 0.15s;
}

.action-btn:hover {
  opacity: 0.85;
}

.action-btn--accept {
  background: #059669;
  color: #ffffff;
}

.action-btn--modify {
  background: #3b82f6;
  color: #ffffff;
}

.action-btn--reject {
  background: #ef4444;
  color: #ffffff;
}

.action-btn--resuggest {
  background: #f59e0b;
  color: #ffffff;
}

.suggestion-context {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.context-list {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.5rem;
  background: #ecfdf5;
  border-radius: 0.25rem;
}

.context-item {
  display: flex;
  gap: 0.5rem;
  align-items: baseline;
  font-size: 0.8125rem;
}

.context-scope {
  font-size: 0.6875rem;
  font-weight: 600;
  color: #6366f1;
  background: #eef2ff;
  padding: 0.0625rem 0.375rem;
  border-radius: 9999px;
  flex-shrink: 0;
}

.context-content {
  color: #374151;
}

.resuggest-input {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.resuggest-textarea {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.25rem;
  font-size: 0.8125rem;
  font-family: inherit;
  resize: vertical;
  box-sizing: border-box;
}

.resuggest-textarea:focus {
  outline: none;
  border-color: #f59e0b;
  box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.2);
}

.resuggest-cancel {
  font-size: 0.75rem;
  color: #6b7280;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
  text-decoration: underline;
  text-align: left;
  align-self: flex-start;
}
</style>
