import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import MemberTagsView from '../MemberTagsView.vue'

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
      { path: '/projects/:projectId/member-tags', name: 'project-member-tags', component: { template: '<div />' } },
    ],
  })
}

describe('MemberTagsView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Member Tags heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/member-tags')
    await router.isReady()

    const wrapper = mount(MemberTagsView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Member Tags')
  })

  it('renders tag list after loading', async () => {
    const mockTags = [
      { id: 't1', projectId: 'proj-1', name: 'dev', description: 'Developers', memberCount: 3, contactCount: 1, createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: mockTags } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/member-tags')
    await router.isReady()

    const wrapper = mount(MemberTagsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('dev')
    expect(wrapper.text()).toContain('Developers')
    expect(wrapper.text()).toContain('3 members, 1 contacts')
  })

  it('shows empty message when no tags', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/member-tags')
    await router.isReady()

    const wrapper = mount(MemberTagsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('No tags yet.')
  })

  it('renders create tag form', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/member-tags')
    await router.isReady()

    const wrapper = mount(MemberTagsView, {
      global: { plugins: [router] },
    })

    expect(wrapper.text()).toContain('Create Tag')
    const createButton = wrapper.findAll('button').find((b) => b.text() === 'Create')
    expect(createButton).toBeTruthy()
  })

  it('does not show tag detail when no tag is selected', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/member-tags')
    await router.isReady()

    const wrapper = mount(MemberTagsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).not.toContain('Tag Detail')
  })
})
