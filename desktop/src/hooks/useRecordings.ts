'use client'

import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Recording, RecordingWithMetadata, RecordingUpdate } from '@/types/database'

export function useRecordings(limit?: number) {
  const [recordings, setRecordings] = useState<RecordingWithMetadata[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Fetch all recordings or recent recordings with limit
  const fetchRecordings = useCallback(async () => {
    try {
      setLoading(true)
      setError(null)

      let result: RecordingWithMetadata[]
      if (limit) {
        result = await invoke<RecordingWithMetadata[]>('db_get_recent_recordings', { limit })
      } else {
        result = await invoke<RecordingWithMetadata[]>('db_get_all_recordings')
      }

      setRecordings(result)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to fetch recordings: ${errorMessage}`)
      console.error('Fetch recordings error:', err)
    } finally {
      setLoading(false)
    }
  }, [limit])

  useEffect(() => {
    fetchRecordings()
  }, [fetchRecordings])

  // Get a single recording by ID
  const getRecording = useCallback(async (id: string): Promise<RecordingWithMetadata | null> => {
    try {
      const result = await invoke<RecordingWithMetadata | null>('db_get_recording', { id })
      return result
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to get recording: ${errorMessage}`)
      console.error('Get recording error:', err)
      return null
    }
  }, [])

  // Create a new recording
  const createRecording = useCallback(async (
    title: string,
    meetingFolderPath?: string,
    microphoneDevice?: string,
    systemAudioDevice?: string,
    sampleRate?: number,
    transcriptionModel?: string,
    language?: string
  ): Promise<string | null> => {
    try {
      setError(null)
      const id = await invoke<string>('db_create_recording', {
        title,
        meetingFolderPath,
        microphoneDevice,
        systemAudioDevice,
        sampleRate: sampleRate ?? 48000,
        transcriptionModel,
        language,
      })

      // Refresh the list
      await fetchRecordings()
      return id
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to create recording: ${errorMessage}`)
      console.error('Create recording error:', err)
      return null
    }
  }, [fetchRecordings])

  // Update a recording
  const updateRecording = useCallback(async (id: string, updates: RecordingUpdate): Promise<void> => {
    try {
      setError(null)
      await invoke('db_update_recording', { id, updates })

      // Update local state
      setRecordings(prev => prev.map(r => {
        if (r.recording.id === id) {
          return {
            ...r,
            recording: {
              ...r.recording,
              ...updates,
            }
          }
        }
        return r
      }))
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to update recording: ${errorMessage}`)
      console.error('Update recording error:', err)
      throw err
    }
  }, [])

  // Complete a recording (set status to completed, duration, etc.)
  const completeRecording = useCallback(async (
    id: string,
    audioFilePath?: string,
    durationSeconds?: number
  ): Promise<void> => {
    try {
      setError(null)
      await invoke('db_complete_recording', {
        id,
        audioFilePath,
        durationSeconds,
      })

      // Refresh the list
      await fetchRecordings()
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to complete recording: ${errorMessage}`)
      console.error('Complete recording error:', err)
      throw err
    }
  }, [fetchRecordings])

  // Delete a recording
  const deleteRecording = useCallback(async (id: string): Promise<void> => {
    try {
      setError(null)
      await invoke('db_delete_recording', { id })

      // Update local state
      setRecordings(prev => prev.filter(r => r.recording.id !== id))
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to delete recording: ${errorMessage}`)
      console.error('Delete recording error:', err)
      throw err
    }
  }, [])

  return {
    recordings,
    setRecordings,
    loading,
    error,
    refresh: fetchRecordings,
    getRecording,
    createRecording,
    updateRecording,
    completeRecording,
    deleteRecording,
  }
}
