import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { McpServer, McpServerWithTools, CreateMcpServer, UpdateMcpServer } from '@/types/mcp'
import type { Tool } from '@/types/tools'

export function useMcpServers() {
  const [servers, setServers] = useState<McpServerWithTools[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Load all servers with tool counts
  const loadServers = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      const result = await invoke<McpServerWithTools[]>('mcp_list_servers_with_tools')
      setServers(result)
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setError(message)
      console.error('Failed to load MCP servers:', message)
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Load on mount
  useEffect(() => {
    loadServers()
  }, [loadServers])

  // Create a new server
  const createServer = useCallback(async (input: CreateMcpServer): Promise<string> => {
    const id = await invoke<string>('mcp_create_server', {
      name: input.name,
      command: input.command,
      args: input.args,
      env: input.env,
      workingDirectory: input.working_directory,
      autoStart: input.auto_start,
    })
    await loadServers()
    return id
  }, [loadServers])

  // Import servers from standard MCP config JSON
  const importConfig = useCallback(async (configJson: string): Promise<string[]> => {
    const ids = await invoke<string[]>('mcp_import_config', { configJson })
    await loadServers()
    return ids
  }, [loadServers])

  // Update a server
  const updateServer = useCallback(async (id: string, input: UpdateMcpServer): Promise<void> => {
    await invoke('mcp_update_server', {
      id,
      name: input.name,
      command: input.command,
      args: input.args,
      env: input.env,
      workingDirectory: input.working_directory,
      autoStart: input.auto_start,
      enabled: input.enabled,
    })
    await loadServers()
  }, [loadServers])

  // Delete a server
  const deleteServer = useCallback(async (id: string): Promise<void> => {
    await invoke('mcp_delete_server', { id })
    await loadServers()
  }, [loadServers])

  // Start a server
  const startServer = useCallback(async (id: string): Promise<Tool[]> => {
    const tools = await invoke<Tool[]>('mcp_start_server', { id })
    await loadServers()
    return tools
  }, [loadServers])

  // Stop a server
  const stopServer = useCallback(async (id: string): Promise<void> => {
    await invoke('mcp_stop_server', { id })
    await loadServers()
  }, [loadServers])

  // Restart a server
  const restartServer = useCallback(async (id: string): Promise<Tool[]> => {
    const tools = await invoke<Tool[]>('mcp_restart_server', { id })
    await loadServers()
    return tools
  }, [loadServers])

  // Refresh tools from a server
  const refreshTools = useCallback(async (id: string): Promise<Tool[]> => {
    const tools = await invoke<Tool[]>('mcp_refresh_tools', { id })
    await loadServers()
    return tools
  }, [loadServers])

  // Get server status
  const getServerStatus = useCallback(async (id: string): Promise<string> => {
    return await invoke<string>('mcp_get_server_status', { id })
  }, [])

  // Check if server is running
  const isServerRunning = useCallback(async (id: string): Promise<boolean> => {
    return await invoke<boolean>('mcp_is_server_running', { id })
  }, [])

  // Get running servers
  const getRunningServers = useCallback(async (): Promise<string[]> => {
    return await invoke<string[]>('mcp_get_running_servers')
  }, [])

  // Get tools for a server
  const getServerTools = useCallback(async (serverId: string): Promise<Tool[]> => {
    return await invoke<Tool[]>('mcp_get_server_tools', { serverId })
  }, [])

  return {
    servers,
    isLoading,
    error,
    loadServers,
    createServer,
    importConfig,
    updateServer,
    deleteServer,
    startServer,
    stopServer,
    restartServer,
    refreshTools,
    getServerStatus,
    isServerRunning,
    getRunningServers,
    getServerTools,
  }
}
