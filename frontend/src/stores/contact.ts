import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type ContactResponse = components['schemas']['ContactResponse']

export const useContactStore = defineStore('contact', () => {
  const contacts = ref<ContactResponse[]>([])
  const currentContact = ref<ContactResponse | null>(null)
  const loading = ref(false)
  const error = ref('')

  async function fetchContacts(orgId: string, search?: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/organizations/{orgId}/contacts', {
        params: {
          path: { orgId },
          ...(search ? { query: { search } } : {}),
        },
      })
      if (data) {
        contacts.value = data.contacts
      }
    } catch {
      error.value = 'Failed to load contacts.'
    } finally {
      loading.value = false
    }
  }

  async function createContact(orgId: string, name: string, email: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/organizations/{orgId}/contacts', {
        params: { path: { orgId } },
        body: { name, email },
      })
      if (data) {
        await fetchContacts(orgId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create contact.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function getContact(orgId: string, contactId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/organizations/{orgId}/contacts/{contactId}', {
        params: { path: { orgId, contactId } },
      })
      if (data) {
        currentContact.value = data
      }
    } catch {
      error.value = 'Failed to load contact.'
    } finally {
      loading.value = false
    }
  }

  async function updateContact(
    orgId: string,
    contactId: string,
    updates: { name?: string; email?: string },
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/organizations/{orgId}/contacts/{contactId}', {
        params: { path: { orgId, contactId } },
        body: updates,
      })
      if (data) {
        currentContact.value = data
        await fetchContacts(orgId)
      }
    } catch {
      error.value = 'Failed to update contact.'
    } finally {
      loading.value = false
    }
  }

  async function deleteContact(orgId: string, contactId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE('/api/v1/organizations/{orgId}/contacts/{contactId}', {
        params: { path: { orgId, contactId } },
      })
      await fetchContacts(orgId)
    } catch {
      error.value = 'Failed to delete contact.'
    } finally {
      loading.value = false
    }
  }

  async function mergeContacts(orgId: string, sourceIds: string[], targetId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/organizations/{orgId}/contacts/merge', {
        params: { path: { orgId } },
        body: { sourceIds, targetId },
      })
      if (data) {
        await fetchContacts(orgId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to merge contacts.'
      return null
    } finally {
      loading.value = false
    }
  }

  return {
    contacts,
    currentContact,
    loading,
    error,
    fetchContacts,
    createContact,
    getContact,
    updateContact,
    deleteContact,
    mergeContacts,
  }
})
