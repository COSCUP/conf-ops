import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { mount, flushPromises } from '@vue/test-utils'

vi.mock('@/api/client', () => ({
  default: {
    POST: vi.fn().mockResolvedValue({ data: { token: 'test-token' } }),
    use: vi.fn(),
  },
  setupAuthInterceptor: vi.fn(),
}))

import client from '@/api/client'
import { useWebSocket } from '../useWebSocket'
import type { UseWebSocketOptions } from '../useWebSocket'

// ---- MockWebSocket setup ----

interface MockWsInstance {
  url: string
  readyState: number
  onopen: (() => void) | null
  onclose: (() => void) | null
  onmessage: ((ev: { data: string }) => void) | null
  onerror: (() => void) | null
  send: ReturnType<typeof vi.fn>
  close: ReturnType<typeof vi.fn>
}

const wsInstances: MockWsInstance[] = []

class MockWebSocket implements MockWsInstance {
  static OPEN = 1
  static CLOSED = 3
  readyState = MockWebSocket.OPEN
  url: string
  onopen: (() => void) | null = null
  onclose: (() => void) | null = null
  onmessage: ((ev: { data: string }) => void) | null = null
  onerror: (() => void) | null = null
  send = vi.fn()
  close = vi.fn()

  constructor(url: string) {
    this.url = url
    wsInstances.push(this)
  }
}

vi.stubGlobal('WebSocket', MockWebSocket)

// ---- Helper ----

function mountWithWebSocket(options: UseWebSocketOptions) {
  let composable: ReturnType<typeof useWebSocket> | undefined

  const TestComponent = defineComponent({
    setup() {
      composable = useWebSocket(options)
      return composable
    },
    render() {
      return h('div')
    },
  })

  const wrapper = mount(TestComponent)

  if (!composable) throw new Error('composable not initialized')

  return { wrapper, composable }
}

async function flushMicrotasks() {
  await flushPromises()
}

// ---- Tests ----

