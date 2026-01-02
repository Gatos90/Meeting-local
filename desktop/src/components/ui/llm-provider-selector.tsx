'use client'

import { useState, useEffect } from 'react'
import { useLlm, ProviderType, LlmModelInfo } from '@/hooks/useLlm'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import { Loader2, Check, Wrench } from 'lucide-react'
import { cn } from '@/lib/utils'

interface LlmProviderSelectorProps {
  /** Currently selected provider */
  selectedProvider: ProviderType | null
  /** Currently selected model */
  selectedModel: string | null
  /** Called when provider changes */
  onProviderChange: (provider: ProviderType) => void
  /** Called when model changes */
  onModelChange: (modelId: string) => void
  /** Optional className */
  className?: string
  /** Whether to show labels */
  showLabels?: boolean
  /** Custom label for provider */
  providerLabel?: string
  /** Custom label for model */
  modelLabel?: string
  /** Whether the component is disabled */
  disabled?: boolean
  /** Compact mode - smaller dropdowns */
  compact?: boolean
}

export function LlmProviderSelector({
  selectedProvider,
  selectedModel,
  onProviderChange,
  onModelChange,
  className,
  showLabels = true,
  providerLabel = 'Provider',
  modelLabel = 'Model',
  disabled = false,
  compact = false,
}: LlmProviderSelectorProps) {
  const {
    providers,
    models,
    loadModelsForProvider,
    ollamaConnected,
    checkOllamaConnection,
  } = useLlm()

  const [isLoadingModels, setIsLoadingModels] = useState(false)

  // Filter to available providers
  const availableProviders = providers.filter(p => {
    if (p.provider_type === 'ollama') return ollamaConnected
    if (p.provider_type === 'embedded') return true
    return p.is_available
  })

  // Load models when provider changes
  useEffect(() => {
    if (selectedProvider) {
      setIsLoadingModels(true)
      loadModelsForProvider(selectedProvider).finally(() => {
        setIsLoadingModels(false)
      })
    }
  }, [selectedProvider, loadModelsForProvider])

  // Get provider display info
  const getProviderLabel = (type: ProviderType) => {
    switch (type) {
      case 'ollama': return 'Ollama'
      case 'embedded': return 'Local'
      case 'openai': return 'OpenAI'
      case 'claude': return 'Claude'
      default: return type
    }
  }

  const triggerHeight = compact ? 'h-8' : 'h-9'

  return (
    <div className={cn('flex gap-3', className)}>
      {/* Provider Selector */}
      <div className={cn('flex-1', showLabels && 'space-y-1.5')}>
        {showLabels && (
          <Label className="text-xs text-muted-foreground">{providerLabel}</Label>
        )}
        <Select
          value={selectedProvider || ''}
          onValueChange={(value) => onProviderChange(value as ProviderType)}
          disabled={disabled}
        >
          <SelectTrigger className={triggerHeight}>
            <SelectValue placeholder="Select provider..." />
          </SelectTrigger>
          <SelectContent>
            {availableProviders.length === 0 ? (
              <SelectItem value="__none__" disabled>
                No providers available
              </SelectItem>
            ) : (
              availableProviders.map((provider) => (
                <SelectItem key={provider.provider_type} value={provider.provider_type}>
                  <div className="flex items-center gap-2">
                    <span>{getProviderLabel(provider.provider_type)}</span>
                    {provider.provider_type === 'ollama' && !ollamaConnected && (
                      <Badge variant="outline" className="text-xs">Offline</Badge>
                    )}
                  </div>
                </SelectItem>
              ))
            )}
          </SelectContent>
        </Select>
      </div>

      {/* Model Selector */}
      <div className={cn('flex-1', showLabels && 'space-y-1.5')}>
        {showLabels && (
          <Label className="text-xs text-muted-foreground">{modelLabel}</Label>
        )}
        <Select
          value={selectedModel || ''}
          onValueChange={onModelChange}
          disabled={disabled || !selectedProvider || isLoadingModels}
        >
          <SelectTrigger className={triggerHeight}>
            {isLoadingModels ? (
              <div className="flex items-center gap-2">
                <Loader2 className="w-3 h-3 animate-spin" />
                <span className="text-muted-foreground">Loading...</span>
              </div>
            ) : (
              <SelectValue placeholder={
                !selectedProvider
                  ? "Select provider first"
                  : models.length === 0
                    ? "No models available"
                    : "Select model..."
              } />
            )}
          </SelectTrigger>
          <SelectContent>
            {models.length === 0 ? (
              <SelectItem value="__none__" disabled>
                {selectedProvider === 'embedded'
                  ? 'Download models in Settings'
                  : selectedProvider === 'ollama'
                    ? 'Pull models with ollama pull'
                    : 'No models available'
                }
              </SelectItem>
            ) : (
              models.map((model) => (
                <SelectItem key={model.id} value={model.id}>
                  <div className="flex items-center gap-2">
                    <span>{model.name}</span>
                    {model.is_loaded && (
                      <Check className="h-3 w-3 text-green-500" />
                    )}
                    {model.has_native_tool_support && (
                      <Badge variant="outline" className="h-4 px-1 text-[10px] gap-0.5">
                        <Wrench className="h-2.5 w-2.5" />
                        Tools
                      </Badge>
                    )}
                    {model.size_bytes && (
                      <span className="text-xs text-muted-foreground">
                        ({(model.size_bytes / 1024 / 1024 / 1024).toFixed(1)}GB)
                      </span>
                    )}
                  </div>
                </SelectItem>
              ))
            )}
          </SelectContent>
        </Select>
      </div>
    </div>
  )
}

/**
 * Hook to manage LLM provider/model selection state
 * Use this with the LlmProviderSelector component
 */
export function useLlmSelection(initialProvider?: ProviderType | null, initialModel?: string | null) {
  const { activeProvider, currentModel } = useLlm()

  const [selectedProvider, setSelectedProvider] = useState<ProviderType | null>(
    initialProvider ?? activeProvider
  )
  const [selectedModel, setSelectedModel] = useState<string | null>(
    initialModel ?? currentModel
  )

  // Sync with active provider/model when they change (and we don't have a selection)
  useEffect(() => {
    if (!selectedProvider && activeProvider) {
      setSelectedProvider(activeProvider)
    }
  }, [activeProvider, selectedProvider])

  useEffect(() => {
    if (!selectedModel && currentModel) {
      setSelectedModel(currentModel)
    }
  }, [currentModel, selectedModel])

  const handleProviderChange = (provider: ProviderType) => {
    setSelectedProvider(provider)
    setSelectedModel(null) // Reset model when provider changes
  }

  const handleModelChange = (modelId: string) => {
    setSelectedModel(modelId)
  }

  const reset = () => {
    setSelectedProvider(activeProvider)
    setSelectedModel(currentModel)
  }

  return {
    selectedProvider,
    selectedModel,
    setSelectedProvider: handleProviderChange,
    setSelectedModel: handleModelChange,
    reset,
    hasSelection: !!selectedProvider && !!selectedModel,
  }
}
