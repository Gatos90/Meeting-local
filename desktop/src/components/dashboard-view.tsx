'use client'

import { useRouter } from 'next/navigation'
import { Sparkles, FileText, Loader2 } from 'lucide-react'
import { RecordingCard, formatDate, formatDuration } from './recording-card'
import { useRecordings } from '@/hooks/useRecordings'

interface DashboardViewProps {
  currentModel?: string
  currentLanguage?: string
}

export function DashboardView({
  currentModel = 'base',
  currentLanguage = 'English',
}: DashboardViewProps) {
  const router = useRouter()
  // Load recent recordings from database (limit to 3)
  const { recordings, loading, error } = useRecordings(3)

  return (
    <div className="mx-auto max-w-5xl space-y-8">
      {/* Hero Section */}
      <div className="flex flex-col items-center justify-center py-12 text-center space-y-4">
        <div className="p-4 rounded-full bg-muted mb-4">
          <Sparkles className="h-8 w-8 text-muted-foreground" />
        </div>
        <h2 className="text-3xl font-bold tracking-tight text-foreground">
          Ready to record
        </h2>
        <p className="text-muted-foreground max-w-lg">
          Your settings are optimized for{' '}
          <span className="font-medium text-foreground">{currentLanguage}</span>{' '}
          using{' '}
          <span className="font-medium text-foreground">Whisper {currentModel}</span>.
        </p>
      </div>

      {/* Recent Recordings */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-medium text-foreground">Recent Recordings</h3>
          <a
            href="/transcripts"
            className="text-sm text-muted-foreground hover:text-foreground transition-colors"
          >
            View all
          </a>
        </div>

        {/* Loading State */}
        {loading && (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        )}

        {/* Error State */}
        {error && (
          <div className="text-center py-8 text-muted-foreground">
            <p>Could not load recordings</p>
          </div>
        )}

        {/* Empty State */}
        {!loading && !error && recordings.length === 0 && (
          <div className="flex flex-col items-center justify-center py-8 text-center">
            <div className="p-3 rounded-full bg-muted mb-3">
              <FileText className="h-6 w-6 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground">
              No recordings yet. Start your first recording!
            </p>
          </div>
        )}

        {/* Recordings Grid */}
        {!loading && !error && recordings.length > 0 && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {recordings.map((item) => (
              <RecordingCard
                key={item.recording.id}
                id={item.recording.id}
                title={item.recording.title}
                date={formatDate(item.recording.created_at)}
                duration={formatDuration(item.recording.duration_seconds)}
                categories={item.categories}
                transcriptCount={item.transcript_count}
                onClick={() => {
                  router.push(`/transcripts/view?id=${item.recording.id}`)
                }}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
