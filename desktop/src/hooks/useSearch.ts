'use client'

import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { SearchFilters, SearchResult } from '@/types/database'

export interface SearchOptions {
  categoryIds?: string[]
  tagIds?: string[]
  dateFrom?: string
  dateTo?: string
  searchTranscripts?: boolean
}

export function useSearch() {
  const [results, setResults] = useState<SearchResult[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [lastQuery, setLastQuery] = useState<string>('')
  const [lastFilters, setLastFilters] = useState<SearchOptions>({})

  // Search recordings
  const search = useCallback(async (query: string, options: SearchOptions = {}): Promise<void> => {
    try {
      setLoading(true)
      setError(null)
      setLastQuery(query)
      setLastFilters(options)

      // Convert to snake_case for Rust backend
      const filters: SearchFilters = {
        category_ids: options.categoryIds,
        tag_ids: options.tagIds,
        date_from: options.dateFrom,
        date_to: options.dateTo,
        search_transcripts: options.searchTranscripts ?? true,
      }

      const searchResults = await invoke<SearchResult[]>('db_search_recordings', {
        query,
        filters,
      })

      setResults(searchResults)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Search failed: ${errorMessage}`)
      console.error('Search error:', err)
      setResults([])
    } finally {
      setLoading(false)
    }
  }, [])

  // Clear search results
  const clearResults = useCallback(() => {
    setResults([])
    setLastQuery('')
    setLastFilters({})
    setError(null)
  }, [])

  // Re-run last search (useful after data changes)
  const refresh = useCallback(async () => {
    if (lastQuery || Object.keys(lastFilters).length > 0) {
      await search(lastQuery, lastFilters)
    }
  }, [lastQuery, lastFilters, search])

  return {
    results,
    loading,
    error,
    search,
    clearResults,
    refresh,
    hasResults: results.length > 0,
    resultCount: results.length,
  }
}
