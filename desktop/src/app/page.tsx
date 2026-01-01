'use client'

import { useEffect, useState, useRef } from 'react'
import { Search, User } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { FloatingControl } from '@/components/floating-control'
import { RecordingView } from '@/components/recording-view'
import { DashboardView } from '@/components/dashboard-view'
import { PostRecordingModal } from '@/components/post-recording-modal'
import { useRecording } from '@/hooks/useRecording'
import { useAudioDevices } from '@/hooks/useAudioDevices'
import { useSettings } from '@/hooks/useSettings'

export default function Home() {
  const { devices, loading: devicesLoading, refresh: refreshDevices } = useAudioDevices()
  const {
    isRecording,
    transcripts,
    startRecording,
    stopRecording,
    error,
    // Post-recording modal
    showPostRecordingModal,
    completedRecording,
    completedTranscripts,
    saveRecording,
    discardRecording,
    closePostRecordingModal,
  } = useRecording()
  const { settings, loading: settingsLoading } = useSettings()

  const [selectedMic, setSelectedMic] = useState<string>('')
  const [selectedSystem, setSelectedSystem] = useState<string>('')
  const [recordingTime, setRecordingTime] = useState(0)

  // Track if we've initialized devices from saved settings
  const initializedRef = useRef(false)

  // Initialize devices from saved settings or fall back to first available
  useEffect(() => {
    // Wait for both devices and settings to load
    if (devicesLoading || settingsLoading) return
    if (devices.microphones.length === 0 && devices.speakers.length === 0) return

    // Only initialize once
    if (initializedRef.current) return
    initializedRef.current = true

    // Try to use saved microphone, or fall back to first available
    if (devices.microphones.length > 0) {
      const savedMic = settings.lastMicrophone
      const savedMicExists = savedMic && devices.microphones.some(m => m.name === savedMic)
      if (savedMicExists) {
        setSelectedMic(savedMic)
        console.log('Restored saved microphone:', savedMic)
      } else {
        setSelectedMic(devices.microphones[0].name)
        console.log('Using default microphone:', devices.microphones[0].name)
      }
    }

    // Try to use saved system audio, or fall back to first available
    if (devices.speakers.length > 0) {
      const savedSystem = settings.lastSystemAudio
      const savedSystemExists = savedSystem && devices.speakers.some(s => s.name === savedSystem)
      if (savedSystemExists) {
        setSelectedSystem(savedSystem)
        console.log('Restored saved system audio:', savedSystem)
      } else {
        setSelectedSystem(devices.speakers[0].name)
        console.log('Using default system audio:', devices.speakers[0].name)
      }
    }
  }, [devices, settings, devicesLoading, settingsLoading])

  // Recording timer
  useEffect(() => {
    let interval: NodeJS.Timeout
    if (isRecording) {
      interval = setInterval(() => {
        setRecordingTime(prev => prev + 1)
      }, 1000)
    } else {
      setRecordingTime(0)
    }
    return () => clearInterval(interval)
  }, [isRecording])

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  const handleStartRecording = async () => {
    await startRecording(
      selectedMic || undefined,
      selectedSystem || undefined,
      undefined
    )
  }

  const handleStopRecording = async () => {
    await stopRecording()
  }

  return (
    <>
      {/* Header */}
      <header className="flex h-16 items-center justify-between border-b border-border bg-background/50 backdrop-blur-sm px-8">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-semibold text-foreground">
            Dashboard
          </h1>
          {isRecording && (
            <Badge variant="destructive" className="flex items-center gap-2">
              <span className="relative flex h-2 w-2">
                <span className="absolute inline-flex h-full w-full rounded-full bg-white opacity-75 animate-ping" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-white" />
              </span>
              {formatTime(recordingTime)}
            </Badge>
          )}
        </div>

        <div className="flex items-center gap-4">
          <div className="relative hidden sm:block w-64">
            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search transcripts..."
              className="pl-9 bg-muted/50 border-transparent focus:bg-background transition-all"
            />
          </div>
          <Button variant="ghost" size="icon" className="rounded-full">
            <User className="h-5 w-5" />
          </Button>
        </div>
      </header>

      {/* Error Display */}
      {error && (
        <div className="mx-8 mt-4 p-4 bg-destructive/10 border border-destructive/20 text-destructive rounded-lg">
          {error}
        </div>
      )}

      {/* Main Content */}
      <div className="flex-1 overflow-y-auto p-8 pb-32">
        {isRecording ? (
          <RecordingView transcripts={transcripts} />
        ) : (
          <DashboardView />
        )}
      </div>

      {/* Floating Control */}
      <FloatingControl
        isRecording={isRecording}
        onStartRecording={handleStartRecording}
        onStopRecording={handleStopRecording}
        micDevice={selectedMic}
        systemDevice={selectedSystem}
        onMicChange={setSelectedMic}
        onSystemChange={setSelectedSystem}
        microphones={devices.microphones}
        speakers={devices.speakers}
      />

      {/* Post-Recording Modal */}
      {completedRecording && (
        <PostRecordingModal
          isOpen={showPostRecordingModal}
          onClose={closePostRecordingModal}
          onSave={saveRecording}
          onDiscard={discardRecording}
          recording={completedRecording}
          transcripts={completedTranscripts}
        />
      )}
    </>
  )
}
