<script setup lang="ts">
import { computed } from 'vue'
import type { AwarenessEntry } from '@/composables/useWebSocket'

const props = defineProps<{
  entries: AwarenessEntry[]
}>()

const typingUsers = computed(() => props.entries.filter((e) => e.isTyping))

const typingText = computed(() => {
  const names = typingUsers.value.map((e) => e.displayName)
  if (names.length === 0) return ''
  if (names.length === 1) return `${names[0]} is typing...`
  if (names.length === 2) return `${names[0]} and ${names[1]} are typing...`
  return `${names[0]} and ${names.length - 1} others are typing...`
})
</script>

<template>
  <div class="awareness-bar">
    <div v-if="entries.length > 0" class="online-users">
      <span
        v-for="entry in entries"
        :key="entry.memberId"
        class="user-dot"
        :style="{ backgroundColor: entry.color }"
        :title="entry.displayName"
      />
      <span class="online-count">{{ entries.length }} online</span>
    </div>
    <div v-if="typingText" class="typing-indicator">
      {{ typingText }}
    </div>
  </div>
</template>

<style scoped>
.awareness-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  min-height: 1.5rem;
  font-size: 0.75rem;
  color: #6b7280;
  padding: 0.25rem 0;
}

.online-users {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.user-dot {
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 50%;
  display: inline-block;
}

.online-count {
  margin-left: 0.25rem;
}

.typing-indicator {
  font-style: italic;
}
</style>
