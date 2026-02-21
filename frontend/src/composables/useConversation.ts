import { ref, onMounted, onUnmounted } from 'vue'
import { useConversationStore } from '@/stores/conversation'
import { useWebSocket, type AwarenessEntry } from '@/composables/useWebSocket'

export interface UseConversationOptions {
  projectId: string
  taskId: string
}

export function useConversation(options: UseConversationOptions) {
  const store = useConversationStore()
  const awarenessEntries = ref<AwarenessEntry[]>([])

  const {
    connected,
    reconnecting,
    connect,
    disconnect,
    sendSyncStep2,
    sendAwarenessUpdate,
  } = useWebSocket({
    projectId: options.projectId,
    taskId: options.taskId,
    onPeerUpdate() {
      // Refresh messages when peer updates arrive
      void store.fetchMessages(options.projectId, options.taskId)
    },
    onAwarenessChange(entries) {
      awarenessEntries.value = entries
    },
  })

  onMounted(async () => {
    await Promise.all([
      store.fetchMessages(options.projectId, options.taskId),
      store.getLastSeen(options.projectId, options.taskId),
    ])
    void connect()
  })

  onUnmounted(() => {
    disconnect()
    store.$reset()
  })

  async function sendMessage(text: string) {
    const lastId = store.messages[0]?.id
    const result = await store.sendMessage(
      options.projectId,
      options.taskId,
      { text, mentions: [] },
      lastId,
    )
    if (result) {
      await store.updateLastSeen(options.projectId, options.taskId, result.id)
    }
    return result
  }

  async function loadMore() {
    if (!store.hasMore || !store.nextCursor) return
    await store.fetchMessages(options.projectId, options.taskId, store.nextCursor)
  }

  function setTyping(isTyping: boolean) {
    sendAwarenessUpdate(isTyping)
  }

  return {
    messages: store.messages,
    loading: store.loading,
    error: store.error,
    hasMore: store.hasMore,
    lastReadMessageId: store.lastReadMessageId,
    connected,
    reconnecting,
    awarenessEntries,
    sendMessage,
    loadMore,
    setTyping,
    sendSyncStep2,
  }
}
