import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import ProjectListView from '../ProjectListView.vue'

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
      { path: '/organizations/:orgId/projects', name: 'org-projects', component: { template: '<div />' } },
      { path: '/projects/:projectId', name: 'project-dashboard', component: { template: '<div />' } },
    ],
  })
}

describe('ProjectListView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders projects heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { projects: [] } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/projects')
    await router.isReady()

    const wrapper = mount(ProjectListView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Projects')
  })

  it('shows empty state when no projects', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { projects: [] } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/projects')
    await router.isReady()

    const wrapper = mount(ProjectListView, {
      global: { plugins: [router] },
    })

    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('No projects yet')
  })

  it('renders project cards', async () => {
    const mockProjects = [
      { id: '1', name: 'Test Project', description: 'A test', status: 'preparing', createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { projects: mockProjects } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/projects')
    await router.isReady()

    const wrapper = mount(ProjectListView, {
      global: { plugins: [router] },
    })

    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('Test Project')
    expect(wrapper.text()).toContain('preparing')
  })

  it('shows Copy Project button', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { projects: [] } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/projects')
    await router.isReady()

    const wrapper = mount(ProjectListView, {
      global: { plugins: [router] },
    })

    const buttons = wrapper.findAll('button')
    const copyButton = buttons.find((b) => b.text() === 'Copy Project')
    expect(copyButton).toBeTruthy()
  })

  it('shows copy form when Copy Project clicked', async () => {
    const mockProjects = [
      { id: '1', name: 'Source Project', description: null, status: 'active', createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { projects: mockProjects } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/projects')
    await router.isReady()

    const wrapper = mount(ProjectListView, {
      global: { plugins: [router] },
    })

    await vi.dynamicImportSettled()

    const copyButton = wrapper.findAll('button').find((b) => b.text() === 'Copy Project')
    await copyButton!.trigger('click')

    expect(wrapper.text()).toContain('Copy Project')
    expect(wrapper.text()).toContain('Source Project')
    expect(wrapper.text()).toContain('New Project Name')
  })

  it('copy form has source project options', async () => {
    const mockProjects = [
      { id: 'p1', name: 'Project Alpha', description: null, status: 'active', createdAt: '', updatedAt: '' },
      { id: 'p2', name: 'Project Beta', description: null, status: 'preparing', createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { projects: mockProjects } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/projects')
    await router.isReady()

    const wrapper = mount(ProjectListView, {
      global: { plugins: [router] },
    })

    await vi.dynamicImportSettled()

    const copyButton = wrapper.findAll('button').find((b) => b.text() === 'Copy Project')
    await copyButton!.trigger('click')

    const options = wrapper.findAll('.source-select option')
    // 1 placeholder + 2 project options
    expect(options.length).toBe(3)
    expect(options[1]!.text()).toBe('Project Alpha')
    expect(options[2]!.text()).toBe('Project Beta')
  })
})
