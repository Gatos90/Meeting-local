'use client'

import { useState, useEffect } from 'react'
import { Clock, FileText, Sparkles, Trash2, Save, ChevronDown, Folder, ExternalLink } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { CategoryTagSelector } from './CategoryTagSelector'
import { CategoryBadge, TagBadge } from './CategoryBadge'
import type { Category, Tag } from '@/types/database'

interface TranscriptSegment {
  id: string
  text: string
  timestamp: string
  audioStartTime?: number
  audioEndTime?: number
  confidence?: number
  sequenceId?: number
}

interface RecordingInfo {
  id: string
  title: string
  duration: number
  transcriptCount: number
  audioPath: string | null
  meetingFolderPath: string | null
}

interface PostRecordingModalProps {
  isOpen: boolean
  onClose: () => void
  onSave: (title: string, retranscribe: boolean, model?: string) => void
  onDiscard: () => void
  recording: RecordingInfo
  transcripts: TranscriptSegment[]
}

const AVAILABLE_MODELS = [
  { value: 'current', label: 'Same as live', description: 'Use the model from live transcription' },
  { value: 'base', label: 'Base', description: 'Fast, good for clear audio' },
  { value: 'small', label: 'Small', description: 'Balanced speed and accuracy' },
  { value: 'medium', label: 'Medium', description: 'Better accuracy, slower' },
  { value: 'large-v3', label: 'Large v3', description: 'Best accuracy, slowest' },
  { value: 'large-v3-turbo', label: 'Large v3 Turbo', description: 'Near-best accuracy, faster' },
]

