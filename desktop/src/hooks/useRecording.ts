'use client'

import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface Transcript {
  id: string
  text: string
  timestamp: string
  // Additional fields for database storage
  audioStartTime?: number
  audioEndTime?: number
  confidence?: number
  sequenceId?: number
}

interface TranscriptUpdate {
  text: string
  timestamp: string
  audio_start_time?: number
  audio_end_time?: number
  confidence?: number
}

// Information about a completed recording for the post-recording modal
export interface CompletedRecordingInfo {
  id: string
  title: string
  duration: number
  transcriptCount: number
  audioPath: string | null
  meetingFolderPath: string | null
}

export function useRecording() {
  const [isRecording, setIsRecording] = useState(false)
  const [transcripts, setTranscripts] = useState<Transcript[]>([])
  const [error, setError] = useState<string | null>(null)
  const [currentRecordingId, setCurrentRecordingId] = useState<string | null>(null)

  // Post-recording state
  const [showPostRecordingModal, setShowPostRecordingModal] = useState(false)
  const [completedRecording, setCompletedRecording] = useState<CompletedRecordingInfo | null>(null)
  const [completedTranscripts, setCompletedTranscripts] = useState<Transcript[]>([])

  // Track transcript sequence for database ordering
  const sequenceRef = useRef(0)
  const recordingStartTimeRef = useRef<Date | null>(null)
  const recordingTitleRef = useRef<string>('')

  // Ref to store the audio folder path from the backend
  const audioFolderPathRef = useRef<string | null>(null)

  // Listen for recording state changes
  useEffect(() => {
    const checkRecordingState = async () => {
      try {
        const recording = await invoke<boolean>('is_recording')
        setIsRecording(recording)
      } catch (err) {
        console.error('Failed to check recording state:', err)
      }
    }

    checkRecordingState()
    const interval = setInterval(checkRecordingState, 1000)
    return () => clearInterval(interval)
  }, [])

  // Listen for recording-stopped event to get the actual audio file path
  useEffect(() => {
    let unlisten: (() => void) | undefined

    const setupListener = async () => {
      try {
        unlisten = await listen<{ folder_path: string; meeting_name: string }>(
          'recording-stopped',
          (event) => {
            const { folder_path } = event.payload
            console.log('Recording stopped, folder path:', folder_path)
            audioFolderPathRef.current = folder_path

            // Update completedRecording with actual audio path if modal is showing
            if (folder_path) {
              const audioFilePath = `${folder_path}/audio.mp4`
              setCompletedRecording(prev =>
                prev ? { ...prev, audioPath: audioFilePath, meetingFolderPath: folder_path } : prev
              )
            }
          }
        )
      } catch (err) {
        console.error('Failed to setup recording-stopped listener:', err)
      }
    }

    setupListener()
    return () => {
      if (unlisten) unlisten()
    }
  }, [])

  // Listen for transcript updates
  useEffect(() => {
    let unlisten: (() => void) | undefined

    const setupListener = async () => {
      try {
        unlisten = await listen<TranscriptUpdate>('transcript-update', (event) => {
          const update = event.payload
          const sequenceId = sequenceRef.current++

          const transcript: Transcript = {
            id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
            text: update.text,
            timestamp: update.timestamp || new Date().toLocaleTimeString(),
            audioStartTime: update.audio_start_time,
            audioEndTime: update.audio_end_time,
            confidence: update.confidence,
            sequenceId,
          }

          setTranscripts((prev) => [...prev, transcript])
        })
      } catch (err) {
        console.error('Failed to setup transcript listener:', err)
      }
    }

    setupListener()
    return () => {
      if (unlisten) unlisten()
    }
  }, [])

  const startRecording = useCallback(
    async (
      micDevice?: string,
      systemDevice?: string,
      meetingName?: string
    ) => {
      try {
        setError(null)
        setTranscripts([]) // Clear previous transcripts
        sequenceRef.current = 0
        recordingStartTimeRef.current = new Date()

        // Create recording entry in database first
        const title = meetingName || `Recording ${new Date().toLocaleString()}`
        recordingTitleRef.current = title
        let recordingId: string | null = null

        try {
          // Build the full Recording object expected by the Rust command
          const recordingData = {
            id: `rec-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
            title,
            created_at: new Date().toISOString(),
            completed_at: null,
            duration_seconds: null,
            status: 'recording',
            audio_file_path: null,
            meeting_folder_path: null,
            microphone_device: micDevice || null,
            system_audio_device: systemDevice || null,
            sample_rate: 48000,
            transcription_model: null,
            language: null,
          }

          recordingId = await invoke<string>('db_create_recording', {
            recording: recordingData,
          })
          setCurrentRecordingId(recordingId)
          console.log('Created recording in database:', recordingId)
        } catch (dbErr) {
          console.warn('Failed to create recording in database:', dbErr)
          // Continue with recording even if database fails
        }

        // Start the actual recording
        await invoke('start_recording', {
          args: {
            mic_device_name: micDevice,
            system_device_name: systemDevice,
            meeting_name: meetingName,
          }
        })

        setIsRecording(true)
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err)
        setError(`Failed to start recording: ${errorMessage}`)
        console.error('Start recording error:', err)
      }
    },
    []
  )

  const stopRecording = useCallback(async () => {
    console.log('stopRecording called, currentRecordingId:', currentRecordingId)

    // Calculate duration first (we need this regardless of success/failure)
    const durationSeconds = recordingStartTimeRef.current
      ? Math.floor((Date.now() - recordingStartTimeRef.current.getTime()) / 1000)
      : 0

    // Get default save path (use absolute path from backend)
    let savePath: string
    try {
      const defaultFolder = await invoke<string>('get_default_recordings_folder_path')
      savePath = `${defaultFolder}/${Date.now()}.wav`
    } catch {
      // Fallback to home directory if command fails
      savePath = `~/Documents/meetlocal-recordings/${Date.now()}.wav`
    }

    // Set up post-recording modal data BEFORE calling stop_recording
    // This ensures the modal shows even if stop_recording has issues
    const recordingId = currentRecordingId || `temp-${Date.now()}`

    console.log('Setting up post-recording modal with:', {
      recordingId,
      title: recordingTitleRef.current,
      duration: durationSeconds,
      transcriptCount: transcripts.length,
    })

    setCompletedRecording({
      id: recordingId,
      title: recordingTitleRef.current || `Recording ${new Date().toLocaleString()}`,
      duration: durationSeconds,
      transcriptCount: transcripts.length,
      audioPath: null, // Will be updated when recording-stopped event fires
      meetingFolderPath: null, // Will be updated when recording-stopped event fires
    })
    setCompletedTranscripts([...transcripts])
    setIsRecording(false)
    recordingStartTimeRef.current = null

    // Show modal immediately
    console.log('Setting showPostRecordingModal to true')
    setShowPostRecordingModal(true)

    // Now try to stop the actual recording
    try {
      setError(null)
      await invoke('stop_recording', {
        args: { save_path: savePath },
      })
      console.log('stop_recording command completed successfully')
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to stop recording: ${errorMessage}`)
      console.error('Stop recording error:', err)
      // Modal is already shown, user can still save/discard
    }
  }, [currentRecordingId, transcripts])

  // Save recording to database (called from modal)
  const saveRecording = useCallback(async (
    newTitle: string,
    retranscribe: boolean,
    model?: string
  ) => {
    if (!completedRecording) return

    // Use a mutable reference to track the actual recording ID
    let recordingId = completedRecording.id

    try {
      // If recording wasn't created in DB at start (temp ID), create it now
      if (recordingId.startsWith('temp-')) {
        console.log('Recording has temp ID, creating in database first...')
        // Build the full Recording object expected by the Rust command
        const recordingData = {
          id: `rec-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
          title: newTitle,
          created_at: new Date().toISOString(),
          completed_at: null,
          duration_seconds: null,
          status: 'recording',
          audio_file_path: completedRecording.audioPath || null,
          meeting_folder_path: completedRecording.meetingFolderPath || null,
          microphone_device: null,
          system_audio_device: null,
          sample_rate: 48000,
          transcription_model: null,
          language: null,
        }

        const newRecordingId = await invoke<string>('db_create_recording', {
          recording: recordingData,
        })
        recordingId = newRecordingId
        console.log('Created recording in database with ID:', recordingId)
      }

      // Update recording with title and paths
      const updates: Record<string, string | undefined> = {}

      if (newTitle !== completedRecording.title) {
        updates.title = newTitle
      }

      // Always save the audio path and folder path if available
      if (completedRecording.audioPath) {
        updates.audio_file_path = completedRecording.audioPath
      }
      if (completedRecording.meetingFolderPath) {
        updates.meeting_folder_path = completedRecording.meetingFolderPath
      }

      // Only call update if there are changes
      if (Object.keys(updates).length > 0) {
        await invoke('db_update_recording', {
          id: recordingId,
          updates,
        })
        console.log('Updated recording with:', updates)
      }

      // Complete the recording in database
      await invoke('db_complete_recording', {
        id: recordingId,
        duration: completedRecording.duration,
      })
      console.log('Completed recording in database:', recordingId)

      // Save transcript segments to database
      if (completedTranscripts.length > 0) {
        const timestamp = Date.now()
        // Filter out empty transcripts and build valid segments
        const segments = completedTranscripts
          .filter(t => t.text && t.text.trim().length > 0)
          .map((t, index) => ({
            id: `${recordingId}-seg-${index}-${timestamp}`,
            recording_id: recordingId,
            text: t.text.trim(),
            audio_start_time: t.audioStartTime ?? index * 5,
            audio_end_time: t.audioEndTime ?? (index + 1) * 5,
            duration: (t.audioEndTime ?? (index + 1) * 5) - (t.audioStartTime ?? index * 5),
            display_time: t.timestamp || `[${Math.floor((t.audioStartTime ?? index * 5) / 60)}:${String(Math.floor((t.audioStartTime ?? index * 5) % 60)).padStart(2, '0')}]`,
            confidence: t.confidence ?? 1.0,
            sequence_id: t.sequenceId ?? index,
          }))

        if (segments.length > 0) {
          console.log('Saving segments:', JSON.stringify(segments[0], null, 2))
          await invoke('db_save_transcript_segments_batch', {
            segments,
          })
          console.log(`Saved ${segments.length} transcript segments to database`)
        }
      }

      // If user wants retranscription, trigger it
      if (retranscribe && completedRecording.audioPath) {
        console.log(`Retranscription requested with model: ${model || 'current'}`)
        try {
          await invoke('retranscribe_recording', {
            recordingId: recordingId,
            audioFilePath: completedRecording.audioPath,
            modelName: model !== 'current' ? model : undefined,
            language: null,
          })
          console.log('Retranscription started')
        } catch (retransErr) {
          console.error('Failed to start retranscription:', retransErr)
          // Don't fail the save if retranscription fails to start
        }
      }

      // Close modal and clean up
      closePostRecordingModal()
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to save recording: ${errorMessage}`)
      console.error('Save recording error:', err)
    }
  }, [completedRecording, completedTranscripts])

  // Discard recording (delete from database)
  const discardRecording = useCallback(async () => {
    if (!completedRecording) return

    try {
      await invoke('db_delete_recording', {
        id: completedRecording.id,
      })
      console.log('Discarded recording:', completedRecording.id)
      closePostRecordingModal()
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to discard recording: ${errorMessage}`)
      console.error('Discard recording error:', err)
    }
  }, [completedRecording])

  // Close modal and clean up state
  const closePostRecordingModal = useCallback(() => {
    setShowPostRecordingModal(false)
    setCompletedRecording(null)
    setCompletedTranscripts([])
    setCurrentRecordingId(null)
    setTranscripts([])
  }, [])

  return {
    isRecording,
    transcripts,
    error,
    currentRecordingId,
    startRecording,
    stopRecording,
    // Post-recording modal state and actions
    showPostRecordingModal,
    completedRecording,
    completedTranscripts,
    saveRecording,
    discardRecording,
    closePostRecordingModal,
  }
}
