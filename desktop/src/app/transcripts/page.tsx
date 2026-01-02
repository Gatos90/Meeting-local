'use client'

import { useState, useMemo, useCallback } from 'react'
import { useRouter } from 'next/navigation'
import { invoke } from '@tauri-apps/api/core'
import { Search, Filter, FileText, Loader2 } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { RecordingCard, formatDate, formatDuration } from '@/components/recording-card'
import { useRecordings } from '@/hooks/useRecordings'
import { useCategories } from '@/hooks/useCategories'
import { useSearch } from '@/hooks/useSearch'
import type { Category, Tag } from '@/types/database'

export default function TranscriptsPage() {
  const router = useRouter()
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedCategoryId, setSelectedCategoryId] = useState<string | null>(null)

  // Load data from database
  const { recordings, setRecordings, loading: recordingsLoading, error: recordingsError } = useRecordings()
  const { categories, loading: categoriesLoading } = useCategories()
  const { results: searchResults, loading: searchLoading, search } = useSearch()

  // Handle title change for a recording
  const handleTitleChange = useCallback((recordingId: string, newTitle: string) => {
    setRecordings(prev => prev.map(item =>
      item.recording.id === recordingId
        ? { ...item, recording: { ...item.recording, title: newTitle } }
        : item
    ))
  }, [setRecordings])

  // Handle categories change for a recording
  const handleCategoriesChange = useCallback((recordingId: string, newCategories: Category[]) => {
    setRecordings(prev => prev.map(item =>
      item.recording.id === recordingId
        ? { ...item, categories: newCategories }
        : item
    ))
  }, [setRecordings])

  // Handle tags change for a recording
  const handleTagsChange = useCallback((recordingId: string, newTags: Tag[]) => {
    setRecordings(prev => prev.map(item =>
      item.recording.id === recordingId
        ? { ...item, tags: newTags }
        : item
    ))
  }, [setRecordings])

  // Handle delete recording
  const handleDelete = useCallback(async (recordingId: string) => {
    try {
      await invoke('db_delete_recording', { id: recordingId })
      // Remove from local state
      setRecordings(prev => prev.filter(item => item.recording.id !== recordingId))
      console.log('Deleted recording:', recordingId)
    } catch (err) {
      console.error('Failed to delete recording:', err)
    }
  }, [setRecordings])

  // Determine if we're searching or just filtering
  const isSearching = searchQuery.length > 0

  // Handle search input change
  const handleSearchChange = async (query: string) => {
    setSearchQuery(query)
    if (query.length > 0) {
      await search(query, {
        categoryIds: selectedCategoryId ? [selectedCategoryId] : undefined,
        searchTranscripts: true,
      })
    }
  }

  // Handle category filter change
  const handleCategoryChange = async (categoryId: string | null) => {
    setSelectedCategoryId(categoryId)
    if (searchQuery.length > 0) {
      await search(searchQuery, {
        categoryIds: categoryId ? [categoryId] : undefined,
        searchTranscripts: true,
      })
    }
  }

  // Filter recordings locally when not searching
  const filteredRecordings = useMemo(() => {
    if (isSearching) {
      return searchResults.map(result => ({
        recording: result.recording,
        categories: result.categories,
        tags: result.tags,
        transcript_count: 0, // Search results don't include transcript count
        matched_text: result.matched_text,
      }))
    }

    return recordings.filter((item) => {
      // Filter by category if selected
      if (selectedCategoryId) {
        const hasCategory = item.categories.some(c => c.id === selectedCategoryId)
        if (!hasCategory) return false
      }
      return true
    })
  }, [isSearching, searchResults, recordings, selectedCategoryId])

  const isLoading = recordingsLoading || categoriesLoading || searchLoading
  const totalCount = isSearching ? searchResults.length : recordings.length

  return (
    <>
      {/* Header */}
      <header className="flex h-16 items-center justify-between border-b border-border bg-background/50 backdrop-blur-sm px-8">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-semibold text-foreground">Transcripts</h1>
          <Badge variant="secondary">
            {isLoading ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              `${totalCount} recordings`
            )}
          </Badge>
        </div>

        <div className="flex items-center gap-4">
          <div className="relative w-64">
            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search transcripts..."
              value={searchQuery}
              onChange={(e) => handleSearchChange(e.target.value)}
              className="pl-9 bg-muted/50 border-transparent focus:bg-background transition-all"
            />
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 overflow-y-auto p-8">
        <div className="mx-auto max-w-6xl space-y-6">
          {/* Filter Categories */}
          <div className="flex items-center gap-2 flex-wrap">
            <Filter className="h-4 w-4 text-muted-foreground" />
            <Button
              variant={selectedCategoryId === null ? 'default' : 'outline'}
              size="sm"
              onClick={() => handleCategoryChange(null)}
              className="h-7"
            >
              All
            </Button>
            {categories.map((category) => (
              <Button
                key={category.id}
                variant={selectedCategoryId === category.id ? 'default' : 'outline'}
                size="sm"
                onClick={() => handleCategoryChange(category.id)}
                className="h-7"
                style={category.color && selectedCategoryId !== category.id ? {
                  borderColor: category.color,
                } : undefined}
              >
                {category.name}
              </Button>
            ))}
          </div>

          {/* Error State */}
          {recordingsError && (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <div className="p-4 rounded-full bg-destructive/10 mb-4">
                <FileText className="h-8 w-8 text-destructive" />
              </div>
              <h3 className="text-lg font-medium text-foreground">
                Error loading recordings
              </h3>
              <p className="text-muted-foreground mt-1">
                {recordingsError}
              </p>
            </div>
          )}

          {/* Loading State */}
          {isLoading && !recordingsError && (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mb-4" />
              <p className="text-muted-foreground">Loading recordings...</p>
            </div>
          )}

          {/* Empty State */}
          {!isLoading && !recordingsError && filteredRecordings.length === 0 && (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <div className="p-4 rounded-full bg-muted mb-4">
                <FileText className="h-8 w-8 text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium text-foreground">
                {isSearching ? 'No results found' : 'No recordings yet'}
              </h3>
              <p className="text-muted-foreground mt-1">
                {isSearching
                  ? 'Try adjusting your search or filter criteria'
                  : 'Start a new recording to see it here'}
              </p>
            </div>
          )}

          {/* Results */}
          {!isLoading && !recordingsError && filteredRecordings.length > 0 && (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              {filteredRecordings.map((item) => (
                <RecordingCard
                  key={item.recording.id}
                  id={item.recording.id}
                  title={item.recording.title}
                  date={formatDate(item.recording.created_at)}
                  duration={formatDuration(item.recording.duration_seconds)}
                  categories={item.categories}
                  tags={item.tags}
                  transcriptCount={item.transcript_count}
                  onClick={() => {
                    router.push(`/transcripts/view?id=${item.recording.id}`)
                  }}
                  onTitleChange={(newTitle) => handleTitleChange(item.recording.id, newTitle)}
                  onCategoriesChange={(newCategories) => handleCategoriesChange(item.recording.id, newCategories)}
                  onTagsChange={(newTags) => handleTagsChange(item.recording.id, newTags)}
                  onDelete={() => handleDelete(item.recording.id)}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </>
  )
}
