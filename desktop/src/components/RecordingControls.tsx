'use client'

interface RecordingControlsProps {
  isRecording: boolean
  onStart: () => Promise<void>
  onStop: () => Promise<void>
}

export function RecordingControls({ isRecording, onStart, onStop }: RecordingControlsProps) {
  return (
    <div className="flex items-center justify-center gap-4">
      {/* Recording Status Indicator */}
      <div className="flex items-center gap-2">
        <div
          className={`w-3 h-3 rounded-full ${
            isRecording ? 'bg-red-500 animate-pulse' : 'bg-gray-300'
          }`}
        />
        <span className="text-sm text-gray-600">
          {isRecording ? 'Recording...' : 'Ready'}
        </span>
      </div>

      {/* Control Buttons */}
      {!isRecording ? (
        <button
          onClick={onStart}
          className="px-6 py-3 bg-red-500 text-white font-semibold rounded-full hover:bg-red-600 transition-colors shadow-lg hover:shadow-xl flex items-center gap-2"
        >
          <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
            <circle cx="10" cy="10" r="8" />
          </svg>
          Start Recording
        </button>
      ) : (
        <button
          onClick={onStop}
          className="px-6 py-3 bg-gray-700 text-white font-semibold rounded-full hover:bg-gray-800 transition-colors shadow-lg hover:shadow-xl flex items-center gap-2"
        >
          <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
            <rect x="4" y="4" width="12" height="12" rx="2" />
          </svg>
          Stop Recording
        </button>
      )}
    </div>
  )
}
