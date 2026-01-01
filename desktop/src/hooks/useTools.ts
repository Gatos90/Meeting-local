'use client'

import { useState, useEffect, useCallback, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Tool, CreateTool, UpdateTool } from '@/types/tools'

export function useTools() {
  const [tools, setTools] = useState<Tool[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Load tools on mount
  useEffect(() => {
    loadTools()
  }, [])

  // Load all tools
  const loadTools = useCallback(async () => {
    try {
      setIsLoading(true)
      setError(null)
      const toolList = await invoke<Tool[]>('tools_list')
      setTools(toolList)
    } catch (err) {
      console.error('Failed to load tools:', err)
      setError(String(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Get a single tool by ID
  const getTool = useCallback(async (id: string): Promise<Tool | null> => {
    try {
      return await invoke<Tool | null>('tools_get', { id })
    } catch (err) {
      console.error('Failed to get tool:', err)
      setError(String(err))
      return null
    }
  }, [])

  // Create a new tool
  const createTool = useCallback(async (input: CreateTool): Promise<string | null> => {
    try {
      setError(null)
      const id = await invoke<string>('tools_create', {
        name: input.name,
        functionSchema: input.function_schema,
        description: input.description || null,
        executionLocation: input.execution_location || null,
        icon: input.icon || null,
      })
      await loadTools() // Refresh list
      return id
    } catch (err) {
      console.error('Failed to create tool:', err)
      setError(String(err))
      return null
    }
  }, [loadTools])

  // Update an existing tool
  const updateTool = useCallback(async (id: string, input: UpdateTool): Promise<boolean> => {
    try {
      setError(null)
      await invoke('tools_update', {
        id,
        name: input.name || null,
        description: input.description || null,
        functionSchema: input.function_schema || null,
        executionLocation: input.execution_location || null,
        enabled: input.enabled ?? null,
        isDefault: input.is_default ?? null,
        icon: input.icon || null,
        sortOrder: input.sort_order ?? null,
      })
      await loadTools() // Refresh list
      return true
    } catch (err) {
      console.error('Failed to update tool:', err)
      setError(String(err))
      return false
    }
  }, [loadTools])

  // Delete a tool
  const deleteTool = useCallback(async (id: string): Promise<boolean> => {
    try {
      setError(null)
      await invoke('tools_delete', { id })
      await loadTools() // Refresh list
      return true
    } catch (err) {
      console.error('Failed to delete tool:', err)
      setError(String(err))
      return false
    }
  }, [loadTools])

  // Set tool default status
  const setToolDefault = useCallback(async (id: string, isDefault: boolean): Promise<boolean> => {
    try {
      setError(null)
      await invoke('tools_set_default', { id, isDefault })
      await loadTools() // Refresh list
      return true
    } catch (err) {
      console.error('Failed to set tool default:', err)
      setError(String(err))
      return false
    }
  }, [loadTools])

  // Toggle tool enabled status
  const toggleToolEnabled = useCallback(async (id: string, enabled: boolean): Promise<boolean> => {
    try {
      setError(null)
      await invoke('tools_update', {
        id,
        enabled,
        // Pass nulls for other fields
        name: null,
        description: null,
        functionSchema: null,
        executionLocation: null,
        isDefault: null,
        icon: null,
        sortOrder: null,
      })
      await loadTools() // Refresh list
      return true
    } catch (err) {
      console.error('Failed to toggle tool enabled:', err)
      setError(String(err))
      return false
    }
  }, [loadTools])

  // Helper: Get built-in tools only
  const builtinTools = useMemo(() => tools.filter(t => t.tool_type === 'builtin'), [tools])

  // Helper: Get custom tools only
  const customTools = useMemo(() => tools.filter(t => t.tool_type === 'custom'), [tools])

  // Helper: Get MCP tools only
  const mcpTools = useMemo(() => tools.filter(t => t.tool_type === 'mcp'), [tools])

  // Helper: Get default tools only (enabled and marked as default)
  const defaultTools = useMemo(() => tools.filter(t => t.is_default && t.enabled), [tools])

  // Helper: Get enabled tools only
  const enabledTools = useMemo(() => tools.filter(t => t.enabled), [tools])

  return {
    tools,
    builtinTools,
    customTools,
    mcpTools,
    defaultTools,
    enabledTools,
    isLoading,
    error,
    loadTools,
    getTool,
    createTool,
    updateTool,
    deleteTool,
    setToolDefault,
    toggleToolEnabled,
  }
}

// Hook for managing tools for a specific chat session
export function useSessionTools(sessionId: string | null) {
  const [sessionTools, setSessionTools] = useState<Tool[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Load session tools when sessionId changes
  useEffect(() => {
    if (sessionId) {
      loadSessionTools()
    } else {
      setSessionTools([])
    }
  }, [sessionId])

  // Load tools for the session
  const loadSessionTools = useCallback(async () => {
    if (!sessionId) return
    try {
      setIsLoading(true)
      setError(null)
      const tools = await invoke<Tool[]>('tools_get_for_session', { sessionId })
      setSessionTools(tools)
    } catch (err) {
      console.error('Failed to load session tools:', err)
      setError(String(err))
    } finally {
      setIsLoading(false)
    }
  }, [sessionId])

  // Set tools for the session (optimistic update - no reload to preserve scroll position)
  const setSessionToolIds = useCallback(async (toolIds: string[], allTools?: Tool[]): Promise<boolean> => {
    if (!sessionId) return false
    try {
      setError(null)
      // Optimistic update: update local state immediately if allTools provided
      if (allTools) {
        const toolSet = new Set(toolIds)
        setSessionTools(allTools.filter(t => toolSet.has(t.id)))
      }
      // Persist to backend (don't await reload - optimistic)
      await invoke('tools_set_for_session', { sessionId, toolIds })
      return true
    } catch (err) {
      console.error('Failed to set session tools:', err)
      setError(String(err))
      // Reload on error to revert optimistic update
      await loadSessionTools()
      return false
    }
  }, [sessionId, loadSessionTools])

  // Initialize session with default tools
  const initSessionTools = useCallback(async (): Promise<boolean> => {
    if (!sessionId) return false
    try {
      setError(null)
      await invoke('tools_init_for_session', { sessionId })
      await loadSessionTools() // Refresh
      return true
    } catch (err) {
      console.error('Failed to init session tools:', err)
      setError(String(err))
      return false
    }
  }, [sessionId, loadSessionTools])

  return {
    sessionTools,
    isLoading,
    error,
    loadSessionTools,
    setSessionToolIds,
    initSessionTools,
  }
}
