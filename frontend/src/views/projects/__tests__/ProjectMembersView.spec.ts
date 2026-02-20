import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import ProjectMembersView from '../ProjectMembersView.vue'

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
      { path: '/projects/:projectId/members', name: 'project-members', component: { template: '<div />' } },
    ],
  })
}

describe('ProjectMembersView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Project Members heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { members: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/members')
    await router.isReady()

    const wrapper = mount(ProjectMembersView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Project Members')
  })

  it('renders member list after loading', async () => {
    const mockMembers = [
      { id: 'm1', projectId: 'proj-1', accountId: 'a1', role: 'owner', name: 'Alice', email: 'alice@test.com', avatarUrl: null, createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { members: mockMembers } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/members')
    await router.isReady()

    const wrapper = mount(ProjectMembersView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Alice')
    expect(wrapper.text()).toContain('alice@test.com')
  })

  it('shows empty message when no members', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { members: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/members')
    await router.isReady()

    const wrapper = mount(ProjectMembersView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('No members yet.')
  })

  it('renders invite form', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { members: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/members')
    await router.isReady()

    const wrapper = mount(ProjectMembersView, {
      global: { plugins: [router] },
    })

    expect(wrapper.text()).toContain('Invite Member')
    const inviteButton = wrapper.findAll('button').find((b) => b.text() === 'Invite')
    expect(inviteButton).toBeTruthy()
  })

  it('renders role badge for members', async () => {
    const mockMembers = [
      { id: 'm1', projectId: 'proj-1', accountId: 'a1', role: 'tag_admin', name: 'Bob', email: 'bob@test.com', avatarUrl: null, createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { members: mockMembers } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/members')
    await router.isReady()

    const wrapper = mount(ProjectMembersView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    const badge = wrapper.find('.role-badge')
    expect(badge.exists()).toBe(true)
    expect(badge.text()).toBe('tag_admin')
  })
})
