'use client'

import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// Registered speaker info
export interface RegisteredSpeaker {
  id: string
  name: string
  created_at: string
  sample_count: number
  last_seen: string | null
}

// Model info
export interface DiarizationModelInfo {
  name: string
  size_mb: number
  is_downloaded: boolean
  path: string | null
}

// Download progress event
interface DownloadProgress {
  progress: number
  model: string
}

export function useDiarization() {
  const [registeredSpeakers, setRegisteredSpeakers] = useState<RegisteredSpeaker[]>([])
  const [modelsReady, setModelsReady] = useState(false)
  const [modelsInfo, setModelsInfo] = useState<DiarizationModelInfo[]>([])
  const [isDownloading, setIsDownloading] = useState(false)
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  // Check models on mount
  useEffect(() => {
    checkModels()
    loadRegisteredSpeakers()
  }, [])

  // Listen for download events
  useEffect(() => {
    let unlistenProgress: (() => void) | undefined
    let unlistenReady: (() => void) | undefined
    let unlistenError: (() => void) | undefined

    const setupListeners = async () => {
      unlistenProgress = await listen<DownloadProgress>(
        'diarization-model-download-progress',
        (event) => {
          setDownloadProgress(event.payload)
        }
      )

      unlistenReady = await listen(
        'diarization-models-ready',
        () => {
          setModelsReady(true)
          setIsDownloading(false)
          setDownloadProgress(null)
          checkModels()
        }
      )

      unlistenError = await listen<{ error: string }>(
        'diarization-model-download-error',
        (event) => {
          console.error('Diarization model download error:', event.payload.error)
          setIsDownloading(false)
          setDownloadProgress(null)
        }
      )
    }

    setupListeners()

    return () => {
      if (unlistenProgress) unlistenProgress()
      if (unlistenReady) unlistenReady()
      if (unlistenError) unlistenError()
    }
  }, [])

  const checkModels = useCallback(async () => {
    try {
      setIsLoading(true)
      const [ready, info] = await Promise.all([
        invoke<boolean>('are_diarization_models_ready'),
        invoke<DiarizationModelInfo[]>('check_diarization_models'),
      ])
      setModelsReady(ready)
      setModelsInfo(info)
    } catch (err) {
      console.error('Failed to check diarization models:', err)
    } finally {
      setIsLoading(false)
    }
  }, [])

  const downloadModels = useCallback(async () => {
    try {
      setIsDownloading(true)
      await invoke('download_diarization_models')
    } catch (err) {
      console.error('Failed to download diarization models:', err)
      setIsDownloading(false)
      throw err
    }
  }, [])

  const loadRegisteredSpeakers = useCallback(async () => {
    try {
      const speakers = await invoke<RegisteredSpeaker[]>('get_registered_speakers')
      setRegisteredSpeakers(speakers)
    } catch (err) {
      console.error('Failed to load registered speakers:', err)
    }
  }, [])

  const registerVoice = useCallback(async (name: string, audioSamples: Float32Array) => {
    try {
      const id = await invoke<string>('register_speaker_voice', {
        name,
        audioSamples: Array.from(audioSamples),
      })
      await loadRegisteredSpeakers()
      return id
    } catch (err) {
      console.error('Failed to register voice:', err)
      throw err
    }
  }, [loadRegisteredSpeakers])

  const deleteRegisteredSpeaker = useCallback(async (speakerId: string) => {
    try {
      await invoke('delete_registered_speaker', { speakerId })
      await loadRegisteredSpeakers()
    } catch (err) {
      console.error('Failed to delete registered speaker:', err)
      throw err
    }
  }, [loadRegisteredSpeakers])

  const renameSpeaker = useCallback(async (speakerId: string, newLabel: string) => {
    try {
      // First, always persist to database (update all transcript segments with this speaker_id)
      // This works even when viewing old transcripts without an active diarization session
      const rowsUpdated = await invoke<number>('db_update_speaker_label', { speakerId, newLabel })
      console.log(`Updated ${rowsUpdated} transcript segments with new speaker label`)

      // Try to update in-memory session state (may fail if engine not active, which is OK)
      try {
        await invoke('rename_speaker', { speakerId, newLabel })
      } catch (engineErr) {
        // This is expected when viewing old transcripts without active diarization
        console.log('Diarization engine not active, skipping in-memory update')
      }

      // Refresh registered speakers list in case this was a registered speaker
      await loadRegisteredSpeakers()
    } catch (err) {
      console.error('Failed to rename speaker:', err)
      throw err
    }
  }, [loadRegisteredSpeakers])

  return {
    // Model state
    modelsReady,
    modelsInfo,
    isLoading,
    isDownloading,
    downloadProgress,
    downloadModels,
    checkModels,

    // Registered speakers
    registeredSpeakers,
    loadRegisteredSpeakers,
    registerVoice,
    deleteRegisteredSpeaker,
    renameSpeaker,
  }
}
