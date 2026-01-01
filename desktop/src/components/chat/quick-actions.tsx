'use client'

import { Button } from '@/components/ui/button'
import { useTemplates } from '@/hooks/useTemplates'
import {
  FileText,
  List,
  CheckSquare,
  MessageSquare,
  Sparkles,
  ClipboardList,
  Users,
  Target,
  Calendar,
  Lightbulb,
  LucideIcon,
} from 'lucide-react'

// Map icon names to Lucide components
const iconMap: Record<string, LucideIcon> = {
  FileText,
  List,
  CheckSquare,
  MessageSquare,
  Sparkles,
  ClipboardList,
  Users,
  Target,
  Calendar,
  Lightbulb,
}

interface QuickActionsProps {
  /** Called with the template prompt when a button is clicked */
  onAction: (prompt: string) => void
  disabled?: boolean
}

export function QuickActions({ onAction, disabled = false }: QuickActionsProps) {
  const { templates, isLoading } = useTemplates()

  // Get the icon component for a template
  const getIcon = (iconName?: string): LucideIcon => {
    if (iconName && iconMap[iconName]) {
      return iconMap[iconName]
    }
    return MessageSquare // Default icon
  }

  if (isLoading) {
    return (
      <div className="flex flex-wrap gap-2">
        {/* Loading skeleton */}
        {[1, 2, 3].map(i => (
          <div key={i} className="h-8 w-24 bg-muted animate-pulse rounded-md" />
        ))}
      </div>
    )
  }

  if (templates.length === 0) {
    return null
  }

  return (
    <div className="flex flex-wrap gap-2">
      {templates.map(template => {
        const Icon = getIcon(template.icon || undefined)
        return (
          <Button
            key={template.id}
            variant="outline"
            size="sm"
            onClick={() => onAction(template.prompt)}
            disabled={disabled}
            className="gap-1.5"
            title={template.description || template.name}
          >
            <Icon className="w-3.5 h-3.5" />
            {template.name}
          </Button>
        )
      })}
    </div>
  )
}
