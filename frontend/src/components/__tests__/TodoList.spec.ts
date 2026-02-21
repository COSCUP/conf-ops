import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import TodoList from '../task/TodoList.vue'

function createWrapper(props?: Record<string, unknown>) {
  return mount(TodoList, {
    props: {
      todos: [],
      loading: false,
      ...props,
    } as InstanceType<typeof TodoList>['$props'],
  })
}

describe('TodoList', () => {
  it('renders create form with Title, Description and Add Todo', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('Title')
    expect(wrapper.text()).toContain('Description')
    expect(wrapper.text()).toContain('Add Todo')
  })

  it('emits create with title and description on submit', async () => {
    const wrapper = createWrapper()

    const titleInput = wrapper.find('input[placeholder="New todo"]')
    await titleInput.setValue('My Todo')

    const descInput = wrapper.find('input[placeholder="Optional description"]')
    await descInput.setValue('A description')

    const form = wrapper.find('form')
    await form.trigger('submit')

    expect(wrapper.emitted('create')).toHaveLength(1)
    expect(wrapper.emitted('create')![0]).toEqual(['My Todo', 'A description'])
  })

  it('clears inputs after submit', async () => {
    const wrapper = createWrapper()

    const titleInput = wrapper.find('input[placeholder="New todo"]')
    await titleInput.setValue('My Todo')

    const form = wrapper.find('form')
    await form.trigger('submit')

    expect((titleInput.element as HTMLInputElement).value).toBe('')
  })

  it('does not emit create when title is empty', async () => {
    const wrapper = createWrapper()

    const form = wrapper.find('form')
    await form.trigger('submit')

    expect(wrapper.emitted('create')).toBeUndefined()
  })

  it('renders todo items', () => {
    const todos = [
      {
        id: 't1',
        taskId: 'task1',
        title: 'Todo A',
        description: 'Desc A',
        status: 'open',
        todoType: 'manual',
        dueDate: null,
        parentId: null,
        sortOrder: 0,
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    ]

    const wrapper = createWrapper({ todos })
    expect(wrapper.text()).toContain('Todo A')
    expect(wrapper.text()).toContain('Desc A')
  })

  it('applies todo-completed class for completed todos', () => {
    const todos = [
      {
        id: 't1',
        taskId: 'task1',
        title: 'Done Todo',
        description: null,
        status: 'completed',
        todoType: 'manual',
        dueDate: null,
        parentId: null,
        sortOrder: 0,
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    ]

    const wrapper = createWrapper({ todos })
    expect(wrapper.find('.todo-completed').exists()).toBe(true)
  })

  it('emits toggle on checkbox change', async () => {
    const todos = [
      {
        id: 't1',
        taskId: 'task1',
        title: 'Todo',
        description: null,
        status: 'open',
        todoType: 'manual',
        dueDate: null,
        parentId: null,
        sortOrder: 0,
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    ]

    const wrapper = createWrapper({ todos })
    const checkbox = wrapper.find('input[type="checkbox"]')
    await checkbox.trigger('change')

    expect(wrapper.emitted('toggle')).toHaveLength(1)
    expect(wrapper.emitted('toggle')![0]).toEqual(['t1', 'open'])
  })

  it('emits delete on Delete button click', async () => {
    const todos = [
      {
        id: 't1',
        taskId: 'task1',
        title: 'Todo',
        description: null,
        status: 'open',
        todoType: 'manual',
        dueDate: null,
        parentId: null,
        sortOrder: 0,
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    ]

    const wrapper = createWrapper({ todos })
    const deleteBtn = wrapper.findAll('button').find((b) => b.text() === 'Delete')!
    await deleteBtn.trigger('click')

    expect(wrapper.emitted('delete')).toHaveLength(1)
    expect(wrapper.emitted('delete')![0]).toEqual(['t1'])
  })

  it('shows empty message when no todos', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('No todos yet.')
  })
})
