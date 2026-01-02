'use client'

import { cn } from '@/lib/utils'
import type { ChatMessage as ChatMessageType } from '@/types/chat'
import { User, Bot, AlertCircle, Loader2, XCircle } from 'lucide-react'
import { MarkdownPreview } from '@/components/ui/markdown-preview'

interface ChatMessageProps {
  message: ChatMessageType
  streamingContent?: string
}

export function ChatMessage({ message, streamingContent }: ChatMessageProps) {
  const isUser = message.role === 'user'
  const isStreaming = message.status === 'streaming' || message.status === 'pending'
  const hasError = message.status === 'error'
  const isCancelled = message.status === 'cancelled'

  // Use streaming content if available, otherwise use message content
  const displayContent = isStreaming && streamingContent !== undefined
    ? streamingContent
    : message.content

  return (
    <div
      className={cn(
        'flex gap-3 p-4 rounded-lg',
        isUser ? 'bg-primary/5' : 'bg-muted/50',
        hasError && 'border border-destructive/50',
        isCancelled && 'opacity-60'
      )}
    >
      {/* Avatar */}
      <div
        className={cn(
          'flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center',
          isUser ? 'bg-primary text-primary-foreground' : 'bg-secondary'
        )}
      >
        {isUser ? (
          <User className="w-4 h-4" />
        ) : (
          <Bot className="w-4 h-4" />
        )}
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-sm font-medium">
            {isUser ? 'You' : 'Assistant'}
          </span>
          {isStreaming && (
            <Loader2 className="w-3 h-3 animate-spin text-muted-foreground" />
          )}
          {hasError && (
            <AlertCircle className="w-3 h-3 text-destructive" />
          )}
          {isCancelled && (
            <XCircle className="w-3 h-3 text-muted-foreground" />
          )}
        </div>

        {/* Message content */}
        <div className="text-sm text-foreground break-words">
          {displayContent ? (
            <div className="relative">
              <MarkdownPreview content={displayContent} />
              {isStreaming && (
                <span className="inline-block w-2 h-4 bg-foreground/50 animate-pulse ml-0.5" />
              )}
            </div>
          ) : (
            isStreaming && (
              <span className="text-muted-foreground italic">Thinking...</span>
            )
          )}
        </div>

        {/* Error message */}
        {hasError && message.error_message && (
          <div className="mt-2 text-xs text-destructive bg-destructive/10 p-2 rounded">
            {message.error_message}
          </div>
        )}

        {/* Cancelled indicator */}
        {isCancelled && (
          <div className="mt-2 text-xs text-muted-foreground italic">
            Message cancelled
          </div>
        )}
      </div>
    </div>
  )
}
