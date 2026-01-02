'use client'

import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Category, Tag } from '@/types/database'

export function useCategories() {
  const [categories, setCategories] = useState<Category[]>([])
  const [tags, setTags] = useState<Tag[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Fetch all categories
  const fetchCategories = useCallback(async () => {
    try {
      const result = await invoke<Category[]>('db_get_all_categories')
      setCategories(result)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      console.error('Fetch categories error:', err)
      throw new Error(`Failed to fetch categories: ${errorMessage}`)
    }
  }, [])

  // Fetch all tags
  const fetchTags = useCallback(async () => {
    try {
      const result = await invoke<Tag[]>('db_get_all_tags')
      setTags(result)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      console.error('Fetch tags error:', err)
      throw new Error(`Failed to fetch tags: ${errorMessage}`)
    }
  }, [])

  // Fetch both categories and tags
  const fetchAll = useCallback(async () => {
    try {
      setLoading(true)
      setError(null)
      await Promise.all([fetchCategories(), fetchTags()])
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(errorMessage)
    } finally {
      setLoading(false)
    }
  }, [fetchCategories, fetchTags])

  useEffect(() => {
    fetchAll()
  }, [fetchAll])

  // Create a new category
  const createCategory = useCallback(async (name: string, color?: string): Promise<string | null> => {
    try {
      setError(null)
      const id = await invoke<string>('db_create_category', { name, color })

      // Refresh categories
      await fetchCategories()
      return id
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to create category: ${errorMessage}`)
      console.error('Create category error:', err)
      return null
    }
  }, [fetchCategories])

  // Create a new tag
  const createTag = useCallback(async (name: string, color?: string): Promise<string | null> => {
    try {
      setError(null)
      const id = await invoke<string>('db_create_tag', { name, color })

      // Refresh tags
      await fetchTags()
      return id
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to create tag: ${errorMessage}`)
      console.error('Create tag error:', err)
      return null
    }
  }, [fetchTags])

  // Get or create a tag by name
  const getOrCreateTag = useCallback(async (name: string): Promise<string | null> => {
    try {
      setError(null)
      const id = await invoke<string>('db_get_or_create_tag', { name })

      // Refresh tags
      await fetchTags()
      return id
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to get or create tag: ${errorMessage}`)
      console.error('Get or create tag error:', err)
      return null
    }
  }, [fetchTags])

  // Assign a category to a recording
  const assignCategory = useCallback(async (recordingId: string, categoryId: string): Promise<void> => {
    try {
      setError(null)
      await invoke('db_assign_category', { recordingId, categoryId })
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to assign category: ${errorMessage}`)
      console.error('Assign category error:', err)
      throw err
    }
  }, [])

  // Remove a category from a recording
  const removeCategory = useCallback(async (recordingId: string, categoryId: string): Promise<void> => {
    try {
      setError(null)
      await invoke('db_remove_category', { recordingId, categoryId })
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to remove category: ${errorMessage}`)
      console.error('Remove category error:', err)
      throw err
    }
  }, [])

  // Assign a tag to a recording
  const assignTag = useCallback(async (recordingId: string, tagId: string): Promise<void> => {
    try {
      setError(null)
      await invoke('db_assign_tag', { recordingId, tagId })
      // Refresh tags to update usage count
      await fetchTags()
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to assign tag: ${errorMessage}`)
      console.error('Assign tag error:', err)
      throw err
    }
  }, [fetchTags])

  // Remove a tag from a recording
  const removeTag = useCallback(async (recordingId: string, tagId: string): Promise<void> => {
    try {
      setError(null)
      await invoke('db_remove_tag', { recordingId, tagId })
      // Refresh tags to update usage count
      await fetchTags()
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to remove tag: ${errorMessage}`)
      console.error('Remove tag error:', err)
      throw err
    }
  }, [fetchTags])

  // Delete a category completely
  const deleteCategory = useCallback(async (categoryId: string): Promise<void> => {
    try {
      setError(null)
      await invoke('db_delete_category', { categoryId })
      // Refresh categories
      await fetchCategories()
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to delete category: ${errorMessage}`)
      console.error('Delete category error:', err)
      throw err
    }
  }, [fetchCategories])

  // Delete a tag completely
  const deleteTag = useCallback(async (tagId: string): Promise<void> => {
    try {
      setError(null)
      await invoke('db_delete_tag', { tagId })
      // Refresh tags
      await fetchTags()
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to delete tag: ${errorMessage}`)
      console.error('Delete tag error:', err)
      throw err
    }
  }, [fetchTags])

  return {
    categories,
    tags,
    loading,
    error,
    refresh: fetchAll,
    refreshCategories: fetchCategories,
    refreshTags: fetchTags,
    createCategory,
    createTag,
    getOrCreateTag,
    assignCategory,
    removeCategory,
    deleteCategory,
    assignTag,
    removeTag,
    deleteTag,
  }
}
