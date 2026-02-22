import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import ToolParameterForm from '../ToolParameterForm.vue'

describe('ToolParameterForm', () => {
  const schema = {
    type: 'object',
    properties: {
      name: { type: 'string', description: 'Task name' },
      count: { type: 'number', description: 'Item count' },
      status: {
        type: 'string',
        enum: ['active', 'inactive'],
        description: 'Status',
      },
    },
    required: ['name'],
  }

  it('renders fields from schema', () => {
    const wrapper = mount(ToolParameterForm, {
      props: {
        schema,
        modelValue: {},
      },
    })

    const labels = wrapper.findAll('.field-label')
    expect(labels.length).toBe(3)
    expect(labels[0]?.text()).toContain('name')
  })

  it('shows required marker for required fields', () => {
    const wrapper = mount(ToolParameterForm, {
      props: {
        schema,
        modelValue: {},
      },
    })

    const requiredMarkers = wrapper.findAll('.required-marker')
    expect(requiredMarkers.length).toBe(1)
  })

  it('renders select for enum fields', () => {
    const wrapper = mount(ToolParameterForm, {
      props: {
        schema,
        modelValue: {},
      },
    })

    const selects = wrapper.findAll('select')
    expect(selects.length).toBe(1)
  })

  it('renders text inputs for string fields', () => {
    const wrapper = mount(ToolParameterForm, {
      props: {
        schema,
        modelValue: {},
      },
    })

    const inputs = wrapper.findAll('input')
    expect(inputs.length).toBeGreaterThanOrEqual(2)
  })

  it('shows no-params message when schema has no properties', () => {
    const wrapper = mount(ToolParameterForm, {
      props: {
        schema: { type: 'object', properties: {} },
        modelValue: {},
      },
    })

    expect(wrapper.find('.no-params').exists()).toBe(true)
  })

  it('emits update:modelValue on field change', async () => {
    const wrapper = mount(ToolParameterForm, {
      props: {
        schema: {
          type: 'object',
          properties: {
            title: { type: 'string', description: 'Title' },
          },
          required: [],
        },
        modelValue: { title: '' },
      },
    })

    const input = wrapper.find('input')
    await input.setValue('New Title')

    const events = wrapper.emitted('update:modelValue')
    expect(events).toBeDefined()
    expect(events?.length).toBeGreaterThan(0)
  })
})
