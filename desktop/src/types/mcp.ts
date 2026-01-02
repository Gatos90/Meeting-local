// MCP (Model Context Protocol) server types

export type McpServerStatus = 'stopped' | 'starting' | 'running' | 'error'

export interface McpServer {
  id: string
  name: string
  command: string
  args: string // JSON array
  env: string // JSON object
  working_directory?: string
  auto_start: boolean
  enabled: boolean
  status: McpServerStatus
  last_error?: string
  created_at: string
}

export interface McpServerWithTools extends McpServer {
  tool_count: number
}

export interface CreateMcpServer {
  name: string
  command: string
  args: string[]
  env: Record<string, string>
  working_directory?: string
  auto_start: boolean
}

export interface UpdateMcpServer {
  name?: string
  command?: string
  args?: string[]
  env?: Record<string, string>
  working_directory?: string
  auto_start?: boolean
  enabled?: boolean
}

// Standard MCP server config format for import
// Format: { "server_name": { "command": "...", "args": [...], "env": {...} } }
export interface McpServerConfig {
  command: string
  args?: string[]
  env?: Record<string, string>
  working_directory?: string
}

export type McpConfigFile = Record<string, McpServerConfig>

// Helper functions to parse the JSON fields
export function parseServerArgs(server: McpServer): string[] {
  try {
    return JSON.parse(server.args)
  } catch {
    return []
  }
}

export function parseServerEnv(server: McpServer): Record<string, string> {
  try {
    return JSON.parse(server.env)
  } catch {
    return {}
  }
}