describe('useWebSocket', () => {
  const defaultOptions: UseWebSocketOptions = {
    projectId: 'project-1',
    taskId: 'task-1',
  }

  beforeEach(() => {
    vi.clearAllMocks()
    vi.useFakeTimers()
    wsInstances.length = 0
    vi.mocked(client.POST).mockResolvedValue({ data: { token: 'test-token' } } as never)
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('should initialize with disconnected state', () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    expect(composable.connected.value).toBe(false)
    expect(composable.reconnecting.value).toBe(false)
  })

  it('should attempt to connect and set connected state', async () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    expect(client.POST).toHaveBeenCalledWith(
      '/api/v1/projects/{projectId}/tasks/{taskId}/conversation/ws-token',
      expect.objectContaining({
        params: { path: { projectId: 'project-1', taskId: 'task-1' } },
      }),
    )

    expect(wsInstances).toHaveLength(1)
    wsInstances[0]?.onopen?.()

    expect(composable.connected.value).toBe(true)
    expect(composable.reconnecting.value).toBe(false)
  })

  it('should reset reconnectAttempts to 0 on successful open', async () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    // Simulate a failed close → reconnect cycle first
    wsInstances[0]?.onopen?.()
    wsInstances[0]?.onclose?.()
    expect(composable.reconnecting.value).toBe(true)

    // Let first reconnect fire
    await vi.advanceTimersByTimeAsync(1000)
    await flushMicrotasks()
    expect(wsInstances).toHaveLength(2)

    // Now open the second WS — reconnectAttempts must reset → reconnecting goes false
    wsInstances[1]?.onopen?.()
    expect(composable.connected.value).toBe(true)
    expect(composable.reconnecting.value).toBe(false)
  })

  it('should handle reconnection with exponential backoff', async () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    wsInstances[0]?.onopen?.()
    expect(composable.connected.value).toBe(true)

    wsInstances[0]?.onclose?.()
    expect(composable.connected.value).toBe(false)
    expect(composable.reconnecting.value).toBe(true)

    // Attempt 1: delay = 1000 * 2^0 = 1000ms
    await vi.advanceTimersByTimeAsync(999)
    await flushMicrotasks()
    expect(client.POST).toHaveBeenCalledTimes(1)

    await vi.advanceTimersByTimeAsync(1)
    await flushMicrotasks()
    expect(client.POST).toHaveBeenCalledTimes(2) // attempt 1

    wsInstances[1]?.onclose?.()

    // Attempt 2: delay = 1000 * 2^1 = 2000ms
    await vi.advanceTimersByTimeAsync(1999)
    await flushMicrotasks()
    expect(client.POST).toHaveBeenCalledTimes(2)

    await vi.advanceTimersByTimeAsync(1)
    await flushMicrotasks()
    expect(client.POST).toHaveBeenCalledTimes(3) // attempt 2

    wsInstances[2]?.onclose?.()

    // Attempt 3: delay = 1000 * 2^2 = 4000ms
    await vi.advanceTimersByTimeAsync(3999)
    await flushMicrotasks()
    expect(client.POST).toHaveBeenCalledTimes(3)

    await vi.advanceTimersByTimeAsync(1)
    await flushMicrotasks()
    expect(client.POST).toHaveBeenCalledTimes(4) // attempt 3
  })

  it('should stop reconnecting after max attempts (10)', async () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    wsInstances[0]?.onopen?.()
    wsInstances[0]?.onclose?.()

    // 10 reconnect attempts — advance past max delay (30s cap) each time
    for (let i = 0; i < 10; i++) {
      await vi.advanceTimersByTimeAsync(31000)
      await flushMicrotasks()
      wsInstances[i + 1]?.onclose?.()
    }

    // Total POST calls: 1 initial + 10 reconnect attempts = 11
    expect(vi.mocked(client.POST).mock.calls.length).toBe(11)

    const totalCallsAfterMax = vi.mocked(client.POST).mock.calls.length

    // No further reconnects should be scheduled
    await vi.advanceTimersByTimeAsync(31000)
    await flushMicrotasks()

    expect(vi.mocked(client.POST).mock.calls.length).toBe(totalCallsAfterMax)
  })

  it('should handle SyncDiff server messages correctly', async () => {
    const onSyncDiff = vi.fn()
    const { composable } = mountWithWebSocket({ ...defaultOptions, onSyncDiff })

    await composable.connect()
    await flushMicrotasks()

    wsInstances[0]?.onopen?.()

    const update = [1, 2, 3]
    wsInstances[0]?.onmessage?.({ data: JSON.stringify({ type: 'SyncDiff', update }) })

    expect(onSyncDiff).toHaveBeenCalledExactlyOnceWith(new Uint8Array(update))
  })

  it('should handle PeerUpdate server messages correctly', async () => {
    const onPeerUpdate = vi.fn()
    const { composable } = mountWithWebSocket({ ...defaultOptions, onPeerUpdate })

    await composable.connect()
    await flushMicrotasks()

    wsInstances[0]?.onopen?.()

    const update = [4, 5, 6]
    wsInstances[0]?.onmessage?.({ data: JSON.stringify({ type: 'PeerUpdate', update }) })

    expect(onPeerUpdate).toHaveBeenCalledExactlyOnceWith(new Uint8Array(update))
  })

  it('should handle AwarenessChange server messages correctly', async () => {
    const onAwarenessChange = vi.fn()
    const { composable } = mountWithWebSocket({ ...defaultOptions, onAwarenessChange })

    await composable.connect()
    await flushMicrotasks()

    wsInstances[0]?.onopen?.()

    const entries = [
      {
        memberId: 'member-1',
        displayName: 'Alice',
        color: '#ff0000',
        cursorPosition: 5,
        isTyping: true,
      },
    ]
    wsInstances[0]?.onmessage?.({ data: JSON.stringify({ type: 'AwarenessChange', entries }) })

    expect(onAwarenessChange).toHaveBeenCalledExactlyOnceWith(entries)
  })

  it('should handle WriteRejected server messages correctly', async () => {
    const onWriteRejected = vi.fn()
    const { composable } = mountWithWebSocket({ ...defaultOptions, onWriteRejected })

    await composable.connect()
    await flushMicrotasks()

    wsInstances[0]?.onopen?.()

    wsInstances[0]?.onmessage?.({
      data: JSON.stringify({ type: 'WriteRejected', reason: 'conflict', latestMessageId: 'msg-99' }),
    })

    expect(onWriteRejected).toHaveBeenCalledExactlyOnceWith('conflict', 'msg-99')
  })

  it('should ignore malformed server messages without throwing', async () => {
    const onSyncDiff = vi.fn()
    const { composable } = mountWithWebSocket({ ...defaultOptions, onSyncDiff })

    await composable.connect()
    await flushMicrotasks()

    wsInstances[0]?.onopen?.()

    expect(() => {
      wsInstances[0]?.onmessage?.({ data: 'not-valid-json{{{' })
    }).not.toThrow()

    expect(onSyncDiff).not.toHaveBeenCalled()
  })

  it('should disconnect and cleanup', async () => {
    const { composable, wrapper } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    const ws = wsInstances[0]
    ws?.onopen?.()
    expect(composable.connected.value).toBe(true)

    composable.disconnect()

    expect(composable.connected.value).toBe(false)
    expect(composable.reconnecting.value).toBe(false)
    expect(ws?.close).toHaveBeenCalled()

    // Closing the WS after disconnect must not schedule a reconnect
    ws?.onclose?.()
    await vi.advanceTimersByTimeAsync(31000)
    await flushMicrotasks()

    expect(vi.mocked(client.POST).mock.calls.length).toBe(1)

    wrapper.unmount()
  })

  it('should not reconnect after component is unmounted', async () => {
    const { composable, wrapper } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    const ws = wsInstances[0]
    ws?.onopen?.()

    // onUnmounted triggers disconnect; subsequent onclose must not reconnect
    wrapper.unmount()
    ws?.onclose?.()

    await vi.advanceTimersByTimeAsync(31000)
    await flushMicrotasks()

    expect(vi.mocked(client.POST).mock.calls.length).toBe(1)
  })

  it('should send sendSyncStep1 when connected', async () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    const ws = wsInstances[0]
    ws?.onopen?.()

    const stateVector = new Uint8Array([10, 20, 30])
    composable.sendSyncStep1(stateVector)

    expect(ws?.send).toHaveBeenCalledExactlyOnceWith(
      JSON.stringify({ type: 'SyncStep1', state_vector: [10, 20, 30] }),
    )
  })

  it('should send sendSyncStep2 when connected', async () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    const ws = wsInstances[0]
    ws?.onopen?.()

    const update = new Uint8Array([7, 8, 9])
    composable.sendSyncStep2(update)

    expect(ws?.send).toHaveBeenCalledExactlyOnceWith(
      JSON.stringify({ type: 'SyncStep2', update: [7, 8, 9] }),
    )
  })

  it('should send sendAwarenessUpdate with isTyping and cursorPosition when connected', async () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    const ws = wsInstances[0]
    ws?.onopen?.()

    composable.sendAwarenessUpdate(true, 42)

    expect(ws?.send).toHaveBeenCalledExactlyOnceWith(
      JSON.stringify({ type: 'AwarenessUpdate', state: { is_typing: true, cursor_position: 42 } }),
    )
  })

  it('should send sendAwarenessUpdate with null cursor position when not provided', async () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    const ws = wsInstances[0]
    ws?.onopen?.()

    composable.sendAwarenessUpdate(false)

    expect(ws?.send).toHaveBeenCalledExactlyOnceWith(
      JSON.stringify({ type: 'AwarenessUpdate', state: { is_typing: false, cursor_position: null } }),
    )
  })

  it('should not send messages when WebSocket is not connected', () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    // Never call connect — ws is null
    composable.sendSyncStep1(new Uint8Array([1, 2]))
    composable.sendSyncStep2(new Uint8Array([3, 4]))
    composable.sendAwarenessUpdate(true, 0)

    expect(wsInstances).toHaveLength(0)
  })

  it('should build a WebSocket URL with the correct project and task IDs', async () => {
    const { composable } = mountWithWebSocket({ projectId: 'proj-42', taskId: 'task-99' })

    await composable.connect()
    await flushMicrotasks()

    expect(wsInstances).toHaveLength(1)
    const url = wsInstances[0]?.url ?? ''
    expect(url).toMatch(/^ws/)
    expect(url).toContain('/api/v1/projects/proj-42/tasks/task-99/conversation/ws')
    expect(url).toContain('token=test-token')
  })

  it('should schedule reconnect if token POST fails during connect', async () => {
    vi.mocked(client.POST).mockRejectedValueOnce(new Error('network error'))

    const { composable } = mountWithWebSocket(defaultOptions)
    await composable.connect()
    await flushMicrotasks()

    expect(composable.reconnecting.value).toBe(true)

    vi.mocked(client.POST).mockResolvedValueOnce({ data: { token: 'test-token' } } as never)
    await vi.advanceTimersByTimeAsync(1000)
    await flushMicrotasks()

    expect(vi.mocked(client.POST).mock.calls.length).toBe(2)
  })

  it('should trigger onerror by closing the WebSocket', async () => {
    const { composable } = mountWithWebSocket(defaultOptions)

    await composable.connect()
    await flushMicrotasks()

    const ws = wsInstances[0]
    ws?.onopen?.()
    expect(composable.connected.value).toBe(true)

    // Trigger onerror — the composable calls ws.close() internally
    ws?.onerror?.()

    expect(ws?.close).toHaveBeenCalled()
  })
})
