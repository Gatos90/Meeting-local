'use client'

import { Card } from '@/components/ui/card'

interface Transcript {
  id: string
  text: string
  timestamp: string
}

interface RecordingViewProps {
  transcripts: Transcript[]
}

export function RecordingView({ transcripts }: RecordingViewProps) {
  return (
    <div className="h-full flex flex-col items-center justify-center space-y-8 animate-in fade-in zoom-in-95 duration-500">
      {/* Pulsing Visual */}
      <div className="relative">
        {/* Outer ring */}
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-64 h-64 bg-primary/5 rounded-full animate-ping opacity-75" />
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-48 h-48 bg-primary/10 rounded-full animate-pulse" />

        {/* Central Visual - Audio Bars */}
        <div className="relative z-10 w-32 h-32 bg-white rounded-full shadow-2xl flex items-center justify-center border-4 border-primary/10">
          <div className="space-x-1 flex items-center h-8">
            {[...Array(5)].map((_, i) => (
              <div
                key={i}
                className="w-1.5 bg-primary rounded-full animate-music-bar"
                style={{
                  height: '20%',
                  animationDelay: `${i * 0.1}s`,
                }}
              />
            ))}
          </div>
        </div>
      </div>

      {/* Status Text */}
      <div className="text-center space-y-2">
        <h3 className="text-2xl font-semibold text-foreground">Listening...</h3>
        <p className="text-muted-foreground">Transcribing audio in real-time</p>
      </div>

      {/* Live Transcript Box */}
      <Card className="max-w-2xl w-full p-6 border-border/50">
        <div className="max-h-64 overflow-y-auto space-y-3">
          {transcripts.length === 0 ? (
            <p className="text-muted-foreground text-center italic">
              Waiting for speech...
            </p>
          ) : (
            transcripts.map((transcript) => (
              <div key={transcript.id} className="text-lg text-foreground leading-relaxed">
                <span className="text-xs text-muted-foreground mr-2">
                  {transcript.timestamp}
                </span>
                <span>{transcript.text}</span>
              </div>
            ))
          )}
        </div>
      </Card>
    </div>
  )
}
