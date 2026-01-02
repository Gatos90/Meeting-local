'use client'

import { X } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

interface CategoryBadgeProps {
  name: string
  color?: string | null
  isSystem?: boolean
  onRemove?: () => void
  className?: string
  size?: 'sm' | 'md'
}

export function CategoryBadge({
  name,
  color,
  isSystem = false,
  onRemove,
  className,
  size = 'md',
}: CategoryBadgeProps) {
  const sizeClasses = size === 'sm' ? 'text-[10px] px-1.5 py-0' : 'text-xs px-2 py-0.5'

  return (
    <Badge
      variant="outline"
      className={cn(
        'font-medium transition-colors',
        sizeClasses,
        onRemove && 'pr-1',
        className
      )}
      style={color ? {
        borderColor: color,
        backgroundColor: `${color}15`,
        color: color,
      } : undefined}
    >
      <span className="truncate max-w-[100px]">{name}</span>
      {isSystem && (
        <span className="ml-1 opacity-50 text-[9px]">*</span>
      )}
      {onRemove && (
        <button
          onClick={(e) => {
            e.stopPropagation()
            onRemove()
          }}
          className="ml-1 hover:bg-black/10 rounded p-0.5 transition-colors"
          aria-label={`Remove ${name}`}
        >
          <X className="h-3 w-3" />
        </button>
      )}
    </Badge>
  )
}

// Tag badge variant (slightly different styling)
interface TagBadgeProps {
  name: string
  color?: string | null
  usageCount?: number
  onRemove?: () => void
  className?: string
  size?: 'sm' | 'md'
}

export function TagBadge({
  name,
  color,
  usageCount,
  onRemove,
  className,
  size = 'md',
}: TagBadgeProps) {
  const sizeClasses = size === 'sm' ? 'text-[10px] px-1.5 py-0' : 'text-xs px-2 py-0.5'

  return (
    <Badge
      variant="secondary"
      className={cn(
        'font-normal transition-colors',
        sizeClasses,
        onRemove && 'pr-1',
        className
      )}
      style={color ? {
        backgroundColor: `${color}20`,
        color: color,
      } : undefined}
    >
      <span className="truncate max-w-[100px]">{name}</span>
      {usageCount !== undefined && usageCount > 0 && (
        <span className="ml-1 opacity-60 text-[9px]">({usageCount})</span>
      )}
      {onRemove && (
        <button
          onClick={(e) => {
            e.stopPropagation()
            onRemove()
          }}
          className="ml-1 hover:bg-black/10 rounded p-0.5 transition-colors"
          aria-label={`Remove ${name}`}
        >
          <X className="h-3 w-3" />
        </button>
      )}
    </Badge>
  )
}
