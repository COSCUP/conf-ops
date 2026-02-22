import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import ManualToolDialog from '../ManualToolDialog.vue'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

describe('ManualToolDialog', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders when visible', () => {
    const wrapper = mount(ManualToolDialog, {
      props: {
        projectId: 'project-1',
        taskId: 'task-1',
        visible: true,
      },
    })

    expect(wrapper.find('.dialog-overlay').exists()).toBe(true)
    expect(wrapper.find('.dialog-header h2').text()).toBe('Execute Tool')
  })

  it('does not render when not visible', () => {
    const wrapper = mount(ManualToolDialog, {
      props: {
        projectId: 'project-1',
        taskId: 'task-1',
        visible: false,
      },
    })

    expect(wrapper.find('.dialog-overlay').exists()).toBe(false)
  })

  it('shows tool select dropdown', () => {
    const wrapper = mount(ManualToolDialog, {
      props: {
        projectId: 'project-1',
        taskId: 'task-1',
        visible: true,
      },
    })

    expect(wrapper.find('#tool-select').exists()).toBe(true)
  })

  it('emits close when cancel is clicked', async () => {
    const wrapper = mount(ManualToolDialog, {
      props: {
        projectId: 'project-1',
        taskId: 'task-1',
        visible: true,
      },
    })

    const cancelButton = wrapper.findAll('.btn-secondary').find((b) => b.text() === 'Cancel')
    expect(cancelButton).toBeDefined()
    await cancelButton?.trigger('click')

    expect(wrapper.emitted('close')).toBeDefined()
  })

  it('disables execute button when no tool selected', () => {
    const wrapper = mount(ManualToolDialog, {
      props: {
        projectId: 'project-1',
        taskId: 'task-1',
        visible: true,
      },
    })

    const executeButton = wrapper.find('.btn-primary')
    expect((executeButton.element as HTMLButtonElement).disabled).toBe(true)
  })
})
