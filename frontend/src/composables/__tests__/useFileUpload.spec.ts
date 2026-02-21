import { describe, it, expect } from 'vitest'
import { validateFile } from '../useFileUpload'

describe('validateFile', () => {
  function createFile(name: string, type: string, size: number): File {
    const blob = new Blob([new ArrayBuffer(size)], { type })
    return new File([blob], name, { type })
  }

  it('accepts valid image types', () => {
    expect(validateFile(createFile('photo.png', 'image/png', 1024))).toBeNull()
    expect(validateFile(createFile('photo.jpg', 'image/jpeg', 1024))).toBeNull()
    expect(validateFile(createFile('photo.gif', 'image/gif', 1024))).toBeNull()
    expect(validateFile(createFile('photo.webp', 'image/webp', 1024))).toBeNull()
  })

  it('accepts valid document types', () => {
    expect(validateFile(createFile('doc.pdf', 'application/pdf', 1024))).toBeNull()
    expect(validateFile(createFile('file.txt', 'text/plain', 1024))).toBeNull()
    expect(validateFile(createFile('data.csv', 'text/csv', 1024))).toBeNull()
  })

  it('rejects unsupported MIME types', () => {
    const result = validateFile(createFile('archive.zip', 'application/zip', 1024))
    expect(result).not.toBeNull()
    expect(result!.type).toBe('mime')
    expect(result!.message).toContain('Unsupported file type')
  })

  it('rejects images exceeding 10MB', () => {
    const size = 10 * 1024 * 1024 + 1
    const result = validateFile(createFile('huge.png', 'image/png', size))
    expect(result).not.toBeNull()
    expect(result!.type).toBe('size')
    expect(result!.message).toContain('File too large')
  })

  it('accepts images at exactly 10MB', () => {
    const size = 10 * 1024 * 1024
    expect(validateFile(createFile('big.png', 'image/png', size))).toBeNull()
  })

  it('rejects documents exceeding 50MB', () => {
    const size = 50 * 1024 * 1024 + 1
    const result = validateFile(createFile('huge.pdf', 'application/pdf', size))
    expect(result).not.toBeNull()
    expect(result!.type).toBe('size')
  })

  it('rejects text files exceeding 20MB', () => {
    const size = 20 * 1024 * 1024 + 1
    const result = validateFile(createFile('huge.txt', 'text/plain', size))
    expect(result).not.toBeNull()
    expect(result!.type).toBe('size')
  })
})
