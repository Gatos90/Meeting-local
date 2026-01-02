'use client'

import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import type {
  ChatMessage,
  SendMessageResponse,
  ChatStreamEvent,
  ChatCompleteEvent,
  ChatMessageStatusResponse,
  ChatConfig
} from '@/types/chat'

interface UseChatOptions {
  sessionId: string
  /** Provider type for messages */
  providerType?: string
  /** Model ID for messages */
  modelId?: string
  /** Tool IDs to enable for this chat */
  toolIds?: string[]
  /** Poll interval for status updates (ms). Set to 0 to disable polling. */
  pollInterval?: number
}

export function useChat({
  sessionId,
  providerType,
  modelId,
  toolIds,
  pollInterval = 500
}: UseChatOptions) {
  // Message state
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Processing state
  const [isProcessing, setIsProcessing] = useState(false)
  const [streamingMessageId, setStreamingMessageId] = useState<string | null>(null)
  const [streamingContent, setStreamingContent] = useState('')

  // Session config
  const [sessionConfig, setSessionConfig] = useState<ChatConfig | null>(null)

  // Refs for cleanup
  const streamListenerRef = useRef<UnlistenFn | null>(null)
  const completeListenerRef = useRef<UnlistenFn | null>(null)
  const pollIntervalRef = useRef<NodeJS.Timeout | null>(null)

  // Load messages from database
  const loadMessages = useCallback(async () => {
    if (!sessionId) return

    try {
      setIsLoading(true)
      const msgs = await invoke<ChatMessage[]>('chat_get_messages', { sessionId })
      setMessages(msgs)

      // Check if any message is still processing
      const processingMsg = msgs.find(
        m => m.status === 'pending' || m.status === 'streaming'
      )
      if (processingMsg) {
        setIsProcessing(true)
        setStreamingMessageId(processingMsg.id)
        setStreamingContent(processingMsg.content)
      } else {
        setIsProcessing(false)
        setStreamingMessageId(null)
        setStreamingContent('')
      }

      setError(null)
    } catch (err) {
      console.error('Failed to load chat messages:', err)
      setError(String(err))
    } finally {
      setIsLoading(false)
    }
  }, [sessionId])

  // Load session config
  const loadSessionConfig = useCallback(async () => {
    if (!sessionId) return

    try {
      const config = await invoke<ChatConfig | null>('chat_get_config', { sessionId })
      setSessionConfig(config)
    } catch (err) {
      console.error('Failed to load session config:', err)
    }
  }, [sessionId])

  // Set up event listeners for streaming
  useEffect(() => {
    if (!sessionId) return

    const setupListeners = async () => {
      // Stream event listener - receives tokens as they arrive
      const unlistenStream = await listen<ChatStreamEvent>(
        `chat-stream-${sessionId}`,
        (event) => {
          const { message_id, content } = event.payload
          setStreamingMessageId(message_id)
          setStreamingContent(content)

          // Update message in local state
          setMessages(prev =>
            prev.map(m =>
              m.id === message_id
                ? { ...m, content, status: 'streaming' as const }
                : m
            )
          )
        }
      )
      streamListenerRef.current = unlistenStream

      // Complete event listener - fires when streaming finishes
      const unlistenComplete = await listen<ChatCompleteEvent>(
        `chat-complete-${sessionId}`,
        (event) => {
          const { message_id, status, error: errorMsg } = event.payload
          setIsProcessing(false)
          setStreamingMessageId(null)

          // Update message in local state
          setMessages(prev =>
            prev.map(m =>
              m.id === message_id
                ? {
                    ...m,
                    status: status === 'complete' ? 'complete' as const : 'error' as const,
                    error_message: errorMsg
                  }
                : m
            )
          )

          // Reload to get final content from database
          loadMessages()
        }
      )
      completeListenerRef.current = unlistenComplete
    }

    setupListeners()

    return () => {
      streamListenerRef.current?.()
      completeListenerRef.current?.()
    }
  }, [sessionId, loadMessages])

  // Load initial messages and config when sessionId changes
  useEffect(() => {
    if (sessionId) {
      loadMessages()
      loadSessionConfig()
    }
  }, [sessionId, loadMessages, loadSessionConfig])

  // Poll for status updates when processing
  useEffect(() => {
    if (!isProcessing || !streamingMessageId || pollInterval === 0) {
      if (pollIntervalRef.current) {
        clearInterval(pollIntervalRef.current)
        pollIntervalRef.current = null
      }
      return
    }

    pollIntervalRef.current = setInterval(async () => {
      try {
        const status = await invoke<ChatMessageStatusResponse | null>('chat_get_status', {
          messageId: streamingMessageId
        })

        if (status) {
          setStreamingContent(status.content)
          setMessages(prev =>
            prev.map(m =>
              m.id === status.message_id
                ? { ...m, content: status.content, status: status.status as any }
                : m
            )
          )

          // Check if completed
          if (status.status === 'complete' || status.status === 'error' || status.status === 'cancelled') {
            setIsProcessing(false)
            setStreamingMessageId(null)
            if (pollIntervalRef.current) {
              clearInterval(pollIntervalRef.current)
              pollIntervalRef.current = null
            }
          }
        }
      } catch (err) {
        console.error('Failed to poll message status:', err)
      }
    }, pollInterval)

    return () => {
      if (pollIntervalRef.current) {
        clearInterval(pollIntervalRef.current)
        pollIntervalRef.current = null
      }
    }
  }, [isProcessing, streamingMessageId, pollInterval])

  // Send a new message
  const sendMessage = useCallback(async (content: string) => {
    if (!sessionId) {
      throw new Error('No session selected')
    }

    try {
      setError(null)
      setIsProcessing(true)

      const response = await invoke<SendMessageResponse>('chat_send_message', {
        sessionId,
        content,
        providerType,
        modelId,
        toolIds
      })

      // Add user message immediately to local state
      const userMessage: ChatMessage = {
        id: response.user_message_id,
        recording_id: '', // Will be filled from backend
        session_id: sessionId,
        role: 'user',
        content,
        created_at: new Date().toISOString(),
        sequence_id: messages.length + 1,
        status: 'complete'
      }

      // Add pending assistant message
      const assistantMessage: ChatMessage = {
        id: response.assistant_message_id,
        recording_id: '',
        session_id: sessionId,
        role: 'assistant',
        content: '',
        created_at: new Date().toISOString(),
        sequence_id: messages.length + 2,
        status: 'pending',
        provider_type: providerType,
        model_id: modelId
      }

      setMessages(prev => [...prev, userMessage, assistantMessage])
      setStreamingMessageId(response.assistant_message_id)
      setStreamingContent('')

      return response
    } catch (err) {
      console.error('Failed to send chat message:', err)
      setError(String(err))
      setIsProcessing(false)
      throw err
    }
  }, [sessionId, messages.length, providerType, modelId, toolIds])

  // Cancel an in-progress message
  const cancelMessage = useCallback(async (messageId?: string) => {
    const targetId = messageId || streamingMessageId
    if (!targetId) return

    try {
      await invoke('chat_cancel_message', { messageId: targetId })
      setIsProcessing(false)
      setStreamingMessageId(null)

      // Update local state
      setMessages(prev =>
        prev.map(m =>
          m.id === targetId
            ? { ...m, status: 'cancelled' as const }
            : m
        )
      )
    } catch (err) {
      console.error('Failed to cancel message:', err)
      throw err
    }
  }, [streamingMessageId])

  // Clear session messages (keep the session)
  const clearHistory = useCallback(async () => {
    if (!sessionId) return

    try {
      await invoke('chat_clear_session', { sessionId })
      setMessages([])
      setIsProcessing(false)
      setStreamingMessageId(null)
      setStreamingContent('')
      setError(null)
    } catch (err) {
      console.error('Failed to clear chat session:', err)
      setError(String(err))
      throw err
    }
  }, [sessionId])

  // Check if currently processing
  const checkProcessing = useCallback(async () => {
    if (!sessionId) return false

    try {
      const processing = await invoke<boolean>('chat_is_processing', { sessionId })
      setIsProcessing(processing)
      return processing
    } catch (err) {
      console.error('Failed to check processing status:', err)
      return false
    }
  }, [sessionId])

  // Quick action helpers
  const sendQuickAction = useCallback(async (action: 'summarize' | 'key-points' | 'action-items') => {
    const prompts = {
      'summarize': 'Please provide a concise summary of this meeting.',
      'key-points': 'What are the key points discussed in this meeting?',
      'action-items': 'What action items or next steps were mentioned in this meeting?'
    }
    return sendMessage(prompts[action])
  }, [sendMessage])

  return {
    // State
    messages,
    isLoading,
    isProcessing,
    error,
    streamingMessageId,
    streamingContent,
    sessionConfig,

    // Actions
    sendMessage,
    cancelMessage,
    clearHistory,
    loadMessages,
    checkProcessing,
    loadSessionConfig,

    // Quick actions
    sendQuickAction,
  }
}
