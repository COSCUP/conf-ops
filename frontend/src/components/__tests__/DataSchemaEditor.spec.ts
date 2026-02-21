import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import DataSchemaEditor from '../data-schema/DataSchemaEditor.vue'

function createWrapper(props?: Record<string, unknown>) {
  return mount(DataSchemaEditor, {
    props: {
      schemas: [],
      loading: false,
      ...props,
    } as InstanceType<typeof DataSchemaEditor>['$props'],
  })
}

describe('DataSchemaEditor', () => {
  it('renders Schema Name input and action buttons', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('Schema Name')
    expect(wrapper.text()).toContain('Add Field')
    expect(wrapper.text()).toContain('Create Schema')
  })

  it('adds a field row when Add Field is clicked', async () => {
    const wrapper = createWrapper()

    const addBtn = wrapper.findAll('button').find((b) => b.text() === 'Add Field')!
    await addBtn.trigger('click')

    expect(wrapper.text()).toContain('Key')
    expect(wrapper.text()).toContain('Label')
    expect(wrapper.text()).toContain('Type')
    expect(wrapper.findAll('.field-row')).toHaveLength(1)
  })

  it('field type select contains all 10 types', async () => {
    const wrapper = createWrapper()

    const addBtn = wrapper.findAll('button').find((b) => b.text() === 'Add Field')!
    await addBtn.trigger('click')

    const options = wrapper.findAll('select option')
    expect(options).toHaveLength(10)
    const values = options.map((o) => o.attributes('value'))
    expect(values).toContain('single_line_text')
    expect(values).toContain('multi_line_text')
    expect(values).toContain('number')
    expect(values).toContain('date')
    expect(values).toContain('email')
    expect(values).toContain('url')
    expect(values).toContain('select')
    expect(values).toContain('boolean')
    expect(values).toContain('image')
    expect(values).toContain('file')
  })

  it('removes a field row when Remove is clicked', async () => {
    const wrapper = createWrapper()

    const addBtn = wrapper.findAll('button').find((b) => b.text() === 'Add Field')!
    await addBtn.trigger('click')
    expect(wrapper.findAll('.field-row')).toHaveLength(1)

    const removeBtn = wrapper.findAll('button').find((b) => b.text() === 'Remove')!
    await removeBtn.trigger('click')
    expect(wrapper.findAll('.field-row')).toHaveLength(0)
  })

  it('emits create with name and fields on submit', async () => {
    const wrapper = createWrapper()

    // Set name
    const nameInput = wrapper.find('input[placeholder="Schema name"]')
    await nameInput.setValue('TestSchema')

    // Add a field
    const addBtn = wrapper.findAll('button').find((b) => b.text() === 'Add Field')!
    await addBtn.trigger('click')

    // Fill field key
    const keyInput = wrapper.find('input[placeholder="field_key"]')
    await keyInput.setValue('my_field')

    // Submit
    const createBtn = wrapper.findAll('button').find((b) => b.text() === 'Create Schema')!
    await createBtn.trigger('click')

    expect(wrapper.emitted('create')).toHaveLength(1)
    const [name, fields] = wrapper.emitted('create')![0]! as [string, unknown[]]
    expect(name).toBe('TestSchema')
    expect(fields).toHaveLength(1)
  })

  it('does not emit create when name is empty', async () => {
    const wrapper = createWrapper()

    const addBtn = wrapper.findAll('button').find((b) => b.text() === 'Add Field')!
    await addBtn.trigger('click')

    const createBtn = wrapper.findAll('button').find((b) => b.text() === 'Create Schema')!
    await createBtn.trigger('click')

    expect(wrapper.emitted('create')).toBeUndefined()
  })

  it('does not emit create when no fields', async () => {
    const wrapper = createWrapper()

    const nameInput = wrapper.find('input[placeholder="Schema name"]')
    await nameInput.setValue('TestSchema')

    const createBtn = wrapper.findAll('button').find((b) => b.text() === 'Create Schema')!
    await createBtn.trigger('click')

    expect(wrapper.emitted('create')).toBeUndefined()
  })

  it('renders existing schemas list', () => {
    const schemas = [
      {
        id: 's1',
        taskTemplateId: 'tt1',
        name: 'Survey',
        fields: [
          { key: 'q1', label: 'Question 1', description: '', type: 'single_line_text', required: false, constraints: null },
        ],
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    ]

    const wrapper = createWrapper({ schemas })
    expect(wrapper.text()).toContain('Survey')
    expect(wrapper.text()).toContain('1 fields')
  })

  it('emits delete when Delete button is clicked', async () => {
    const schemas = [
      {
        id: 's1',
        taskTemplateId: 'tt1',
        name: 'Schema1',
        fields: [],
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    ]

    const wrapper = createWrapper({ schemas })

    const deleteBtn = wrapper.findAll('button').find((b) => b.text() === 'Delete')!
    await deleteBtn.trigger('click')

    expect(wrapper.emitted('delete')).toHaveLength(1)
    expect(wrapper.emitted('delete')![0]).toEqual(['s1'])
  })

  it('shows empty message when no schemas', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('No data schemas yet.')
  })
})
