'use client'

import { useState } from 'react'
import { Mic, StopCircle, Settings, List } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { SettingsPopover } from './settings-popover'

interface FloatingControlProps {
  isRecording: boolean
  onStartRecording: () => void
  onStopRecording: () => void
  micDevice?: string
  systemDevice?: string
  onMicChange: (device: string) => void
  onSystemChange: (device: string) => void
  microphones: Array<{ name: string }>
  speakers: Array<{ name: string }>
}

export function FloatingControl({
  isRecording,
  onStartRecording,
  onStopRecording,
  micDevice,
  systemDevice,
  onMicChange,
  onSystemChange,
  microphones,
  speakers,
}: FloatingControlProps) {
  const [showSettings, setShowSettings] = useState(false)

  const handleToggleRecording = () => {
    if (isRecording) {
      onStopRecording()
    } else {
      onStartRecording()
    }
  }

  return (
    <div className="fixed bottom-8 left-1/2 -translate-x-1/2 z-50">
      <div className="flex items-center gap-2 p-2 bg-white/90 backdrop-blur-xl border border-border/60 shadow-2xl shadow-slate-200/50 rounded-full ring-1 ring-slate-900/5 transition-all duration-300 hover:scale-[1.02]">
        {/* Settings Button */}
        <SettingsPopover
          open={showSettings}
          onOpenChange={setShowSettings}
          micDevice={micDevice}
          systemDevice={systemDevice}
          onMicChange={onMicChange}
          onSystemChange={onSystemChange}
          microphones={microphones}
          speakers={speakers}
        >
          <Button
            variant="ghost"
            size="icon"
            className="rounded-full h-12 w-12 text-muted-foreground hover:text-foreground"
          >
            <Settings className={cn(
              "h-5 w-5 transition-transform duration-500",
              showSettings && "rotate-90"
            )} />
          </Button>
        </SettingsPopover>

        {/* Main Record Button */}
        <button
          onClick={handleToggleRecording}
          className={cn(
            "flex items-center justify-center h-14 w-14 rounded-full transition-all duration-300 shadow-lg hover:shadow-xl active:scale-95",
            isRecording
              ? "bg-white border-2 border-primary text-primary"
              : "bg-primary text-primary-foreground hover:bg-primary/90"
          )}
        >
          {isRecording ? (
            <StopCircle className="h-7 w-7" fill="currentColor" />
          ) : (
            <Mic className="h-6 w-6" />
          )}
        </button>

        {/* List Button */}
        <Button
          variant="ghost"
          size="icon"
          className="rounded-full h-12 w-12 text-muted-foreground hover:text-foreground"
        >
          <List className="h-5 w-5" />
        </Button>
      </div>
    </div>
  )
}
