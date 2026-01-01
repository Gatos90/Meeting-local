'use client'

import { useState, useEffect, useCallback, Suspense, useRef } from 'react'
import { useSearchParams, useRouter } from 'next/navigation'
import { invoke } from '@tauri-apps/api/core'
import { ArrowLeft, Clock, Calendar, FileAudio, FolderOpen, Loader2, RefreshCw, Mic, AlertCircle, Check, XCircle, Users, Download, Edit2, X, ChevronDown, ChevronUp, MessageSquare, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import { Progress } from '@/components/ui/progress'
import { Slider } from '@/components/ui/slider'
import { Checkbox } from '@/components/ui/checkbox'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import type { RecordingWithMetadata, TranscriptSegment } from '@/types/database'
import { ChatPanel } from '@/components/chat'
import { getSpeakerColor } from '@/types/database'
import { useRetranscription } from '@/hooks/useRetranscription'
import { useDiarization } from '@/hooks/useDiarization'
import { SpeakerLabel, SpeakerBadge } from '@/components/speaker-label'
import { CategoryTagSelector } from '@/components/CategoryTagSelector'

// Model info from the Rust backend
// Note: Rust enums serialize unit variants as strings, variants with data as objects
type ModelStatus =
  | 'Available'
  | 'Missing'
  | { Downloading: { progress: number } }
  | { Error: string }
  | { Corrupted: { file_size: number, expected_min_size: number } }

interface ModelInfo {
  name: string
  path: string
  size_mb: number
  accuracy: string
  speed: string
  status: ModelStatus
  description: string
}

// Format duration from seconds to "Xh Ym Zs" or "Xm Zs"
function formatDuration(seconds: number | null): string {
  if (seconds === null || seconds === 0) return '0s'
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = Math.floor(seconds % 60)

  if (hours > 0) {
    return `${hours}h ${minutes}m ${secs}s`
  } else if (minutes > 0) {
    return `${minutes}m ${secs}s`
  }
  return `${secs}s`
}

// Format date to readable string
function formatDate(dateString: string): string {
  try {
    const date = new Date(dateString)
    return date.toLocaleDateString('en-US', {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    })
  } catch {
    return dateString
  }
}

// Check if model is available
function isModelAvailable(model: ModelInfo): boolean {
  return model.status === 'Available'
}

// Language options
const languages = [
  { value: 'en', label: 'English' },
  { value: 'es', label: 'Spanish' },
  { value: 'fr', label: 'French' },
  { value: 'de', label: 'German' },
  { value: 'it', label: 'Italian' },
  { value: 'pt', label: 'Portuguese' },
  { value: 'nl', label: 'Dutch' },
  { value: 'pl', label: 'Polish' },
  { value: 'ru', label: 'Russian' },
  { value: 'zh', label: 'Chinese' },
  { value: 'ja', label: 'Japanese' },
  { value: 'ko', label: 'Korean' },
  { value: 'ar', label: 'Arabic' },
  { value: 'hi', label: 'Hindi' },
  { value: 'auto', label: 'Auto-detect' },
]

function RecordingDetailContent() {
  const searchParams = useSearchParams()
  const router = useRouter()
  const recordingId = searchParams.get('id')

  const [recording, setRecording] = useState<RecordingWithMetadata | null>(null)
  const [transcripts, setTranscripts] = useState<TranscriptSegment[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Editing state for transcript text
  const [editingSegmentId, setEditingSegmentId] = useState<string | null>(null)
  const [editingText, setEditingText] = useState('')
  const editTextareaRef = useRef<HTMLTextAreaElement>(null)

  // Editing state for recording title
  const [isEditingTitle, setIsEditingTitle] = useState(false)
  const [editingTitle, setEditingTitle] = useState('')
  const editTitleRef = useRef<HTMLInputElement>(null)

  // Collapsible state for re-transcribe section
  const [showRetranscribe, setShowRetranscribe] = useState(false)

  // Delete confirmation state
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)
  const [isDeleting, setIsDeleting] = useState(false)

  // Model state
  const [availableModels, setAvailableModels] = useState<ModelInfo[]>([])
  const [currentModel, setCurrentModel] = useState<string | null>(null)
  const [selectedModel, setSelectedModel] = useState<string>('')
  const [isLoadingModel, setIsLoadingModel] = useState(false)

  // Retranscription hook
  const { getStatus, startRetranscription, cancelRetranscription, onComplete } = useRetranscription()
  const retranscriptionStatus = recordingId ? getStatus(recordingId) : { status: 'idle', progress: 0, currentChunk: 0, totalChunks: 0, message: '' }
  const isRetranscribing = retranscriptionStatus.status === 'loading' || retranscriptionStatus.status === 'processing' || retranscriptionStatus.status === 'diarizing'
  const [isCancelling, setIsCancelling] = useState(false)

  // Diarization hook
  const {
    modelsReady: diarizationModelsReady,
    isDownloading: isDiarizationDownloading,
    downloadProgress: diarizationDownloadProgress,
    downloadModels: downloadDiarizationModels,
    renameSpeaker,
  } = useDiarization()
  const [enableDiarization, setEnableDiarization] = useState(false)
  const [diarizationProvider, setDiarizationProvider] = useState<'pyannote' | 'sortformer'>('pyannote')
  const [sortformerModelReady, setSortformerModelReady] = useState(false)
  const [sortformerDownloading, setSortformerDownloading] = useState(false)

  // PyAnnote diarization settings
  const [maxSpeakers, setMaxSpeakers] = useState<string>('auto')
  const [similarityThreshold, setSimilarityThreshold] = useState(0.4)

  // Language selection
  const [selectedLanguage, setSelectedLanguage] = useState('en')

  // Fetch available models
  const fetchModels = useCallback(async () => {
    try {
      const models = await invoke<ModelInfo[]>('whisper_get_available_models')
      setAvailableModels(models)

      // Get current model
      const current = await invoke<string | null>('whisper_get_current_model')
      setCurrentModel(current)

      // Set selected model to current or first available
      if (current) {
        setSelectedModel(current)
      } else {
        const firstAvailable = models.find(isModelAvailable)
        if (firstAvailable) {
          setSelectedModel(firstAvailable.name)
        }
      }
    } catch (err) {
      console.error('Failed to fetch models:', err)
    }
  }, [])

  // Fetch recording and transcripts
  const fetchData = useCallback(async (showLoading = true) => {
    if (!recordingId) {
      setError('No recording ID provided')
      setLoading(false)
      return
    }

    try {
      if (showLoading) {
        setLoading(true)
      }
      setError(null)

      // Fetch recording details
      const recordingData = await invoke<RecordingWithMetadata | null>('db_get_recording', {
        id: recordingId
      })

      if (!recordingData) {
        setError('Recording not found')
        setLoading(false)
        return
      }

      setRecording(recordingData)

      // Fetch transcript segments
      const transcriptData = await invoke<TranscriptSegment[]>('db_get_transcript_segments', {
        recordingId: recordingId,
      })

      setTranscripts(transcriptData)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to load recording: ${errorMessage}`)
      console.error('Fetch recording error:', err)
    } finally {
      if (showLoading) {
        setLoading(false)
      }
    }
  }, [recordingId])

  useEffect(() => {
    fetchData()
    fetchModels()
    // Check Sortformer model availability
    const checkSortformer = async () => {
      try {
        const available = await invoke<boolean>('is_sortformer_model_available')
        setSortformerModelReady(available)
      } catch (e) {
        console.error('Failed to check Sortformer model:', e)
      }
    }
    checkSortformer()
  }, [fetchData, fetchModels])

  // Register callback to refetch data when retranscription completes
  // This is more reliable than watching status because it runs after DB operations complete
  useEffect(() => {
    onComplete((completedRecordingId, result) => {
      if (completedRecordingId === recordingId && result.success) {
        // Small delay to ensure DB transaction is fully committed
        // Use fetchData(false) to avoid showing loading state and page jumping
        setTimeout(() => {
          fetchData(false)
        }, 100)
      }
    })
  }, [onComplete, recordingId, fetchData])

  const handleLoadModel = async (modelName: string) => {
    try {
      setIsLoadingModel(true)
      setError(null)
      await invoke('whisper_load_model', { modelName })
      setCurrentModel(modelName)
      setSelectedModel(modelName)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to load model: ${errorMessage}`)
    } finally {
      setIsLoadingModel(false)
    }
  }

  const handleDownloadSortformer = async () => {
    setSortformerDownloading(true)
    try {
      await invoke('download_sortformer_model')
      setSortformerModelReady(true)
    } catch (e) {
      console.error('Failed to download Sortformer model:', e)
    } finally {
      setSortformerDownloading(false)
    }
  }

  const handleRetranscribe = async () => {
    if (!recording?.recording.audio_file_path || !recordingId) {
      setError('No audio file available for retranscription')
      return
    }

    if (!selectedModel) {
      setError('No model selected. Please select a model or download one in Settings.')
      return
    }

    // Check if diarization is enabled but models not ready for selected provider
    if (enableDiarization) {
      if (diarizationProvider === 'pyannote' && !diarizationModelsReady) {
        setError('PyAnnote diarization models not downloaded. Please download them first.')
        return
      }
      if (diarizationProvider === 'sortformer' && !sortformerModelReady) {
        setError('Sortformer model not downloaded. Please download it first.')
        return
      }
    }

    try {
      setError(null)

      // The backend will handle loading the model if needed
      // Parse maxSpeakers - 'auto' means undefined (use default)
      const maxSpeakersNum = maxSpeakers === 'auto' ? undefined : parseInt(maxSpeakers, 10)

      await startRetranscription(
        recordingId,
        recording.recording.audio_file_path,
        selectedModel,
        selectedLanguage === 'auto' ? undefined : selectedLanguage,
        enableDiarization,
        enableDiarization ? diarizationProvider : undefined,
        enableDiarization && diarizationProvider === 'pyannote' ? maxSpeakersNum : undefined,
        enableDiarization && diarizationProvider === 'pyannote' ? similarityThreshold : undefined
      )

      // Update current model after successful start (backend will load it)
      setCurrentModel(selectedModel)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to start retranscription: ${errorMessage}`)
    }
  }

  const handleOpenFolder = async () => {
    if (!recording?.recording.meeting_folder_path) return

    try {
      await invoke('open_folder', { path: recording.recording.meeting_folder_path })
    } catch (err) {
      console.error('Failed to open folder:', err)
    }
  }

  const handleDeleteRecording = async () => {
    if (!recordingId) return

    try {
      setIsDeleting(true)
      await invoke('db_delete_recording', { id: recordingId })
      console.log('Deleted recording:', recordingId)
      // Navigate back to transcripts list
      router.push('/transcripts')
    } catch (err) {
      console.error('Failed to delete recording:', err)
      setError(`Failed to delete recording: ${err}`)
      setIsDeleting(false)
      setShowDeleteConfirm(false)
    }
  }

  const handleCancelRetranscription = async () => {
    if (!recordingId) return

    try {
      setIsCancelling(true)
      await cancelRetranscription(recordingId)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to cancel: ${errorMessage}`)
    } finally {
      setIsCancelling(false)
    }
  }

  // Start editing a transcript segment
  const handleStartEditSegment = (segment: TranscriptSegment) => {
    setEditingSegmentId(segment.id)
    setEditingText(segment.text)
    // Focus the textarea after render
    setTimeout(() => editTextareaRef.current?.focus(), 0)
  }

  // Start editing the recording title
  const handleStartEditTitle = () => {
    if (!recording) return
    setIsEditingTitle(true)
    setEditingTitle(recording.recording.title)
    setTimeout(() => editTitleRef.current?.focus(), 0)
  }

  // Save edited title
  const handleSaveTitle = async () => {
    if (!recordingId || !recording) return

    const trimmedTitle = editingTitle.trim()
    if (!trimmedTitle || trimmedTitle === recording.recording.title) {
      setIsEditingTitle(false)
      setEditingTitle('')
      return
    }

    try {
      await invoke('db_update_recording', {
        id: recordingId,
        updates: { title: trimmedTitle },
      })

      // Update local state
      setRecording(prev => prev ? {
        ...prev,
        recording: { ...prev.recording, title: trimmedTitle }
      } : null)

      setIsEditingTitle(false)
      setEditingTitle('')
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to save title: ${errorMessage}`)
    }
  }

  // Cancel editing title
  const handleCancelEditTitle = () => {
    setIsEditingTitle(false)
    setEditingTitle('')
  }

  // Save edited transcript text
  const handleSaveSegmentText = async () => {
    if (!editingSegmentId) return

    const originalSegment = transcripts.find(s => s.id === editingSegmentId)
    if (!originalSegment || originalSegment.text === editingText) {
      // No changes, just cancel
      setEditingSegmentId(null)
      setEditingText('')
      return
    }

    try {
      await invoke('db_update_transcript_text', {
        segmentId: editingSegmentId,
        newText: editingText,
      })

      // Update local state
      setTranscripts(prev =>
        prev.map(s =>
          s.id === editingSegmentId ? { ...s, text: editingText } : s
        )
      )

      setEditingSegmentId(null)
      setEditingText('')
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to save: ${errorMessage}`)
    }
  }

  // Cancel editing
  const handleCancelEditSegment = () => {
    setEditingSegmentId(null)
    setEditingText('')
  }

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center h-full">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mb-4" />
        <p className="text-muted-foreground">Loading recording...</p>
      </div>
    )
  }

  if (error && !recording) {
    return (
      <div className="flex flex-col items-center justify-center h-full">
        <div className="text-center">
          <h2 className="text-xl font-semibold text-foreground mb-2">
            {error || 'Recording not found'}
          </h2>
          <Button variant="outline" onClick={() => router.push('/transcripts')}>
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back to Transcripts
          </Button>
        </div>
      </div>
    )
  }

  if (!recording) return null

  const rec = recording.recording
  const availableModelsList = availableModels.filter(isModelAvailable)

  return (
    <>
      {/* Header */}
      <header className="flex h-16 items-center justify-between border-b border-border bg-background/50 backdrop-blur-sm px-8">
        <div className="flex items-center gap-4">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => router.push('/transcripts')}
          >
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back
          </Button>
          <Separator orientation="vertical" className="h-6" />
          {isEditingTitle ? (
            <div className="flex items-center gap-1">
              <input
                ref={editTitleRef}
                type="text"
                value={editingTitle}
                onChange={(e) => setEditingTitle(e.target.value)}
                className="text-xl font-semibold text-foreground bg-transparent border-b border-primary focus:outline-none py-0.5 max-w-md"
                onKeyDown={(e) => {
                  if (e.key === 'Escape') {
                    handleCancelEditTitle()
                  } else if (e.key === 'Enter') {
                    handleSaveTitle()
                  }
                }}
                onBlur={() => {
                  setTimeout(() => {
                    if (isEditingTitle) {
                      handleSaveTitle()
                    }
                  }, 150)
                }}
              />
              <button
                onMouseDown={(e) => {
                  e.preventDefault()
                  handleSaveTitle()
                }}
                className="p-1 hover:bg-green-100 dark:hover:bg-green-900 rounded"
                title="Save (Enter)"
              >
                <Check className="h-4 w-4 text-green-600" />
              </button>
              <button
                onMouseDown={(e) => {
                  e.preventDefault()
                  handleCancelEditTitle()
                }}
                className="p-1 hover:bg-red-100 dark:hover:bg-red-900 rounded"
                title="Cancel (Esc)"
              >
                <X className="h-4 w-4 text-red-600" />
              </button>
            </div>
          ) : (
            <h1
              className="text-xl font-semibold text-foreground truncate max-w-md group/title cursor-pointer flex items-center gap-1"
              onClick={handleStartEditTitle}
            >
              <span className="hover:bg-muted/50 rounded px-1 -mx-1 transition-colors">
                {rec.title}
              </span>
              <button
                className="opacity-0 group-hover/title:opacity-100 transition-opacity p-1 hover:bg-muted rounded"
                onClick={(e) => {
                  e.stopPropagation()
                  handleStartEditTitle()
                }}
                title="Edit title"
              >
                <Edit2 className="h-3.5 w-3.5 text-muted-foreground" />
              </button>
            </h1>
          )}
          <Badge variant={rec.status === 'completed' ? 'default' : 'secondary'}>
            {rec.status}
          </Badge>
        </div>

        <div className="flex items-center gap-2">
          {rec.meeting_folder_path && (
            <Button variant="outline" size="sm" onClick={handleOpenFolder}>
              <FolderOpen className="h-4 w-4 mr-2" />
              Open Folder
            </Button>
          )}
          <Button
            variant="outline"
            size="sm"
            onClick={() => setShowDeleteConfirm(true)}
            className="text-destructive hover:text-destructive hover:bg-destructive/10"
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Delete
          </Button>
        </div>
      </header>

      {/* Delete Confirmation Dialog */}
      {showDeleteConfirm && (
        <div className="fixed inset-0 z-50 bg-background/80 backdrop-blur-sm flex items-center justify-center">
          <div className="bg-background border rounded-lg shadow-lg p-6 max-w-md mx-4">
            <div className="flex flex-col items-center text-center">
              <div className="h-12 w-12 rounded-full bg-destructive/10 flex items-center justify-center mb-4">
                <Trash2 className="h-6 w-6 text-destructive" />
              </div>
              <h3 className="text-lg font-semibold mb-2">Delete Recording?</h3>
              <p className="text-muted-foreground mb-6">
                This will permanently delete the audio file, all transcripts, and chat history. This action cannot be undone.
              </p>
              <div className="flex gap-3">
                <Button
                  variant="outline"
                  onClick={() => setShowDeleteConfirm(false)}
                  disabled={isDeleting}
                >
                  Cancel
                </Button>
                <Button
                  variant="destructive"
                  onClick={handleDeleteRecording}
                  disabled={isDeleting}
                >
                  {isDeleting ? (
                    <>
                      <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                      Deleting...
                    </>
                  ) : (
                    'Delete'
                  )}
                </Button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Main Content with Tabs at Top */}
      <div className="flex-1 flex flex-col overflow-hidden">
        <Tabs defaultValue="transcript" className="flex-1 flex flex-col overflow-hidden">
          {/* Tab triggers at top */}
          <div className="flex-shrink-0 border-b px-8 py-3">
            <TabsList>
              <TabsTrigger value="transcript" className="gap-2">
                <FileAudio className="h-4 w-4" />
                Transcript
              </TabsTrigger>
              <TabsTrigger value="chat" className="gap-2">
                <MessageSquare className="h-4 w-4" />
                Chat
              </TabsTrigger>
            </TabsList>
          </div>

          {/* Transcript Tab - Scrollable Content */}
          <TabsContent value="transcript" className="flex-1 overflow-y-auto mt-0 p-8 data-[state=inactive]:hidden">
            <div className="space-y-6">
              {/* Error Alert */}
              {error && (
                <Card className="border-destructive/50 bg-destructive/10">
                  <CardContent className="pt-6">
                    <div className="flex items-center gap-2 text-destructive">
                      <AlertCircle className="h-4 w-4" />
                      <p className="text-sm">{error}</p>
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Recording Info Card */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-lg">Recording Details</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <div className="flex items-center gap-2">
                      <Calendar className="h-4 w-4 text-muted-foreground" />
                      <div>
                        <p className="text-sm text-muted-foreground">Date</p>
                        <p className="text-sm font-medium">{formatDate(rec.created_at)}</p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Clock className="h-4 w-4 text-muted-foreground" />
                      <div>
                        <p className="text-sm text-muted-foreground">Duration</p>
                        <p className="text-sm font-medium">{formatDuration(rec.duration_seconds)}</p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <FileAudio className="h-4 w-4 text-muted-foreground" />
                      <div>
                        <p className="text-sm text-muted-foreground">Segments</p>
                        <p className="text-sm font-medium">{transcripts.length}</p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Mic className="h-4 w-4 text-muted-foreground" />
                      <div>
                        <p className="text-sm text-muted-foreground">Model</p>
                        <p className="text-sm font-medium">{rec.transcription_model || 'Default'}</p>
                      </div>
                    </div>
                    {rec.diarization_provider && (
                      <div className="flex items-center gap-2">
                        <Users className="h-4 w-4 text-muted-foreground" />
                        <div>
                          <p className="text-sm text-muted-foreground">Diarization</p>
                          <p className="text-sm font-medium capitalize">{rec.diarization_provider}</p>
                        </div>
                      </div>
                    )}
                  </div>

                  {/* Categories */}
                  <div className="mt-4">
                    <div className="flex items-center justify-between mb-2">
                      <p className="text-sm text-muted-foreground">Categories</p>
                      {recordingId && (
                        <CategoryTagSelector
                          recordingId={recordingId}
                          selectedCategories={recording.categories}
                          selectedTags={recording.tags}
                          onCategoryChange={(newCategories) => {
                            // Update local state without triggering full refresh
                            setRecording(prev => prev ? {
                              ...prev,
                              categories: newCategories
                            } : null)
                          }}
                          onTagChange={(newTags) => {
                            // Update local state without triggering full refresh
                            setRecording(prev => prev ? {
                              ...prev,
                              tags: newTags
                            } : null)
                          }}
                        />
                      )}
                    </div>
                    {recording.categories.length > 0 ? (
                      <div className="flex flex-wrap gap-2">
                        {recording.categories.map((cat) => (
                          <Badge
                            key={cat.id}
                            variant="outline"
                            style={cat.color ? { borderColor: cat.color, color: cat.color } : undefined}
                          >
                            {cat.name}
                          </Badge>
                        ))}
                      </div>
                    ) : (
                      <p className="text-sm text-muted-foreground italic">No categories assigned</p>
                    )}
                  </div>
                </CardContent>
              </Card>

              {/* Re-transcription Card (Collapsible) */}
              {rec.audio_file_path && (
                <Card>
                  <CardHeader
                    className="cursor-pointer hover:bg-muted/50 transition-colors"
                    onClick={() => setShowRetranscribe(!showRetranscribe)}
                  >
                    <div className="flex items-center justify-between">
                      <CardTitle className="text-lg flex items-center gap-2">
                        <RefreshCw className="h-4 w-4" />
                        Re-transcribe
                      </CardTitle>
                      {showRetranscribe ? (
                        <ChevronUp className="h-5 w-5 text-muted-foreground" />
                      ) : (
                        <ChevronDown className="h-5 w-5 text-muted-foreground" />
                      )}
                    </div>
                    {!showRetranscribe && (
                      <p className="text-sm text-muted-foreground mt-1">
                        Click to re-process audio with a different model
                      </p>
                    )}
                  </CardHeader>
                  {showRetranscribe && (
                  <CardContent>
                    <div className="space-y-4">
                      <p className="text-sm text-muted-foreground">
                        Re-process the audio file with a different model for potentially better accuracy.
                      </p>

                      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                        {/* Model Selector */}
                        <div>
                          <label className="text-sm text-muted-foreground mb-2 block">
                            Whisper Model
                          </label>
                          <Select
                            value={selectedModel}
                            onValueChange={setSelectedModel}
                            disabled={isRetranscribing || isLoadingModel}
                          >
                            <SelectTrigger className="w-full">
                              <SelectValue placeholder="Select a model" />
                            </SelectTrigger>
                            <SelectContent>
                              {availableModelsList.map((model) => (
                                <SelectItem key={model.name} value={model.name}>
                                  <div className="flex items-center gap-2">
                                    <span>{model.name}</span>
                                    <span className="text-xs text-muted-foreground">
                                      ({model.size_mb}MB - {model.accuracy})
                                    </span>
                                    {model.name === currentModel && (
                                      <Check className="h-3 w-3 text-green-500" />
                                    )}
                                  </div>
                                </SelectItem>
                              ))}
                              {availableModelsList.length === 0 && (
                                <SelectItem value="__none__" disabled>
                                  No models available - download in Settings
                                </SelectItem>
                              )}
                            </SelectContent>
                          </Select>
                        </div>

                        {/* Language Selector */}
                        <div>
                          <label className="text-sm text-muted-foreground mb-2 block">
                            Language
                          </label>
                          <Select
                            value={selectedLanguage}
                            onValueChange={setSelectedLanguage}
                            disabled={isRetranscribing}
                          >
                            <SelectTrigger className="w-full">
                              <SelectValue placeholder="Select language" />
                            </SelectTrigger>
                            <SelectContent>
                              {languages.map((lang) => (
                                <SelectItem key={lang.value} value={lang.value}>
                                  {lang.label}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        </div>
                      </div>

                      {/* Re-transcribe Button */}
                      <div className="flex justify-end">
                        <Button
                          onClick={handleRetranscribe}
                          disabled={isRetranscribing || isLoadingModel || !selectedModel}
                          className="w-full sm:w-auto"
                        >
                          {isLoadingModel ? (
                            <>
                              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                              Loading Model...
                            </>
                          ) : isRetranscribing ? (
                            <>
                              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                              Processing...
                            </>
                          ) : (
                            <>
                              <RefreshCw className="h-4 w-4 mr-2" />
                              Re-transcribe
                            </>
                          )}
                        </Button>
                      </div>

                      {/* Diarization Option */}
                      <div className="space-y-3 pt-2 border-t">
                        <div className="flex items-center gap-2">
                          <Checkbox
                            id="enable-diarization"
                            checked={enableDiarization}
                            onCheckedChange={(checked) => setEnableDiarization(checked === true)}
                            disabled={isRetranscribing || isDiarizationDownloading || sortformerDownloading}
                          />
                          <label
                            htmlFor="enable-diarization"
                            className="text-sm font-medium cursor-pointer flex items-center gap-1"
                          >
                            <Users className="h-4 w-4" />
                            Identify speakers (diarization)
                          </label>
                        </div>

                        {enableDiarization && (
                          <div className="ml-6 space-y-3">
                            {/* Provider Selector */}
                            <div className="flex items-center gap-4">
                              <label className="text-sm text-muted-foreground">Provider:</label>
                              <Select
                                value={diarizationProvider}
                                onValueChange={(v: 'pyannote' | 'sortformer') => setDiarizationProvider(v)}
                                disabled={isRetranscribing}
                              >
                                <SelectTrigger className="w-48">
                                  <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                  <SelectItem value="pyannote">
                                    <span>PyAnnote</span>
                                    <span className="text-xs text-muted-foreground ml-2">Unlimited speakers</span>
                                  </SelectItem>
                                  <SelectItem value="sortformer">
                                    <span>Sortformer v2</span>
                                    <span className="text-xs text-muted-foreground ml-2">Up to 4 speakers</span>
                                  </SelectItem>
                                </SelectContent>
                              </Select>
                            </div>

                            {/* PyAnnote-specific settings */}
                            {diarizationProvider === 'pyannote' && (
                              <>
                                {/* Max Speakers Selector */}
                                <div className="flex items-center gap-4">
                                  <label className="text-sm text-muted-foreground">Max speakers:</label>
                                  <Select
                                    value={maxSpeakers}
                                    onValueChange={setMaxSpeakers}
                                    disabled={isRetranscribing}
                                  >
                                    <SelectTrigger className="w-32">
                                      <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                      <SelectItem value="auto">Auto</SelectItem>
                                      <SelectItem value="2">2</SelectItem>
                                      <SelectItem value="3">3</SelectItem>
                                      <SelectItem value="4">4</SelectItem>
                                      <SelectItem value="5">5</SelectItem>
                                      <SelectItem value="6">6</SelectItem>
                                      <SelectItem value="8">8</SelectItem>
                                      <SelectItem value="10">10</SelectItem>
                                    </SelectContent>
                                  </Select>
                                </div>

                                {/* Similarity Threshold Slider */}
                                <div className="space-y-2">
                                  <div className="flex items-center justify-between">
                                    <label className="text-sm text-muted-foreground">
                                      Speaker matching sensitivity:
                                    </label>
                                    <span className="text-sm font-mono text-muted-foreground">
                                      {similarityThreshold.toFixed(2)}
                                    </span>
                                  </div>
                                  <Slider
                                    value={[similarityThreshold]}
                                    onValueChange={([v]) => setSimilarityThreshold(v)}
                                    min={0.2}
                                    max={0.8}
                                    step={0.05}
                                    disabled={isRetranscribing}
                                    className="w-full"
                                  />
                                  <p className="text-xs text-muted-foreground">
                                    Lower = fewer speakers (more lenient), Higher = more speakers (stricter)
                                  </p>
                                </div>
                              </>
                            )}

                            {/* Model Status for selected provider */}
                            <div className="flex items-center gap-2">
                              {diarizationProvider === 'pyannote' ? (
                                <>
                                  {diarizationModelsReady ? (
                                    <Badge variant="outline" className="text-green-600 border-green-600">
                                      <Check className="h-3 w-3 mr-1" />
                                      PyAnnote Models Ready
                                    </Badge>
                                  ) : (
                                    <Button
                                      variant="outline"
                                      size="sm"
                                      onClick={downloadDiarizationModels}
                                      disabled={isDiarizationDownloading}
                                    >
                                      {isDiarizationDownloading ? (
                                        <>
                                          <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                                          {diarizationDownloadProgress
                                            ? `${diarizationDownloadProgress.progress}%`
                                            : 'Downloading...'}
                                        </>
                                      ) : (
                                        <>
                                          <Download className="h-3 w-3 mr-1" />
                                          Download PyAnnote (~32MB)
                                        </>
                                      )}
                                    </Button>
                                  )}
                                </>
                              ) : (
                                <>
                                  {sortformerModelReady ? (
                                    <Badge variant="outline" className="text-green-600 border-green-600">
                                      <Check className="h-3 w-3 mr-1" />
                                      Sortformer Model Ready
                                    </Badge>
                                  ) : (
                                    <Button
                                      variant="outline"
                                      size="sm"
                                      onClick={handleDownloadSortformer}
                                      disabled={sortformerDownloading}
                                    >
                                      {sortformerDownloading ? (
                                        <>
                                          <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                                          Downloading...
                                        </>
                                      ) : (
                                        <>
                                          <Download className="h-3 w-3 mr-1" />
                                          Download Sortformer (~25MB)
                                        </>
                                      )}
                                    </Button>
                                  )}
                                </>
                              )}
                            </div>
                          </div>
                        )}
                      </div>

                      {/* Current model indicator */}
                      {currentModel && (
                        <p className="text-xs text-muted-foreground">
                          Current loaded model: <span className="font-medium">{currentModel}</span>
                        </p>
                      )}
                    </div>
                  </CardContent>
                  )}
                </Card>
              )}

              {/* Retranscription Progress */}
              {isRetranscribing && (
                <Card className="border-primary/50">
                  <CardContent className="pt-6">
                    <div className="flex items-center gap-4">
                      <Loader2 className="h-5 w-5 animate-spin text-primary" />
                      <div className="flex-1">
                        <p className="text-sm font-medium">{retranscriptionStatus.message}</p>
                        <Progress value={retranscriptionStatus.progress} className="mt-2" />
                      </div>
                      <Badge variant="secondary">
                        {retranscriptionStatus.currentChunk}/{retranscriptionStatus.totalChunks} chunks
                      </Badge>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleCancelRetranscription}
                        disabled={isCancelling}
                        className="text-destructive hover:text-destructive hover:bg-destructive/10"
                      >
                        {isCancelling ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <>
                            <XCircle className="h-4 w-4 mr-1" />
                            Cancel
                          </>
                        )}
                      </Button>
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Transcripts */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-lg">Transcript</CardTitle>
                </CardHeader>
                <CardContent>
                  {transcripts.length === 0 ? (
                    <div className="text-center py-8">
                      <FileAudio className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
                      <p className="text-muted-foreground">No transcript segments available</p>
                      <p className="text-sm text-muted-foreground mt-2">
                        Use the Re-transcribe section above to generate a transcript.
                      </p>
                    </div>
                  ) : (
                    <div className="space-y-4">
                      {transcripts.map((segment) => (
                        <div
                          key={segment.id}
                          className="flex gap-4"
                          style={{
                            borderLeft: segment.speaker_id
                              ? `3px solid ${getSpeakerColor(segment.speaker_id)}`
                              : undefined,
                            paddingLeft: segment.speaker_id ? '12px' : undefined,
                          }}
                        >
                          <div className="flex-shrink-0 w-16">
                            <span className="text-xs text-muted-foreground font-mono">
                              {segment.display_time}
                            </span>
                          </div>
                          <div className="flex-1">
                            {segment.speaker_id && (
                              <div className="mb-1">
                                <SpeakerLabel
                                  speakerId={segment.speaker_id}
                                  speakerLabel={segment.speaker_label}
                                  isRegistered={segment.is_registered_speaker}
                                  onRename={async (newLabel) => {
                                    if (segment.speaker_id) {
                                      await renameSpeaker(segment.speaker_id, newLabel)
                                      // Refresh transcript data to show updated speaker labels
                                      await fetchData()
                                    }
                                  }}
                                  size="sm"
                                />
                              </div>
                            )}
                            {editingSegmentId === segment.id ? (
                              <div className="space-y-1">
                                <textarea
                                  ref={editTextareaRef}
                                  value={editingText}
                                  onChange={(e) => setEditingText(e.target.value)}
                                  className="w-full text-sm text-foreground leading-relaxed bg-muted/30 border border-primary rounded px-2 py-1 focus:outline-none focus:ring-1 focus:ring-primary resize-none min-h-[60px]"
                                  onKeyDown={(e) => {
                                    if (e.key === 'Escape') {
                                      handleCancelEditSegment()
                                    } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                                      handleSaveSegmentText()
                                    }
                                  }}
                                  rows={Math.max(2, Math.ceil(editingText.length / 80))}
                                />
                                <div className="flex items-center gap-1">
                                  <button
                                    onClick={handleSaveSegmentText}
                                    className="px-2 py-0.5 text-xs bg-primary text-primary-foreground hover:bg-primary/90 rounded flex items-center gap-1"
                                  >
                                    <Check className="h-3 w-3" />
                                    Save
                                  </button>
                                  <button
                                    onClick={handleCancelEditSegment}
                                    className="px-2 py-0.5 text-xs bg-muted hover:bg-muted/80 rounded flex items-center gap-1"
                                  >
                                    <X className="h-3 w-3" />
                                    Cancel
                                  </button>
                                  <span className="text-[10px] text-muted-foreground ml-1">
                                    Ctrl+Enter to save, Esc to cancel
                                  </span>
                                </div>
                              </div>
                            ) : (
                              <p
                                className="text-sm text-foreground leading-relaxed group/text cursor-pointer"
                                onClick={() => handleStartEditSegment(segment)}
                              >
                                <span className="hover:bg-muted/50 rounded px-1 -mx-1 transition-colors inline">
                                  {segment.text}
                                </span>
                                <button
                                  className="ml-1 opacity-0 group-hover/text:opacity-100 transition-opacity p-0.5 hover:bg-muted rounded inline-flex align-middle"
                                  onClick={(e) => {
                                    e.stopPropagation()
                                    handleStartEditSegment(segment)
                                  }}
                                  title="Edit text"
                                >
                                  <Edit2 className="h-3 w-3 text-muted-foreground" />
                                </button>
                              </p>
                            )}
                            {segment.confidence < 0.8 && (
                              <Badge variant="outline" className="mt-1 text-xs">
                                Low confidence: {Math.round(segment.confidence * 100)}%
                              </Badge>
                            )}
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* Full Transcript Text */}
              {transcripts.length > 0 && (
                <Card>
                  <CardHeader>
                    <CardTitle className="text-lg">Full Text</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="bg-muted/50 rounded-lg p-4">
                      <p className="text-sm text-foreground leading-relaxed whitespace-pre-wrap">
                        {transcripts.map(s => s.text).join(' ')}
                      </p>
                    </div>
                  </CardContent>
                </Card>
              )}
            </div>
          </TabsContent>

          {/* Chat Tab - Full Height */}
          <TabsContent value="chat" className="flex-1 flex flex-col mt-0 overflow-hidden data-[state=inactive]:hidden">
            <div className="flex-1 flex flex-col p-8 overflow-hidden">
              <Card className="flex-1 flex flex-col overflow-hidden">
                <CardHeader className="flex-shrink-0">
                  <CardTitle className="text-lg flex items-center gap-2">
                    <MessageSquare className="h-4 w-4" />
                    Chat with AI
                  </CardTitle>
                </CardHeader>
                <CardContent className="flex-1 flex flex-col overflow-hidden p-0">
                  <ChatPanel recordingId={recordingId || ''} className="flex-1" />
                </CardContent>
              </Card>
            </div>
          </TabsContent>
        </Tabs>
      </div>
    </>
  )
}

export default function RecordingDetailPage() {
  return (
    <Suspense fallback={
      <div className="flex flex-col items-center justify-center h-full">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mb-4" />
        <p className="text-muted-foreground">Loading...</p>
      </div>
    }>
      <RecordingDetailContent />
    </Suspense>
  )
}
