'use client'

import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { DefaultLlmConfig } from '@/types/chat'

export function useDefaultModel() {
  const [defaultModel, setDefaultModel] = useState<DefaultLlmConfig | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Load default model from settings
  const loadDefaultModel = useCallback(async () => {
    try {
      setIsLoading(true)
      const config = await invoke<DefaultLlmConfig | null>('llm_get_default_model')
      setDefaultModel(config)
      setError(null)
    } catch (err) {
      console.error('Failed to load default model:', err)
      setError(String(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Set default model
  const setDefault = useCallback(async (
    providerType?: string,
    modelId?: string
  ) => {
    try {
      await invoke('llm_set_default_model', { providerType, modelId })
      setDefaultModel({
        provider_type: providerType,
        model_id: modelId
      })
    } catch (err) {
      console.error('Failed to set default model:', err)
      throw err
    }
  }, [])

  // Clear default model
  const clearDefault = useCallback(async () => {
    try {
      await invoke('llm_clear_default_model')
      setDefaultModel(null)
    } catch (err) {
      console.error('Failed to clear default model:', err)
      throw err
    }
  }, [])

  // Load on mount
  useEffect(() => {
    loadDefaultModel()
  }, [loadDefaultModel])

  return {
    defaultModel,
    hasDefault: !!defaultModel?.provider_type && !!defaultModel?.model_id,
    isLoading,
    error,

    // Actions
    setDefault,
    clearDefault,
    loadDefaultModel,
  }
}
