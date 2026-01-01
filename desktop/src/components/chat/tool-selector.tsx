'use client'

import { useState, useEffect, useMemo } from 'react'
import { useTools, useSessionTools } from '@/hooks/useTools'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible'
import {
  Wrench,
  ChevronDown,
  ChevronRight,
  Check,
  Star,
  Loader2,
  Server,
  Search,
} from 'lucide-react'
import type { Tool } from '@/types/tools'

/** Group tools by their type and MCP server */
interface ToolGroup {
  type: 'builtin' | 'custom' | 'mcp'
  serverName?: string
  serverId?: string
  tools: Tool[]
}

interface ToolSelectorProps {
  sessionId: string | null
  disabled?: boolean
  compact?: boolean
}

export function ToolSelector({ sessionId, disabled, compact }: ToolSelectorProps) {
  const [open, setOpen] = useState(false)

  // All available tools
  const {
    tools: allTools,
    defaultTools,
    enabledTools,
    isLoading: toolsLoading,
  } = useTools()

  // Tools for this session
  const {
    sessionTools,
    isLoading: sessionLoading,
    setSessionToolIds,
    initSessionTools,
  } = useSessionTools(sessionId)

  // Track selected tools locally
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())

  // Track if user intentionally cleared all tools (prevent auto-re-init)
  const [userCleared, setUserCleared] = useState(false)

  // Reset userCleared when session changes
  useEffect(() => {
    setUserCleared(false)
  }, [sessionId])

  // Initialize selected tools from session
  useEffect(() => {
    // Don't override user's intentional clear
    if (userCleared) return

    if (sessionTools.length > 0) {
      setSelectedIds(new Set(sessionTools.map(t => t.id)))
    } else if (sessionId && defaultTools.length > 0 && !sessionLoading) {
      // Auto-initialize with defaults if session has no tools yet
      initSessionTools()
    }
  }, [sessionTools, defaultTools, sessionId, sessionLoading, userCleared])

  // Update session when selection changes (optimistic - no reload)
  const handleToolToggle = async (toolId: string, checked: boolean) => {
    const newSelection = new Set(selectedIds)
    if (checked) {
      newSelection.add(toolId)
    } else {
      newSelection.delete(toolId)
    }
    setSelectedIds(newSelection)

    // Save to session with optimistic update (pass all tools for local state update)
    if (sessionId) {
      await setSessionToolIds(Array.from(newSelection), enabledTools)
    }
  }

  // Use defaults
  const handleUseDefaults = async () => {
    const defaultIds = defaultTools.map(t => t.id)
    setSelectedIds(new Set(defaultIds))
    if (sessionId) {
      await setSessionToolIds(defaultIds, enabledTools)
    }
  }

  // Clear all
  const handleClearAll = async () => {
    setUserCleared(true)  // Prevent auto-re-init with defaults
    setSelectedIds(new Set())
    if (sessionId) {
      await setSessionToolIds([], enabledTools)
    }
  }

  // Group tools by type and MCP server
  const toolGroups = useMemo((): ToolGroup[] => {
    const groups: ToolGroup[] = []

    // Built-in tools
    const builtinTools = enabledTools.filter(t => t.tool_type === 'builtin')
    if (builtinTools.length > 0) {
      groups.push({ type: 'builtin', tools: builtinTools })
    }

    // Custom tools
    const customTools = enabledTools.filter(t => t.tool_type === 'custom')
    if (customTools.length > 0) {
      groups.push({ type: 'custom', tools: customTools })
    }

    // MCP tools grouped by server
    const mcpTools = enabledTools.filter(t => t.tool_type === 'mcp')
    const serverMap = new Map<string, Tool[]>()
    for (const tool of mcpTools) {
      const serverId = tool.mcp_server_id || 'unknown'
      if (!serverMap.has(serverId)) {
        serverMap.set(serverId, [])
      }
      serverMap.get(serverId)!.push(tool)
    }
    for (const [serverId, tools] of serverMap) {
      const serverName = tools[0]?.mcp_server_name || serverId
      groups.push({ type: 'mcp', serverId, serverName, tools })
    }

    return groups
  }, [enabledTools])

  const isLoading = toolsLoading || sessionLoading
  const selectedCount = selectedIds.size

  if (compact) {
    return (
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            className="h-8 gap-1"
            disabled={disabled || isLoading}
          >
            <Wrench className="w-3.5 h-3.5" />
            {selectedCount > 0 && (
              <Badge variant="secondary" className="h-5 px-1.5 text-xs">
                {selectedCount}
              </Badge>
            )}
            <ChevronDown className="w-3 h-3 opacity-50" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-72 p-0" align="start">
          <ToolSelectorContent
            toolGroups={toolGroups}
            selectedIds={selectedIds}
            onToggle={handleToolToggle}
            onUseDefaults={handleUseDefaults}
            onClearAll={handleClearAll}
            isLoading={isLoading}
          />
        </PopoverContent>
      </Popover>
    )
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Tools</span>
        <Badge variant="secondary" className="text-xs">
          {selectedCount} active
        </Badge>
      </div>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            className="w-full justify-between"
            disabled={disabled || isLoading}
          >
            <span className="flex items-center gap-2">
              <Wrench className="w-4 h-4" />
              {selectedCount === 0
                ? 'No tools selected'
                : `${selectedCount} tool${selectedCount !== 1 ? 's' : ''} active`}
            </span>
            <ChevronDown className="w-4 h-4 opacity-50" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-80 p-0" align="start">
          <ToolSelectorContent
            toolGroups={toolGroups}
            selectedIds={selectedIds}
            onToggle={handleToolToggle}
            onUseDefaults={handleUseDefaults}
            onClearAll={handleClearAll}
            isLoading={isLoading}
          />
        </PopoverContent>
      </Popover>
    </div>
  )
}

