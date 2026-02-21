import { ref, onUnmounted } from 'vue'
import client from '@/api/client'

export interface AwarenessEntry {
  memberId: string
  displayName: string
  color: string
  cursorPosition: number | null
  isTyping: boolean
}

interface ServerWsMessage {
  type: string
  update?: number[]
  entries?: AwarenessEntry[]
  reason?: string
  latestMessageId?: string
}

export interface UseWebSocketOptions {
  projectId: string
  taskId: string
  onPeerUpdate?: (update: Uint8Array) => void
  onSyncDiff?: (update: Uint8Array) => void
  onAwarenessChange?: (entries: AwarenessEntry[]) => void
  onWriteRejected?: (reason: string, latestMessageId: string) => void
}

export function useWebSocket(options: UseWebSocketOptions) {
  const connected = ref(false)
  const reconnecting = ref(false)

  let ws: WebSocket | null = null
  let reconnectAttempts = 0
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let destroyed = false
  const maxReconnectAttempts = 10

  async function connect() {
    if (destroyed) return

    try {
      // Get one-time token
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/tasks/{taskId}/conversation/ws-token' as never,
        { params: { path: { projectId: options.projectId, taskId: options.taskId } } } as never,
      )
      const tokenData = data as { token: string } | undefined
      if (!tokenData) return

      const baseUrl = (import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080') as string
      const wsUrl = baseUrl.replace(/^http/, 'ws')
      const url = `${wsUrl}/api/v1/projects/${options.projectId}/tasks/${options.taskId}/conversation/ws?token=${tokenData.token}`

      ws = new WebSocket(url)

      ws.onopen = () => {
        connected.value = true
        reconnecting.value = false
        reconnectAttempts = 0
      }

      ws.onmessage = (event: MessageEvent) => {
        try {
          const msg = JSON.parse(event.data as string) as ServerWsMessage
          handleMessage(msg)
        } catch {
          // Ignore malformed messages
        }
      }

      ws.onclose = () => {
        connected.value = false
        if (!destroyed) {
          scheduleReconnect()
        }
      }

      ws.onerror = () => {
        ws?.close()
      }
    } catch {
      if (!destroyed) {
        scheduleReconnect()
      }
    }
  }

  function handleMessage(msg: ServerWsMessage) {
    switch (msg.type) {
      case 'SyncDiff':
        if (msg.update && options.onSyncDiff) {
          options.onSyncDiff(new Uint8Array(msg.update))
        }
        break
      case 'PeerUpdate':
        if (msg.update && options.onPeerUpdate) {
          options.onPeerUpdate(new Uint8Array(msg.update))
        }
        break
      case 'AwarenessChange':
        if (msg.entries && options.onAwarenessChange) {
          options.onAwarenessChange(msg.entries)
        }
        break
      case 'WriteRejected':
        if (msg.reason && msg.latestMessageId && options.onWriteRejected) {
          options.onWriteRejected(msg.reason, msg.latestMessageId)
        }
        break
    }
  }

  function scheduleReconnect() {
    if (destroyed || reconnectAttempts >= maxReconnectAttempts) return
    reconnecting.value = true
    reconnectAttempts++
    // Exponential backoff: 1s, 2s, 4s, 8s, ... capped at 30s
    const delay = Math.min(1000 * 2 ** (reconnectAttempts - 1), 30000)
    reconnectTimer = setTimeout(() => {
      void connect()
    }, delay)
  }

  function send(message: Record<string, unknown>) {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(message))
    }
  }

  function sendSyncStep1(stateVector: Uint8Array) {
    send({ type: 'SyncStep1', state_vector: Array.from(stateVector) })
  }

  function sendSyncStep2(update: Uint8Array) {
    send({ type: 'SyncStep2', update: Array.from(update) })
  }

  function sendAwarenessUpdate(isTyping: boolean, cursorPosition?: number) {
    send({
      type: 'AwarenessUpdate',
      state: { is_typing: isTyping, cursor_position: cursorPosition ?? null },
    })
  }

  function disconnect() {
    destroyed = true
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    ws?.close()
    ws = null
    connected.value = false
    reconnecting.value = false
  }

  onUnmounted(() => {
    disconnect()
  })

  return {
    connected,
    reconnecting,
    connect,
    disconnect,
    sendSyncStep1,
    sendSyncStep2,
    sendAwarenessUpdate,
  }
}
