'use client'

import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { PromptTemplate, CreatePromptTemplate, UpdatePromptTemplate } from '@/types/templates'

export function useTemplates() {
  const [templates, setTemplates] = useState<PromptTemplate[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Load templates on mount
  useEffect(() => {
    loadTemplates()
  }, [])

  // Load all templates
  const loadTemplates = useCallback(async () => {
    try {
      setIsLoading(true)
      setError(null)
      const templateList = await invoke<PromptTemplate[]>('template_list')
      setTemplates(templateList)
    } catch (err) {
      console.error('Failed to load templates:', err)
      setError(String(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Get a single template by ID
  const getTemplate = useCallback(async (id: string): Promise<PromptTemplate | null> => {
    try {
      return await invoke<PromptTemplate | null>('template_get', { id })
    } catch (err) {
      console.error('Failed to get template:', err)
      setError(String(err))
      return null
    }
  }, [])

  // Create a new template
  const createTemplate = useCallback(async (input: CreatePromptTemplate): Promise<string | null> => {
    try {
      setError(null)
      const id = await invoke<string>('template_create', {
        name: input.name,
        prompt: input.prompt,
        description: input.description || null,
        icon: input.icon || null,
        sortOrder: input.sort_order || null,
      })
      await loadTemplates() // Refresh list
      return id
    } catch (err) {
      console.error('Failed to create template:', err)
      setError(String(err))
      return null
    }
  }, [loadTemplates])

  // Update an existing template
  const updateTemplate = useCallback(async (id: string, input: UpdatePromptTemplate): Promise<boolean> => {
    try {
      setError(null)
      await invoke('template_update', {
        id,
        name: input.name || null,
        prompt: input.prompt || null,
        description: input.description || null,
        icon: input.icon || null,
        sortOrder: input.sort_order || null,
      })
      await loadTemplates() // Refresh list
      return true
    } catch (err) {
      console.error('Failed to update template:', err)
      setError(String(err))
      return false
    }
  }, [loadTemplates])

  // Delete a template
  const deleteTemplate = useCallback(async (id: string): Promise<boolean> => {
    try {
      setError(null)
      await invoke('template_delete', { id })
      await loadTemplates() // Refresh list
      return true
    } catch (err) {
      console.error('Failed to delete template:', err)
      setError(String(err))
      return false
    }
  }, [loadTemplates])

  // Duplicate a template
  const duplicateTemplate = useCallback(async (id: string): Promise<string | null> => {
    try {
      setError(null)
      const newId = await invoke<string>('template_duplicate', { id })
      await loadTemplates() // Refresh list
      return newId
    } catch (err) {
      console.error('Failed to duplicate template:', err)
      setError(String(err))
      return null
    }
  }, [loadTemplates])

  // Helper: Get built-in templates only
  const builtinTemplates = templates.filter(t => t.is_builtin)

  // Helper: Get custom templates only
  const customTemplates = templates.filter(t => !t.is_builtin)

  return {
    templates,
    builtinTemplates,
    customTemplates,
    isLoading,
    error,
    loadTemplates,
    getTemplate,
    createTemplate,
    updateTemplate,
    deleteTemplate,
    duplicateTemplate,
  }
}
