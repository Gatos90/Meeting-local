'use client'

import { useState } from 'react'
import { Filter, Calendar, X, Search } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import { CategoryBadge, TagBadge } from './CategoryBadge'
import { useCategories } from '@/hooks/useCategories'
import type { Category, Tag } from '@/types/database'
import { cn } from '@/lib/utils'

export interface SearchFilterValues {
  categoryIds: string[]
  tagIds: string[]
  dateFrom: string | null
  dateTo: string | null
  searchTranscripts: boolean
}

interface SearchFiltersProps {
  filters: SearchFilterValues
  onFiltersChange: (filters: SearchFilterValues) => void
  className?: string
}

export function SearchFilters({
  filters,
  onFiltersChange,
  className,
}: SearchFiltersProps) {
  const [open, setOpen] = useState(false)
  const { categories, tags } = useCategories()

  const activeFilterCount =
    filters.categoryIds.length +
    filters.tagIds.length +
    (filters.dateFrom ? 1 : 0) +
    (filters.dateTo ? 1 : 0)

  const handleCategoryToggle = (categoryId: string) => {
    const newCategoryIds = filters.categoryIds.includes(categoryId)
      ? filters.categoryIds.filter(id => id !== categoryId)
      : [...filters.categoryIds, categoryId]
    onFiltersChange({ ...filters, categoryIds: newCategoryIds })
  }

  const handleTagToggle = (tagId: string) => {
    const newTagIds = filters.tagIds.includes(tagId)
      ? filters.tagIds.filter(id => id !== tagId)
      : [...filters.tagIds, tagId]
    onFiltersChange({ ...filters, tagIds: newTagIds })
  }

  const handleDateChange = (field: 'dateFrom' | 'dateTo', value: string) => {
    onFiltersChange({ ...filters, [field]: value || null })
  }

  const handleTranscriptsToggle = (enabled: boolean) => {
    onFiltersChange({ ...filters, searchTranscripts: enabled })
  }

  const clearFilters = () => {
    onFiltersChange({
      categoryIds: [],
      tagIds: [],
      dateFrom: null,
      dateTo: null,
      searchTranscripts: true,
    })
  }

  const selectedCategories = categories.filter(c => filters.categoryIds.includes(c.id))
  const selectedTags = tags.filter(t => filters.tagIds.includes(t.id))

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          className={cn('h-8 gap-1.5', className)}
        >
          <Filter className="h-3.5 w-3.5" />
          Filters
          {activeFilterCount > 0 && (
            <span className="ml-1 h-4 w-4 rounded-full bg-primary text-[10px] text-primary-foreground flex items-center justify-center">
              {activeFilterCount}
            </span>
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-80 p-0" align="start">
        {/* Header */}
        <div className="flex items-center justify-between p-3 border-b border-border">
          <span className="text-sm font-medium">Search Filters</span>
          {activeFilterCount > 0 && (
            <Button
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-xs"
              onClick={clearFilters}
            >
              Clear all
            </Button>
          )}
        </div>

        <div className="p-3 space-y-4">
          {/* Search in Transcripts Toggle */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Search className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm">Search in transcripts</span>
            </div>
            <Switch
              checked={filters.searchTranscripts}
              onCheckedChange={handleTranscriptsToggle}
            />
          </div>

          {/* Date Range */}
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <Calendar className="h-4 w-4" />
              Date Range
            </div>
            <div className="grid grid-cols-2 gap-2">
              <div>
                <label className="text-[10px] text-muted-foreground">From</label>
                <Input
                  type="date"
                  value={filters.dateFrom || ''}
                  onChange={(e) => handleDateChange('dateFrom', e.target.value)}
                  className="h-8 text-sm"
                />
              </div>
              <div>
                <label className="text-[10px] text-muted-foreground">To</label>
                <Input
                  type="date"
                  value={filters.dateTo || ''}
                  onChange={(e) => handleDateChange('dateTo', e.target.value)}
                  className="h-8 text-sm"
                />
              </div>
            </div>
          </div>

          {/* Categories */}
          <div className="space-y-2">
            <span className="text-sm font-medium text-muted-foreground">Categories</span>
            {selectedCategories.length > 0 && (
              <div className="flex flex-wrap gap-1 mb-2">
                {selectedCategories.map((cat) => (
                  <CategoryBadge
                    key={cat.id}
                    name={cat.name}
                    color={cat.color}
                    size="sm"
                    onRemove={() => handleCategoryToggle(cat.id)}
                  />
                ))}
              </div>
            )}
            <div className="flex flex-wrap gap-1">
              {categories
                .filter(c => !filters.categoryIds.includes(c.id))
                .map((category) => (
                  <button
                    key={category.id}
                    onClick={() => handleCategoryToggle(category.id)}
                    className="px-2 py-0.5 text-xs border rounded hover:bg-muted/50 transition-colors"
                    style={category.color ? { borderColor: category.color } : undefined}
                  >
                    {category.name}
                  </button>
                ))}
            </div>
          </div>

          {/* Tags */}
          <div className="space-y-2">
            <span className="text-sm font-medium text-muted-foreground">Tags</span>
            {selectedTags.length > 0 && (
              <div className="flex flex-wrap gap-1 mb-2">
                {selectedTags.map((tag) => (
                  <TagBadge
                    key={tag.id}
                    name={tag.name}
                    color={tag.color}
                    size="sm"
                    onRemove={() => handleTagToggle(tag.id)}
                  />
                ))}
              </div>
            )}
            <div className="flex flex-wrap gap-1 max-h-24 overflow-y-auto">
              {tags
                .filter(t => !filters.tagIds.includes(t.id))
                .map((tag) => (
                  <button
                    key={tag.id}
                    onClick={() => handleTagToggle(tag.id)}
                    className="px-2 py-0.5 text-xs bg-muted rounded hover:bg-muted/80 transition-colors"
                  >
                    {tag.name}
                  </button>
                ))}
            </div>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  )
}

// Default filter values
export const defaultSearchFilters: SearchFilterValues = {
  categoryIds: [],
  tagIds: [],
  dateFrom: null,
  dateTo: null,
  searchTranscripts: true,
}
