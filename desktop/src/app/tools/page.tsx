'use client'

import { useState } from 'react'
import { useTools } from '@/hooks/useTools'
import { useMcpServers } from '@/hooks/useMcpServers'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import { Card } from '@/components/ui/card'
import { Switch } from '@/components/ui/switch'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible'
import {
  Plus,
  Pencil,
  Trash2,
  Loader2,
  LucideIcon,
  Filter,
  Wrench,
  Clock,
  Search,
  Users,
  FileText,
  Zap,
  Code,
  Terminal,
  Bot,
  Sparkles,
  Star,
  Server,
  Link,
  Unplug,
  Play,
  Square,
  RotateCw,
  ChevronDown,
  ChevronRight,
  AlertCircle,
  Import,
  RefreshCw,
} from 'lucide-react'
import type { Tool, CreateTool, UpdateTool, FunctionSchema } from '@/types/tools'
import { parseFunctionSchema, isValidFunctionSchema } from '@/types/tools'
import type { McpServerWithTools, CreateMcpServer, McpServerStatus } from '@/types/mcp'
import { parseServerArgs, parseServerEnv } from '@/types/mcp'

// Available icons for tools
const availableIcons = [
  { value: 'Wrench', label: 'Wrench', icon: Wrench },
  { value: 'Clock', label: 'Clock', icon: Clock },
  { value: 'Search', label: 'Search', icon: Search },
  { value: 'Users', label: 'Users', icon: Users },
  { value: 'FileText', label: 'Document', icon: FileText },
  { value: 'Zap', label: 'Zap', icon: Zap },
  { value: 'Code', label: 'Code', icon: Code },
  { value: 'Terminal', label: 'Terminal', icon: Terminal },
  { value: 'Bot', label: 'Bot', icon: Bot },
  { value: 'Sparkles', label: 'Sparkles', icon: Sparkles },
  { value: 'Server', label: 'Server', icon: Server },
]

// Map icon names to components
const iconMap: Record<string, LucideIcon> = {
  Wrench,
  Clock,
  Search,
  Users,
  FileText,
  Zap,
  Code,
  Terminal,
  Bot,
  Sparkles,
  Server,
}

type FilterType = 'all' | 'builtin' | 'custom' | 'mcp' | 'defaults'

