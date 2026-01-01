'use client'

import { useState } from 'react'
import { Plus, ChevronDown, Trash2, MessageSquare } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'
import type { ChatSession } from '@/types/chat'

interface ChatSessionSelectorProps {
  sessions: ChatSession[]
  currentSession: ChatSession | null
  onSelectSession: (sessionId: string) => void
  onCreateSession: () => void
  onDeleteSession: (sessionId: string) => void
  isLoading?: boolean
  disabled?: boolean
  className?: string
}

export function ChatSessionSelector({
  sessions,
  currentSession,
  onSelectSession,
  onCreateSession,
  onDeleteSession,
  isLoading,
  disabled,
  className,
}: ChatSessionSelectorProps) {
  const [isDeleting, setIsDeleting] = useState<string | null>(null)

  const handleDelete = async (e: React.MouseEvent, sessionId: string) => {
    e.stopPropagation()

    if (!window.confirm('Delete this chat? This cannot be undone.')) {
      return
    }

    try {
      setIsDeleting(sessionId)
      await onDeleteSession(sessionId)
    } finally {
      setIsDeleting(null)
    }
  }

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr)
    const now = new Date()
    const diffDays = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24))

    if (diffDays === 0) {
      return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    } else if (diffDays === 1) {
      return 'Yesterday'
    } else if (diffDays < 7) {
      return date.toLocaleDateString([], { weekday: 'short' })
    } else {
      return date.toLocaleDateString([], { month: 'short', day: 'numeric' })
    }
  }

  const truncateTitle = (title: string, maxLength: number = 30) => {
    if (title.length <= maxLength) return title
    return title.slice(0, maxLength) + '...'
  }

  return (
    <div className={cn('flex items-center gap-2', className)}>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            disabled={disabled || isLoading}
            className="flex-1 justify-between max-w-[250px]"
          >
            <div className="flex items-center gap-2 truncate">
              <MessageSquare className="h-4 w-4 shrink-0" />
              <span className="truncate">
                {isLoading ? 'Loading...' : (currentSession?.title || 'Select Chat')}
              </span>
            </div>
            <ChevronDown className="h-4 w-4 shrink-0 opacity-50" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-[280px]">
          {sessions.length === 0 ? (
            <div className="px-2 py-4 text-center text-sm text-muted-foreground">
              No chat sessions yet
            </div>
          ) : (
            sessions.map((session) => (
              <DropdownMenuItem
                key={session.id}
                onClick={() => onSelectSession(session.id)}
                className={cn(
                  'flex items-center justify-between group',
                  currentSession?.id === session.id && 'bg-accent'
                )}
              >
                <div className="flex flex-col gap-0.5 overflow-hidden">
                  <span className="truncate font-medium">
                    {truncateTitle(session.title)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {formatDate(session.created_at)}
                    {session.model_id && (
                      <span className="ml-2">
                        {session.model_id}
                      </span>
                    )}
                  </span>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6 opacity-0 group-hover:opacity-100 shrink-0"
                  onClick={(e) => handleDelete(e, session.id)}
                  disabled={isDeleting === session.id}
                >
                  <Trash2 className="h-3.5 w-3.5 text-muted-foreground hover:text-destructive" />
                </Button>
              </DropdownMenuItem>
            ))
          )}
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={onCreateSession}>
            <Plus className="h-4 w-4 mr-2" />
            New Chat
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <Button
        variant="outline"
        size="icon"
        className="h-8 w-8 shrink-0"
        onClick={onCreateSession}
        disabled={disabled || isLoading}
        title="New Chat"
      >
        <Plus className="h-4 w-4" />
      </Button>
    </div>
  )
}
