import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import DataEntryForm from '../data-schema/DataEntryForm.vue'

const textSchema = {
  id: 'schema-1',
  taskTemplateId: 'tmpl-1',
  name: 'Contact Info',
  fields: [
    {
      key: 'name',
      label: 'Full Name',
      description: 'Enter your full name',
      type: 'single_line_text' as const,
      required: true,
    },
    {
      key: 'bio',
      label: 'Bio',
      description: 'Short biography',
      type: 'multi_line_text' as const,
      required: false,
    },
  ],
  createdAt: '2025-01-01T00:00:00Z',
  updatedAt: '2025-01-01T00:00:00Z',
}

const numberSchema = {
  id: 'schema-2',
  taskTemplateId: 'tmpl-1',
  name: 'Metrics',
  fields: [
    {
      key: 'age',
      label: 'Age',
      description: 'Age in years',
      type: 'number' as const,
      required: false,
      constraints: { min: 0, max: 200 },
    },
    {
      key: 'active',
      label: 'Active',
      description: 'Is active',
      type: 'boolean' as const,
      required: false,
    },
    {
      key: 'role',
      label: 'Role',
      description: 'Select role',
      type: 'select' as const,
      required: true,
      constraints: { options: ['Admin', 'User', 'Guest'] },
    },
  ],
  createdAt: '2025-01-01T00:00:00Z',
  updatedAt: '2025-01-01T00:00:00Z',
}

function createWrapper(props?: Record<string, unknown>) {
  return mount(DataEntryForm, {
    props: {
      schemas: [],
      entries: [],
      ...props,
    } as InstanceType<typeof DataEntryForm>['$props'],
  })
}

describe('DataEntryForm', () => {
  it('shows empty message when no schemas', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('No data schemas defined for this template.')
  })

  it('renders schema name as title', () => {
    const wrapper = createWrapper({ schemas: [textSchema] })
    expect(wrapper.text()).toContain('Contact Info')
  })

  it('renders different field types', () => {
    const wrapper = createWrapper({ schemas: [textSchema, numberSchema] })

    // Text fields
    expect(wrapper.text()).toContain('Full Name')
    expect(wrapper.text()).toContain('Bio')

    // Number / boolean / select
    expect(wrapper.text()).toContain('Age')
    expect(wrapper.text()).toContain('Active')
    expect(wrapper.text()).toContain('Role')

    // Textarea for multi_line_text
    expect(wrapper.find('textarea').exists()).toBe(true)

    // Number input
    expect(wrapper.find('input[type="number"]').exists()).toBe(true)

    // Checkbox for boolean
    expect(wrapper.find('input[type="checkbox"]').exists()).toBe(true)

    // Select for select type
    expect(wrapper.find('select').exists()).toBe(true)
  })

  it('renders select options from constraints', () => {
    const wrapper = createWrapper({ schemas: [numberSchema] })
    const options = wrapper.findAll('select option')
    // First option is placeholder "-- Select --"
    expect(options.length).toBe(4)
    expect(options[1]!.text()).toBe('Admin')
    expect(options[2]!.text()).toBe('User')
    expect(options[3]!.text()).toBe('Guest')
  })

  it('displays existing entry values', () => {
    const entries = [
      {
        id: 'e1',
        taskId: 'task1',
        dataSchemaId: 'schema-1',
        values: { name: 'Alice', bio: 'Hello world' },
        sourceLinks: null,
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    ]

    const wrapper = createWrapper({ schemas: [textSchema], entries })

    // The input should have the value "Alice"
    const inputs = wrapper.findAll('input')
    const nameInput = inputs.find((i) => i.element.value === 'Alice')
    expect(nameInput).toBeTruthy()

    // Textarea should have "Hello world"
    const textarea = wrapper.find('textarea')
    expect(textarea.element.value).toBe('Hello world')
  })

  it('emits save with form values on submit', async () => {
    const wrapper = createWrapper({ schemas: [textSchema] })

    // Fill in name field
    const nameInput = wrapper.find('.schema-form input')
    await nameInput.setValue('Bob')

    // Submit form
    await wrapper.find('.schema-form').trigger('submit')

    expect(wrapper.emitted('save')).toBeTruthy()
    const emitted = wrapper.emitted('save')![0]!
    expect(emitted[0]).toBe('schema-1')
    expect(emitted[1]).toEqual({ name: 'Bob' })
  })

  it('shows required mark for required fields', () => {
    const wrapper = createWrapper({ schemas: [textSchema] })
    const requiredMarks = wrapper.findAll('.required-mark')
    expect(requiredMarks.length).toBe(1)
  })

  it('shows field descriptions', () => {
    const wrapper = createWrapper({ schemas: [textSchema] })
    expect(wrapper.text()).toContain('Enter your full name')
    expect(wrapper.text()).toContain('Short biography')
  })
})
