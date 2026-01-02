'use client'

import { useState, useRef } from 'react'
import { FileText, Clock, CheckCircle2, Tag, Edit2, Check, X, MoreVertical, Trash2 } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { CategoryTagSelector } from '@/components/CategoryTagSelector'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import type { Category, Tag as TagType } from '@/types/database'

interface RecordingCardProps {
  id?: string
  title: string
  date: string
  duration: string
  // Support both old tag string and new categories array
  tag?: string
  categories?: Category[]
  tags?: TagType[]
  transcribed?: boolean
  transcriptCount?: number
  onClick?: () => void
  // Callbacks for inline editing
  onTitleChange?: (newTitle: string) => void
  onCategoriesChange?: (categories: Category[]) => void
  onTagsChange?: (tags: TagType[]) => void
  onDelete?: () => void
}

// Format seconds to human readable duration (e.g., "15:20" or "1:12:05")
export function formatDuration(seconds: number | null | undefined): string {
  if (!seconds) return '0:00'
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = Math.floor(seconds % 60)

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`
}

// Format ISO date to human readable format
export function formatDate(isoDate: string | null | undefined): string {
  if (!isoDate) return ''
  try {
    const date = new Date(isoDate)
    const now = new Date()
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate())
    const yesterday = new Date(today.getTime() - 86400000)
    const recordDate = new Date(date.getFullYear(), date.getMonth(), date.getDate())

    const timeStr = date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })

    if (recordDate.getTime() === today.getTime()) {
      return `Today, ${timeStr}`
    }
    if (recordDate.getTime() === yesterday.getTime()) {
      return `Yesterday, ${timeStr}`
    }
    return date.toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' })
  } catch {
    return isoDate
  }
}

export function RecordingCard({
  id,
  title,
  date,
  duration,
  tag,
  categories,
  tags,
  transcribed = false,
  transcriptCount,
  onClick,
  onTitleChange,
  onCategoriesChange,
  onTagsChange,
  onDelete,
}: RecordingCardProps) {
  // Editing state
  const [isEditingTitle, setIsEditingTitle] = useState(false)
  const [editingTitle, setEditingTitle] = useState('')
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)
  const editTitleRef = useRef<HTMLInputElement>(null)

  // Determine what to display for the badge
  const primaryCategory = categories?.[0]
  const displayTag = primaryCategory?.name || tag || 'Uncategorized'
  const hasMoreCategories = (categories?.length || 0) > 1

  // Determine if transcribed based on transcriptCount if available
  const isTranscribed = transcribed || (transcriptCount !== undefined && transcriptCount > 0)

  // Start editing title
  const handleStartEditTitle = (e: React.MouseEvent) => {
    e.stopPropagation()
    setIsEditingTitle(true)
    setEditingTitle(title)
    setTimeout(() => editTitleRef.current?.focus(), 0)
  }

  // Save title
  const handleSaveTitle = async () => {
    const trimmedTitle = editingTitle.trim()
    if (!trimmedTitle || trimmedTitle === title || !id) {
      setIsEditingTitle(false)
      setEditingTitle('')
      return
    }

    try {
      await invoke('db_update_recording', {
        id,
        updates: { title: trimmedTitle },
      })
      onTitleChange?.(trimmedTitle)
      setIsEditingTitle(false)
      setEditingTitle('')
    } catch (err) {
      console.error('Failed to save title:', err)
    }
  }

  // Cancel editing title
  const handleCancelEditTitle = () => {
    setIsEditingTitle(false)
    setEditingTitle('')
  }

  return (
    <Card
      className="group cursor-pointer hover:shadow-md transition-all border-border/60 overflow-hidden relative"
      onClick={isEditingTitle || showDeleteConfirm ? undefined : onClick}
    >
      {/* Delete Confirmation Overlay */}
      {showDeleteConfirm && (
        <div
          className="absolute inset-0 z-10 bg-background/95 backdrop-blur-sm flex flex-col items-center justify-center p-4"
          onClick={(e) => e.stopPropagation()}
        >
          <Trash2 className="h-8 w-8 text-destructive mb-3" />
          <p className="text-sm font-medium text-center mb-1">Delete this recording?</p>
          <p className="text-xs text-muted-foreground text-center mb-4">
            This will permanently delete the audio file and all transcripts.
          </p>
          <div className="flex gap-2">
            <button
              onClick={() => setShowDeleteConfirm(false)}
              className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={() => {
                setShowDeleteConfirm(false)
                onDelete?.()
              }}
              className="px-3 py-1.5 text-sm rounded-md bg-destructive text-destructive-foreground hover:bg-destructive/90 transition-colors"
            >
              Delete
            </button>
          </div>
        </div>
      )}

      <div className="p-5 space-y-4">
        {/* Header */}
        <div className="flex justify-between items-start">
          <div className="flex items-start gap-2">
            <div className="h-10 w-10 rounded-full bg-muted flex items-center justify-center text-muted-foreground group-hover:bg-primary/10 group-hover:text-primary transition-colors">
              <FileText className="h-5 w-5" />
            </div>
            {/* More menu with delete option */}
            {id && onDelete && (
              <div onClick={(e) => e.stopPropagation()}>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <button className="p-1.5 rounded-md opacity-0 group-hover:opacity-100 hover:bg-muted transition-all">
                      <MoreVertical className="h-4 w-4 text-muted-foreground" />
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="start">
                    <DropdownMenuItem
                      className="text-destructive focus:text-destructive cursor-pointer"
                      onClick={() => setShowDeleteConfirm(true)}
                    >
                      <Trash2 className="h-4 w-4 mr-2" />
                      Delete recording
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            )}
          </div>
          <div className="flex flex-col items-end gap-1">
            <div className="flex items-center gap-1">
              {id && (onCategoriesChange || onTagsChange) ? (
                <div onClick={(e) => e.stopPropagation()}>
                  <CategoryTagSelector
                    recordingId={id}
                    selectedCategories={categories || []}
                    selectedTags={tags || []}
                    onCategoryChange={onCategoriesChange}
                    onTagChange={onTagsChange}
                    className="h-6 text-xs"
                  />
                </div>
              ) : (
                <>
                  <Badge
                    variant="outline"
                    style={primaryCategory?.color ? {
                      borderColor: primaryCategory.color,
                      backgroundColor: `${primaryCategory.color}10`
                    } : undefined}
                  >
                    {displayTag}
                  </Badge>
                  {hasMoreCategories && (
                    <Badge variant="secondary" className="text-xs">
                      +{(categories?.length || 0) - 1}
                    </Badge>
                  )}
                </>
              )}
            </div>
            {/* Show category/tag badges when editing is enabled */}
            {id && (onCategoriesChange || onTagsChange) && (categories?.length || 0) > 0 && (
              <div className="flex items-center gap-1 flex-wrap justify-end">
                {categories?.slice(0, 2).map((c) => (
                  <Badge
                    key={c.id}
                    variant="outline"
                    className="text-[10px] px-1.5 py-0 h-4"
                    style={c.color ? { borderColor: c.color, backgroundColor: `${c.color}10` } : undefined}
                  >
                    {c.name}
                  </Badge>
                ))}
                {(categories?.length || 0) > 2 && (
                  <Badge variant="secondary" className="text-[10px] px-1.5 py-0 h-4">
                    +{(categories?.length || 0) - 2}
                  </Badge>
                )}
              </div>
            )}
            {tags && tags.length > 0 && (
              <div className="flex items-center gap-1 flex-wrap justify-end">
                {tags.slice(0, 2).map((t) => (
                  <Badge
                    key={t.id}
                    variant="secondary"
                    className="text-[10px] px-1.5 py-0 h-4"
                    style={t.color ? { backgroundColor: `${t.color}20`, color: t.color } : undefined}
                  >
                    {t.name}
                  </Badge>
                ))}
                {tags.length > 2 && (
                  <Badge variant="secondary" className="text-[10px] px-1.5 py-0 h-4">
                    +{tags.length - 2}
                  </Badge>
                )}
              </div>
            )}
          </div>
        </div>

        {/* Content */}
        <div>
          {isEditingTitle ? (
            <div className="flex items-center gap-1" onClick={(e) => e.stopPropagation()}>
              <input
                ref={editTitleRef}
                type="text"
                value={editingTitle}
                onChange={(e) => setEditingTitle(e.target.value)}
                className="flex-1 font-semibold text-foreground bg-transparent border-b border-primary focus:outline-none py-0.5"
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
                className="p-0.5 hover:bg-green-100 dark:hover:bg-green-900 rounded"
                title="Save (Enter)"
              >
                <Check className="h-3.5 w-3.5 text-green-600" />
              </button>
              <button
                onMouseDown={(e) => {
                  e.preventDefault()
                  handleCancelEditTitle()
                }}
                className="p-0.5 hover:bg-red-100 dark:hover:bg-red-900 rounded"
                title="Cancel (Esc)"
              >
                <X className="h-3.5 w-3.5 text-red-600" />
              </button>
            </div>
          ) : (
            <div className="group/title flex items-center gap-1">
              <h4
                className="font-semibold text-foreground line-clamp-1 hover:bg-muted/50 rounded px-1 -mx-1 transition-colors cursor-text"
                onClick={id ? handleStartEditTitle : undefined}
              >
                {title}
              </h4>
              {id && (
                <button
                  className="opacity-0 group-hover/title:opacity-100 transition-opacity p-0.5 hover:bg-muted rounded flex-shrink-0"
                  onClick={handleStartEditTitle}
                  title="Edit title"
                >
                  <Edit2 className="h-3 w-3 text-muted-foreground" />
                </button>
              )}
            </div>
          )}
          <p className="text-sm text-muted-foreground mt-1">{date}</p>
        </div>

        {/* Footer */}
        <div className="flex items-center gap-4 text-xs font-medium text-muted-foreground pt-2 border-t border-border/50">
          <span className="flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {duration}
          </span>
          {isTranscribed && (
            <span className="flex items-center gap-1 text-emerald-600">
              <CheckCircle2 className="h-3 w-3" />
              {transcriptCount !== undefined ? `${transcriptCount} segments` : 'Transcribed'}
            </span>
          )}
        </div>
      </div>
    </Card>
  )
}
