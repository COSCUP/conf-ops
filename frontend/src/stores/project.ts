import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type ProjectResponse = components['schemas']['ProjectResponse']
type ProjectListItemResponse = components['schemas']['ProjectListItemResponse']

export const useProjectStore = defineStore('project', () => {
  const projects = ref<ProjectListItemResponse[]>([])
  const currentProject = ref<ProjectResponse | null>(null)
  const permissionSettings = ref<Record<string, unknown>>({})
  const loading = ref(false)
  const error = ref('')

  async function fetchProjects(orgId: string, status?: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/organizations/{orgId}/projects', {
        params: {
          path: { orgId },
          ...(status ? { query: { status: status as components['schemas']['ProjectStatus'] } } : {}),
        },
      })
      if (data) {
        projects.value = data.projects
      }
    } catch {
      error.value = 'Failed to load projects.'
    } finally {
      loading.value = false
    }
  }

  async function fetchProject(projectId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/projects/{projectId}', {
        params: { path: { projectId } },
      })
      if (data) {
        currentProject.value = data
      }
    } catch {
      error.value = 'Failed to load project.'
    } finally {
      loading.value = false
    }
  }

  async function createProject(orgId: string, name: string, description?: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/organizations/{orgId}/projects', {
        params: { path: { orgId } },
        body: { name, description: description ?? null },
      })
      if (data) {
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create project.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function copyProject(
    orgId: string,
    sourceProjectId: string,
    name: string,
    description?: string,
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/organizations/{orgId}/projects/copy', {
        params: { path: { orgId } },
        body: { sourceProjectId, name, description: description ?? null },
      })
      if (data) {
        return data
      }
      return null
    } catch {
      error.value = 'Failed to copy project.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateProject(
    projectId: string,
    updates: { name?: string; description?: string | null },
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/projects/{projectId}', {
        params: { path: { projectId } },
        body: updates,
      })
      if (data) {
        currentProject.value = data
      }
    } catch {
      error.value = 'Failed to update project.'
    } finally {
      loading.value = false
    }
  }

  async function deleteProject(projectId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE('/api/v1/projects/{projectId}', {
        params: { path: { projectId } },
      })
      currentProject.value = null
    } catch {
      error.value = 'Failed to delete project.'
    } finally {
      loading.value = false
    }
  }

  async function updateProjectStatus(projectId: string, status: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/projects/{projectId}/status', {
        params: { path: { projectId } },
        body: { status: status as components['schemas']['ProjectStatus'] },
      })
      if (data) {
        currentProject.value = data
      }
    } catch {
      error.value = 'Failed to update project status.'
    } finally {
      loading.value = false
    }
  }

  async function fetchPermissionSettings(projectId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/projects/{projectId}/permission-settings', {
        params: { path: { projectId } },
      })
      if (data) {
        permissionSettings.value = data.settings as Record<string, unknown>
      }
    } catch {
      error.value = 'Failed to load permission settings.'
    } finally {
      loading.value = false
    }
  }

  async function updatePermissionSettings(
    projectId: string,
    settings: Record<string, unknown>,
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/projects/{projectId}/permission-settings', {
        params: { path: { projectId } },
        body: { settings } as never,
      })
      if (data) {
        permissionSettings.value = data.settings as Record<string, unknown>
      }
    } catch {
      error.value = 'Failed to update permission settings.'
    } finally {
      loading.value = false
    }
  }

  return {
    projects,
    currentProject,
    permissionSettings,
    loading,
    error,
    fetchProjects,
    fetchProject,
    createProject,
    copyProject,
    updateProject,
    deleteProject,
    updateProjectStatus,
    fetchPermissionSettings,
    updatePermissionSettings,
  }
})
