'use client'

import { useState, useRef, useEffect } from 'react'
import { getSpeakerColor } from '@/types/database'
import { User, Check, X, Edit2 } from 'lucide-react'

interface SpeakerLabelProps {
  speakerId: string | null | undefined
  speakerLabel: string | null | undefined
  isRegistered?: boolean
  onRename?: (newLabel: string) => void
  size?: 'sm' | 'md'
  showEditOnHover?: boolean
}

export function SpeakerLabel({
  speakerId,
  speakerLabel,
  isRegistered = false,
  onRename,
  size = 'sm',
  showEditOnHover = true,
}: SpeakerLabelProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [editValue, setEditValue] = useState(speakerLabel || '')
  const inputRef = useRef<HTMLInputElement>(null)

  const color = getSpeakerColor(speakerId)
  const displayLabel = speakerLabel || 'Unknown'

  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus()
      inputRef.current.select()
    }
  }, [isEditing])

  const handleSave = () => {
    if (editValue.trim() && onRename) {
      onRename(editValue.trim())
    }
    setIsEditing(false)
  }

  const handleCancel = () => {
    setEditValue(speakerLabel || '')
    setIsEditing(false)
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSave()
    } else if (e.key === 'Escape') {
      handleCancel()
    }
  }

  if (!speakerId) {
    return null
  }

  const sizeClasses = size === 'sm'
    ? 'text-xs px-1.5 py-0.5 gap-1'
    : 'text-sm px-2 py-1 gap-1.5'

  const iconSize = size === 'sm' ? 10 : 12

  if (isEditing) {
    return (
      <div className="inline-flex items-center gap-1">
        <input
          ref={inputRef}
          type="text"
          value={editValue}
          onChange={(e) => setEditValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleCancel}
          className={`${sizeClasses} rounded border border-gray-300 dark:border-gray-600
                     bg-white dark:bg-gray-800 focus:outline-none focus:ring-1 focus:ring-blue-500`}
          style={{ width: `${Math.max(editValue.length * 8 + 20, 60)}px` }}
        />
        <button
          onMouseDown={(e) => {
            e.preventDefault()
            handleSave()
          }}
          className="p-0.5 hover:bg-green-100 dark:hover:bg-green-900 rounded"
        >
          <Check size={iconSize} className="text-green-600" />
        </button>
        <button
          onMouseDown={(e) => {
            e.preventDefault()
            handleCancel()
          }}
          className="p-0.5 hover:bg-red-100 dark:hover:bg-red-900 rounded"
        >
          <X size={iconSize} className="text-red-600" />
        </button>
      </div>
    )
  }

  return (
    <span
      className={`inline-flex items-center ${sizeClasses} rounded-full font-medium
                  transition-colors cursor-default group`}
      style={{
        backgroundColor: `${color}20`,
        color: color,
        borderLeft: `3px solid ${color}`,
      }}
      title={isRegistered ? 'Registered speaker' : `Speaker ID: ${speakerId}`}
    >
      <User size={iconSize} />
      <span>{displayLabel}</span>
      {isRegistered && (
        <span className="ml-0.5 text-[8px] opacity-75">*</span>
      )}
      {showEditOnHover && onRename && (
        <button
          onClick={() => setIsEditing(true)}
          className="ml-0.5 opacity-0 group-hover:opacity-100 transition-opacity p-0.5
                     hover:bg-white/30 rounded"
        >
          <Edit2 size={iconSize - 2} />
        </button>
      )}
    </span>
  )
}

// Simplified version for inline use in transcripts
export function SpeakerBadge({
  speakerId,
  speakerLabel,
}: {
  speakerId: string | null | undefined
  speakerLabel: string | null | undefined
}) {
  if (!speakerId) return null

  const color = getSpeakerColor(speakerId)
  const label = speakerLabel || 'Unknown'

  return (
    <span
      className="inline-block w-2 h-2 rounded-full mr-1.5 flex-shrink-0"
      style={{ backgroundColor: color }}
      title={label}
    />
  )
}
