import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import ProjectSettingsView from '../ProjectSettingsView.vue'

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

function mockProject(status: string) {
  return {
    id: 'proj-1',
    organizationId: 'org-1',
    name: 'Test Project',
    description: 'A test project',
    status,
    sourceProjectId: null,
    permissionSettings: {},
    createdBy: 'user-1',
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
  }
}

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/projects/:projectId/settings', name: 'project-settings', component: { template: '<div />' } },
      { path: '/organizations', name: 'organizations', component: { template: '<div />' } },
    ],
  })
}

describe('ProjectSettingsView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Project Settings heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: mockProject('preparing') } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/settings')
    await router.isReady()

    const wrapper = mount(ProjectSettingsView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Project Settings')
  })

  it('shows project data after loading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: mockProject('preparing') } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/settings')
    await router.isReady()

    const wrapper = mount(ProjectSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    const inputs = wrapper.findAll('input')
    const nameInput = inputs.find((i) => i.element.value === 'Test Project')
    const descInput = inputs.find((i) => i.element.value === 'A test project')
    expect(nameInput).toBeTruthy()
    expect(descInput).toBeTruthy()
  })

  it('shows Activate button when status is preparing', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: mockProject('preparing') } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/settings')
    await router.isReady()

    const wrapper = mount(ProjectSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    const activateButton = wrapper.findAll('button').find((b) => b.text() === 'Activate')
    expect(activateButton).toBeTruthy()
  })

  it('shows Complete and Archive buttons when status is active', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: mockProject('active') } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/settings')
    await router.isReady()

    const wrapper = mount(ProjectSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    const completeButton = wrapper.findAll('button').find((b) => b.text() === 'Complete')
    const archiveButton = wrapper.findAll('button').find((b) => b.text() === 'Archive')
    expect(completeButton).toBeTruthy()
    expect(archiveButton).toBeTruthy()
  })

  it('does not show Archive button when status is archived', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: mockProject('archived') } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/settings')
    await router.isReady()

    const wrapper = mount(ProjectSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    const archiveButton = wrapper.findAll('button').find((b) => b.text() === 'Archive')
    expect(archiveButton).toBeUndefined()
  })

  it('renders danger zone delete button', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: mockProject('preparing') } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/settings')
    await router.isReady()

    const wrapper = mount(ProjectSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Danger Zone')
    const deleteButton = wrapper.findAll('button').find((b) => b.text() === 'Delete Project')
    expect(deleteButton).toBeTruthy()
  })
})
