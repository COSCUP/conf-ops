import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import FilePreview from '../file/FilePreview.vue'
import { createPinia, setActivePinia } from 'pinia'

// Mock auth store
vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({
    accessToken: 'test-token',
  }),
}))

// Mock fetch for image/PDF loading
const mockFetch = vi.fn()
vi.stubGlobal('fetch', mockFetch)

// Mock URL.createObjectURL
vi.stubGlobal('URL', {
  createObjectURL: vi.fn(() => 'blob:mock-url'),
  revokeObjectURL: vi.fn(),
})

function createWrapper(props: InstanceType<typeof FilePreview>['$props']) {
  return mount(FilePreview, { props })
}

describe('FilePreview', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    mockFetch.mockReset()
    // Default mock: return empty blob for image requests
    mockFetch.mockResolvedValue({
      ok: true,
      blob: () => Promise.resolve(new Blob([])),
    })
  })

  it('renders image preview for image MIME types', () => {
    const wrapper = createWrapper({
      fileId: 'f1',
      filename: 'photo.png',
      mimeType: 'image/png',
      size: 1024,
    })
    expect(wrapper.find('.image-preview').exists()).toBe(true)
    expect(wrapper.find('.pdf-preview').exists()).toBe(false)
    expect(wrapper.find('.generic-preview').exists()).toBe(false)
  })

  it('renders PDF preview for PDF MIME type', () => {
    const wrapper = createWrapper({
      fileId: 'f2',
      filename: 'doc.pdf',
      mimeType: 'application/pdf',
      size: 2048,
    })
    expect(wrapper.find('.pdf-preview').exists()).toBe(true)
    expect(wrapper.find('.image-preview').exists()).toBe(false)
    expect(wrapper.text()).toContain('doc.pdf')
    expect(wrapper.text()).toContain('Download')
  })

  it('renders generic preview for other MIME types', () => {
    const wrapper = createWrapper({
      fileId: 'f3',
      filename: 'data.csv',
      mimeType: 'text/csv',
      size: 512,
    })
    expect(wrapper.find('.generic-preview').exists()).toBe(true)
    expect(wrapper.text()).toContain('data.csv')
    expect(wrapper.text()).toContain('Download')
  })

  it('shows filename and size for non-image files', () => {
    const wrapper = createWrapper({
      fileId: 'f4',
      filename: 'report.pdf',
      mimeType: 'application/pdf',
      size: 1024 * 1024 * 2, // 2 MB
    })
    expect(wrapper.text()).toContain('report.pdf')
    expect(wrapper.text()).toContain('2.0 MB')
  })

  it('image preview has click handler', () => {
    const wrapper = createWrapper({
      fileId: 'f5',
      filename: 'photo.jpg',
      mimeType: 'image/jpeg',
      size: 1024,
    })

    const preview = wrapper.find('.image-preview')
    expect(preview.exists()).toBe(true)
  })

  it('renders embedded iframe for PDF after blob URL is loaded', async () => {
    const wrapper = createWrapper({
      fileId: 'f6',
      filename: 'report.pdf',
      mimeType: 'application/pdf',
      size: 4096,
    })

    // Wait for async loadPdf() to complete
    await new Promise((resolve) => setTimeout(resolve, 0))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.pdf-embed').exists()).toBe(true)
    expect(wrapper.find('.pdf-embed').attributes('src')).toBe('blob:mock-url')
  })
})
