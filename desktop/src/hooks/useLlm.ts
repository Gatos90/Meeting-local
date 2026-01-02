'use client'

import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'

// Download progress info from backend
export interface DownloadProgress {
  model_id: string
  downloaded_bytes: number
  total_bytes: number
  percent: number
  status: 'Pending' | 'Downloading' | 'Verifying' | 'Complete' | { Failed: string }
}

// Provider types matching Rust enums
export type ProviderType = 'embedded' | 'ollama' | 'openai' | 'claude'

// Provider capabilities
export interface ProviderCapabilities {
  streaming: boolean
  chat: boolean
  function_calling: boolean
  vision: boolean
  embedded: boolean
  requires_api_key: boolean
  supports_download: boolean
}

// Provider info
export interface ProviderInfo {
  provider_type: ProviderType
  name: string
  capabilities: ProviderCapabilities
  is_available: boolean
}

// Model info
export interface LlmModelInfo {
  id: string
  name: string
  description: string | null
  size_bytes: number | null
  is_local: boolean
  is_loaded: boolean
  context_length: number | null
  provider: string
  /** Whether this model has native function calling support */
  has_native_tool_support?: boolean
}

// Message for chat
export interface Message {
  role: 'system' | 'user' | 'assistant'
  content: string
}

// Completion request input
export interface CompletionRequestInput {
  messages: Message[]
  max_tokens?: number
  temperature?: number
  stream?: boolean
}

// Completion response
export interface CompletionResponse {
  content: string
  model: string
  prompt_tokens: number | null
  completion_tokens: number | null
  truncated: boolean
  finish_reason: string | null
}

// Downloadable model info
export interface DownloadableModel {
  id: string
  name: string
  description: string
  size_bytes: number
  url: string
  sha256: string | null
  context_length: number
  recommended_for: string[]
}

// Local model info (detailed)
export interface LocalModelInfo {
  id: string
  name: string
  size_bytes: number
  is_curated: boolean
  description: string | null
  context_length: number | null
  /** Whether this model has native function calling support */
  has_native_tool_support: boolean
}

// Model configuration
export interface ModelConfig {
  model_id: string
  has_native_tool_support: boolean
  created_at: string
  updated_at: string
}

