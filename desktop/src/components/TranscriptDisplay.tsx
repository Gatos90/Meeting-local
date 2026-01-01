'use client'

interface Transcript {
  id: string
  text: string
  timestamp: string
}

interface TranscriptDisplayProps {
  transcripts: Transcript[]
}

export function TranscriptDisplay({ transcripts }: TranscriptDisplayProps) {
  if (transcripts.length === 0) {
    return (
      <div className="text-center text-gray-500 py-8">
        <svg
          className="w-12 h-12 mx-auto mb-4 text-gray-300"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z"
          />
        </svg>
        <p>No transcripts yet. Start recording to see live transcription.</p>
      </div>
    )
  }

  return (
    <div className="space-y-4 max-h-96 overflow-y-auto">
      {transcripts.map((transcript) => (
        <div
          key={transcript.id}
          className="p-4 bg-gray-50 rounded-lg border border-gray-200"
        >
          <div className="flex justify-between items-start mb-2">
            <span className="text-xs text-gray-500">{transcript.timestamp}</span>
          </div>
          <p className="text-gray-800">{transcript.text}</p>
        </div>
      ))}
    </div>
  )
}
