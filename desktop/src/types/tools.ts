// Types for AI tools (function calling)

export type ToolType = 'builtin' | 'custom' | 'mcp'
export type ExecutionLocation = 'backend' | 'frontend'

export interface Tool {
  id: string
  name: string
  description?: string
  tool_type: ToolType
  function_schema: string // JSON string
  execution_location: ExecutionLocation
  enabled: boolean
  is_default: boolean
  icon?: string
  sort_order: number
  created_at: string
  /** MCP server ID (for MCP tools only) */
  mcp_server_id?: string
  /** MCP server name (for MCP tools, joined from mcp_servers table) */
  mcp_server_name?: string
}

export interface CreateTool {
  name: string
  function_schema: string
  description?: string
  execution_location?: ExecutionLocation
  icon?: string
}

export interface UpdateTool {
  name?: string
  description?: string
  function_schema?: string
  execution_location?: ExecutionLocation
  enabled?: boolean
  is_default?: boolean
  icon?: string
  sort_order?: number
}

// Parsed function schema structure
export interface FunctionSchema {
  name: string
  description: string
  parameters: {
    type: 'object'
    properties: Record<string, ParameterSchema>
    required?: string[]
  }
}

export interface ParameterSchema {
  type: string
  description?: string
  enum?: string[]
  default?: unknown
}

// Tool call from LLM
export interface ToolCall {
  id: string
  name: string
  arguments: string // JSON string
}

// Tool execution result
export interface ToolResult {
  tool_call_id: string
  content: string
  success: boolean
  error?: string
}

// Helper to parse function schema from JSON string
export function parseFunctionSchema(schemaJson: string): FunctionSchema | null {
  try {
    return JSON.parse(schemaJson) as FunctionSchema
  } catch {
    return null
  }
}

// Helper to validate function schema structure
export function isValidFunctionSchema(schema: unknown): schema is FunctionSchema {
  if (!schema || typeof schema !== 'object') return false
  const s = schema as Record<string, unknown>
  if (typeof s.name !== 'string') return false
  if (typeof s.description !== 'string') return false
  if (!s.parameters || typeof s.parameters !== 'object') return false
  const params = s.parameters as Record<string, unknown>
  if (params.type !== 'object') return false
  if (!params.properties || typeof params.properties !== 'object') return false
  return true
}
