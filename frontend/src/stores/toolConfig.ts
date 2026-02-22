import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'

export interface ToolSummary {
  name: string
  displayName?: string
  category: 'core' | 'configurable' | 'external'
  description: string
  requiresConfirmation: boolean
}

export interface ToolConfig {
  id: string
  scopeType: string
  scopeId: string
  toolType: string
  toolName: string
  displayName?: string
  description?: string
  enabled: boolean
  config: Record<string, unknown>
  mcpServerConfig?: Record<string, unknown>
  createdAt: string
  updatedAt: string
}

export interface CreateToolConfigPayload {
  toolType?: string
  toolName: string
  displayName?: string
  description?: string
  enabled: boolean
  config: Record<string, unknown>
  mcpServerConfig?: Record<string, unknown>
}

export interface UpdateToolConfigPayload {
  displayName?: string
  description?: string
  enabled?: boolean
  config?: Record<string, unknown>
  mcpServerConfig?: Record<string, unknown>
}

export interface ExecuteToolPayload {
  taskId: string
  parameters: Record<string, unknown>
  suggestionId?: string
}

export interface ExecuteToolResult {
  success: boolean
  result: unknown
  durationMs: number
}

export const useToolConfigStore = defineStore('toolConfig', () => {
  const tools = ref<ToolSummary[]>([])
  const toolConfigs = ref<ToolConfig[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function listTools(projectId: string) {
    loading.value = true
    error.value = null
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/tools' as never,
        { params: { path: { projectId } } } as never,
      )
      const result = data as { tools: ToolSummary[] } | undefined
      if (result) {
        tools.value = result.tools
      }
    } catch {
      error.value = 'Failed to load tools.'
    } finally {
      loading.value = false
    }
  }

  async function listToolConfigs(projectId: string) {
    loading.value = true
    error.value = null
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/tool-configs' as never,
        { params: { path: { projectId } } } as never,
      )
      const result = data as { toolConfigs: ToolConfig[] } | undefined
      if (result) {
        toolConfigs.value = result.toolConfigs
      }
    } catch {
      error.value = 'Failed to load tool configs.'
    } finally {
      loading.value = false
    }
  }

  async function createToolConfig(
    projectId: string,
    payload: CreateToolConfigPayload,
  ): Promise<ToolConfig | null> {
    loading.value = true
    error.value = null
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/tool-configs' as never,
        { params: { path: { projectId } }, body: payload } as never,
      )
      const result = data as ToolConfig | undefined
      if (result) {
        toolConfigs.value.push(result)
        return result
      }
      return null
    } catch {
      error.value = 'Failed to create tool config.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateToolConfig(
    projectId: string,
    configId: string,
    payload: UpdateToolConfigPayload,
  ): Promise<ToolConfig | null> {
    loading.value = true
    error.value = null
    try {
      const { data } = await client.PUT(
        '/api/v1/projects/{projectId}/tool-configs/{configId}' as never,
        { params: { path: { projectId, configId } }, body: payload } as never,
      )
      const result = data as ToolConfig | undefined
      if (result) {
        const idx = toolConfigs.value.findIndex((c) => c.id === configId)
        if (idx >= 0) {
          toolConfigs.value[idx] = result
        }
        return result
      }
      return null
    } catch {
      error.value = 'Failed to update tool config.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function deleteToolConfig(projectId: string, configId: string): Promise<boolean> {
    loading.value = true
    error.value = null
    try {
      await client.DELETE(
        '/api/v1/projects/{projectId}/tool-configs/{configId}' as never,
        { params: { path: { projectId, configId } } } as never,
      )
      toolConfigs.value = toolConfigs.value.filter((c) => c.id !== configId)
      return true
    } catch {
      error.value = 'Failed to delete tool config.'
      return false
    } finally {
      loading.value = false
    }
  }

  async function executeTool(
    projectId: string,
    toolName: string,
    payload: ExecuteToolPayload,
  ): Promise<ExecuteToolResult | null> {
    loading.value = true
    error.value = null
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/tools/{toolName}/execute' as never,
        { params: { path: { projectId, toolName } }, body: payload } as never,
      )
      return (data as ExecuteToolResult | undefined) ?? null
    } catch {
      error.value = 'Failed to execute tool.'
      return null
    } finally {
      loading.value = false
    }
  }

  return {
    tools,
    toolConfigs,
    loading,
    error,
    listTools,
    listToolConfigs,
    createToolConfig,
    updateToolConfig,
    deleteToolConfig,
    executeTool,
  }
})
