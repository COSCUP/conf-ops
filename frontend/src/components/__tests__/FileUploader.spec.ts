import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import FileUploader from '../file/FileUploader.vue'
import { createPinia, setActivePinia } from 'pinia'

// Mock useFileUpload
vi.mock('@/composables/useFileUpload', async () => {
  const { ref } = await import('vue')
  return {
    validateFile: vi.fn(() => null),
    useFileUpload: () => ({
      uploading: ref(false),
      progress: ref(0),
      error: ref(null),
      uploadFile: vi.fn().mockResolvedValue({
        id: 'file-1',
        filename: 'test.txt',
        mimeType: 'text/plain',
        size: 1024,
        createdAt: '2025-01-01T00:00:00Z',
      }),
      deleteFile: vi.fn().mockResolvedValue(true),
    }),
  }
})

function createWrapper(props?: Record<string, unknown>) {
  return mount(FileUploader, {
    props: {
      scopeType: 'task',
      scopeId: 'task-123',
      ...props,
    } as InstanceType<typeof FileUploader>['$props'],
  })
}

describe('FileUploader', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders drop zone', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.drop-zone').exists()).toBe(true)
    expect(wrapper.text()).toContain('Drop files here or click to browse')
  })

  it('contains a hidden file input', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('input[type="file"]').exists()).toBe(true)
  })

  it('sets accept attribute when provided', () => {
    const wrapper = createWrapper({ accept: 'image/*' })
    const input = wrapper.find('input[type="file"]')
    expect(input.attributes('accept')).toBe('image/*')
  })

  it('sets multiple attribute when provided', () => {
    const wrapper = createWrapper({ multiple: true })
    const input = wrapper.find('input[type="file"]')
    expect(input.attributes('multiple')).toBeDefined()
  })

  it('activates drop zone on dragover', async () => {
    const wrapper = createWrapper()
    const dropZone = wrapper.find('.drop-zone')
    await dropZone.trigger('dragover', { preventDefault: () => {} })
    expect(dropZone.classes()).toContain('drop-zone--active')
  })

  it('deactivates drop zone on dragleave', async () => {
    const wrapper = createWrapper()
    const dropZone = wrapper.find('.drop-zone')
    await dropZone.trigger('dragover', { preventDefault: () => {} })
    await dropZone.trigger('dragleave')
    expect(dropZone.classes()).not.toContain('drop-zone--active')
  })
})
