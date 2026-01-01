'use client'

import { useState, useCallback } from 'react'
import { Check, Plus, ChevronDown, Tag, Folder, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { CategoryBadge, TagBadge } from './CategoryBadge'
import { useCategories } from '@/hooks/useCategories'
import type { Category, Tag as TagType } from '@/types/database'
import { cn } from '@/lib/utils'

interface CategoryTagSelectorProps {
  recordingId: string
  selectedCategories: Category[]
  selectedTags: TagType[]
  onCategoryChange?: (categories: Category[]) => void
  onTagChange?: (tags: TagType[]) => void
  className?: string
}

export function CategoryTagSelector({
  recordingId,
  selectedCategories,
  selectedTags,
  onCategoryChange,
  onTagChange,
  className,
}: CategoryTagSelectorProps) {
  const [open, setOpen] = useState(false)
  const [newTagName, setNewTagName] = useState('')
  const [newCategoryName, setNewCategoryName] = useState('')
  const [deleteConfirm, setDeleteConfirm] = useState<{ type: 'category' | 'tag', id: string, name: string } | null>(null)

  const {
    categories,
    tags,
    assignCategory,
    removeCategory,
    assignTag,
    removeTag,
    getOrCreateTag,
    createCategory,
    deleteCategory,
    deleteTag,
  } = useCategories()

  const handleCategoryToggle = useCallback(async (category: Category) => {
    const isSelected = selectedCategories.some(c => c.id === category.id)

    try {
      if (isSelected) {
        await removeCategory(recordingId, category.id)
        onCategoryChange?.(selectedCategories.filter(c => c.id !== category.id))
      } else {
        await assignCategory(recordingId, category.id)
        onCategoryChange?.([...selectedCategories, category])
      }
    } catch (err) {
      console.error('Failed to toggle category:', err)
    }
  }, [recordingId, selectedCategories, assignCategory, removeCategory, onCategoryChange])

  const handleCreateCategory = useCallback(async () => {
    if (!newCategoryName.trim()) return

    try {
      const categoryId = await createCategory(newCategoryName.trim())
      if (categoryId) {
        // Assign the new category to the recording
        await assignCategory(recordingId, categoryId)
        const newCategory: Category = {
          id: categoryId,
          name: newCategoryName.trim(),
          color: null,
          is_system: false,
        }
        onCategoryChange?.([...selectedCategories, newCategory])
      }
      setNewCategoryName('')
    } catch (err) {
      console.error('Failed to create category:', err)
    }
  }, [recordingId, newCategoryName, createCategory, assignCategory, selectedCategories, onCategoryChange])

  const handleTagToggle = useCallback(async (tag: TagType) => {
    const isSelected = selectedTags.some(t => t.id === tag.id)

    try {
      if (isSelected) {
        await removeTag(recordingId, tag.id)
        onTagChange?.(selectedTags.filter(t => t.id !== tag.id))
      } else {
        await assignTag(recordingId, tag.id)
        onTagChange?.([...selectedTags, tag])
      }
    } catch (err) {
      console.error('Failed to toggle tag:', err)
    }
  }, [recordingId, selectedTags, assignTag, removeTag, onTagChange])

  const handleCreateTag = useCallback(async () => {
    if (!newTagName.trim()) return

    try {
      const tagId = await getOrCreateTag(newTagName.trim())
      if (tagId) {
        await assignTag(recordingId, tagId)
        const newTag: TagType = {
          id: tagId,
          name: newTagName.trim(),
          color: null,
          usage_count: 1,
        }
        onTagChange?.([...selectedTags, newTag])
      }
      setNewTagName('')
    } catch (err) {
      console.error('Failed to create tag:', err)
    }
  }, [recordingId, newTagName, getOrCreateTag, assignTag, selectedTags, onTagChange])

  const handleConfirmDelete = useCallback(async () => {
    if (!deleteConfirm) return

    try {
      if (deleteConfirm.type === 'category') {
        await deleteCategory(deleteConfirm.id)
        // Remove from selected if it was selected
        onCategoryChange?.(selectedCategories.filter(c => c.id !== deleteConfirm.id))
      } else {
        await deleteTag(deleteConfirm.id)
        // Remove from selected if it was selected
        onTagChange?.(selectedTags.filter(t => t.id !== deleteConfirm.id))
      }
    } catch (err) {
      console.error('Failed to delete:', err)
    } finally {
      setDeleteConfirm(null)
    }
  }, [deleteConfirm, deleteCategory, deleteTag, selectedCategories, selectedTags, onCategoryChange, onTagChange])

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          className={cn('h-8 gap-1', className)}
        >
          <Tag className="h-3.5 w-3.5" />
          Organize
          <ChevronDown className="h-3 w-3 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-72 p-0 max-h-[400px] overflow-y-auto" align="start">
        {/* Categories Section */}
        <div className="p-3 border-b border-border">
          <div className="flex items-center gap-2 mb-2 text-xs font-medium text-muted-foreground">
            <Folder className="h-3.5 w-3.5" />
            Categories
          </div>
          <div className="space-y-1">
            {categories.map((category) => {
              const isSelected = selectedCategories.some(c => c.id === category.id)
              return (
                <div key={category.id} className="flex items-center group">
                  <button
                    onClick={() => handleCategoryToggle(category)}
                    className={cn(
                      'flex-1 flex items-center gap-2 px-2 py-1.5 rounded-l text-sm hover:bg-muted/50 transition-colors',
                      isSelected && 'bg-muted'
                    )}
                  >
                    <div
                      className={cn(
                        'h-4 w-4 rounded border flex items-center justify-center',
                        isSelected ? 'bg-primary border-primary' : 'border-border'
                      )}
                      style={category.color && isSelected ? {
                        backgroundColor: category.color,
                        borderColor: category.color,
                      } : category.color ? {
                        borderColor: category.color,
                      } : undefined}
                    >
                      {isSelected && <Check className="h-3 w-3 text-primary-foreground" />}
                    </div>
                    <span className="flex-1 text-left truncate">{category.name}</span>
                  </button>
                  {!category.is_system && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        setDeleteConfirm({ type: 'category', id: category.id, name: category.name })
                      }}
                      className="p-1.5 opacity-0 group-hover:opacity-100 hover:bg-destructive/10 hover:text-destructive rounded-r transition-all"
                      title="Delete category"
                    >
                      <Trash2 className="h-3 w-3" />
                    </button>
                  )}
                </div>
              )
            })}
          </div>
          {/* Create New Category */}
          <div className="flex gap-1 mt-2">
            <Input
              placeholder="New category..."
              value={newCategoryName}
              onChange={(e) => setNewCategoryName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault()
                  handleCreateCategory()
                }
              }}
              className="h-7 text-sm"
            />
            <Button
              size="sm"
              variant="ghost"
              className="h-7 px-2"
              onClick={handleCreateCategory}
              disabled={!newCategoryName.trim()}
            >
              <Plus className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>

        {/* Tags Section */}
        <div className="p-3">
          <div className="flex items-center gap-2 mb-2 text-xs font-medium text-muted-foreground">
            <Tag className="h-3.5 w-3.5" />
            Tags
          </div>

          {/* Selected Tags */}
          {selectedTags.length > 0 && (
            <div className="flex flex-wrap gap-1 mb-2">
              {selectedTags.map((tag) => (
                <TagBadge
                  key={tag.id}
                  name={tag.name}
                  color={tag.color}
                  size="sm"
                  onRemove={() => handleTagToggle(tag)}
                />
              ))}
            </div>
          )}

          {/* Available Tags */}
          <div className="space-y-1 mb-2">
            {tags
              .filter(t => !selectedTags.some(st => st.id === t.id))
              .map((tag) => (
                <div key={tag.id} className="flex items-center group">
                  <button
                    onClick={() => handleTagToggle(tag)}
                    className="flex-1 flex items-center gap-2 px-2 py-1 rounded-l text-sm hover:bg-muted/50 transition-colors"
                  >
                    <Plus className="h-3 w-3 text-muted-foreground" />
                    <span className="flex-1 text-left truncate">{tag.name}</span>
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation()
                      setDeleteConfirm({ type: 'tag', id: tag.id, name: tag.name })
                    }}
                    className="p-1 opacity-0 group-hover:opacity-100 hover:bg-destructive/10 hover:text-destructive rounded-r transition-all"
                    title="Delete tag"
                  >
                    <Trash2 className="h-3 w-3" />
                  </button>
                </div>
              ))}
          </div>

          {/* Create New Tag */}
          <div className="flex gap-1">
            <Input
              placeholder="New tag..."
              value={newTagName}
              onChange={(e) => setNewTagName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault()
                  handleCreateTag()
                }
              }}
              className="h-7 text-sm"
            />
            <Button
              size="sm"
              variant="ghost"
              className="h-7 px-2"
              onClick={handleCreateTag}
              disabled={!newTagName.trim()}
            >
              <Plus className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      </PopoverContent>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteConfirm !== null} onOpenChange={(open) => !open && setDeleteConfirm(null)}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Delete {deleteConfirm?.type === 'category' ? 'Category' : 'Tag'}</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete &quot;{deleteConfirm?.name}&quot;?
              This will remove it from all recordings. This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="gap-2 sm:gap-0">
            <Button variant="outline" onClick={() => setDeleteConfirm(null)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleConfirmDelete}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Popover>
  )
}