export default function ToolsPage() {
  const {
    tools,
    builtinTools,
    customTools,
    mcpTools,
    defaultTools,
    isLoading,
    error,
    createTool,
    updateTool,
    deleteTool,
    setToolDefault,
    toggleToolEnabled,
  } = useTools()

  const {
    servers: mcpServers,
    isLoading: mcpLoading,
    error: mcpError,
    createServer: createMcpServer,
    importConfig: importMcpConfig,
    deleteServer: deleteMcpServer,
    startServer: startMcpServer,
    stopServer: stopMcpServer,
    restartServer: restartMcpServer,
    updateServer: updateMcpServer,
  } = useMcpServers()

  // Filter state
  const [filterType, setFilterType] = useState<FilterType>('all')

  // MCP section state
  const [isMcpExpanded, setIsMcpExpanded] = useState(true)
  const [isMcpDialogOpen, setIsMcpDialogOpen] = useState(false)
  const [mcpDialogMode, setMcpDialogMode] = useState<'manual' | 'import'>('manual')
  const [mcpServerActionLoading, setMcpServerActionLoading] = useState<string | null>(null)
  const [mcpDeleteConfirm, setMcpDeleteConfirm] = useState<McpServerWithTools | null>(null)

  // MCP form state
  const [mcpFormName, setMcpFormName] = useState('')
  const [mcpFormCommand, setMcpFormCommand] = useState('')
  const [mcpFormArgs, setMcpFormArgs] = useState('')
  const [mcpFormEnv, setMcpFormEnv] = useState('')
  const [mcpFormWorkingDir, setMcpFormWorkingDir] = useState('')
  const [mcpFormAutoStart, setMcpFormAutoStart] = useState(false)
  const [mcpImportJson, setMcpImportJson] = useState('')

  // Dialog state
  const [isEditorOpen, setIsEditorOpen] = useState(false)
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false)
  const [editingTool, setEditingTool] = useState<Tool | null>(null)
  const [toolToDelete, setToolToDelete] = useState<Tool | null>(null)
  const [isSaving, setIsSaving] = useState(false)

  // Form state
  const [formName, setFormName] = useState('')
  const [formDescription, setFormDescription] = useState('')
  const [formSchema, setFormSchema] = useState('')
  const [formIcon, setFormIcon] = useState('Wrench')
  const [formExecutionLocation, setFormExecutionLocation] = useState<'backend' | 'frontend'>('backend')
  const [schemaError, setSchemaError] = useState<string | null>(null)

  // MCP status helpers
  const getStatusColor = (status: McpServerStatus) => {
    switch (status) {
      case 'running': return 'text-green-600 bg-green-500/20'
      case 'starting': return 'text-yellow-600 bg-yellow-500/20'
      case 'error': return 'text-red-600 bg-red-500/20'
      default: return 'text-muted-foreground bg-muted'
    }
  }

  const getStatusIcon = (status: McpServerStatus) => {
    switch (status) {
      case 'running': return <div className="w-2 h-2 rounded-full bg-green-500" />
      case 'starting': return <Loader2 className="w-3 h-3 animate-spin text-yellow-500" />
      case 'error': return <AlertCircle className="w-3 h-3 text-red-500" />
      default: return <div className="w-2 h-2 rounded-full bg-muted-foreground" />
    }
  }

  // MCP server actions
  const handleMcpStart = async (server: McpServerWithTools) => {
    setMcpServerActionLoading(server.id)
    try {
      await startMcpServer(server.id)
    } catch (err) {
      console.error('Failed to start server:', err)
    } finally {
      setMcpServerActionLoading(null)
    }
  }

  const handleMcpStop = async (server: McpServerWithTools) => {
    setMcpServerActionLoading(server.id)
    try {
      await stopMcpServer(server.id)
    } catch (err) {
      console.error('Failed to stop server:', err)
    } finally {
      setMcpServerActionLoading(null)
    }
  }

  const handleMcpRestart = async (server: McpServerWithTools) => {
    setMcpServerActionLoading(server.id)
    try {
      await restartMcpServer(server.id)
    } catch (err) {
      console.error('Failed to restart server:', err)
    } finally {
      setMcpServerActionLoading(null)
    }
  }

  const handleMcpDelete = async () => {
    if (!mcpDeleteConfirm) return
    setMcpServerActionLoading(mcpDeleteConfirm.id)
    try {
      await deleteMcpServer(mcpDeleteConfirm.id)
    } catch (err) {
      console.error('Failed to delete server:', err)
    } finally {
      setMcpServerActionLoading(null)
      setMcpDeleteConfirm(null)
    }
  }

  const handleMcpToggleAutoStart = async (server: McpServerWithTools) => {
    try {
      await updateMcpServer(server.id, { auto_start: !server.auto_start })
    } catch (err) {
      console.error('Failed to toggle auto-start:', err)
    }
  }

  const handleMcpCreate = async () => {
    if (mcpDialogMode === 'import') {
      try {
        setIsSaving(true)
        await importMcpConfig(mcpImportJson)
        setIsMcpDialogOpen(false)
        setMcpImportJson('')
      } catch (err) {
        console.error('Failed to import config:', err)
      } finally {
        setIsSaving(false)
      }
    } else {
      try {
        setIsSaving(true)
        const args = mcpFormArgs.trim() ? mcpFormArgs.split('\n').filter(a => a.trim()) : []
        const env: Record<string, string> = {}
        if (mcpFormEnv.trim()) {
          mcpFormEnv.split('\n').forEach(line => {
            const idx = line.indexOf('=')
            if (idx > 0) {
              env[line.slice(0, idx).trim()] = line.slice(idx + 1).trim()
            }
          })
        }
        await createMcpServer({
          name: mcpFormName,
          command: mcpFormCommand,
          args,
          env,
          working_directory: mcpFormWorkingDir || undefined,
          auto_start: mcpFormAutoStart,
        })
        setIsMcpDialogOpen(false)
        resetMcpForm()
      } catch (err) {
        console.error('Failed to create server:', err)
      } finally {
        setIsSaving(false)
      }
    }
  }

  const resetMcpForm = () => {
    setMcpFormName('')
    setMcpFormCommand('')
    setMcpFormArgs('')
    setMcpFormEnv('')
    setMcpFormWorkingDir('')
    setMcpFormAutoStart(false)
  }

  const openMcpDialog = (mode: 'manual' | 'import') => {
    setMcpDialogMode(mode)
    resetMcpForm()
    setMcpImportJson('')
    setIsMcpDialogOpen(true)
  }

  // Get icon component
  const getIcon = (iconName?: string): LucideIcon => {
    if (iconName && iconMap[iconName]) {
      return iconMap[iconName]
    }
    return Wrench
  }

  // Filter tools based on selected filter
  const filteredTools = filterType === 'all'
    ? tools
    : filterType === 'builtin'
      ? builtinTools
      : filterType === 'custom'
        ? customTools
        : filterType === 'mcp'
          ? mcpTools
          : defaultTools

  // Validate JSON schema
  const validateSchema = (schemaStr: string): boolean => {
    try {
      const parsed = JSON.parse(schemaStr)
      if (!isValidFunctionSchema(parsed)) {
        setSchemaError('Schema must have name, description, and parameters with properties')
        return false
      }
      setSchemaError(null)
      return true
    } catch {
      setSchemaError('Invalid JSON')
      return false
    }
  }

  // Generate default schema template
  const getDefaultSchema = (name: string, description: string): string => {
    return JSON.stringify({
      name: name.toLowerCase().replace(/\s+/g, '_'),
      description: description || 'Tool description',
      parameters: {
        type: 'object',
        properties: {
          example_param: {
            type: 'string',
            description: 'An example parameter'
          }
        },
        required: []
      }
    }, null, 2)
  }

  // Open editor for creating new tool
  const handleCreate = () => {
    setEditingTool(null)
    setFormName('')
    setFormDescription('')
    setFormSchema(getDefaultSchema('new_tool', 'A new custom tool'))
    setFormIcon('Wrench')
    setFormExecutionLocation('backend')
    setSchemaError(null)
    setIsEditorOpen(true)
  }

  // Open editor for editing existing tool
  const handleEdit = (tool: Tool) => {
    if (tool.tool_type === 'builtin') return // Can't edit builtin tools

    setEditingTool(tool)
    setFormName(tool.name)
    setFormDescription(tool.description || '')

    // Pretty-print the schema
    try {
      const parsed = JSON.parse(tool.function_schema)
      setFormSchema(JSON.stringify(parsed, null, 2))
    } catch {
      setFormSchema(tool.function_schema)
    }

    setFormIcon(tool.icon || 'Wrench')
    setFormExecutionLocation(tool.execution_location as 'backend' | 'frontend')
    setSchemaError(null)
    setIsEditorOpen(true)
  }

  // Confirm delete
  const handleDeleteClick = (tool: Tool) => {
    setToolToDelete(tool)
    setIsDeleteDialogOpen(true)
  }

  // Execute delete
  const handleConfirmDelete = async () => {
    if (!toolToDelete) return
    try {
      setIsSaving(true)
      await deleteTool(toolToDelete.id)
    } finally {
      setIsSaving(false)
      setIsDeleteDialogOpen(false)
      setToolToDelete(null)
    }
  }

  // Toggle default status
  const handleToggleDefault = async (tool: Tool) => {
    await setToolDefault(tool.id, !tool.is_default)
  }

  // Toggle enabled status
  const handleToggleEnabled = async (tool: Tool) => {
    await toggleToolEnabled(tool.id, !tool.enabled)
  }

  // Save tool (create or update)
  const handleSave = async () => {
    if (!formName.trim() || !formSchema.trim()) return
    if (!validateSchema(formSchema)) return

    try {
      setIsSaving(true)

      // Update schema with form name/description
      let schema = JSON.parse(formSchema)
      schema.name = formName.toLowerCase().replace(/\s+/g, '_')
      schema.description = formDescription || schema.description
      const finalSchema = JSON.stringify(schema)

      if (editingTool) {
        // Update existing tool
        const updates: UpdateTool = {
          name: formName,
          description: formDescription || undefined,
          function_schema: finalSchema,
          execution_location: formExecutionLocation,
          icon: formIcon,
        }
        await updateTool(editingTool.id, updates)
      } else {
        // Create new tool
        const input: CreateTool = {
          name: formName,
          description: formDescription || undefined,
          function_schema: finalSchema,
          execution_location: formExecutionLocation,
          icon: formIcon,
        }
        await createTool(input)
      }
      setIsEditorOpen(false)
    } finally {
      setIsSaving(false)
    }
  }

  // Update schema when name changes
  const handleNameChange = (name: string) => {
    setFormName(name)
    try {
      const schema = JSON.parse(formSchema)
      schema.name = name.toLowerCase().replace(/\s+/g, '_')
      setFormSchema(JSON.stringify(schema, null, 2))
    } catch {
      // Ignore if schema is invalid
    }
  }

  return (
    <>
      {/* Header */}
      <header className="flex h-16 items-center justify-between border-b border-border bg-background/50 backdrop-blur-sm px-8">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-semibold text-foreground">AI Tools</h1>
          <Badge variant="secondary">
            {isLoading ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              `${tools.length} tools`
            )}
          </Badge>
        </div>

        <Button onClick={handleCreate}>
          <Plus className="w-4 h-4 mr-2" />
          Create Tool
        </Button>
      </header>

      {/* Main Content */}
      <div className="flex-1 overflow-y-auto p-8">
        <div className="mx-auto max-w-6xl space-y-6">
          {/* MCP Servers Section */}
          <Collapsible open={isMcpExpanded} onOpenChange={setIsMcpExpanded}>
            <Card className="p-4">
              <CollapsibleTrigger asChild>
                <div className="flex items-center justify-between cursor-pointer">
                  <div className="flex items-center gap-3">
                    {isMcpExpanded ? (
                      <ChevronDown className="w-4 h-4 text-muted-foreground" />
                    ) : (
                      <ChevronRight className="w-4 h-4 text-muted-foreground" />
                    )}
                    <div className="flex items-center gap-2">
                      <Server className="w-5 h-5 text-primary" />
                      <h2 className="text-lg font-medium">MCP Servers</h2>
                    </div>
                    <Badge variant="secondary">
                      {mcpServers.length} server{mcpServers.length !== 1 ? 's' : ''}
                    </Badge>
                    {mcpServers.filter(s => s.status === 'running').length > 0 && (
                      <Badge variant="outline" className="text-green-600 border-green-500">
                        {mcpServers.filter(s => s.status === 'running').length} running
                      </Badge>
                    )}
                  </div>
                  <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
                    <Button variant="outline" size="sm" onClick={() => openMcpDialog('import')}>
                      <Import className="w-4 h-4 mr-2" />
                      Import JSON
                    </Button>
                    <Button size="sm" onClick={() => openMcpDialog('manual')}>
                      <Plus className="w-4 h-4 mr-2" />
                      Add Server
                    </Button>
                  </div>
                </div>
              </CollapsibleTrigger>

              <CollapsibleContent className="mt-4">
                {mcpError && (
                  <div className="p-4 rounded-lg bg-destructive/10 text-destructive text-sm mb-4">
                    {mcpError}
                  </div>
                )}

                {mcpLoading && mcpServers.length === 0 && (
                  <div className="flex items-center justify-center py-8">
                    <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
                  </div>
                )}

                {!mcpLoading && mcpServers.length === 0 && (
                  <div className="text-center py-8 text-muted-foreground">
                    <Unplug className="w-8 h-8 mx-auto mb-2 opacity-50" />
                    <p>No MCP servers configured</p>
                    <p className="text-xs mt-1">Add a server or import from MCP JSON config</p>
                  </div>
                )}

                {mcpServers.length > 0 && (
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    {mcpServers.map((server) => {
                      const args = parseServerArgs(server)
                      const isActionLoading = mcpServerActionLoading === server.id
                      const isRunning = server.status === 'running'
                      const isStarting = server.status === 'starting'

                      return (
                        <Card key={server.id} className="p-4 border-l-4" style={{
                          borderLeftColor: server.status === 'running' ? 'rgb(34 197 94)' :
                                          server.status === 'error' ? 'rgb(239 68 68)' :
                                          server.status === 'starting' ? 'rgb(234 179 8)' : 'rgb(156 163 175)'
                        }}>
                          <div className="flex items-start justify-between">
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <h3 className="font-medium truncate">{server.name}</h3>
                                <div className="flex items-center gap-1">
                                  {getStatusIcon(server.status as McpServerStatus)}
                                  <Badge className={`text-xs ${getStatusColor(server.status as McpServerStatus)}`}>
                                    {server.status}
                                  </Badge>
                                </div>
                              </div>
                              <div className="text-sm text-muted-foreground mt-1 font-mono truncate">
                                {server.command} {args.slice(0, 2).join(' ')}{args.length > 2 ? '...' : ''}
                              </div>
                              {server.last_error && (
                                <div className="text-xs text-destructive mt-1 truncate">
                                  {server.last_error}
                                </div>
                              )}
                              <div className="flex items-center gap-3 mt-2">
                                <Badge variant="outline" className="text-xs">
                                  {server.tool_count} tool{server.tool_count !== 1 ? 's' : ''}
                                </Badge>
                                <div className="flex items-center gap-1">
                                  <Switch
                                    checked={server.auto_start}
                                    onCheckedChange={() => handleMcpToggleAutoStart(server)}
                                    disabled={isActionLoading}
                                  />
                                  <span className="text-xs text-muted-foreground">Auto-start</span>
                                </div>
                              </div>
                            </div>

                            <div className="flex items-center gap-1 ml-2">
                              {isRunning ? (
                                <>
                                  <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={() => handleMcpRestart(server)}
                                    disabled={isActionLoading}
                                    title="Restart server"
                                  >
                                    {isActionLoading ? (
                                      <Loader2 className="w-4 h-4 animate-spin" />
                                    ) : (
                                      <RotateCw className="w-4 h-4" />
                                    )}
                                  </Button>
                                  <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={() => handleMcpStop(server)}
                                    disabled={isActionLoading}
                                    title="Stop server"
                                  >
                                    <Square className="w-4 h-4" />
                                  </Button>
                                </>
                              ) : (
                                <Button
                                  variant="outline"
                                  size="sm"
                                  onClick={() => handleMcpStart(server)}
                                  disabled={isActionLoading || isStarting}
                                  title="Start server"
                                >
                                  {isActionLoading || isStarting ? (
                                    <Loader2 className="w-4 h-4 animate-spin" />
                                  ) : (
                                    <Play className="w-4 h-4" />
                                  )}
                                </Button>
                              )}
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setMcpDeleteConfirm(server)}
                                disabled={isActionLoading}
                                className="text-destructive hover:text-destructive"
                                title="Delete server"
                              >
                                <Trash2 className="w-4 h-4" />
                              </Button>
                            </div>
                          </div>
                        </Card>
                      )
                    })}
                  </div>
                )}
              </CollapsibleContent>
            </Card>
          </Collapsible>

          {/* Filter Buttons */}
          <div className="flex items-center gap-2 flex-wrap">
            <Filter className="h-4 w-4 text-muted-foreground" />
            <Button
              variant={filterType === 'all' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilterType('all')}
              className="h-7"
            >
              All
            </Button>
            <Button
              variant={filterType === 'builtin' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilterType('builtin')}
              className="h-7"
            >
              Built-in
            </Button>
            <Button
              variant={filterType === 'custom' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilterType('custom')}
              className="h-7"
            >
              Custom
            </Button>
            <Button
              variant={filterType === 'mcp' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilterType('mcp')}
              className="h-7"
            >
              <Server className="w-3 h-3 mr-1" />
              MCP
            </Button>
            <Button
              variant={filterType === 'defaults' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilterType('defaults')}
              className="h-7"
            >
              <Star className="w-3 h-3 mr-1" />
              Defaults
            </Button>
          </div>

          {/* Error State */}
          {error && (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <div className="p-4 rounded-full bg-destructive/10 mb-4">
                <Wrench className="h-8 w-8 text-destructive" />
              </div>
              <h3 className="text-lg font-medium text-foreground">
                Error loading tools
              </h3>
              <p className="text-muted-foreground mt-1">{error}</p>
            </div>
          )}

          {/* Loading State */}
          {isLoading && !error && (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mb-4" />
              <p className="text-muted-foreground">Loading tools...</p>
            </div>
          )}

          {/* Empty State */}
          {!isLoading && !error && filteredTools.length === 0 && (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <div className="p-4 rounded-full bg-muted mb-4">
                {filterType === 'mcp' ? (
                  <Server className="h-8 w-8 text-muted-foreground" />
                ) : (
                  <Wrench className="h-8 w-8 text-muted-foreground" />
                )}
              </div>
              <h3 className="text-lg font-medium text-foreground">
                {filterType === 'custom' ? 'No custom tools yet' :
                 filterType === 'mcp' ? 'No MCP servers connected' :
                 filterType === 'defaults' ? 'No default tools set' : 'No tools found'}
              </h3>
              <p className="text-muted-foreground mt-1">
                {filterType === 'custom'
                  ? 'Create your first custom tool to get started'
                  : filterType === 'mcp'
                    ? 'Connect to MCP servers to discover their tools'
                    : filterType === 'defaults'
                      ? 'Mark tools as default to include them in all chats'
                      : 'Try adjusting your filter'}
              </p>
              {filterType === 'mcp' && (
                <p className="text-xs text-muted-foreground mt-4 max-w-md">
                  MCP (Model Context Protocol) servers expose tools that can be used by AI.
                  Configure MCP servers in Settings to discover and use their tools.
                </p>
              )}
            </div>
          )}

          {/* Tools Grid */}
          {!isLoading && !error && filteredTools.length > 0 && (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              {filteredTools.map((tool) => {
                const Icon = getIcon(tool.icon || undefined)
                const schema = parseFunctionSchema(tool.function_schema)
                const paramCount = schema ? Object.keys(schema.parameters.properties).length : 0

                return (
                  <Card
                    key={tool.id}
                    className={`p-4 transition-shadow group ${
                      tool.tool_type !== 'builtin' ? 'hover:shadow-md cursor-pointer' : ''
                    } ${!tool.enabled ? 'opacity-60' : ''}`}
                    onClick={() => tool.tool_type !== 'builtin' && handleEdit(tool)}
                  >
                    <div className="flex items-start gap-3">
                      <div className={`flex-shrink-0 w-10 h-10 rounded-lg flex items-center justify-center ${
                        tool.is_default ? 'bg-yellow-500/20' : 'bg-primary/10'
                      }`}>
                        <Icon className={`w-5 h-5 ${tool.is_default ? 'text-yellow-600' : 'text-primary'}`} />
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <h3 className="font-medium truncate">{tool.name}</h3>
                          {tool.tool_type === 'builtin' && (
                            <Badge variant="secondary" className="flex-shrink-0 text-xs">
                              Built-in
                            </Badge>
                          )}
                          {tool.tool_type === 'mcp' && (
                            <Badge variant="outline" className="flex-shrink-0 text-xs border-blue-500 text-blue-600">
                              <Server className="w-2.5 h-2.5 mr-1" />
                              MCP
                            </Badge>
                          )}
                          {tool.is_default && (
                            <Star className="w-3 h-3 text-yellow-500 fill-yellow-500" />
                          )}
                        </div>
                        {tool.description && (
                          <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
                            {tool.description}
                          </p>
                        )}
                        <div className="flex items-center gap-2 mt-2">
                          <Badge variant="outline" className="text-xs">
                            {paramCount} param{paramCount !== 1 ? 's' : ''}
                          </Badge>
                          <Badge variant="outline" className="text-xs">
                            {tool.execution_location}
                          </Badge>
                        </div>
                      </div>
                    </div>

                    {/* Toggles and Actions */}
                    <div className="flex items-center justify-between mt-4 pt-3 border-t">
                      <div className="flex items-center gap-4">
                        <div className="flex items-center gap-2">
                          <Switch
                            checked={tool.enabled}
                            onCheckedChange={() => handleToggleEnabled(tool)}
                            onClick={(e) => e.stopPropagation()}
                          />
                          <span className="text-xs text-muted-foreground">
                            {tool.enabled ? 'Enabled' : 'Disabled'}
                          </span>
                        </div>
                        <div className="flex items-center gap-2">
                          <Switch
                            checked={tool.is_default}
                            onCheckedChange={() => handleToggleDefault(tool)}
                            onClick={(e) => e.stopPropagation()}
                          />
                          <span className="text-xs text-muted-foreground">Default</span>
                        </div>
                      </div>

                      {tool.tool_type !== 'builtin' && (
                        <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation()
                              handleEdit(tool)
                            }}
                            disabled={isSaving}
                          >
                            <Pencil className="w-4 h-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation()
                              handleDeleteClick(tool)
                            }}
                            disabled={isSaving}
                            className="text-destructive hover:text-destructive"
                          >
                            <Trash2 className="w-4 h-4" />
                          </Button>
                        </div>
                      )}
                    </div>
                  </Card>
                )
              })}
            </div>
          )}
        </div>

        {/* Tool Editor Dialog */}
        <Dialog open={isEditorOpen} onOpenChange={setIsEditorOpen}>
          <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
            <DialogHeader>
              <DialogTitle>
                {editingTool ? 'Edit Tool' : 'Create Tool'}
              </DialogTitle>
              <DialogDescription>
                {editingTool
                  ? 'Update your custom tool configuration'
                  : 'Define a new tool with its function schema'
                }
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              {/* Name field */}
              <div className="space-y-2">
                <Label htmlFor="name">Name</Label>
                <Input
                  id="name"
                  value={formName}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) => handleNameChange(e.target.value)}
                  placeholder="e.g., search_web"
                />
              </div>

              {/* Description field */}
              <div className="space-y-2">
                <Label htmlFor="description">Description</Label>
                <Input
                  id="description"
                  value={formDescription}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) => setFormDescription(e.target.value)}
                  placeholder="Brief description of what this tool does"
                />
              </div>

              {/* Function Schema */}
              <div className="space-y-2">
                <Label htmlFor="schema">Function Schema (JSON)</Label>
                <Textarea
                  id="schema"
                  value={formSchema}
                  onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => {
                    setFormSchema(e.target.value)
                    validateSchema(e.target.value)
                  }}
                  placeholder="Enter JSON schema..."
                  rows={12}
                  className="font-mono text-sm"
                />
                {schemaError && (
                  <p className="text-sm text-destructive">{schemaError}</p>
                )}
                <p className="text-xs text-muted-foreground">
                  Define the parameters the AI can pass to this tool using JSON Schema format.
                </p>
              </div>

              {/* Execution Location */}
              <div className="space-y-2">
                <Label htmlFor="execution">Execution Location</Label>
                <Select value={formExecutionLocation} onValueChange={(v) => setFormExecutionLocation(v as 'backend' | 'frontend')}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="backend">Backend (Rust)</SelectItem>
                    <SelectItem value="frontend">Frontend (JavaScript)</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Where the tool code will execute when called by the AI.
                </p>
              </div>

              {/* Icon selector */}
              <div className="space-y-2">
                <Label htmlFor="icon">Icon</Label>
                <Select value={formIcon} onValueChange={setFormIcon}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {availableIcons.map((icon) => {
                      const IconComponent = icon.icon
                      return (
                        <SelectItem key={icon.value} value={icon.value}>
                          <div className="flex items-center gap-2">
                            <IconComponent className="w-4 h-4" />
                            <span>{icon.label}</span>
                          </div>
                        </SelectItem>
                      )
                    })}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setIsEditorOpen(false)}
                disabled={isSaving}
              >
                Cancel
              </Button>
              <Button
                onClick={handleSave}
                disabled={isSaving || !formName.trim() || !formSchema.trim() || !!schemaError}
              >
                {isSaving && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                {editingTool ? 'Save Changes' : 'Create Tool'}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Delete Confirmation Dialog */}
        <Dialog open={isDeleteDialogOpen} onOpenChange={setIsDeleteDialogOpen}>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Delete Tool</DialogTitle>
              <DialogDescription>
                Are you sure you want to delete "{toolToDelete?.name}"? This action cannot be undone.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setIsDeleteDialogOpen(false)}
                disabled={isSaving}
              >
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={handleConfirmDelete}
                disabled={isSaving}
              >
                {isSaving && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                Delete
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* MCP Server Add/Import Dialog */}
        <Dialog open={isMcpDialogOpen} onOpenChange={setIsMcpDialogOpen}>
          <DialogContent className="sm:max-w-xl">
            <DialogHeader>
              <DialogTitle>
                {mcpDialogMode === 'import' ? 'Import MCP Servers' : 'Add MCP Server'}
              </DialogTitle>
              <DialogDescription>
                {mcpDialogMode === 'import'
                  ? 'Paste a standard MCP configuration JSON to import servers'
                  : 'Configure a new MCP server to connect to'
                }
              </DialogDescription>
            </DialogHeader>

            {/* Mode Switcher */}
            <div className="flex items-center gap-2 border-b pb-4">
              <Button
                variant={mcpDialogMode === 'manual' ? 'default' : 'ghost'}
                size="sm"
                onClick={() => setMcpDialogMode('manual')}
              >
                <Terminal className="w-4 h-4 mr-2" />
                Manual Entry
              </Button>
              <Button
                variant={mcpDialogMode === 'import' ? 'default' : 'ghost'}
                size="sm"
                onClick={() => setMcpDialogMode('import')}
              >
                <Import className="w-4 h-4 mr-2" />
                Import JSON
              </Button>
            </div>

            {mcpDialogMode === 'import' ? (
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="import-json">MCP Configuration JSON</Label>
                  <Textarea
                    id="import-json"
                    value={mcpImportJson}
                    onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setMcpImportJson(e.target.value)}
                    placeholder={`{
  "server-name": {
    "command": "node",
    "args": ["path/to/server.js"],
    "env": { "API_KEY": "..." }
  }
}`}
                    rows={10}
                    className="font-mono text-sm"
                  />
                  <p className="text-xs text-muted-foreground">
                    Standard MCP config format used by Claude Desktop, Cursor, etc.
                  </p>
                </div>
              </div>
            ) : (
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="mcp-name">Server Name</Label>
                  <Input
                    id="mcp-name"
                    value={mcpFormName}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setMcpFormName(e.target.value)}
                    placeholder="e.g., my-mcp-server"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="mcp-command">Command</Label>
                  <Input
                    id="mcp-command"
                    value={mcpFormCommand}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setMcpFormCommand(e.target.value)}
                    placeholder="e.g., node, python, npx"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="mcp-args">Arguments (one per line)</Label>
                  <Textarea
                    id="mcp-args"
                    value={mcpFormArgs}
                    onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setMcpFormArgs(e.target.value)}
                    placeholder="path/to/server.js&#10;--port&#10;3000"
                    rows={3}
                    className="font-mono text-sm"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="mcp-env">Environment Variables (KEY=value, one per line)</Label>
                  <Textarea
                    id="mcp-env"
                    value={mcpFormEnv}
                    onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setMcpFormEnv(e.target.value)}
                    placeholder="API_KEY=your-key&#10;DEBUG=true"
                    rows={3}
                    className="font-mono text-sm"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="mcp-workdir">Working Directory (optional)</Label>
                  <Input
                    id="mcp-workdir"
                    value={mcpFormWorkingDir}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setMcpFormWorkingDir(e.target.value)}
                    placeholder="e.g., C:\path\to\server"
                  />
                </div>

                <div className="flex items-center gap-2">
                  <Switch
                    checked={mcpFormAutoStart}
                    onCheckedChange={setMcpFormAutoStart}
                  />
                  <Label>Auto-start on app launch</Label>
                </div>
              </div>
            )}

            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setIsMcpDialogOpen(false)}
                disabled={isSaving}
              >
                Cancel
              </Button>
              <Button
                onClick={handleMcpCreate}
                disabled={isSaving || (mcpDialogMode === 'manual' ? !mcpFormName.trim() || !mcpFormCommand.trim() : !mcpImportJson.trim())}
              >
                {isSaving && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                {mcpDialogMode === 'import' ? 'Import' : 'Add Server'}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* MCP Server Delete Confirmation Dialog */}
        <Dialog open={!!mcpDeleteConfirm} onOpenChange={(open) => !open && setMcpDeleteConfirm(null)}>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Delete MCP Server</DialogTitle>
              <DialogDescription>
                Are you sure you want to delete "{mcpDeleteConfirm?.name}"? This will also remove all tools discovered from this server.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setMcpDeleteConfirm(null)}
                disabled={!!mcpServerActionLoading}
              >
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={handleMcpDelete}
                disabled={!!mcpServerActionLoading}
              >
                {mcpServerActionLoading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                Delete
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </>
  )
}
