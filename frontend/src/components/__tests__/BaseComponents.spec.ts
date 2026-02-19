import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import BaseButton from '../base/BaseButton.vue'
import BaseCard from '../base/BaseCard.vue'

describe('BaseButton', () => {
  it('renders slot content', () => {
    const wrapper = mount(BaseButton, { slots: { default: 'Click me' } })
    expect(wrapper.text()).toContain('Click me')
  })

  it('applies variant class', () => {
    const wrapper = mount(BaseButton, {
      props: { variant: 'danger' },
      slots: { default: 'Delete' },
    })
    expect(wrapper.classes()).toContain('base-button--danger')
  })

  it('is disabled when prop is set', () => {
    const wrapper = mount(BaseButton, {
      props: { disabled: true },
      slots: { default: 'Disabled' },
    })
    expect(wrapper.attributes('disabled')).toBeDefined()
  })
})

describe('BaseCard', () => {
  it('renders title when provided', () => {
    const wrapper = mount(BaseCard, {
      props: { title: 'Test Card' },
      slots: { default: 'Card content' },
    })
    expect(wrapper.text()).toContain('Test Card')
    expect(wrapper.text()).toContain('Card content')
  })

  it('hides title when not provided', () => {
    const wrapper = mount(BaseCard, {
      slots: { default: 'Content only' },
    })
    expect(wrapper.find('.base-card__header').exists()).toBe(false)
  })
})