function formatDuration(seconds: number): string {
  const hrs = Math.floor(seconds / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  const secs = seconds % 60

  if (hrs > 0) {
    return `${hrs}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`
  }
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

export function PostRecordingModal({
  isOpen,
  onClose,
  onSave,
  onDiscard,
  recording,
  transcripts,
}: PostRecordingModalProps) {
  const [title, setTitle] = useState(recording.title)
  const [selectedModel, setSelectedModel] = useState('current')
  const [wantsRetranscription, setWantsRetranscription] = useState(false)
  const [selectedCategories, setSelectedCategories] = useState<Category[]>([])
  const [selectedTags, setSelectedTags] = useState<Tag[]>([])

  // Reset state when modal opens with new recording
  useEffect(() => {
    if (isOpen) {
      setTitle(recording.title)
      setSelectedModel('current')
      setWantsRetranscription(false)
      setSelectedCategories([])
      setSelectedTags([])
    }
  }, [isOpen, recording.title])

  // Combine all transcript text for preview
  const transcriptPreview = transcripts
    .map(t => t.text)
    .join(' ')
    .slice(0, 500)

  const handleSave = () => {
    onSave(
      title,
      wantsRetranscription,
      wantsRetranscription && selectedModel !== 'current' ? selectedModel : undefined
    )
  }

  const handleDiscard = () => {
    onDiscard()
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            Recording Complete
          </DialogTitle>
          <DialogDescription>
            Review and save your recording, or enhance it with better transcription.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Title */}
          <div className="space-y-2">
            <label className="text-sm font-medium">Title</label>
            <Input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Enter a title for this recording..."
              className="text-lg"
            />
          </div>

          {/* Duration & Stats */}
          <div className="flex items-center gap-6 text-sm text-muted-foreground">
            <div className="flex items-center gap-2">
              <Clock className="h-4 w-4" />
              <span>Duration: {formatDuration(recording.duration)}</span>
            </div>
            <div className="flex items-center gap-2">
              <FileText className="h-4 w-4" />
              <span>{transcripts.length} transcript segments</span>
            </div>
          </div>

          {/* Audio File Location */}
          {recording.meetingFolderPath && (
            <div className="space-y-2">
              <label className="text-sm font-medium">Recording Location</label>
              <div className="flex items-center gap-2 p-2 bg-muted/50 rounded-lg">
                <Folder className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                <span className="text-sm text-muted-foreground truncate flex-1">
                  {recording.meetingFolderPath}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 px-2"
                  onClick={async () => {
                    try {
                      await invoke('open_folder', { path: recording.meetingFolderPath })
                    } catch (err) {
                      console.error('Failed to open folder:', err)
                    }
                  }}
                >
                  <ExternalLink className="h-4 w-4" />
                </Button>
              </div>
            </div>
          )}

          {/* Transcript Preview */}
          {transcriptPreview && (
            <div className="space-y-2">
              <label className="text-sm font-medium">Transcript Preview</label>
              <div className="p-3 bg-muted/50 rounded-lg max-h-32 overflow-y-auto text-sm text-muted-foreground">
                {transcriptPreview}
                {transcripts.map(t => t.text).join(' ').length > 500 && (
                  <span className="text-muted-foreground/50">...</span>
                )}
              </div>
            </div>
          )}

          {/* Categories & Tags */}
          <div className="space-y-2">
            <label className="text-sm font-medium">Organization</label>
            <div className="flex flex-wrap items-center gap-2">
              {selectedCategories.map((cat) => (
                <CategoryBadge
                  key={cat.id}
                  name={cat.name}
                  color={cat.color}
                  onRemove={() => setSelectedCategories(prev => prev.filter(c => c.id !== cat.id))}
                />
              ))}
              {selectedTags.map((tag) => (
                <TagBadge
                  key={tag.id}
                  name={tag.name}
                  color={tag.color}
                  onRemove={() => setSelectedTags(prev => prev.filter(t => t.id !== tag.id))}
                />
              ))}
              <CategoryTagSelector
                recordingId={recording.id}
                selectedCategories={selectedCategories}
                selectedTags={selectedTags}
                onCategoryChange={setSelectedCategories}
                onTagChange={setSelectedTags}
              />
            </div>
          </div>

          {/* Re-transcription Section */}
          <div className="space-y-3 p-4 border rounded-lg bg-muted/30">
            <div className="flex items-start justify-between">
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <Sparkles className="h-4 w-4 text-primary" />
                  <span className="text-sm font-medium">Enhance Transcription</span>
                </div>
                <p className="text-xs text-muted-foreground">
                  Re-transcribe the full audio file for better accuracy.
                  This runs in the background after you save.
                </p>
              </div>
              <Button
                variant={wantsRetranscription ? "default" : "outline"}
                size="sm"
                onClick={() => setWantsRetranscription(!wantsRetranscription)}
              >
                {wantsRetranscription ? "Enabled" : "Enable"}
              </Button>
            </div>

            {wantsRetranscription && (
              <div className="pt-3 border-t space-y-2">
                <label className="text-xs font-medium text-muted-foreground">
                  Model for re-transcription
                </label>
                <TooltipProvider>
                  <Select value={selectedModel} onValueChange={setSelectedModel}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="Select model" />
                    </SelectTrigger>
                    <SelectContent>
                      {AVAILABLE_MODELS.map((model) => (
                        <Tooltip key={model.value}>
                          <TooltipTrigger asChild>
                            <SelectItem value={model.value}>
                              {model.label}
                            </SelectItem>
                          </TooltipTrigger>
                          <TooltipContent side="right">
                            <p>{model.description}</p>
                          </TooltipContent>
                        </Tooltip>
                      ))}
                    </SelectContent>
                  </Select>
                </TooltipProvider>
                <p className="text-xs text-muted-foreground">
                  Larger models are more accurate but take longer to process.
                </p>
              </div>
            )}
          </div>
        </div>

        <DialogFooter className="gap-2 sm:gap-0">
          <Button
            variant="ghost"
            onClick={handleDiscard}
            className="text-destructive hover:text-destructive hover:bg-destructive/10"
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Discard Recording
          </Button>
          <div className="flex-1" />
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSave}>
            <Save className="h-4 w-4 mr-2" />
            Save Recording
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
