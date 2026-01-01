'use client'

import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// Progress information from backend
interface RetranscriptionProgress {
  recording_id: string
  status: 'loading' | 'processing' | 'diarizing' | 'completed' | 'failed'
  progress_percent: number
  current_chunk: number
  total_chunks: number
  message: string
}

// Transcript segment from retranscription
interface RetranscriptionSegment {
  text: string
  audio_start_time: number
  audio_end_time: number
  confidence: number
  sequence_id: number
  // Speaker diarization fields
  speaker_id?: string | null
  speaker_label?: string | null
  is_registered_speaker?: boolean
}

// Result from completed retranscription
interface RetranscriptionResult {
  recording_id: string
  success: boolean
  transcripts: RetranscriptionSegment[]
  error: string | null
  model_used: string
}

// Database format for transcript segment
interface TranscriptSegmentDb {
  id: string
  recording_id: string
  text: string
  audio_start_time: number
  audio_end_time: number
  duration: number
  display_time: string
  confidence: number
  sequence_id: number
  // Speaker diarization fields
  speaker_id?: string | null
  speaker_label?: string | null
  is_registered_speaker?: boolean
}

// Format seconds to [MM:SS] display time
function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `[${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]`
}

// Status for a single recording
export interface RetranscriptionStatus {
  status: 'idle' | 'loading' | 'processing' | 'diarizing' | 'completed' | 'failed'
  progress: number
  currentChunk: number
  totalChunks: number
  message: string
  error?: string
  result?: RetranscriptionResult
}

