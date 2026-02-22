import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import MemoryPanel from '../MemoryPanel.vue'
import { useMemoryStore } from '@/stores/memory'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

const makeMemory = (overrides = {}) => ({
  id: 'mem1',
  content: 'Test memory content',
  source: 'manual',
  scopeType: 'organization',
  scopeId: 'org1',
  createdBy: 'user1',
  libraryRef: null,
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
  ...overrides,
})

describe('MemoryPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.mocked(client.GET).mockResolvedValue({ data: { items: [], nextCursor: null } } as never)
  })

  describe('single-scope mode', () => {
    it('renders the panel header with title', async () => {
      const wrapper = mount(MemoryPanel, {
        props: { scopeType: 'organization', scopeId: 'org1' },
      })
      await wrapper.vm.$nextTick()

      expect(wrapper.find('h3').text()).toBe('Memories')
    })

    it('shows Add Memory button', async () => {
      const wrapper = mount(MemoryPanel, {
        props: { scopeType: 'organization', scopeId: 'org1' },
      })
      await wrapper.vm.$nextTick()

      expect(wrapper.text()).toContain('Add Memory')
    })

    it('calls listMemories on mount', async () => {
      mount(MemoryPanel, {
        props: { scopeType: 'organization', scopeId: 'org1' },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(client.GET).toHaveBeenCalledWith('/api/v1/memories', expect.objectContaining({
        params: { query: { scopeType: 'organization', scopeId: 'org1' } },
      }))
    })

    it('shows empty text when no memories', async () => {
      const wrapper = mount(MemoryPanel, {
        props: { scopeType: 'organization', scopeId: 'org1' },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))
      await wrapper.vm.$nextTick()

      expect(wrapper.text()).toContain('No memories yet.')
    })

    it('displays memories when store has items', async () => {
      const mockMemories = [makeMemory({ content: 'Remember this important thing' })]
      vi.mocked(client.GET).mockResolvedValue({ data: { items: mockMemories, nextCursor: null } } as never)

      const wrapper = mount(MemoryPanel, {
        props: { scopeType: 'organization', scopeId: 'org1' },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))
      await wrapper.vm.$nextTick()

      expect(wrapper.text()).toContain('Remember this important thing')
      expect(wrapper.text()).toContain('manual')
    })

    it('toggles add form on button click', async () => {
      const wrapper = mount(MemoryPanel, {
        props: { scopeType: 'organization', scopeId: 'org1' },
      })
      await wrapper.vm.$nextTick()

      expect(wrapper.text()).not.toContain('Cancel')

      const addButton = wrapper.find('button')
      await addButton.trigger('click')
      await wrapper.vm.$nextTick()

      expect(wrapper.text()).toContain('Cancel')
    })

    it('shows error message when store has error', async () => {
      vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

      const wrapper = mount(MemoryPanel, {
        props: { scopeType: 'organization', scopeId: 'org1' },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))
      await wrapper.vm.$nextTick()

      expect(wrapper.text()).toContain('Failed to load memories.')
    })

    it('renders Edit and Delete buttons for each memory', async () => {
      const mockMemories = [makeMemory()]
      vi.mocked(client.GET).mockResolvedValue({ data: { items: mockMemories, nextCursor: null } } as never)

      const wrapper = mount(MemoryPanel, {
        props: { scopeType: 'organization', scopeId: 'org1' },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))
      await wrapper.vm.$nextTick()

      const buttons = wrapper.findAll('button')
      const buttonTexts = buttons.map((b) => b.text())
      expect(buttonTexts).toContain('Edit')
      expect(buttonTexts).toContain('Delete')
    })

    it('truncates long memory content', async () => {
      const longContent = 'A'.repeat(200)
      const mockMemories = [makeMemory({ content: longContent })]
      vi.mocked(client.GET).mockResolvedValue({ data: { items: mockMemories, nextCursor: null } } as never)

      const store = useMemoryStore()
      store.memories = mockMemories

      const wrapper = mount(MemoryPanel, {
        props: { scopeType: 'organization', scopeId: 'org1' },
      })
      await wrapper.vm.$nextTick()

      const memoryText = wrapper.find('.memory-text')
      expect(memoryText.text()).toContain('...')
      expect(memoryText.text().length).toBeLessThan(longContent.length)
    })

    it('calls deleteMemory when Delete button is clicked', async () => {
      const mockMemories = [makeMemory({ id: 'mem1' })]
      vi.mocked(client.GET).mockResolvedValue({ data: { items: mockMemories, nextCursor: null } } as never)
      vi.mocked(client.DELETE).mockResolvedValue({} as never)

      const wrapper = mount(MemoryPanel, {
        props: { scopeType: 'organization', scopeId: 'org1' },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))
      await wrapper.vm.$nextTick()

      const deleteButton = wrapper.findAll('button').find((b) => b.text() === 'Delete')
      expect(deleteButton).toBeDefined()
      await deleteButton!.trigger('click')
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(client.DELETE).toHaveBeenCalledWith('/api/v1/memories/{memoryId}', expect.objectContaining({
        params: { path: { memoryId: 'mem1' } },
      }))
    })
  })

  describe('grouped mode (six-level display)', () => {
    const taskContext = {
      accountId: 'acc1',
      organizationId: 'org1',
      projectId: 'proj1',
      memberTagId: 'tag1',
      taskTemplateId: 'tmpl1',
      taskId: 'task1',
    }

    it('loads memories for all 6 scopes on mount', async () => {
      mount(MemoryPanel, {
        props: { taskContext },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(client.GET).toHaveBeenCalledTimes(6)
      expect(client.GET).toHaveBeenCalledWith('/api/v1/memories', expect.objectContaining({
        params: { query: { scopeType: 'account', scopeId: 'acc1' } },
      }))
      expect(client.GET).toHaveBeenCalledWith('/api/v1/memories', expect.objectContaining({
        params: { query: { scopeType: 'organization', scopeId: 'org1' } },
      }))
      expect(client.GET).toHaveBeenCalledWith('/api/v1/memories', expect.objectContaining({
        params: { query: { scopeType: 'project', scopeId: 'proj1' } },
      }))
      expect(client.GET).toHaveBeenCalledWith('/api/v1/memories', expect.objectContaining({
        params: { query: { scopeType: 'member_tag', scopeId: 'tag1' } },
      }))
      expect(client.GET).toHaveBeenCalledWith('/api/v1/memories', expect.objectContaining({
        params: { query: { scopeType: 'task_template', scopeId: 'tmpl1' } },
      }))
      expect(client.GET).toHaveBeenCalledWith('/api/v1/memories', expect.objectContaining({
        params: { query: { scopeType: 'task', scopeId: 'task1' } },
      }))
    })

    it('displays scope group headers with memory counts', async () => {
      const orgMemories = [makeMemory({ id: 'mem1', scopeType: 'organization' })]
      const taskMemories = [
        makeMemory({ id: 'mem2', scopeType: 'task', content: 'Task note 1' }),
        makeMemory({ id: 'mem3', scopeType: 'task', content: 'Task note 2' }),
      ]

      vi.mocked(client.GET).mockImplementation((_path: string, options?: unknown) => {
        const opts = options as { params?: { query?: { scopeType?: string } } } | undefined
        const scopeType = opts?.params?.query?.scopeType
        if (scopeType === 'organization') {
          return Promise.resolve({ data: { items: orgMemories, nextCursor: null } } as never)
        }
        if (scopeType === 'task') {
          return Promise.resolve({ data: { items: taskMemories, nextCursor: null } } as never)
        }
        return Promise.resolve({ data: { items: [], nextCursor: null } } as never)
      })

      const wrapper = mount(MemoryPanel, {
        props: { taskContext },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))
      await wrapper.vm.$nextTick()

      const headers = wrapper.findAll('.scope-header')
      expect(headers.length).toBe(2)

      const headerTexts = headers.map((h) => h.text())
      expect(headerTexts.some((t) => t.includes('Organization') && t.includes('(1)'))).toBe(true)
      expect(headerTexts.some((t) => t.includes('Task') && t.includes('(2)'))).toBe(true)
    })

    it('shows empty scopes summary', async () => {
      const wrapper = mount(MemoryPanel, {
        props: { taskContext },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))
      await wrapper.vm.$nextTick()

      expect(wrapper.text()).toContain('No memories yet.')
    })

    it('collapses and expands scope groups on click', async () => {
      const orgMemories = [makeMemory({ id: 'mem1', scopeType: 'organization', content: 'Org memory' })]

      vi.mocked(client.GET).mockImplementation((_path: string, options?: unknown) => {
        const opts = options as { params?: { query?: { scopeType?: string } } } | undefined
        const scopeType = opts?.params?.query?.scopeType
        if (scopeType === 'organization') {
          return Promise.resolve({ data: { items: orgMemories, nextCursor: null } } as never)
        }
        return Promise.resolve({ data: { items: [], nextCursor: null } } as never)
      })

      const wrapper = mount(MemoryPanel, {
        props: { taskContext },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))
      await wrapper.vm.$nextTick()

      expect(wrapper.text()).toContain('Org memory')

      const header = wrapper.find('.scope-header')
      await header.trigger('click')
      await wrapper.vm.$nextTick()

      expect(wrapper.text()).not.toContain('Org memory')

      await header.trigger('click')
      await wrapper.vm.$nextTick()

      expect(wrapper.text()).toContain('Org memory')
    })

    it('skips optional scopes when not provided in context', async () => {
      const minimalContext = {
        accountId: 'acc1',
        organizationId: 'org1',
        projectId: 'proj1',
        taskId: 'task1',
      }

      mount(MemoryPanel, {
        props: { taskContext: minimalContext },
      })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(client.GET).toHaveBeenCalledTimes(4)
    })

    it('shows scope selector in add form', async () => {
      const wrapper = mount(MemoryPanel, {
        props: { taskContext },
      })
      await wrapper.vm.$nextTick()

      const addButton = wrapper.findAll('button').find((b) => b.text() === 'Add Memory')
      await addButton!.trigger('click')
      await wrapper.vm.$nextTick()

      expect(wrapper.find('.form-select').exists()).toBe(true)
      const options = wrapper.findAll('.form-select option')
      expect(options.length).toBeGreaterThan(1)
    })
  })
})