interface ToolSelectorContentProps {
  toolGroups: ToolGroup[]
  selectedIds: Set<string>
  onToggle: (id: string, checked: boolean) => void
  onUseDefaults: () => void
  onClearAll: () => void
  isLoading: boolean
}

function ToolSelectorContent({
  toolGroups,
  selectedIds,
  onToggle,
  onUseDefaults,
  onClearAll,
  isLoading,
}: ToolSelectorContentProps) {
  // Track which MCP servers are expanded
  const [expandedServers, setExpandedServers] = useState<Set<string>>(new Set())
  // Search state
  const [search, setSearch] = useState('')

  const toggleServer = (serverId: string) => {
    setExpandedServers(prev => {
      const next = new Set(prev)
      if (next.has(serverId)) {
        next.delete(serverId)
      } else {
        next.add(serverId)
      }
      return next
    })
  }

  // Filter tools by search
  const filteredGroups = useMemo(() => {
    if (!search.trim()) return toolGroups

    const q = search.toLowerCase()
    return toolGroups
      .map(group => ({
        ...group,
        tools: group.tools.filter(t =>
          t.name.toLowerCase().includes(q) ||
          t.description?.toLowerCase().includes(q)
        )
      }))
      .filter(group => group.tools.length > 0)
  }, [toolGroups, search])

  const totalTools = toolGroups.reduce((sum, g) => sum + g.tools.length, 0)
  const filteredTotal = filteredGroups.reduce((sum, g) => sum + g.tools.length, 0)

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="w-5 h-5 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (totalTools === 0) {
    return (
      <div className="p-4 text-center text-sm text-muted-foreground">
        No tools available. Create tools in the Tools page.
      </div>
    )
  }

  return (
    <>
      {/* Search input */}
      <div className="p-2 border-b">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground" />
          <Input
            placeholder="Search tools..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="h-8 pl-8 text-sm"
          />
        </div>
      </div>

      {/* Quick actions */}
      <div className="flex items-center gap-2 p-2 border-b bg-muted/30">
        <Button variant="ghost" size="sm" onClick={onUseDefaults} className="h-7 text-xs">
          <Star className="w-3 h-3 mr-1" />
          Use Defaults
        </Button>
        <Button variant="ghost" size="sm" onClick={onClearAll} className="h-7 text-xs">
          Clear All
        </Button>
      </div>

      {/* Tool list grouped */}
      <div className="max-h-64 overflow-y-auto p-2 space-y-2">
        {filteredGroups.length === 0 ? (
          <div className="text-center text-sm text-muted-foreground py-4">
            No tools match "{search}"
          </div>
        ) : filteredGroups.map((group, idx) => {
          if (group.type === 'mcp' && group.serverId) {
            // MCP server - collapsible group
            const isExpanded = expandedServers.has(group.serverId)
            const selectedInGroup = group.tools.filter(t => selectedIds.has(t.id)).length

            return (
              <Collapsible
                key={`mcp-${group.serverId}`}
                open={isExpanded}
                onOpenChange={() => toggleServer(group.serverId!)}
              >
                <CollapsibleTrigger className="flex items-center gap-2 w-full p-2 rounded-md hover:bg-muted cursor-pointer text-left">
                  {isExpanded ? (
                    <ChevronDown className="w-4 h-4 text-muted-foreground flex-shrink-0" />
                  ) : (
                    <ChevronRight className="w-4 h-4 text-muted-foreground flex-shrink-0" />
                  )}
                  <Server className="w-4 h-4 text-muted-foreground flex-shrink-0" />
                  <span className="text-sm font-medium flex-1 truncate">
                    {group.serverName}
                  </span>
                  <Badge variant="secondary" className="text-[10px] h-4 px-1">
                    {selectedInGroup}/{group.tools.length}
                  </Badge>
                </CollapsibleTrigger>
                <CollapsibleContent className="pl-6 space-y-1 mt-1">
                  {group.tools.map(tool => (
                    <ToolItem
                      key={tool.id}
                      tool={tool}
                      isSelected={selectedIds.has(tool.id)}
                      onToggle={onToggle}
                    />
                  ))}
                </CollapsibleContent>
              </Collapsible>
            )
          }

          // Built-in or Custom tools - flat list with optional header
          return (
            <div key={`${group.type}-${idx}`} className="space-y-1">
              {group.type === 'builtin' && (
                <div className="text-xs font-medium text-muted-foreground px-2 py-1">
                  Built-in
                </div>
              )}
              {group.type === 'custom' && (
                <div className="text-xs font-medium text-muted-foreground px-2 py-1">
                  Custom
                </div>
              )}
              {group.tools.map(tool => (
                <ToolItem
                  key={tool.id}
                  tool={tool}
                  isSelected={selectedIds.has(tool.id)}
                  onToggle={onToggle}
                />
              ))}
            </div>
          )
        })}
      </div>

      {/* Selected count */}
      <div className="p-2 border-t bg-muted/30 text-xs text-muted-foreground text-center">
        {selectedIds.size} tool{selectedIds.size !== 1 ? 's' : ''} will be available to the AI
      </div>
    </>
  )
}

/** Single tool item in the list */
function ToolItem({
  tool,
  isSelected,
  onToggle,
}: {
  tool: Tool
  isSelected: boolean
  onToggle: (id: string, checked: boolean) => void
}) {
  return (
    <label className="flex items-center gap-3 p-2 rounded-md hover:bg-muted cursor-pointer">
      <Checkbox
        checked={isSelected}
        onCheckedChange={(checked) => onToggle(tool.id, checked as boolean)}
      />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium truncate">{tool.name}</span>
          {tool.is_default && (
            <Star className="w-3 h-3 text-yellow-500 fill-yellow-500 flex-shrink-0" />
          )}
        </div>
        {tool.description && (
          <p className="text-xs text-muted-foreground truncate">
            {tool.description}
          </p>
        )}
      </div>
      {isSelected && (
        <Check className="w-4 h-4 text-primary flex-shrink-0" />
      )}
    </label>
  )
}