export function useLlm() {
  // Provider state
  const [providers, setProviders] = useState<ProviderInfo[]>([])
  const [activeProvider, setActiveProvider] = useState<ProviderType | null>(null)
  const [isProviderReady, setIsProviderReady] = useState(false)

  // Model state
  const [models, setModels] = useState<LlmModelInfo[]>([])
  const [currentModel, setCurrentModel] = useState<string | null>(null)
  const [downloadableModels, setDownloadableModels] = useState<DownloadableModel[]>([])
  const [localModels, setLocalModels] = useState<string[]>([])
  const [localModelsInfo, setLocalModelsInfo] = useState<LocalModelInfo[]>([])

  // Ollama state
  const [ollamaConnected, setOllamaConnected] = useState(false)

  // Loading/processing state
  const [isLoading, setIsLoading] = useState(true)
  const [isProcessing, setIsProcessing] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Streaming state
  const [streamingContent, setStreamingContent] = useState<string>('')
  const streamListenerRef = useRef<UnlistenFn | null>(null)

  // Download state
  const [downloadProgress, setDownloadProgress] = useState<Record<string, DownloadProgress>>({})
  const [downloadingModelId, setDownloadingModelId] = useState<string | null>(null)
  const downloadListenersRef = useRef<UnlistenFn[]>([])

  // Load initial data
  useEffect(() => {
    loadProviders()
    loadDownloadableModels()
    loadLocalModels()
    loadLocalModelsInfo()
  }, [])

  // Set up download event listeners
  useEffect(() => {
    const setupListeners = async () => {
      // Progress listener
      const unlistenProgress = await listen<DownloadProgress>('llm-download-progress', (event) => {
        setDownloadProgress(prev => ({
          ...prev,
          [event.payload.model_id]: event.payload
        }))
      })

      // Complete listener
      const unlistenComplete = await listen<{ model_id: string }>('llm-download-complete', async (event) => {
        setDownloadingModelId(null)
        setDownloadProgress(prev => {
          const updated = { ...prev }
          delete updated[event.payload.model_id]
          return updated
        })
        // Refresh local models lists - call invoke directly to avoid stale closure
        try {
          const models = await invoke<string[]>('llm_get_local_models')
          setLocalModels(models)
          const modelsInfo = await invoke<LocalModelInfo[]>('llm_get_local_models_info')
          setLocalModelsInfo(modelsInfo)
        } catch (err) {
          console.error('Failed to refresh local models after download:', err)
        }
      })

      // Error listener
      const unlistenError = await listen<{ model_id: string; error: string }>('llm-download-error', (event) => {
        setDownloadingModelId(null)
        setDownloadProgress(prev => ({
          ...prev,
          [event.payload.model_id]: {
            ...prev[event.payload.model_id],
            status: { Failed: event.payload.error }
          }
        }))
      })

      downloadListenersRef.current = [unlistenProgress, unlistenComplete, unlistenError]
    }

    setupListeners()

    return () => {
      downloadListenersRef.current.forEach(unlisten => unlisten())
      downloadListenersRef.current = []
    }
  }, [])

  // Check Ollama connection when providers load
  useEffect(() => {
    checkOllamaConnection()
  }, [providers])

  // Load available providers
  const loadProviders = useCallback(async () => {
    try {
      setIsLoading(true)
      const [providerList, active] = await Promise.all([
        invoke<ProviderInfo[]>('llm_get_providers'),
        invoke<ProviderType | null>('llm_get_active_provider'),
      ])
      setProviders(providerList)
      setActiveProvider(active)
    } catch (err) {
      console.error('Failed to load LLM providers:', err)
      setError(String(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Check if Ollama is running
  const checkOllamaConnection = useCallback(async () => {
    try {
      await invoke<string>('llm_ollama_check_connection')
      setOllamaConnected(true)
      return true
    } catch (err) {
      setOllamaConnected(false)
      return false
    }
  }, [])

  // Set active provider
  const selectProvider = useCallback(async (providerType: ProviderType) => {
    try {
      setError(null)
      await invoke('llm_set_active_provider', { providerType })
      setActiveProvider(providerType)

      // Load models for this provider
      await loadModelsForProvider(providerType)
    } catch (err) {
      console.error('Failed to set active provider:', err)
      setError(String(err))
      throw err
    }
  }, [])

  // Load models for the active provider
  const loadModels = useCallback(async () => {
    try {
      const modelList = await invoke<LlmModelInfo[]>('llm_list_models')
      setModels(modelList)
    } catch (err) {
      console.error('Failed to load models:', err)
      // Don't set error - might just be no active provider
    }
  }, [])

  // Load models for a specific provider
  const loadModelsForProvider = useCallback(async (providerType: ProviderType) => {
    try {
      const modelList = await invoke<LlmModelInfo[]>('llm_list_models_for_provider', { providerType })
      setModels(modelList)
    } catch (err) {
      console.error(`Failed to load models for ${providerType}:`, err)
      setModels([])
    }
  }, [])

  // Initialize with a model
  const initializeModel = useCallback(async (modelId: string) => {
    try {
      setError(null)
      setIsLoading(true)
      await invoke('llm_initialize', { modelId })
      setCurrentModel(modelId)
      setIsProviderReady(true)
    } catch (err) {
      console.error('Failed to initialize model:', err)
      setError(String(err))
      setIsProviderReady(false)
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Check if ready
  const checkReady = useCallback(async () => {
    try {
      const ready = await invoke<boolean>('llm_is_ready')
      setIsProviderReady(ready)
      if (ready) {
        const model = await invoke<string | null>('llm_current_model')
        setCurrentModel(model)
      }
      return ready
    } catch (err) {
      setIsProviderReady(false)
      return false
    }
  }, [])

  // Load downloadable models (for embedded provider)
  const loadDownloadableModels = useCallback(async () => {
    try {
      const models = await invoke<DownloadableModel[]>('llm_get_downloadable_models')
      setDownloadableModels(models)
    } catch (err) {
      console.error('Failed to load downloadable models:', err)
    }
  }, [])

  // Load local models
  const loadLocalModels = useCallback(async () => {
    try {
      const models = await invoke<string[]>('llm_get_local_models')
      setLocalModels(models)
    } catch (err) {
      console.error('Failed to load local models:', err)
    }
  }, [])

  // Load detailed local models info
  const loadLocalModelsInfo = useCallback(async () => {
    try {
      const models = await invoke<LocalModelInfo[]>('llm_get_local_models_info')
      setLocalModelsInfo(models)
    } catch (err) {
      console.error('Failed to load local models info:', err)
    }
  }, [])

  // Check if a model is downloaded
  const isModelDownloaded = useCallback(async (modelId: string): Promise<boolean> => {
    try {
      return await invoke<boolean>('llm_is_model_downloaded', { modelId })
    } catch (err) {
      console.error('Failed to check if model is downloaded:', err)
      return false
    }
  }, [])

  // Delete a downloaded model
  const deleteModel = useCallback(async (modelId: string) => {
    try {
      await invoke('llm_delete_model', { modelId })
      await loadLocalModels()
      await loadLocalModelsInfo()
    } catch (err) {
      console.error('Failed to delete model:', err)
      throw err
    }
  }, [loadLocalModels, loadLocalModelsInfo])

  // Download a model
  const downloadModel = useCallback(async (modelId: string) => {
    try {
      setError(null)
      setDownloadingModelId(modelId)
      setDownloadProgress(prev => ({
        ...prev,
        [modelId]: {
          model_id: modelId,
          downloaded_bytes: 0,
          total_bytes: 0,
          percent: 0,
          status: 'Pending'
        }
      }))
      await invoke('llm_download_model', { modelId })
    } catch (err) {
      console.error('Failed to start download:', err)
      setError(String(err))
      setDownloadingModelId(null)
      throw err
    }
  }, [])

  // Download a custom model from URL
  const downloadCustomModel = useCallback(async (name: string, url: string) => {
    try {
      setError(null)
      // Sanitize name to get model_id (same logic as backend)
      const modelId = name
        .split('')
        .map(c => /[a-zA-Z0-9_-]/.test(c) ? c : '-')
        .join('')
        .toLowerCase()

      setDownloadingModelId(modelId)
      setDownloadProgress(prev => ({
        ...prev,
        [modelId]: {
          model_id: modelId,
          downloaded_bytes: 0,
          total_bytes: 0,
          percent: 0,
          status: 'Pending'
        }
      }))
      await invoke('llm_download_custom_model', { name, url })
    } catch (err) {
      console.error('Failed to start custom download:', err)
      setError(String(err))
      setDownloadingModelId(null)
      throw err
    }
  }, [])

  // Cancel a download
  const cancelDownload = useCallback(async (modelId: string) => {
    try {
      await invoke('llm_cancel_download', { modelId })
      setDownloadingModelId(null)
      setDownloadProgress(prev => {
        const updated = { ...prev }
        delete updated[modelId]
        return updated
      })
    } catch (err) {
      console.error('Failed to cancel download:', err)
      throw err
    }
  }, [])

  // Get model tool support override (returns null if no user override)
  const getModelToolSupport = useCallback(async (modelId: string): Promise<boolean | null> => {
    try {
      return await invoke<boolean | null>('llm_get_model_tool_support', { modelId })
    } catch (err) {
      console.error('Failed to get model tool support:', err)
      return null
    }
  }, [])

  // Set model tool support override
  const setModelToolSupport = useCallback(async (modelId: string, hasNativeToolSupport: boolean) => {
    try {
      await invoke('llm_set_model_tool_support', { modelId, hasNativeToolSupport })
      // Refresh local models info to get updated tool support values
      await loadLocalModelsInfo()
    } catch (err) {
      console.error('Failed to set model tool support:', err)
      throw err
    }
  }, [loadLocalModelsInfo])

  // Delete model tool support override (revert to default)
  const deleteModelToolSupport = useCallback(async (modelId: string) => {
    try {
      await invoke('llm_delete_model_tool_support', { modelId })
      // Refresh local models info to get updated tool support values
      await loadLocalModelsInfo()
    } catch (err) {
      console.error('Failed to delete model tool support:', err)
      throw err
    }
  }, [loadLocalModelsInfo])

  // Get effective tool support for a model (checking user config, registry, and fallback)
  const getEffectiveToolSupport = useCallback(async (modelId: string): Promise<boolean> => {
    try {
      return await invoke<boolean>('llm_get_effective_tool_support', { modelId })
    } catch (err) {
      console.error('Failed to get effective tool support:', err)
      return false
    }
  }, [])

  // Get all model configs
  const getAllModelConfigs = useCallback(async (): Promise<ModelConfig[]> => {
    try {
      return await invoke<ModelConfig[]>('llm_get_all_model_configs')
    } catch (err) {
      console.error('Failed to get all model configs:', err)
      return []
    }
  }, [])

  // Run completion (non-streaming)
  const complete = useCallback(async (request: CompletionRequestInput): Promise<CompletionResponse> => {
    try {
      setError(null)
      setIsProcessing(true)
      const response = await invoke<CompletionResponse>('llm_complete', { request })
      return response
    } catch (err) {
      console.error('Failed to complete:', err)
      setError(String(err))
      throw err
    } finally {
      setIsProcessing(false)
    }
  }, [])

  // Run streaming completion
  const completeStreaming = useCallback(async (
    request: CompletionRequestInput,
    onToken?: (token: string) => void
  ): Promise<CompletionResponse> => {
    try {
      setError(null)
      setIsProcessing(true)
      setStreamingContent('')

      // Generate unique event ID
      const eventId = `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`

      // Set up stream listener
      const unlisten = await listen<string>(`llm-stream-${eventId}`, (event) => {
        setStreamingContent(prev => prev + event.payload)
        onToken?.(event.payload)
      })
      streamListenerRef.current = unlisten

      // Set up completion listener
      const completionPromise = new Promise<CompletionResponse>((resolve, reject) => {
        listen<CompletionResponse>(`llm-stream-${eventId}-complete`, (event) => {
          resolve(event.payload)
        }).catch(reject)
      })

      // Start streaming request
      const response = await invoke<CompletionResponse>('llm_complete_streaming', {
        request: { ...request, stream: true },
        eventId
      })

      // Clean up listener
      if (streamListenerRef.current) {
        streamListenerRef.current()
        streamListenerRef.current = null
      }

      return response
    } catch (err) {
      console.error('Failed to complete streaming:', err)
      setError(String(err))
      throw err
    } finally {
      setIsProcessing(false)
    }
  }, [])

  // Cancel streaming
  const cancelStreaming = useCallback(() => {
    if (streamListenerRef.current) {
      streamListenerRef.current()
      streamListenerRef.current = null
    }
    setIsProcessing(false)
    setStreamingContent('')
  }, [])

  // Helper: Summarize transcript
  const summarizeTranscript = useCallback(async (
    transcript: string,
    options?: {
      style?: 'brief' | 'detailed' | 'bullet-points'
      maxLength?: number
    }
  ): Promise<string> => {
    const style = options?.style || 'brief'
    const systemPrompt = style === 'bullet-points'
      ? 'You are a helpful assistant that summarizes meeting transcripts. Create a clear, organized bullet-point summary of the key points, decisions, and action items from the transcript.'
      : style === 'detailed'
      ? 'You are a helpful assistant that summarizes meeting transcripts. Create a comprehensive summary that captures all important details, context, and nuances from the transcript.'
      : 'You are a helpful assistant that summarizes meeting transcripts. Create a brief, concise summary of the main points from the transcript.'

    const response = await complete({
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: `Please summarize the following transcript:\n\n${transcript}` }
      ],
      max_tokens: options?.maxLength || 1000,
      temperature: 0.3,
    })

    return response.content
  }, [complete])

  // Helper: Chat about transcript
  const chatAboutTranscript = useCallback(async (
    transcript: string,
    userMessage: string,
    previousMessages: Message[] = []
  ): Promise<string> => {
    const systemPrompt = `You are a helpful assistant discussing a meeting transcript. Use the transcript context to answer questions and provide insights. Be helpful and accurate.

Here is the transcript:
${transcript}`

    const messages: Message[] = [
      { role: 'system', content: systemPrompt },
      ...previousMessages,
      { role: 'user', content: userMessage }
    ]

    const response = await complete({
      messages,
      temperature: 0.7,
    })

    return response.content
  }, [complete])

  return {
    // Provider state
    providers,
    activeProvider,
    isProviderReady,
    selectProvider,
    loadProviders,

    // Model state
    models,
    currentModel,
    loadModels,
    loadModelsForProvider,
    initializeModel,
    checkReady,

    // Ollama-specific
    ollamaConnected,
    checkOllamaConnection,

    // Downloadable models (embedded)
    downloadableModels,
    localModels,
    localModelsInfo,
    loadDownloadableModels,
    loadLocalModels,
    loadLocalModelsInfo,
    isModelDownloaded,
    deleteModel,
    downloadModel,
    downloadCustomModel,
    cancelDownload,
    downloadProgress,
    downloadingModelId,

    // Model tool support
    getModelToolSupport,
    setModelToolSupport,
    deleteModelToolSupport,
    getEffectiveToolSupport,
    getAllModelConfigs,

    // Completion
    complete,
    completeStreaming,
    cancelStreaming,
    streamingContent,

    // Status
    isLoading,
    isProcessing,
    error,

    // Helpers
    summarizeTranscript,
    chatAboutTranscript,
  }
}
