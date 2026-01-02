'use client'

import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { ChatSession } from '@/types/chat'

interface UseChatSessionsOptions {
  recordingId: string
}

export function useChatSessions({ recordingId }: UseChatSessionsOptions) {
  const [sessions, setSessions] = useState<ChatSession[]>([])
  const [currentSession, setCurrentSession] = useState<ChatSession | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Load sessions for this recording
  const loadSessions = useCallback(async () => {
    try {
      setIsLoading(true)
      const loadedSessions = await invoke<ChatSession[]>('chat_list_sessions', { recordingId })
      setSessions(loadedSessions)

      // Auto-select newest session (first in list since sorted DESC)
      if (loadedSessions.length > 0 && !currentSession) {
        setCurrentSession(loadedSessions[0])
      }

      setError(null)
    } catch (err) {
      console.error('Failed to load chat sessions:', err)
      setError(String(err))
    } finally {
      setIsLoading(false)
    }
  }, [recordingId, currentSession])

  // Get or create a session (auto-creates if none exist)
  const getOrCreateSession = useCallback(async (): Promise<ChatSession> => {
    try {
      const session = await invoke<ChatSession>('chat_get_or_create_session', { recordingId })

      // Refresh sessions list
      await loadSessions()

      setCurrentSession(session)
      return session
    } catch (err) {
      console.error('Failed to get or create session:', err)
      setError(String(err))
      throw err
    }
  }, [recordingId, loadSessions])

  // Create a new session
  const createSession = useCallback(async (
    title?: string,
    providerType?: string,
    modelId?: string
  ): Promise<ChatSession> => {
    try {
      const session = await invoke<ChatSession>('chat_create_session', {
        recordingId,
        title,
        providerType,
        modelId
      })

      // Add to front of list (newest first)
      setSessions(prev => [session, ...prev])
      setCurrentSession(session)

      return session
    } catch (err) {
      console.error('Failed to create chat session:', err)
      setError(String(err))
      throw err
    }
  }, [recordingId])

  // Select a session
  const selectSession = useCallback((sessionId: string) => {
    const session = sessions.find(s => s.id === sessionId)
    if (session) {
      setCurrentSession(session)
    }
  }, [sessions])

  // Update session config (provider/model)
  const updateSessionConfig = useCallback(async (
    sessionId: string,
    providerType?: string,
    modelId?: string
  ) => {
    try {
      await invoke('chat_update_session_config', {
        sessionId,
        providerType,
        modelId
      })

      // Update local state
      setSessions(prev =>
        prev.map(s =>
          s.id === sessionId
            ? { ...s, provider_type: providerType, model_id: modelId }
            : s
        )
      )

      if (currentSession?.id === sessionId) {
        setCurrentSession(prev =>
          prev ? { ...prev, provider_type: providerType, model_id: modelId } : null
        )
      }
    } catch (err) {
      console.error('Failed to update session config:', err)
      throw err
    }
  }, [currentSession])

  // Update session title
  const updateSessionTitle = useCallback(async (sessionId: string, title: string) => {
    try {
      await invoke('chat_update_session_title', { sessionId, title })

      // Update local state
      setSessions(prev =>
        prev.map(s => s.id === sessionId ? { ...s, title } : s)
      )

      if (currentSession?.id === sessionId) {
        setCurrentSession(prev => prev ? { ...prev, title } : null)
      }
    } catch (err) {
      console.error('Failed to update session title:', err)
      throw err
    }
  }, [currentSession])

  // Delete a session
  const deleteSession = useCallback(async (sessionId: string) => {
    try {
      await invoke('chat_delete_session', { sessionId })

      // Remove from local state
      setSessions(prev => {
        const remaining = prev.filter(s => s.id !== sessionId)

        // If deleted session was selected, select newest remaining
        if (currentSession?.id === sessionId) {
          setCurrentSession(remaining[0] || null)
        }

        return remaining
      })
    } catch (err) {
      console.error('Failed to delete chat session:', err)
      throw err
    }
  }, [currentSession])

  // Load sessions on mount and when recordingId changes
  useEffect(() => {
    loadSessions()
  }, [loadSessions])

  // Auto-create session if none exist after loading
  useEffect(() => {
    if (!isLoading && sessions.length === 0 && !currentSession) {
      getOrCreateSession()
    }
  }, [isLoading, sessions.length, currentSession, getOrCreateSession])

  return {
    // State
    sessions,
    currentSession,
    isLoading,
    error,

    // Actions
    loadSessions,
    createSession,
    selectSession,
    updateSessionConfig,
    updateSessionTitle,
    deleteSession,
    getOrCreateSession,
  }
}