export function useRetranscription() {
  // Status per recording ID
  const [statusMap, setStatusMap] = useState<Map<string, RetranscriptionStatus>>(new Map())

  // Track if any retranscription is in progress
  const [isRetranscribing, setIsRetranscribing] = useState(false)

  // Callback for when retranscription completes
  const onCompleteCallbackRef = useRef<((recordingId: string, result: RetranscriptionResult) => void) | null>(null)

  // Track diarization provider per recording (for saving after completion)
  const diarizationProviderMapRef = useRef<Map<string, string | null>>(new Map())

  // Listen for progress events
  useEffect(() => {
    let unlistenProgress: (() => void) | undefined
    let unlistenComplete: (() => void) | undefined

    const setupListeners = async () => {
      try {
        // Listen for progress updates
        unlistenProgress = await listen<RetranscriptionProgress>(
          'retranscription-progress',
          (event) => {
            const progress = event.payload
            console.log('Retranscription progress:', progress)

            setStatusMap((prev) => {
              const newMap = new Map(prev)
              newMap.set(progress.recording_id, {
                status: progress.status as RetranscriptionStatus['status'],
                progress: progress.progress_percent,
                currentChunk: progress.current_chunk,
                totalChunks: progress.total_chunks,
                message: progress.message,
              })
              return newMap
            })
          }
        )

        // Listen for completion events
        unlistenComplete = await listen<RetranscriptionResult>(
          'retranscription-complete',
          async (event) => {
            const result = event.payload
            console.log('Retranscription complete:', result)

            // If successful, save the new transcripts to the database
            if (result.success && result.transcripts.length > 0) {
              try {
                // Convert to database format
                const dbSegments: TranscriptSegmentDb[] = result.transcripts.map((t, idx) => ({
                  id: `retrans-${result.recording_id}-${idx}-${Date.now()}`,
                  recording_id: result.recording_id,
                  text: t.text,
                  audio_start_time: t.audio_start_time,
                  audio_end_time: t.audio_end_time,
                  duration: t.audio_end_time - t.audio_start_time,
                  display_time: formatTime(t.audio_start_time),
                  confidence: t.confidence,
                  sequence_id: t.sequence_id,
                  // Speaker diarization fields
                  speaker_id: t.speaker_id ?? null,
                  speaker_label: t.speaker_label ?? null,
                  is_registered_speaker: t.is_registered_speaker ?? false,
                }))

                // Replace existing transcripts in database
                await invoke('db_replace_transcripts', {
                  recordingId: result.recording_id,
                  segments: dbSegments,
                })
                console.log(`Replaced ${dbSegments.length} transcript segments in database`)

                // Update the recording with the model used and diarization provider
                // Use empty string to explicitly clear diarization_provider when not used
                const diarizationProvider = diarizationProviderMapRef.current.get(result.recording_id)
                await invoke('db_update_recording', {
                  id: result.recording_id,
                  updates: {
                    transcription_model: result.model_used,
                    // Empty string means "clear the field", null/undefined means "don't update"
                    diarization_provider: diarizationProvider ?? '',
                  },
                })
                console.log(`Updated recording with model: ${result.model_used}, diarization: ${diarizationProvider ?? '(cleared)'}`)

                // Clean up the provider map
                diarizationProviderMapRef.current.delete(result.recording_id)
              } catch (err) {
                console.error('Failed to save retranscription to database:', err)
              }
            }

            setStatusMap((prev) => {
              const newMap = new Map(prev)
              newMap.set(result.recording_id, {
                status: result.success ? 'completed' : 'failed',
                progress: 100,
                currentChunk: 0,
                totalChunks: 0,
                message: result.success
                  ? `Completed with ${result.transcripts.length} segments`
                  : result.error || 'Unknown error',
                error: result.error || undefined,
                result,
              })
              return newMap
            })

            // Call the completion callback if registered
            if (onCompleteCallbackRef.current) {
              onCompleteCallbackRef.current(result.recording_id, result)
            }
          }
        )
      } catch (err) {
        console.error('Failed to setup retranscription listeners:', err)
      }
    }

    setupListeners()

    return () => {
      if (unlistenProgress) unlistenProgress()
      if (unlistenComplete) unlistenComplete()
    }
  }, [])

  // Update isRetranscribing based on status map
  useEffect(() => {
    const hasActiveJob = Array.from(statusMap.values()).some(
      (status) => status.status === 'loading' || status.status === 'processing' || status.status === 'diarizing'
    )
    setIsRetranscribing(hasActiveJob)
  }, [statusMap])

  // Start retranscription for a recording
  const startRetranscription = useCallback(
    async (
      recordingId: string,
      audioPath: string,
      modelName?: string,
      language?: string,
      enableDiarization?: boolean,
      diarizationProvider?: 'pyannote' | 'sortformer',
      maxSpeakers?: number,
      similarityThreshold?: number
    ) => {
      console.log('Starting retranscription:', { recordingId, audioPath, modelName, language, enableDiarization, diarizationProvider, maxSpeakers, similarityThreshold })

      // Track diarization provider for this recording (to save after completion)
      if (enableDiarization && diarizationProvider) {
        diarizationProviderMapRef.current.set(recordingId, diarizationProvider)
      } else {
        diarizationProviderMapRef.current.set(recordingId, null)
      }

      // Set initial status
      setStatusMap((prev) => {
        const newMap = new Map(prev)
        newMap.set(recordingId, {
          status: 'loading',
          progress: 0,
          currentChunk: 0,
          totalChunks: 0,
          message: 'Starting retranscription...',
        })
        return newMap
      })

      try {
        await invoke('retranscribe_recording', {
          recordingId,
          audioFilePath: audioPath,
          modelName: modelName || undefined,
          language: language || undefined,
          enableDiarization: enableDiarization || false,
          diarizationProvider: diarizationProvider || undefined,
          maxSpeakers: maxSpeakers || undefined,
          similarityThreshold: similarityThreshold || undefined,
        })
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err)
        console.error('Failed to start retranscription:', err)

        setStatusMap((prev) => {
          const newMap = new Map(prev)
          newMap.set(recordingId, {
            status: 'failed',
            progress: 0,
            currentChunk: 0,
            totalChunks: 0,
            message: 'Failed to start retranscription',
            error: errorMessage,
          })
          return newMap
        })

        throw err
      }
    },
    []
  )

  // Get status for a specific recording
  const getStatus = useCallback(
    (recordingId: string): RetranscriptionStatus => {
      return (
        statusMap.get(recordingId) || {
          status: 'idle',
          progress: 0,
          currentChunk: 0,
          totalChunks: 0,
          message: '',
        }
      )
    },
    [statusMap]
  )

  // Cancel retranscription for a recording
  const cancelRetranscription = useCallback(
    async (recordingId: string) => {
      console.log('Cancelling retranscription:', recordingId)

      try {
        await invoke('cancel_retranscription', { recordingId })

        // Update local status immediately
        setStatusMap((prev) => {
          const newMap = new Map(prev)
          newMap.set(recordingId, {
            status: 'failed',
            progress: 0,
            currentChunk: 0,
            totalChunks: 0,
            message: 'Cancelled by user',
            error: 'Cancelled by user',
          })
          return newMap
        })
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err)
        console.error('Failed to cancel retranscription:', err)
        throw new Error(errorMessage)
      }
    },
    []
  )

  // Clear status for a recording (after user has acknowledged completion)
  const clearStatus = useCallback((recordingId: string) => {
    setStatusMap((prev) => {
      const newMap = new Map(prev)
      newMap.delete(recordingId)
      return newMap
    })
  }, [])

  // Register a callback for when retranscription completes
  const onComplete = useCallback(
    (callback: (recordingId: string, result: RetranscriptionResult) => void) => {
      onCompleteCallbackRef.current = callback
    },
    []
  )

  return {
    startRetranscription,
    cancelRetranscription,
    getStatus,
    clearStatus,
    isRetranscribing,
    statusMap,
    onComplete,
  }
}
