'use client'

import { useEffect, useRef, useState, useMemo } from 'react'
import { useChatSessions } from '@/hooks/useChatSessions'
import { useChat } from '@/hooks/useChat'
import { useDefaultModel } from '@/hooks/useDefaultModel'
import { useSessionTools } from '@/hooks/useTools'
import { ChatMessage } from './chat-message'
import { ChatInput } from './chat-input'
import { QuickActions } from './quick-actions'
import { ChatSettings } from './chat-settings'
import { ChatSessionSelector } from './chat-session-selector'
import { ToolSelector } from './tool-selector'
import { Button } from '@/components/ui/button'
import { Trash2, MessageSquare, AlertCircle } from 'lucide-react'
import { cn } from '@/lib/utils'

interface ChatPanelProps {
  recordingId: string
  className?: string
}

export function ChatPanel({ recordingId, className }: ChatPanelProps) {
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const [activeProvider, setActiveProvider] = useState<string | undefined>()
  const [activeModel, setActiveModel] = useState<string | undefined>()
  const [isProviderReady, setIsProviderReady] = useState(false)
  const [showSaveAsDefault, setShowSaveAsDefault] = useState(false)
  const [llmError, setLlmError] = useState<string | null>(null)

  // Session management
  const {
    sessions,
    currentSession,
    isLoading: sessionsLoading,
    createSession,
    selectSession,
    deleteSession,
    updateSessionConfig,
  } = useChatSessions({ recordingId })

  // Default model
  const {
    defaultModel,
    hasDefault,
    setDefault: setDefaultModel,
  } = useDefaultModel()

  // Session tools - get selected tools for this chat session
  const { sessionTools } = useSessionTools(currentSession?.id || null)

  // Memoize tool IDs to avoid unnecessary re-renders
  const toolIds = useMemo(
    () => sessionTools.map(t => t.id),
    [sessionTools]
  )

  // Chat messages for current session
  const {
    messages,
    isLoading: messagesLoading,
    isProcessing,
    error,
    streamingMessageId,
    streamingContent,
    sendMessage,
    cancelMessage,
    clearHistory,
    sessionConfig,
  } = useChat({
    sessionId: currentSession?.id || '',
    providerType: activeProvider,
    modelId: activeModel,
    toolIds,
  })

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, streamingContent])

  // Initialize provider/model from session config or default
  useEffect(() => {
    if (sessionConfig?.provider_type) {
      setActiveProvider(sessionConfig.provider_type)
    } else if (defaultModel?.provider_type) {
      setActiveProvider(defaultModel.provider_type)
    }

    if (sessionConfig?.model_id) {
      setActiveModel(sessionConfig.model_id)
    } else if (defaultModel?.model_id) {
      setActiveModel(defaultModel.model_id)
    }
  }, [sessionConfig, defaultModel])

  // Handle provider/model change from settings
  const handleConfigChange = (provider: string | undefined, model: string | undefined) => {
    setActiveProvider(provider)
    setActiveModel(model)

    // Update session config
    if (currentSession && provider && model) {
      updateSessionConfig(currentSession.id, provider, model)
    }

    // If no default and user just picked, ask to save
    if (!hasDefault && provider && model) {
      setShowSaveAsDefault(true)
    }
  }

  const handleSend = async (content: string) => {
    try {
      await sendMessage(content)
      // Hide save as default after first message
      setShowSaveAsDefault(false)
    } catch (err) {
      console.error('Failed to send message:', err)
    }
  }

  const handleCancel = () => {
    cancelMessage()
  }

  const handleQuickAction = async (prompt: string) => {
    try {
      await sendMessage(prompt)
      setShowSaveAsDefault(false)
    } catch (err) {
      console.error('Failed to send quick action:', err)
    }
  }

  const handleClearHistory = async () => {
    if (window.confirm('Are you sure you want to clear all messages in this chat?')) {
      try {
        await clearHistory()
      } catch (err) {
        console.error('Failed to clear history:', err)
      }
    }
  }

  const handleCreateSession = async () => {
    try {
      await createSession(undefined, activeProvider, activeModel)
    } catch (err) {
      console.error('Failed to create session:', err)
    }
  }

  const handleSaveAsDefault = async () => {
    if (activeProvider && activeModel) {
      await setDefaultModel(activeProvider, activeModel)
    }
    setShowSaveAsDefault(false)
  }

  const isLoading = sessionsLoading || messagesLoading

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Session selector */}
      <div className="px-4 py-2 border-b bg-muted/30">
        <ChatSessionSelector
          sessions={sessions}
          currentSession={currentSession}
          onSelectSession={selectSession}
          onCreateSession={handleCreateSession}
          onDeleteSession={deleteSession}
          isLoading={sessionsLoading}
          disabled={isProcessing}
        />
      </div>

      {/* Settings bar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b bg-muted/20">
        <div className="flex-1">
          <ChatSettings
            initialProvider={activeProvider}
            initialModel={activeModel}
            onConfigChange={handleConfigChange}
            onReady={setIsProviderReady}
            onError={setLlmError}
          />
        </div>
        <ToolSelector
          sessionId={currentSession?.id || null}
          compact
        />
      </div>

      {/* Save as default prompt */}
      {showSaveAsDefault && activeProvider && activeModel && (
        <div className="px-4 py-2 bg-primary/10 border-b flex items-center justify-between">
          <span className="text-sm">
            Save <strong>{activeModel}</strong> as your default model?
          </span>
          <div className="flex gap-2">
            <Button variant="ghost" size="sm" onClick={() => setShowSaveAsDefault(false)}>
              No
            </Button>
            <Button variant="default" size="sm" onClick={handleSaveAsDefault}>
              Yes
            </Button>
          </div>
        </div>
      )}

      {/* Header with clear button */}
      <div className="flex items-center justify-between px-4 py-2 border-b">
        <div className="flex items-center gap-2">
          <MessageSquare className="w-4 h-4 text-muted-foreground" />
          <span className="text-sm font-medium">
            {currentSession?.title || 'Chat'}
          </span>
        </div>
        {messages.length > 0 && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClearHistory}
            disabled={isProcessing}
            className="text-muted-foreground hover:text-destructive"
          >
            <Trash2 className="w-4 h-4" />
          </Button>
        )}
      </div>

      {/* Messages area */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {isLoading ? (
          <div className="flex items-center justify-center h-32">
            <div className="text-muted-foreground">Loading...</div>
          </div>
        ) : messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-32 text-center">
            <MessageSquare className="w-8 h-8 text-muted-foreground mb-2" />
            <p className="text-muted-foreground text-sm">
              Start a conversation about this recording
            </p>
          </div>
        ) : (
          messages.map((message) => (
            <ChatMessage
              key={message.id}
              message={message}
              streamingContent={
                message.id === streamingMessageId ? streamingContent : undefined
              }
            />
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Error display */}
      {error && (
        <div className="px-4 py-2 bg-destructive/10 border-t border-destructive/20">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {/* Input area */}
      <div className="border-t p-4 space-y-3">
        {!currentSession ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground justify-center py-2">
            <AlertCircle className="w-4 h-4" />
            <span>Loading chat session...</span>
          </div>
        ) : !isProviderReady ? (
          <div className="flex flex-col items-center gap-2 text-sm text-muted-foreground justify-center py-2">
            <div className="flex items-center gap-2">
              <AlertCircle className="w-4 h-4" />
              <span>{llmError ? 'Failed to load model' : 'Select a provider and model above to start chatting'}</span>
            </div>
            {llmError && (
              <p className="text-xs text-destructive max-w-md text-center">{llmError}</p>
            )}
          </div>
        ) : (
          <>
            <ChatInput
              onSend={handleSend}
              onCancel={handleCancel}
              isProcessing={isProcessing}
              disabled={isLoading}
            />

            {/* Quick actions - only show when not processing and no messages */}
            {!isProcessing && messages.length === 0 && (
              <QuickActions
                onAction={handleQuickAction}
                disabled={isLoading}
              />
            )}
          </>
        )}
      </div>
    </div>
  )
}
