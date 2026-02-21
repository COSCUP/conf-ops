import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import DataEntryForm from '../data-schema/DataEntryForm.vue'

function createWrapper(props?: Record<string, unknown>) {
  return mount(DataEntryForm, {
    props: {
      entries: [],
      ...props,
    } as InstanceType<typeof DataEntryForm>['$props'],
  })
}

describe('DataEntryForm', () => {
  it('renders entry schema ID', () => {
    const entries = [
      {
        id: 'e1',
        taskId: 'task1',
        dataSchemaId: 'schema-abc-123',
        values: { name: 'Test' },
        sourceLinks: null,
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    ]

    const wrapper = createWrapper({ entries })
    expect(wrapper.text()).toContain('schema-abc-123')
  })

  it('renders JSON values in a pre element', () => {
    const entries = [
      {
        id: 'e1',
        taskId: 'task1',
        dataSchemaId: 'schema-1',
        values: { foo: 'bar', count: 42 },
        sourceLinks: null,
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    ]

    const wrapper = createWrapper({ entries })
    const pre = wrapper.find('pre')
    expect(pre.exists()).toBe(true)
    expect(pre.text()).toContain('"foo"')
    expect(pre.text()).toContain('"bar"')
    expect(pre.text()).toContain('42')
  })

  it('shows empty message when no entries', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('No data entries yet.')
  })
})
