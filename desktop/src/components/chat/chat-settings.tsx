'use client'

import { useState, useEffect } from 'react'
import { useLlm, ProviderType, LlmModelInfo } from '@/hooks/useLlm'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Settings2, Check, Loader2, AlertCircle, ChevronDown, ChevronUp } from 'lucide-react'
import { cn } from '@/lib/utils'

interface ChatSettingsProps {
  /** Initial provider to select (from session config or default) */
  initialProvider?: string
  /** Initial model to select (from session config or default) */
  initialModel?: string
  /** Callback when provider/model changes */
  onConfigChange?: (provider: string | undefined, model: string | undefined) => void
  onReady?: (ready: boolean) => void
  /** Callback when an error occurs during initialization */
  onError?: (error: string | null) => void
  className?: string
}

export function ChatSettings({
  initialProvider,
  initialModel,
  onConfigChange,
  onReady,
  onError,
  className
}: ChatSettingsProps) {
  const [isExpanded, setIsExpanded] = useState(false)
  const [isInitializing, setIsInitializing] = useState(false)
  // Track which model ID we've attempted to initialize (to prevent re-runs)
  const [initializedModelId, setInitializedModelId] = useState<string | null>(null)
  // Local state for the selected model (separate from the loaded model)
  const [selectedModel, setSelectedModel] = useState<string | null>(null)
  // Local error state for initialization failures
  const [initError, setInitError] = useState<string | null>(null)

  const {
    providers,
    activeProvider,
    selectProvider,
    models,
    currentModel,
    initializeModel,
    loadModelsForProvider,
    isProviderReady,
    ollamaConnected,
    checkOllamaConnection,
    checkReady,
    error,
  } = useLlm()

  // Notify parent of ready state changes
  // Only ready when backend has actually initialized the model
  useEffect(() => {
    onReady?.(isProviderReady)
  }, [isProviderReady, onReady])

  // Load models when provider changes
  useEffect(() => {
    if (activeProvider) {
      loadModelsForProvider(activeProvider)
    }
  }, [activeProvider, loadModelsForProvider])

  // Initialize provider from initial values
  useEffect(() => {
    if (!initialProvider) return
    if (activeProvider === initialProvider) return  // Already set
    selectProvider(initialProvider as ProviderType)
  }, [initialProvider, activeProvider, selectProvider])

  // Initialize model selection once models are loaded
  useEffect(() => {
    console.log('[ChatSettings] Model init effect:', {
      initialModel,
      initializedModelId,
      modelsCount: models.length,
      activeProvider
    })

    if (!initialModel) {
      console.log('[ChatSettings] No initialModel, skipping')
      return
    }
    if (initializedModelId === initialModel) {
      console.log('[ChatSettings] Already initialized this model, skipping')
      return
    }
    if (models.length === 0) {
      console.log('[ChatSettings] Models not loaded yet, waiting...')
      return
    }

    // Check if the initial model is in the loaded models list
    const modelExists = models.some(m => m.id === initialModel)
    if (modelExists) {
      setSelectedModel(initialModel)
      setInitializedModelId(initialModel)

      // Auto-initialize the model
      console.log(`[ChatSettings] Auto-initializing model: ${initialModel}`)
      const autoInitialize = async () => {
        try {
          // Check if this model is ALREADY loaded in the backend - skip re-initialization
          const ready = await checkReady()
          if (ready && currentModel === initialModel) {
            console.log(`[ChatSettings] Model "${initialModel}" is already loaded, skipping initialization`)
            onConfigChange?.(activeProvider || undefined, initialModel)
            return
          }

          setIsInitializing(true)
          setInitError(null)
          onError?.(null)
          await initializeModel(initialModel)
          console.log(`[ChatSettings] Model ${initialModel} initialized successfully`)
          // Notify parent of the config
          onConfigChange?.(activeProvider || undefined, initialModel)
        } catch (err) {
          const errorMsg = String(err)
          console.error('[ChatSettings] Failed to auto-initialize default model:', errorMsg)
          setInitError(errorMsg)
          onError?.(errorMsg)
          onReady?.(false)  // Notify parent that initialization failed
        } finally {
          setIsInitializing(false)
        }
      }
      autoInitialize()
    } else if (models.length > 0) {
      // Model not found - fall back to first available model
      const fallbackModel = models[0].id
      console.warn(`[ChatSettings] Model "${initialModel}" not found, using fallback: ${fallbackModel}`)
      setSelectedModel(fallbackModel)
      setInitializedModelId(initialModel)  // Mark original as attempted

      const autoInitializeFallback = async () => {
        try {
          setIsInitializing(true)
          setInitError(null)
          onError?.(null)
          await initializeModel(fallbackModel)
          onConfigChange?.(activeProvider || undefined, fallbackModel)
        } catch (err) {
          const errorMsg = String(err)
          setInitError(errorMsg)
          onError?.(errorMsg)
          onReady?.(false)
        } finally {
          setIsInitializing(false)
        }
      }
      autoInitializeFallback()
    } else {
      // No models available
      console.warn(`[ChatSettings] Model "${initialModel}" not found and no fallback available`)
      setInitializedModelId(initialModel)
    }
  }, [initialModel, models, initializedModelId, initializeModel, activeProvider, onConfigChange, checkReady, currentModel, onError, onReady])

  const handleProviderChange = async (providerType: string) => {
    try {
      await selectProvider(providerType as ProviderType)
      // Clear model selection when provider changes
      setSelectedModel(null)
      setInitializedModelId(null)  // Allow re-initialization with new provider
      // Notify parent - model will be undefined until user selects one
      onConfigChange?.(providerType, undefined)
    } catch (err) {
      console.error('Failed to select provider:', err)
    }
  }

  const handleModelChange = async (modelId: string) => {
    try {
      setSelectedModel(modelId)
      setInitError(null)
      onError?.(null)

      // Skip initialization if this model is already loaded
      if (currentModel === modelId && isProviderReady) {
        console.log(`Model "${modelId}" is already loaded, skipping re-initialization`)
        onConfigChange?.(activeProvider || undefined, modelId)
        return
      }

      setIsInitializing(true)
      await initializeModel(modelId)
      // Notify parent of the new config
      onConfigChange?.(activeProvider || undefined, modelId)
    } catch (err) {
      const errorMsg = String(err)
      console.error('Failed to initialize model:', errorMsg)
      setInitError(errorMsg)
      onError?.(errorMsg)
      onReady?.(false)
    } finally {
      setIsInitializing(false)
    }
  }

  // Filter to available providers
  const availableProviders = providers.filter(p => {
    if (p.provider_type === 'ollama') return ollamaConnected
    if (p.provider_type === 'embedded') return true // Always show, user can download models
    return p.is_available
  })

  // Get provider display info
  const getProviderLabel = (type: ProviderType) => {
    switch (type) {
      case 'ollama': return 'Ollama'
      case 'embedded': return 'Embedded (Local)'
      case 'openai': return 'OpenAI'
      case 'claude': return 'Claude'
      default: return type
    }
  }

  const getProviderStatus = (type: ProviderType) => {
    if (type === 'ollama' && !ollamaConnected) {
      return { ready: false, message: 'Not running' }
    }
    if (type === 'embedded' && models.length === 0) {
      return { ready: false, message: 'No models downloaded' }
    }
    return { ready: true, message: 'Available' }
  }

  return (
    <div className={cn('border-b', className)}>
      {/* Collapsed header */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full px-4 py-2 flex items-center justify-between hover:bg-muted/50 transition-colors"
      >
        <div className="flex items-center gap-2">
          <Settings2 className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">
            {activeProvider ? getProviderLabel(activeProvider) : 'Select Provider'}
          </span>
          {selectedModel && (
            <>
              <span className="text-muted-foreground">/</span>
              <span className="text-sm text-muted-foreground">{selectedModel}</span>
            </>
          )}
          {isProviderReady && (
            <Check className="h-3.5 w-3.5 text-green-500" />
          )}
          {!isProviderReady && activeProvider && (
            <AlertCircle className="h-3.5 w-3.5 text-amber-500" />
          )}
        </div>
        {isExpanded ? (
          <ChevronUp className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        )}
      </button>

      {/* Expanded settings */}
      {isExpanded && (
        <div className="px-4 py-3 space-y-3 bg-muted/30">
          {/* Provider selector */}
          <div className="space-y-1.5">
            <label className="text-xs text-muted-foreground font-medium">Provider</label>
            <Select
              value={activeProvider || ''}
              onValueChange={handleProviderChange}
              disabled={isInitializing}
            >
              <SelectTrigger className="h-8">
                <SelectValue placeholder="Select a provider" />
              </SelectTrigger>
              <SelectContent>
                {availableProviders.map((provider) => {
                  const status = getProviderStatus(provider.provider_type)
                  return (
                    <SelectItem
                      key={provider.provider_type}
                      value={provider.provider_type}
                    >
                      <div className="flex items-center justify-between gap-4">
                        <span>{getProviderLabel(provider.provider_type)}</span>
                        <span className={cn(
                          'text-xs',
                          status.ready ? 'text-green-600' : 'text-amber-600'
                        )}>
                          {status.message}
                        </span>
                      </div>
                    </SelectItem>
                  )
                })}
                {availableProviders.length === 0 && (
                  <SelectItem value="__none__" disabled>
                    No providers available
                  </SelectItem>
                )}
              </SelectContent>
            </Select>
          </div>

          {/* Model selector */}
          {activeProvider && (
            <div className="space-y-1.5">
              <label className="text-xs text-muted-foreground font-medium">Model</label>
              <Select
                value={selectedModel || ''}
                onValueChange={handleModelChange}
                disabled={isInitializing || models.length === 0}
              >
                <SelectTrigger className="h-8">
                  <SelectValue placeholder={models.length === 0 ? 'No models available' : 'Select a model'} />
                </SelectTrigger>
                <SelectContent>
                  {models.map((model) => (
                    <SelectItem key={model.id} value={model.id}>
                      <div className="flex items-center gap-2">
                        <span>{model.name}</span>
                        {model.is_loaded && (
                          <Check className="h-3 w-3 text-green-500" />
                        )}
                        {model.size_bytes && (
                          <span className="text-xs text-muted-foreground">
                            ({(model.size_bytes / 1024 / 1024 / 1024).toFixed(1)}GB)
                          </span>
                        )}
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {models.length === 0 && activeProvider === 'embedded' && (
                <p className="text-xs text-muted-foreground">
                  Download models in <a href="/settings" className="text-primary hover:underline">Settings</a>
                </p>
              )}
              {models.length === 0 && activeProvider === 'ollama' && (
                <p className="text-xs text-muted-foreground">
                  Pull models using <code className="bg-muted px-1 rounded">ollama pull</code>
                </p>
              )}
            </div>
          )}

          {/* Status/Loading indicator */}
          {isInitializing && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>Loading model...</span>
            </div>
          )}

          {/* Error display */}
          {(error || initError) && (
            <div className="text-xs text-destructive bg-destructive/10 p-2 rounded">
              {initError || error}
            </div>
          )}

          {/* Ollama connection check */}
          {activeProvider === 'ollama' && !ollamaConnected && (
            <div className="flex items-center justify-between">
              <span className="text-xs text-amber-600">Ollama not connected</span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => checkOllamaConnection()}
                className="h-7 text-xs"
              >
                Retry Connection
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
