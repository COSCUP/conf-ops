import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import EmailComposer from '../EmailComposer.vue'

function createWrapper(props?: Record<string, unknown>) {
  return mount(EmailComposer, {
    props: {
      ...props,
    } as InstanceType<typeof EmailComposer>['$props'],
  })
}

describe('EmailComposer', () => {
  it('renders all form fields', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('To')
    expect(wrapper.text()).toContain('CC')
    expect(wrapper.text()).toContain('Subject')
    expect(wrapper.text()).toContain('Body')
    expect(wrapper.find('textarea').exists()).toBe(true)
    expect(wrapper.find('button').exists()).toBe(true)
    expect(wrapper.find('button').text()).toBe('Send')
  })

  it('does not emit send when To is empty', async () => {
    const wrapper = createWrapper()
    const textarea = wrapper.find('textarea')
    await textarea.setValue('Hello body')
    await wrapper.find('form').trigger('submit')

    expect(wrapper.emitted('send')).toBeFalsy()
  })

  it('does not emit send when Body is empty', async () => {
    const wrapper = createWrapper()
    const inputs = wrapper.findAll('input')
    // First input is the To field rendered by BaseInput
    const toInput = inputs[0]
    await toInput!.setValue('recipient@example.com')
    await wrapper.find('form').trigger('submit')

    expect(wrapper.emitted('send')).toBeFalsy()
  })

  it('emits send with correct payload when To and Body are filled', async () => {
    const wrapper = createWrapper()
    const inputs = wrapper.findAll('input')

    // BaseInput renders inputs in order: To, CC, Subject
    await inputs[0]!.setValue('to@example.com')
    await inputs[1]!.setValue('cc@example.com')
    await inputs[2]!.setValue('My Subject')
    await wrapper.find('textarea').setValue('<p>Hello</p>')

    await wrapper.find('form').trigger('submit')

    const emitted = wrapper.emitted('send')
    expect(emitted).toBeTruthy()
    expect(emitted![0]![0]).toEqual({
      toAddresses: ['to@example.com'],
      ccAddresses: ['cc@example.com'],
      subject: 'My Subject',
      htmlBody: '<p>Hello</p>',
    })
  })

  it('splits comma-separated To and CC addresses', async () => {
    const wrapper = createWrapper()
    const inputs = wrapper.findAll('input')

    await inputs[0]!.setValue('a@example.com, b@example.com')
    await inputs[1]!.setValue('c@example.com,d@example.com')
    await wrapper.find('textarea').setValue('Body text')

    await wrapper.find('form').trigger('submit')

    const emitted = wrapper.emitted('send')
    expect(emitted).toBeTruthy()
    const payload = emitted![0]![0] as { toAddresses: string[]; ccAddresses: string[] }
    expect(payload.toAddresses).toEqual(['a@example.com', 'b@example.com'])
    expect(payload.ccAddresses).toEqual(['c@example.com', 'd@example.com'])
  })

  it('clears all fields after successful send', async () => {
    const wrapper = createWrapper()
    const inputs = wrapper.findAll('input')

    await inputs[0]!.setValue('to@example.com')
    await inputs[2]!.setValue('Subject')
    await wrapper.find('textarea').setValue('Body text')

    await wrapper.find('form').trigger('submit')

    expect((inputs[0]!.element as HTMLInputElement).value).toBe('')
    expect((inputs[2]!.element as HTMLInputElement).value).toBe('')
    expect(wrapper.find('textarea').element.value).toBe('')
  })

  it('uses defaultSubject as placeholder when provided', () => {
    const wrapper = createWrapper({ defaultSubject: 'Re: Original Subject' })
    const subjectInput = wrapper.findAll('input')[2]
    expect((subjectInput!.element as HTMLInputElement).placeholder).toBe('Re: Original Subject')
  })

  it('renders Send button as disabled when disabled prop is true', () => {
    const wrapper = createWrapper({ disabled: true })
    const button = wrapper.find('button[type="submit"]')
    expect(button.attributes('disabled')).toBeDefined()
  })
})
