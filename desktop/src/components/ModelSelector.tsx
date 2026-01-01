'use client'

import { useState, useEffect, useCallback } from 'react'
import { listen } from '@tauri-apps/api/event'
import {
  WhisperAPI,
  ModelInfo,
  ModelStatus,
  formatFileSize,
  getModelTagline,
  getRecommendedModel,
  MODEL_CONFIGS,
} from '@/lib/whisper'
import {
  getHardwareRecommendations,
  getWhisperRecommendation,
  getRecommendationBadgeLabel,
  HardwareRecommendations,
  ModelRecommendation,
  RecommendationLevel,
} from '@/lib/hardware'

interface DownloadProgress {
  [modelName: string]: number
}

export function ModelSelector() {
  const [models, setModels] = useState<ModelInfo[]>([])
  const [currentModel, setCurrentModel] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress>({})
  const [isDownloading, setIsDownloading] = useState<string | null>(null)
  const [isLoadingModel, setIsLoadingModel] = useState(false)
  const [recommendations, setRecommendations] = useState<HardwareRecommendations | null>(null)

  // Fetch available models
  const fetchModels = useCallback(async () => {
    try {
      setLoading(true)
      setError(null)
      const availableModels = await WhisperAPI.getAvailableModels()
      setModels(availableModels)
      const current = await WhisperAPI.getCurrentModel()
      setCurrentModel(current)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  // Initialize whisper and set up event listeners
  useEffect(() => {
    const init = async () => {
      try {
        // Fetch hardware recommendations first
        const hwRecommendations = await getHardwareRecommendations()
        setRecommendations(hwRecommendations)

        await WhisperAPI.init()
        await fetchModels()
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
        setLoading(false)
      }
    }

    init()

    // Listen for download progress events
    const unlistenProgress = listen<{ modelName: string; progress: number }>(
      'model-download-progress',
      (event) => {
        console.log('Download progress:', event.payload)
        setDownloadProgress((prev) => ({
          ...prev,
          [event.payload.modelName]: event.payload.progress,
        }))
      }
    )

    // Listen for download completion
    const unlistenComplete = listen<{ modelName: string }>(
      'model-download-complete',
      async (event) => {
        console.log('Download complete:', event.payload)
        setIsDownloading(null)
        setDownloadProgress((prev) => {
          const updated = { ...prev }
          delete updated[event.payload.modelName]
          return updated
        })
        await fetchModels()
      }
    )

    // Listen for download error
    const unlistenError = listen<{ modelName: string; error: string }>(
      'model-download-error',
      (event) => {
        console.log('Download error:', event.payload)
        setIsDownloading(null)
        setError(`Download failed: ${event.payload.error}`)
        setDownloadProgress((prev) => {
          const updated = { ...prev }
          delete updated[event.payload.modelName]
          return updated
        })
      }
    )

    // Listen for model loading events
    const unlistenLoadComplete = listen<{ modelName: string }>(
      'model-loading-completed',
      (event) => {
        setCurrentModel(event.payload.modelName)
        setIsLoadingModel(false)
      }
    )

    return () => {
      unlistenProgress.then((fn) => fn())
      unlistenComplete.then((fn) => fn())
      unlistenError.then((fn) => fn())
      unlistenLoadComplete.then((fn) => fn())
    }
  }, [fetchModels])

  // Download a model
  const handleDownload = async (modelName: string) => {
    try {
      setError(null)
      setIsDownloading(modelName)
      setDownloadProgress((prev) => ({ ...prev, [modelName]: 0 }))
      await WhisperAPI.downloadModel(modelName)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      setIsDownloading(null)
    }
  }

  // Cancel download
  const handleCancelDownload = async (modelName: string) => {
    try {
      await WhisperAPI.cancelDownload(modelName)
      setIsDownloading(null)
      setDownloadProgress((prev) => {
        const updated = { ...prev }
        delete updated[modelName]
        return updated
      })
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  // Load a model
  const handleLoadModel = async (modelName: string) => {
    try {
      setError(null)
      setIsLoadingModel(true)
      await WhisperAPI.loadModel(modelName)
      setCurrentModel(modelName)
      // Save preference
      localStorage.setItem('preferred-whisper-model', modelName)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoadingModel(false)
    }
  }

  // Open models folder
  const handleOpenFolder = async () => {
    try {
      await WhisperAPI.openModelsFolder()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  // Get status badge for a model
  const getStatusBadge = (status: ModelStatus) => {
    if (status === 'Available') {
      return <span className="text-green-600 text-xs font-medium">Available</span>
    }
    if (status === 'Missing') {
      return <span className="text-gray-500 text-xs font-medium">Not Downloaded</span>
    }
    if (typeof status === 'object' && 'Downloading' in status) {
      return <span className="text-blue-600 text-xs font-medium">Downloading...</span>
    }
    if (typeof status === 'object' && 'Error' in status) {
      return <span className="text-red-600 text-xs font-medium">Error</span>
    }
    if (typeof status === 'object' && 'Corrupted' in status) {
      return <span className="text-orange-600 text-xs font-medium">Corrupted</span>
    }
    return null
  }

  // Get recommendation for a model
  const getModelRecommendation = (modelName: string): ModelRecommendation | undefined => {
    if (!recommendations) return undefined
    return getWhisperRecommendation(modelName, recommendations)
  }

  // Group models by recommendation level
  const groupedModels = {
    recommended: models.filter((m) => {
      const rec = getModelRecommendation(m.name)
      return rec?.recommendation === 'Recommended'
    }),
    compatible: models.filter((m) => {
      const rec = getModelRecommendation(m.name)
      return rec?.recommendation === 'Compatible'
    }),
    notRecommended: models.filter((m) => {
      const rec = getModelRecommendation(m.name)
      return rec?.recommendation === 'NotRecommended' || rec?.recommendation === 'TooHeavy'
    }),
  }

  // Best model from hardware analysis, fallback to static
  const recommendedModel = recommendations?.best_whisper_model || getRecommendedModel()

  if (loading) {
    return (
      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <h2 className="text-lg font-semibold text-gray-700 mb-4">Whisper Model</h2>
        <div className="flex items-center justify-center py-4">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span className="ml-2 text-gray-600">Loading models...</span>
        </div>
      </div>
    )
  }

  return (
    <div className="bg-white rounded-lg shadow p-6 mb-6">
      <div className="flex justify-between items-center mb-4">
        <h2 className="text-lg font-semibold text-gray-700">Whisper Model</h2>
        <div className="flex gap-2">
          <button
            onClick={handleOpenFolder}
            className="text-sm text-gray-600 hover:text-gray-800"
            title="Open models folder"
          >
            Open Folder
          </button>
          <button
            onClick={fetchModels}
            className="text-sm text-blue-600 hover:text-blue-800"
          >
            Refresh
          </button>
        </div>
      </div>

      {error && (
        <div className="bg-red-100 border border-red-400 text-red-700 px-3 py-2 rounded mb-4 text-sm">
          {error}
        </div>
      )}

      {/* Hardware Summary Banner */}
      {recommendations && (
        <div className="bg-gradient-to-r from-blue-50 to-indigo-50 border border-blue-200 rounded-md p-3 mb-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-blue-600 font-medium">
                {recommendations.hardware.performance_tier === 'Ultra' && '🚀'}
                {recommendations.hardware.performance_tier === 'High' && '💪'}
                {recommendations.hardware.performance_tier === 'Medium' && '✓'}
                {recommendations.hardware.performance_tier === 'Low' && '📱'}
                {' '}Your System: {recommendations.hardware.performance_tier} Performance
              </span>
            </div>
            <div className="text-xs text-gray-600">
              {recommendations.hardware.cpu_cores} cores
              {recommendations.hardware.has_gpu && ` • ${recommendations.hardware.gpu_type}`}
              {recommendations.hardware.memory_gb > 0 && ` • ${recommendations.hardware.memory_gb}GB RAM`}
            </div>
          </div>
          <div className="text-xs text-gray-500 mt-1">
            {recommendations.hardware.tier_description}
          </div>
        </div>
      )}

      {/* Current Model Status */}
      <div className="bg-gray-50 rounded-md p-3 mb-4">
        {currentModel ? (
          <div className="flex items-center justify-between">
            <div>
              <span className="text-sm text-gray-600">Current Model:</span>
              <span className="ml-2 font-medium text-gray-800">{currentModel}</span>
            </div>
            <span className="text-green-600 text-sm">Ready</span>
          </div>
        ) : (
          <div className="text-amber-600 text-sm">
            No model loaded. Select and download a model below to enable transcription.
          </div>
        )}
      </div>

      {/* Model List */}
      <div className="space-y-4">
        {/* Recommended Models - Best for your hardware */}
        {groupedModels.recommended.length > 0 && (
          <div>
            <h3 className="text-sm font-medium text-green-700 mb-2 flex items-center gap-1">
              <span className="w-2 h-2 bg-green-500 rounded-full"></span>
              Best for Your PC
            </h3>
            <div className="space-y-2">
              {groupedModels.recommended.map((model) => (
                <ModelRow
                  key={model.name}
                  model={model}
                  isCurrentModel={currentModel === model.name}
                  isRecommended={model.name === recommendedModel}
                  isDownloading={isDownloading === model.name}
                  downloadProgress={downloadProgress[model.name]}
                  isLoadingModel={isLoadingModel && currentModel !== model.name}
                  onDownload={handleDownload}
                  onCancelDownload={handleCancelDownload}
                  onLoadModel={handleLoadModel}
                  getStatusBadge={getStatusBadge}
                  recommendation={getModelRecommendation(model.name)}
                />
              ))}
            </div>
          </div>
        )}

        {/* Compatible Models */}
        {groupedModels.compatible.length > 0 && (
          <details className="group" open>
            <summary className="text-sm font-medium text-gray-600 mb-2 cursor-pointer flex items-center gap-1">
              <span className="w-2 h-2 bg-gray-400 rounded-full"></span>
              Other Compatible Models [{groupedModels.compatible.length}]
            </summary>
            <div className="space-y-2 mt-2">
              {groupedModels.compatible.map((model) => (
                <ModelRow
                  key={model.name}
                  model={model}
                  isCurrentModel={currentModel === model.name}
                  isRecommended={false}
                  isDownloading={isDownloading === model.name}
                  downloadProgress={downloadProgress[model.name]}
                  isLoadingModel={isLoadingModel && currentModel !== model.name}
                  onDownload={handleDownload}
                  onCancelDownload={handleCancelDownload}
                  onLoadModel={handleLoadModel}
                  getStatusBadge={getStatusBadge}
                  recommendation={getModelRecommendation(model.name)}
                />
              ))}
            </div>
          </details>
        )}

        {/* Not Recommended Models */}
        {groupedModels.notRecommended.length > 0 && (
          <details className="group">
            <summary className="text-sm font-medium text-amber-600 mb-2 cursor-pointer flex items-center gap-1">
              <span className="w-2 h-2 bg-amber-500 rounded-full"></span>
              May Be Slow on Your PC [{groupedModels.notRecommended.length}]
            </summary>
            <div className="space-y-2 mt-2">
              {groupedModels.notRecommended.map((model) => (
                <ModelRow
                  key={model.name}
                  model={model}
                  isCurrentModel={currentModel === model.name}
                  isRecommended={false}
                  isDownloading={isDownloading === model.name}
                  downloadProgress={downloadProgress[model.name]}
                  isLoadingModel={isLoadingModel && currentModel !== model.name}
                  onDownload={handleDownload}
                  onCancelDownload={handleCancelDownload}
                  onLoadModel={handleLoadModel}
                  getStatusBadge={getStatusBadge}
                  recommendation={getModelRecommendation(model.name)}
                />
              ))}
            </div>
          </details>
        )}
      </div>
    </div>
  )
}

// Individual model row component
interface ModelRowProps {
  model: ModelInfo
  isCurrentModel: boolean
  isRecommended: boolean
  isDownloading: boolean
  downloadProgress?: number
  isLoadingModel: boolean
  onDownload: (modelName: string) => void
  onCancelDownload: (modelName: string) => void
  onLoadModel: (modelName: string) => void
  getStatusBadge: (status: ModelStatus) => React.ReactNode
  recommendation?: ModelRecommendation
}

function ModelRow({
  model,
  isCurrentModel,
  isRecommended,
  isDownloading,
  downloadProgress,
  isLoadingModel,
  onDownload,
  onCancelDownload,
  onLoadModel,
  getStatusBadge,
  recommendation,
}: ModelRowProps) {
  const isAvailable = model.status === 'Available'
  const tagline = getModelTagline(model.name, model.speed, model.accuracy)

  // Get recommendation badge styling
  const getRecBadge = () => {
    if (!recommendation) return null

    switch (recommendation.recommendation) {
      case 'Recommended':
        return (
          <span className="bg-green-100 text-green-700 text-xs px-2 py-0.5 rounded" title={recommendation.reason}>
            Best for you
          </span>
        )
      case 'NotRecommended':
        return (
          <span className="bg-yellow-100 text-yellow-700 text-xs px-2 py-0.5 rounded" title={recommendation.reason}>
            May be slow
          </span>
        )
      case 'TooHeavy':
        return (
          <span className="bg-red-100 text-red-700 text-xs px-2 py-0.5 rounded" title={recommendation.reason}>
            Too demanding
          </span>
        )
      default:
        return null
    }
  }

  return (
    <div
      className={`border rounded-md p-3 ${
        isCurrentModel ? 'border-blue-500 bg-blue-50' : 'border-gray-200'
      }`}
    >
      <div className="flex items-center justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-medium text-gray-800">{model.name}</span>
            <span className="text-xs text-gray-500">{formatFileSize(model.size_mb)}</span>
            {getRecBadge()}
            {isRecommended && !recommendation && (
              <span className="bg-green-100 text-green-700 text-xs px-2 py-0.5 rounded">
                Recommended
              </span>
            )}
            {isCurrentModel && (
              <span className="bg-blue-100 text-blue-700 text-xs px-2 py-0.5 rounded">
                Loaded
              </span>
            )}
          </div>
          <div className="text-xs text-gray-500 mt-1">
            {tagline}
            {recommendation?.reason && recommendation.recommendation !== 'Recommended' && (
              <span className="ml-2 text-amber-600">• {recommendation.reason}</span>
            )}
          </div>
        </div>

        <div className="flex items-center gap-2">
          {getStatusBadge(model.status)}

          {/* Action Button */}
          {isDownloading ? (
            <button
              onClick={() => onCancelDownload(model.name)}
              className="px-3 py-1 text-sm bg-red-100 text-red-700 rounded hover:bg-red-200"
            >
              Cancel
            </button>
          ) : isAvailable ? (
            <button
              onClick={() => onLoadModel(model.name)}
              disabled={isCurrentModel || isLoadingModel}
              className={`px-3 py-1 text-sm rounded ${
                isCurrentModel
                  ? 'bg-gray-100 text-gray-400 cursor-not-allowed'
                  : 'bg-blue-600 text-white hover:bg-blue-700'
              }`}
            >
              {isCurrentModel ? 'Active' : isLoadingModel ? 'Loading...' : 'Use'}
            </button>
          ) : (
            <button
              onClick={() => onDownload(model.name)}
              className="px-3 py-1 text-sm bg-green-600 text-white rounded hover:bg-green-700"
            >
              Download
            </button>
          )}
        </div>
      </div>

      {/* Download Progress Bar */}
      {isDownloading && downloadProgress !== undefined && (
        <div className="mt-2">
          <div className="w-full bg-gray-200 rounded-full h-2">
            <div
              className="bg-blue-600 h-2 rounded-full transition-all duration-300"
              style={{ width: `${downloadProgress}%` }}
            ></div>
          </div>
          <div className="text-xs text-gray-500 mt-1 text-right">
            {downloadProgress.toFixed(1)}%
          </div>
        </div>
      )}
    </div>
  )
}
