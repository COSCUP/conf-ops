import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import LibraryDocumentView from '../LibraryDocumentView.vue'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
    use: vi.fn(),
  },
  setupAuthInterceptor: vi.fn(),
}))

import client from '@/api/client'

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      {
        path: '/organizations/:orgId/library-documents',
        name: 'library-documents',
        component: { template: '<div />' },
      },
    ],
  })
}

const mockDocuments = [
  {
    id: 'doc-1',
    title: 'SOP Guide',
    content: '# Standard Operating Procedure\n\nStep 1: Do this',
    scopeType: 'organization',
    scopeId: 'org-1',
    createdBy: 'user-1',
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-02T00:00:00Z',
  },
  {
    id: 'doc-2',
    title: 'Welcome Template',
    content: 'Hello and welcome!',
    scopeType: 'organization',
    scopeId: 'org-1',
    createdBy: 'user-1',
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-03T00:00:00Z',
  },
]

function mockGetLibraryDocuments(items = mockDocuments) {
  vi.mocked(client.GET).mockResolvedValue({
    data: { items, nextCursor: null },
  } as never)
}

describe('LibraryDocumentView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Library Documents heading', async () => {
    mockGetLibraryDocuments([])

    const router = createTestRouter()
    await router.push('/organizations/org-1/library-documents')
    await router.isReady()

    const wrapper = mount(LibraryDocumentView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Library Documents')
  })

  it('renders document list after loading', async () => {
    mockGetLibraryDocuments()

    const router = createTestRouter()
    await router.push('/organizations/org-1/library-documents')
    await router.isReady()

    const wrapper = mount(LibraryDocumentView, {
      global: { plugins: [router] },
    })
    await flushPromises()

    expect(wrapper.findAll('.document-item')).toHaveLength(2)
    expect(wrapper.text()).toContain('SOP Guide')
    expect(wrapper.text()).toContain('Welcome Template')
  })

  it('shows empty message when no documents', async () => {
    mockGetLibraryDocuments([])

    const router = createTestRouter()
    await router.push('/organizations/org-1/library-documents')
    await router.isReady()

    const wrapper = mount(LibraryDocumentView, {
      global: { plugins: [router] },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('No library documents yet.')
  })

  it('toggles create form on button click', async () => {
    mockGetLibraryDocuments([])

    const router = createTestRouter()
    await router.push('/organizations/org-1/library-documents')
    await router.isReady()

    const wrapper = mount(LibraryDocumentView, {
      global: { plugins: [router] },
    })
    await flushPromises()

    expect(wrapper.find('.form-vertical').exists()).toBe(false)

    await wrapper.find('.view-actions button').trigger('click')
    expect(wrapper.find('.form-vertical').exists()).toBe(true)
  })

  it('renders edit and preview tabs in create form', async () => {
    mockGetLibraryDocuments([])

    const router = createTestRouter()
    await router.push('/organizations/org-1/library-documents')
    await router.isReady()

    const wrapper = mount(LibraryDocumentView, {
      global: { plugins: [router] },
    })
    await flushPromises()

    await wrapper.find('.view-actions button').trigger('click')

    const tabs = wrapper.findAll('.tab-btn')
    expect(tabs).toHaveLength(2)
    expect(tabs[0]!.text()).toBe('Edit')
    expect(tabs[1]!.text()).toBe('Preview')
  })

  it('switches to markdown preview in create form', async () => {
    mockGetLibraryDocuments([])

    const router = createTestRouter()
    await router.push('/organizations/org-1/library-documents')
    await router.isReady()

    const wrapper = mount(LibraryDocumentView, {
      global: { plugins: [router] },
    })
    await flushPromises()

    await wrapper.find('.view-actions button').trigger('click')

    expect(wrapper.find('.form-textarea--md').exists()).toBe(true)
    expect(wrapper.find('.markdown-preview').exists()).toBe(false)

    await wrapper.findAll('.tab-btn')[1]!.trigger('click')
    expect(wrapper.find('.form-textarea--md').exists()).toBe(false)
    expect(wrapper.find('.markdown-preview').exists()).toBe(true)
  })

  it('renders document actions (Edit, Versions, Delete)', async () => {
    mockGetLibraryDocuments()

    const router = createTestRouter()
    await router.push('/organizations/org-1/library-documents')
    await router.isReady()

    const wrapper = mount(LibraryDocumentView, {
      global: { plugins: [router] },
    })
    await flushPromises()

    const actions = wrapper.findAll('.document-actions button')
    expect(actions.length).toBe(6)
  })

  it('enters edit mode when Edit is clicked', async () => {
    mockGetLibraryDocuments()

    const router = createTestRouter()
    await router.push('/organizations/org-1/library-documents')
    await router.isReady()

    const wrapper = mount(LibraryDocumentView, {
      global: { plugins: [router] },
    })
    await flushPromises()

    const editBtn = wrapper.findAll('.document-actions button')[0]!
    await editBtn.trigger('click')

    expect(wrapper.text()).toContain('Editing: SOP Guide')
    expect(wrapper.findAll('.editor-tabs')).toHaveLength(1)
  })

  it('toggles version history', async () => {
    mockGetLibraryDocuments()
    // Mock version history GET (second call)
    vi.mocked(client.GET).mockResolvedValueOnce({
      data: { items: mockDocuments, nextCursor: null },
    } as never).mockResolvedValueOnce({
      data: [],
    } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/library-documents')
    await router.isReady()

    const wrapper = mount(LibraryDocumentView, {
      global: { plugins: [router] },
    })
    await flushPromises()

    const versionsBtn = wrapper.findAll('.document-actions button')[1]!
    expect(wrapper.find('.versions-panel').exists()).toBe(false)

    await versionsBtn.trigger('click')
    await flushPromises()
    expect(wrapper.find('.versions-panel').exists()).toBe(true)
    expect(wrapper.text()).toContain('Version History')
  })
})
