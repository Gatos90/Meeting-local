'use client'

import { useState, useEffect, useCallback } from 'react'
import { FolderOpen, Info, Loader2, Download, Check, X, AlertCircle, Users, Trash2, UserPlus, Bot, Settings, Cpu, Volume2, Mic, Monitor, Zap, HardDrive, Wrench } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import { Progress } from '@/components/ui/progress'
import { Switch } from '@/components/ui/switch'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useAudioDevices } from '@/hooks/useAudioDevices'
import { useDiarization } from '@/hooks/useDiarization'
import { useLlm } from '@/hooks/useLlm'
import { useDefaultModel } from '@/hooks/useDefaultModel'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getHardwareRecommendations, getWhisperRecommendation, getOllamaModelRecommendation, calculateModelRecommendation, HardwareRecommendations } from '@/lib/hardware'

// Model info from backend
interface ModelInfo {
  name: string
  path: string
  size_mb: number
  accuracy: string
  speed: string
  status: string | { Downloading: { progress: number } } | { Error: string } | { Corrupted: { file_size: number, expected_min_size: number } }
  description: string
}

const languages = [
  { value: 'en', label: 'English' },
  { value: 'es', label: 'Spanish' },
  { value: 'fr', label: 'French' },
  { value: 'de', label: 'German' },
  { value: 'it', label: 'Italian' },
  { value: 'pt', label: 'Portuguese' },
  { value: 'nl', label: 'Dutch' },
  { value: 'pl', label: 'Polish' },
  { value: 'ru', label: 'Russian' },
  { value: 'zh', label: 'Chinese' },
  { value: 'ja', label: 'Japanese' },
  { value: 'ko', label: 'Korean' },
  { value: 'ar', label: 'Arabic' },
  { value: 'hi', label: 'Hindi' },
  { value: 'auto', label: 'Auto-detect' },
]

// Helper to get model status string
function getModelStatus(status: ModelInfo['status']): string {
  if (typeof status === 'string') return status
  if ('Downloading' in status) return 'Downloading'
  if ('Error' in status) return 'Error'
  if ('Corrupted' in status) return 'Corrupted'
  return 'Unknown'
}

export default function SettingsPage() {
  const { devices } = useAudioDevices()
  const [currentModel, setCurrentModel] = useState('base')
  const [selectedLanguage, setSelectedLanguage] = useState('en')
  const [selectedMic, setSelectedMic] = useState('')
  const [selectedSystem, setSelectedSystem] = useState('')
  const [allModels, setAllModels] = useState<ModelInfo[]>([])
  const [isLoadingModel, setIsLoadingModel] = useState(false)
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({})
  const [downloadingModel, setDownloadingModel] = useState<string | null>(null)
  const [downloadError, setDownloadError] = useState<string | null>(null)
  const [noiseSuppressionEnabled, setNoiseSuppressionEnabled] = useState(false)
  const [hardwareInfo, setHardwareInfo] = useState<HardwareRecommendations | null>(null)

  // LLM hook
  const {
    ollamaConnected,
    checkOllamaConnection,
    models: llmModels,
    currentModel: currentLlmModel,
    initializeModel: initializeLlmModel,
    selectProvider,
    activeProvider,
    isLoading: isLlmLoading,
    loadModelsForProvider,
    downloadableModels,
    localModels,
    localModelsInfo,
    downloadModel,
    downloadCustomModel,
    cancelDownload,
    downloadProgress: llmDownloadProgress,
    downloadingModelId,
    deleteModel: deleteLlmModel,
    setModelToolSupport,
  } = useLlm()

  // Custom model download state
  const [customModelName, setCustomModelName] = useState('')
  const [customModelUrl, setCustomModelUrl] = useState('')
  const [isDownloadingCustom, setIsDownloadingCustom] = useState(false)
  const [customDownloadError, setCustomDownloadError] = useState<string | null>(null)

  // Default model hook
  const {
    defaultModel,
    hasDefault,
    setDefault: saveDefaultModel,
    clearDefault: clearDefaultModel,
  } = useDefaultModel()

  const [isTestingConnection, setIsTestingConnection] = useState(false)
  const [isLoadingLlmModel, setIsLoadingLlmModel] = useState(false)
  const [isSavingDefault, setIsSavingDefault] = useState(false)

  // Diarization hook
  const {
    modelsReady: diarizationModelsReady,
    modelsInfo: diarizationModelsInfo,
    isDownloading: isDiarizationDownloading,
    downloadProgress: diarizationDownloadProgress,
    downloadModels: downloadDiarizationModels,
    registeredSpeakers,
    deleteRegisteredSpeaker,
  } = useDiarization()
  const [liveDiarizationEnabled, setLiveDiarizationEnabled] = useState(false)
  const [isDeletingSpeaker, setIsDeletingSpeaker] = useState<string | null>(null)

  // Sortformer state
  const [diarizationProvider, setDiarizationProvider] = useState<'pyannote' | 'sortformer'>('pyannote')
  const [sortformerModelReady, setSortformerModelReady] = useState(false)
  const [sortformerDownloading, setSortformerDownloading] = useState(false)
  const [sortformerDownloadProgress, setSortformerDownloadProgress] = useState(0)

  // Fetch models from backend
  const fetchModels = useCallback(async () => {
    try {
      const models = await invoke<ModelInfo[]>('whisper_get_available_models')
      setAllModels(models)
    } catch (e) {
      console.error('Failed to load models:', e)
    }
  }, [])

  // Set up event listeners for download progress
  useEffect(() => {
    const unlistenProgress = listen<{ modelName: string; progress: number }>(
      'model-download-progress',
      (event) => {
        setDownloadProgress((prev) => ({
          ...prev,
          [event.payload.modelName]: event.payload.progress,
        }))
      }
    )

    const unlistenComplete = listen<{ modelName: string }>(
      'model-download-complete',
      async (event) => {
        setDownloadingModel(null)
        setDownloadProgress((prev) => {
          const updated = { ...prev }
          delete updated[event.payload.modelName]
          return updated
        })
        await fetchModels()
      }
    )

    const unlistenError = listen<{ modelName: string; error: string }>(
      'model-download-error',
      (event) => {
        setDownloadingModel(null)
        setDownloadError(`Download failed: ${event.payload.error}`)
        setDownloadProgress((prev) => {
          const updated = { ...prev }
          delete updated[event.payload.modelName]
          return updated
        })
      }
    )

    const unlistenLoadComplete = listen<{ modelName: string }>(
      'model-loading-completed',
      (event) => {
        setCurrentModel(event.payload.modelName)
        setIsLoadingModel(false)
      }
    )

    const unlistenSortformerProgress = listen<{ progress: number }>(
      'sortformer-download-progress',
      (event) => {
        setSortformerDownloadProgress(event.payload.progress)
      }
    )

    const unlistenSortformerComplete = listen(
      'sortformer-download-complete',
      () => {
        setSortformerDownloading(false)
        setSortformerModelReady(true)
        setSortformerDownloadProgress(100)
      }
    )

    return () => {
      unlistenProgress.then((fn) => fn())
      unlistenComplete.then((fn) => fn())
      unlistenError.then((fn) => fn())
      unlistenLoadComplete.then((fn) => fn())
      unlistenSortformerProgress.then((fn) => fn())
      unlistenSortformerComplete.then((fn) => fn())
    }
  }, [fetchModels])

  // Load initial settings
  useEffect(() => {
    const loadSettings = async () => {
      await fetchModels()

      // Fetch hardware recommendations
      try {
        const hwInfo = await getHardwareRecommendations()
        setHardwareInfo(hwInfo)
      } catch (e) {
        console.error('Failed to load hardware info:', e)
      }

      try {
        const model = await invoke<string>('whisper_get_current_model')
        if (model) setCurrentModel(model)
      } catch (e) {
        console.error('Failed to load model:', e)
      }

      try {
        const lang = await invoke<string>('get_language_preference')
        if (lang) setSelectedLanguage(lang)
      } catch (e) {
        console.error('Failed to load language:', e)
      }

      try {
        const enabled = await invoke<boolean>('get_noise_suppression_enabled')
        setNoiseSuppressionEnabled(enabled)
      } catch (e) {
        console.error('Failed to load noise suppression setting:', e)
      }

      try {
        const enabled = await invoke<boolean>('get_live_diarization_enabled')
        setLiveDiarizationEnabled(enabled)
      } catch (e) {
        console.error('Failed to load live diarization setting:', e)
      }

      try {
        const available = await invoke<boolean>('is_sortformer_model_available')
        setSortformerModelReady(available)
      } catch (e) {
        console.error('Failed to check Sortformer model:', e)
      }
    }

    loadSettings()
  }, [fetchModels])

  // Auto-select first devices
  useEffect(() => {
    if (devices.microphones.length > 0 && !selectedMic) {
      setSelectedMic(devices.microphones[0].name)
    }
    if (devices.speakers.length > 0 && !selectedSystem) {
      setSelectedSystem(devices.speakers[0].name)
    }
  }, [devices, selectedMic, selectedSystem])

  const handleModelChange = async (modelName: string) => {
    setIsLoadingModel(true)
    try {
      await invoke('whisper_load_model', { modelName })
      setCurrentModel(modelName)
    } catch (e) {
      console.error('Failed to load model:', e)
    } finally {
      setIsLoadingModel(false)
    }
  }

  const handleDownload = async (modelName: string) => {
    setDownloadError(null)
    setDownloadingModel(modelName)
    setDownloadProgress((prev) => ({ ...prev, [modelName]: 0 }))
    try {
      await invoke('whisper_download_model', { modelName })
    } catch (e) {
      console.error('Failed to start download:', e)
      setDownloadingModel(null)
      setDownloadError(`Failed to start download: ${e}`)
    }
  }

  const handleCancelDownload = async (modelName: string) => {
    try {
      await invoke('whisper_cancel_download', { modelName })
      setDownloadingModel(null)
      setDownloadProgress((prev) => {
        const updated = { ...prev }
        delete updated[modelName]
        return updated
      })
    } catch (e) {
      console.error('Failed to cancel download:', e)
    }
  }

  const handleDeleteModel = async (modelName: string) => {
    // Don't allow deleting the currently loaded model
    if (modelName === currentModel) {
      setDownloadError('Cannot delete the currently loaded model. Please load a different model first.')
      return
    }
    try {
      await invoke('whisper_delete_model', { modelName })
      await fetchModels()
    } catch (e) {
      console.error('Failed to delete model:', e)
      setDownloadError(`Failed to delete model: ${e}`)
    }
  }

  // Helper to get recommendation level
  const getRecommendationLevel = (modelName: string): number => {
    if (!hardwareInfo) return 1 // Default to compatible
    const rec = getWhisperRecommendation(modelName, hardwareInfo)
    if (rec?.recommendation === 'Recommended') return 0
    if (rec?.recommendation === 'Compatible') return 1
    if (rec?.recommendation === 'NotRecommended') return 2
    if (rec?.recommendation === 'TooHeavy') return 3
    return 1
  }

  // Sort and group models by recommendation then category
  const sortedModels = [...allModels].sort((a, b) => {
    // First sort by recommendation level
    const recA = getRecommendationLevel(a.name)
    const recB = getRecommendationLevel(b.name)
    if (recA !== recB) return recA - recB

    // Then by category order
    const getCategory = (name: string): number => {
      if (name.includes('.en')) return 2 // English-only
      if (name.includes('-q5_1')) return 3 // Q5_1 quantized
      if (name.includes('-q5_0')) return 4 // Q5_0 quantized
      if (name.includes('-q8_0')) return 5 // Q8_0 quantized
      return 1 // Standard multilingual
    }

    // Define size order within category
    const getSizeOrder = (name: string): number => {
      if (name.startsWith('tiny')) return 1
      if (name.startsWith('base')) return 2
      if (name.startsWith('small')) return 3
      if (name.startsWith('medium')) return 4
      if (name.includes('large-v3-turbo')) return 5
      if (name.includes('large-v3')) return 6
      return 7
    }

    const catA = getCategory(a.name)
    const catB = getCategory(b.name)
    if (catA !== catB) return catA - catB
    return getSizeOrder(a.name) - getSizeOrder(b.name)
  })

  // Get recommendation section label
  const getRecommendationSection = (modelName: string): string | null => {
    const rec = hardwareInfo ? getWhisperRecommendation(modelName, hardwareInfo) : null
    if (rec?.recommendation === 'Recommended') return 'Recommended for Your System'
    if (rec?.recommendation === 'Compatible') return 'Other Compatible Models'
    if (rec?.recommendation === 'NotRecommended' || rec?.recommendation === 'TooHeavy') return 'May Be Slow on Your System'
    return null
  }

  // Get category label for a model
  const getCategoryLabel = (name: string): string | null => {
    if (name.includes('.en')) return 'English-Only'
    if (name.includes('-q5_1')) return 'Quantized (Q5_1 - Smaller)'
    if (name.includes('-q5_0')) return 'Quantized (Q5_0 - Balanced)'
    if (name.includes('-q8_0')) return 'Quantized (Q8_0 - Higher Quality)'
    return 'Multilingual'
  }

  const handleLanguageChange = async (value: string) => {
    setSelectedLanguage(value)
    try {
      await invoke('set_language_preference', { language: value })
    } catch (e) {
      console.error('Failed to save language:', e)
    }
  }

  const handleNoiseSuppressionChange = async (enabled: boolean) => {
    setNoiseSuppressionEnabled(enabled)
    try {
      await invoke('set_noise_suppression_enabled', { enabled })
    } catch (e) {
      console.error('Failed to save noise suppression setting:', e)
    }
  }

  const handleLiveDiarizationChange = async (enabled: boolean) => {
    setLiveDiarizationEnabled(enabled)
    try {
      await invoke('set_live_diarization_enabled', { enabled })
    } catch (e) {
      console.error('Failed to save live diarization setting:', e)
    }
  }

  const handleDeleteSpeaker = async (speakerId: string) => {
    setIsDeletingSpeaker(speakerId)
    try {
      await deleteRegisteredSpeaker(speakerId)
    } catch (e) {
      console.error('Failed to delete speaker:', e)
    } finally {
      setIsDeletingSpeaker(null)
    }
  }

  const handleDownloadSortformer = async () => {
    setSortformerDownloading(true)
    setSortformerDownloadProgress(0)
    try {
      await invoke('download_sortformer_model')
    } catch (e) {
      console.error('Failed to download Sortformer model:', e)
      setSortformerDownloading(false)
    }
  }

  const handleTestOllamaConnection = async () => {
    setIsTestingConnection(true)
    try {
      await checkOllamaConnection()
      if (ollamaConnected) {
        await selectProvider('ollama')
        await loadModelsForProvider('ollama')
      }
    } catch (e) {
      console.error('Failed to test connection:', e)
    } finally {
      setIsTestingConnection(false)
    }
  }

  const handleLoadLlmModel = async (modelId: string) => {
    setIsLoadingLlmModel(true)
    try {
      await selectProvider('ollama')
      await initializeLlmModel(modelId)
    } catch (e) {
      console.error('Failed to load LLM model:', e)
    } finally {
      setIsLoadingLlmModel(false)
    }
  }

  const availableModels = allModels.filter((m) => getModelStatus(m.status) === 'Available')

  return (
    <>
      {/* Header */}
      <header className="flex h-16 items-center justify-between border-b border-border bg-background/50 backdrop-blur-sm px-8">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-semibold text-foreground">Settings</h1>
        </div>
      </header>

      {/* Main Content with Tabs */}
      <div className="flex-1 overflow-y-auto p-8">
        <div className="mx-auto max-w-6xl">
          <Tabs defaultValue="models" className="w-full">
            <TabsList className="grid w-full grid-cols-4 mb-8">
              <TabsTrigger value="models" className="flex items-center gap-2">
                <Download className="h-4 w-4" />
                <span className="hidden sm:inline">Models</span>
              </TabsTrigger>
              <TabsTrigger value="ai" className="flex items-center gap-2">
                <Bot className="h-4 w-4" />
                <span className="hidden sm:inline">AI Assistant</span>
              </TabsTrigger>
              <TabsTrigger value="speakers" className="flex items-center gap-2">
                <Users className="h-4 w-4" />
                <span className="hidden sm:inline">Speakers</span>
              </TabsTrigger>
              <TabsTrigger value="general" className="flex items-center gap-2">
                <Settings className="h-4 w-4" />
                <span className="hidden sm:inline">General</span>
              </TabsTrigger>
            </TabsList>

            {/* Models Tab */}
            <TabsContent value="models" className="space-y-6">
              <Card className="p-6">
                <div className="flex items-center gap-3 mb-6">
                  <div className="p-2 rounded-lg bg-primary/10">
                    <Download className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <h2 className="font-semibold text-foreground">Whisper Models</h2>
                    <p className="text-sm text-muted-foreground">
                      Download and manage transcription models
                    </p>
                  </div>
                </div>

                {downloadError && (
                  <div className="mb-4 p-3 bg-destructive/10 border border-destructive/20 rounded-lg flex items-center gap-2 text-sm text-destructive">
                    <AlertCircle className="h-4 w-4" />
                    {downloadError}
                    <Button variant="ghost" size="sm" className="ml-auto h-6 w-6 p-0" onClick={() => setDownloadError(null)}>
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                )}

                <div className="space-y-3">
                  {sortedModels.map((model, index) => {
                    const status = getModelStatus(model.status)
                    const isDownloading = downloadingModel === model.name
                    const progress = downloadProgress[model.name] || 0
                    const isCurrentModel = currentModel === model.name

                    // Get recommendation for this model
                    const recommendation = hardwareInfo ? getWhisperRecommendation(model.name, hardwareInfo) : null

                    // Check if we need to show a recommendation section header
                    const currentSection = getRecommendationSection(model.name)
                    const prevSection = index > 0 ? getRecommendationSection(sortedModels[index - 1].name) : null
                    const showSectionHeader = currentSection !== prevSection

                    return (
                      <div key={model.name}>
                        {showSectionHeader && hardwareInfo && (
                          <div className="flex items-center gap-2 mt-4 mb-2 first:mt-0">
                            <div className={`w-2 h-2 rounded-full ${
                              currentSection === 'Recommended for Your System' ? 'bg-green-500' :
                              currentSection === 'Other Compatible Models' ? 'bg-gray-400' :
                              'bg-amber-500'
                            }`}></div>
                            <span className={`text-sm font-medium ${
                              currentSection === 'Recommended for Your System' ? 'text-green-600' :
                              currentSection === 'Other Compatible Models' ? 'text-muted-foreground' :
                              'text-amber-600'
                            }`}>{currentSection}</span>
                            <Separator className="flex-1" />
                          </div>
                        )}
                        <div className="flex items-center justify-between p-3 border border-border rounded-lg hover:bg-muted/50 transition-colors">
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 flex-wrap">
                              <span className="font-medium text-foreground">{model.name}</span>
                              {isCurrentModel && <Badge variant="default" className="text-xs">Loaded</Badge>}
                              {status === 'Missing' && <Badge variant="secondary" className="text-xs">Not Downloaded</Badge>}
                              {status === 'Corrupted' && <Badge variant="destructive" className="text-xs">Corrupted</Badge>}
                              {/* Recommendation badges */}
                              {recommendation?.recommendation === 'Recommended' && (
                                <Badge className="bg-green-500/20 text-green-600 border-green-500/30 text-xs hover:bg-green-500/20 hover:text-green-600">
                                  Best for you
                                </Badge>
                              )}
                              {recommendation?.recommendation === 'NotRecommended' && (
                                <Badge variant="outline" className="text-yellow-600 border-yellow-500/50 text-xs hover:text-yellow-600">
                                  May be slow
                                </Badge>
                              )}
                              {recommendation?.recommendation === 'TooHeavy' && (
                                <Badge variant="outline" className="text-red-600 border-red-500/50 text-xs hover:text-red-600">
                                  Too demanding
                                </Badge>
                              )}
                            </div>
                            <p className="text-xs text-muted-foreground mt-0.5">{model.size_mb}MB • {model.speed} • {model.description}</p>
                          </div>
                          <div className="flex items-center gap-2 ml-4">
                            {isDownloading && (
                              <>
                                <div className="flex items-center gap-2 min-w-[120px]">
                                  <Progress value={progress} className="w-20" />
                                  <span className="text-xs text-muted-foreground w-8">{progress}%</span>
                                </div>
                                <Button variant="ghost" size="sm" onClick={() => handleCancelDownload(model.name)}>
                                  <X className="h-4 w-4" />
                                </Button>
                              </>
                            )}
                            {!isDownloading && status === 'Available' && (
                              <>
                                <Button variant={isCurrentModel ? 'secondary' : 'outline'} size="sm" onClick={() => handleModelChange(model.name)} disabled={isLoadingModel || isCurrentModel}>
                                  {isLoadingModel && currentModel !== model.name ? <Loader2 className="h-4 w-4 animate-spin" /> : isCurrentModel ? <><Check className="h-4 w-4 mr-1" /> Loaded</> : 'Load'}
                                </Button>
                                {!isCurrentModel && (
                                  <Button variant="ghost" size="sm" onClick={() => handleDeleteModel(model.name)} title="Delete model">
                                    <Trash2 className="h-4 w-4 text-muted-foreground hover:text-destructive" />
                                  </Button>
                                )}
                              </>
                            )}
                            {!isDownloading && status === 'Missing' && (
                              <Button variant="outline" size="sm" onClick={() => handleDownload(model.name)}>
                                <Download className="h-4 w-4 mr-1" /> Download
                              </Button>
                            )}
                            {!isDownloading && status === 'Corrupted' && (
                              <>
                                <Button variant="destructive" size="sm" onClick={() => handleDownload(model.name)}>
                                  <Download className="h-4 w-4 mr-1" /> Re-download
                                </Button>
                                <Button variant="ghost" size="sm" onClick={() => handleDeleteModel(model.name)} title="Delete corrupted model">
                                  <Trash2 className="h-4 w-4 text-muted-foreground hover:text-destructive" />
                                </Button>
                              </>
                            )}
                          </div>
                        </div>
                      </div>
                    )
                  })}
                  {sortedModels.length === 0 && (
                    <div className="text-center py-8 text-muted-foreground">
                      <Loader2 className="h-6 w-6 animate-spin mx-auto mb-2" />
                      Loading models...
                    </div>
                  )}
                </div>
              </Card>
            </TabsContent>

            {/* Transcription Tab */}
            <TabsContent value="transcription" className="space-y-6">
              <Card className="p-6">
                <div className="flex items-center gap-3 mb-6">
                  <div className="p-2 rounded-lg bg-primary/10">
                    <Cpu className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <h2 className="font-semibold text-foreground">Transcription</h2>
                    <p className="text-sm text-muted-foreground">Configure Whisper model and language settings</p>
                  </div>
                </div>

                <div className="space-y-6">
                  <div className="space-y-2">
                    <label className="text-sm font-medium text-foreground flex items-center gap-2">
                      Active Model
                      {isLoadingModel && <Loader2 className="h-4 w-4 animate-spin" />}
                    </label>
                    <Select value={currentModel} onValueChange={handleModelChange} disabled={isLoadingModel || availableModels.length === 0}>
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder={availableModels.length === 0 ? "No models available" : "Select model"} />
                      </SelectTrigger>
                      <SelectContent>
                        {availableModels.map((model) => (
                          <SelectItem key={model.name} value={model.name}>
                            <div className="flex items-center justify-between gap-4">
                              <span>{model.name}</span>
                              <span className="text-xs text-muted-foreground">{model.speed}</span>
                            </div>
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <p className="text-xs text-muted-foreground">Select from downloaded models. Download more in the Models tab.</p>
                  </div>

                  <Separator />

                  <div className="space-y-2">
                    <label className="text-sm font-medium text-foreground">Language</label>
                    <Select value={selectedLanguage} onValueChange={handleLanguageChange}>
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder="Select language" />
                      </SelectTrigger>
                      <SelectContent>
                        {languages.map((lang) => (
                          <SelectItem key={lang.value} value={lang.value}>{lang.label}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <p className="text-xs text-muted-foreground">Select the primary language spoken in your meetings for better accuracy.</p>
                  </div>

                  <Separator />

                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <label className="text-sm font-medium text-foreground flex items-center gap-2">
                        <Volume2 className="h-4 w-4" />
                        Noise Suppression
                      </label>
                      <p className="text-xs text-muted-foreground">Apply RNNoise to reduce background noise (10-15 dB reduction)</p>
                    </div>
                    <Switch checked={noiseSuppressionEnabled} onCheckedChange={handleNoiseSuppressionChange} />
                  </div>
                </div>
              </Card>
            </TabsContent>

            {/* AI Assistant Tab */}
            <TabsContent value="ai" className="space-y-6">
              {/* Default Model Card */}
              <Card className="p-6">
                <div className="flex items-center gap-3 mb-6">
                  <div className="p-2 rounded-lg bg-primary/10">
                    <Bot className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <h2 className="font-semibold text-foreground">Default Chat Model</h2>
                    <p className="text-sm text-muted-foreground">Set the default model for new chat sessions</p>
                  </div>
                </div>

                <div className="space-y-4">
                  {hasDefault ? (
                    <div className="flex items-center justify-between p-4 bg-muted/50 rounded-lg">
                      <div>
                        <p className="text-sm font-medium text-foreground">
                          {defaultModel?.provider_type === 'ollama' ? 'Ollama' : 'Embedded'}: {defaultModel?.model_id}
                        </p>
                        <p className="text-xs text-muted-foreground mt-0.5">
                          This model will be used by default when starting new chats
                        </p>
                      </div>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={async () => {
                          setIsSavingDefault(true)
                          try {
                            await clearDefaultModel()
                          } finally {
                            setIsSavingDefault(false)
                          }
                        }}
                        disabled={isSavingDefault}
                      >
                        {isSavingDefault ? <Loader2 className="h-4 w-4 animate-spin" /> : 'Clear Default'}
                      </Button>
                    </div>
                  ) : (
                    <div className="text-center py-6 border border-dashed border-border rounded-lg">
                      <Bot className="h-8 w-8 mx-auto text-muted-foreground mb-2" />
                      <p className="text-sm text-muted-foreground">No default model set</p>
                      <p className="text-xs text-muted-foreground mt-1">
                        Select a model below and click "Set as Default", or choose one in a chat session
                      </p>
                    </div>
                  )}
                </div>
              </Card>

              <Card className="p-6">
                <div className="flex items-center gap-3 mb-6">
                  <div className="p-2 rounded-lg bg-primary/10">
                    <Bot className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <h2 className="font-semibold text-foreground">AI Providers</h2>
                    <p className="text-sm text-muted-foreground">Configure AI models for transcript analysis</p>
                  </div>
                </div>

                <div className="space-y-6">
                  {/* Provider Selection */}
                  <div className="space-y-2">
                    <label className="text-sm font-medium text-foreground">Provider</label>
                    <Select value={activeProvider || 'ollama'} onValueChange={(v) => selectProvider(v as 'embedded' | 'ollama')}>
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder="Select provider" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="embedded">Embedded (Built-in)</SelectItem>
                        <SelectItem value="ollama">Ollama (Local Server)</SelectItem>
                      </SelectContent>
                    </Select>
                    <p className="text-xs text-muted-foreground">
                      {activeProvider === 'embedded'
                        ? 'Run GGUF models directly in the app - no external server needed'
                        : 'Run AI models locally using Ollama server'}
                    </p>
                  </div>

                  <Separator />

                  {/* Embedded Provider UI */}
                  {activeProvider === 'embedded' && (
                    <div className="space-y-4">
                      <div className="space-y-3">
                        <label className="text-sm font-medium text-foreground">Available Models</label>
                        <p className="text-xs text-muted-foreground">Download GGUF models to run locally</p>

                        {downloadableModels.map((model) => {
                          const isDownloaded = localModels.includes(model.id)
                          const isDownloading = downloadingModelId === model.id
                          const progress = llmDownloadProgress[model.id]
                          const progressPercent = progress?.percent || 0
                          const hasError = progress?.status && typeof progress.status === 'object' && 'Failed' in progress.status
                          const isDefault = defaultModel?.provider_type === 'embedded' && defaultModel?.model_id === model.id
                          // Use actual model size for accurate recommendations
                          const modelSizeGB = model.size_bytes / 1_000_000_000
                          const llmRecommendation = hardwareInfo
                            ? calculateModelRecommendation(modelSizeGB, hardwareInfo.hardware.performance_tier, 'llm')
                            : null

                          return (
                            <div key={model.id} className="p-4 border border-border rounded-lg space-y-3">
                              <div className="flex items-start justify-between">
                                <div className="space-y-1">
                                  <div className="flex items-center gap-2 flex-wrap">
                                    <span className="font-medium text-foreground">{model.name}</span>
                                    {isDownloaded && (
                                      <Badge variant="outline" className="text-green-600 border-green-600">
                                        <Check className="h-3 w-3 mr-1" />Downloaded
                                      </Badge>
                                    )}
                                    {currentLlmModel === model.id && (
                                      <Badge variant="default" className="text-xs">Loaded</Badge>
                                    )}
                                    {isDefault && (
                                      <Badge variant="outline" className="text-xs text-green-600 border-green-600">Default</Badge>
                                    )}
                                    {llmRecommendation?.recommendation === 'Recommended' && (
                                      <Badge className="bg-green-500/20 text-green-600 border-green-500/30 text-xs hover:bg-green-500/20 hover:text-green-600">
                                        Best for you
                                      </Badge>
                                    )}
                                    {llmRecommendation?.recommendation === 'NotRecommended' && (
                                      <Badge variant="outline" className="text-yellow-600 border-yellow-500/50 text-xs hover:text-yellow-600">
                                        May be slow
                                      </Badge>
                                    )}
                                    {llmRecommendation?.recommendation === 'TooHeavy' && (
                                      <Badge variant="outline" className="text-red-600 border-red-500/50 text-xs hover:text-red-600">
                                        Too demanding
                                      </Badge>
                                    )}
                                  </div>
                                  <p className="text-xs text-muted-foreground">{model.description}</p>
                                  {llmRecommendation?.reason && (
                                    <p className="text-xs text-muted-foreground">{llmRecommendation.reason}</p>
                                  )}
                                  <p className="text-xs text-muted-foreground">
                                    {(model.size_bytes / 1_000_000_000).toFixed(1)} GB • {model.context_length.toLocaleString()} context
                                  </p>
                                </div>
                                <div className="flex items-center gap-2">
                                  {/* Tool support toggle for downloaded models */}
                                  {isDownloaded && (
                                    <div className="flex items-center gap-1 mr-2">
                                      <button
                                        onClick={async () => {
                                          const localModel = localModelsInfo.find(m => m.id === model.id)
                                          const currentSupport = localModel?.has_native_tool_support ?? false
                                          await setModelToolSupport(model.id, !currentSupport)
                                        }}
                                        className={`flex items-center gap-1 px-2 py-1 text-xs rounded-md border transition-colors ${
                                          localModelsInfo.find(m => m.id === model.id)?.has_native_tool_support
                                            ? 'bg-blue-500/20 text-blue-600 border-blue-500/30 hover:bg-blue-500/30'
                                            : 'bg-muted text-muted-foreground border-border hover:bg-muted/80'
                                        }`}
                                        title={localModelsInfo.find(m => m.id === model.id)?.has_native_tool_support
                                          ? 'Model supports native function calling (click to disable)'
                                          : 'Model uses simulated tool calling (click to enable native tools)'}
                                      >
                                        <Wrench className="h-3 w-3" />
                                        {localModelsInfo.find(m => m.id === model.id)?.has_native_tool_support ? 'Tools' : 'No Tools'}
                                      </button>
                                    </div>
                                  )}
                                  {isDownloading ? (
                                    <Button variant="ghost" size="sm" onClick={() => cancelDownload(model.id)}>
                                      <X className="h-4 w-4" />
                                    </Button>
                                  ) : isDownloaded ? (
                                    <>
                                      {!isDefault && (
                                        <Button
                                          variant="ghost"
                                          size="sm"
                                          onClick={async () => {
                                            setIsSavingDefault(true)
                                            try {
                                              await saveDefaultModel('embedded', model.id)
                                            } finally {
                                              setIsSavingDefault(false)
                                            }
                                          }}
                                          disabled={isSavingDefault}
                                          className="text-xs"
                                        >
                                          Set Default
                                        </Button>
                                      )}
                                      <Button
                                        variant={currentLlmModel === model.id ? 'secondary' : 'outline'}
                                        size="sm"
                                        onClick={() => initializeLlmModel(model.id)}
                                        disabled={isLoadingLlmModel || currentLlmModel === model.id}
                                      >
                                        {isLoadingLlmModel ? (
                                          <Loader2 className="h-4 w-4 animate-spin" />
                                        ) : currentLlmModel === model.id ? (
                                          <><Check className="h-4 w-4 mr-1" /> Loaded</>
                                        ) : (
                                          'Load'
                                        )}
                                      </Button>
                                      <Button
                                        variant="ghost"
                                        size="sm"
                                        onClick={() => deleteLlmModel(model.id)}
                                        className="text-destructive hover:text-destructive hover:bg-destructive/10"
                                      >
                                        <Trash2 className="h-4 w-4" />
                                      </Button>
                                    </>
                                  ) : (
                                    <Button variant="outline" size="sm" onClick={() => downloadModel(model.id)}>
                                      <Download className="h-4 w-4 mr-1" /> Download
                                    </Button>
                                  )}
                                </div>
                              </div>

                              {/* Download Progress */}
                              {isDownloading && (
                                <div className="space-y-2">
                                  <Progress value={progressPercent} className="w-full" />
                                  <div className="flex justify-between text-xs text-muted-foreground">
                                    <span>
                                      {progress?.status === 'Verifying' ? 'Verifying...' : 'Downloading...'}
                                    </span>
                                    <span>{progressPercent.toFixed(1)}%</span>
                                  </div>
                                </div>
                              )}

                              {/* Download Error */}
                              {hasError && (
                                <div className="flex items-center gap-2 text-sm text-destructive">
                                  <AlertCircle className="h-4 w-4" />
                                  <span>{typeof progress.status === 'object' && 'Failed' in progress.status ? progress.status.Failed : 'Download failed'}</span>
                                </div>
                              )}
                            </div>
                          )
                        })}

                        {downloadableModels.length === 0 && (
                          <div className="text-center py-6 border border-dashed border-border rounded-lg">
                            <Bot className="h-8 w-8 mx-auto text-muted-foreground mb-2" />
                            <p className="text-sm text-muted-foreground">No models available</p>
                          </div>
                        )}
                      </div>

                      <Separator />

                      {/* Custom Model Download */}
                      <div className="space-y-3">
                        <label className="text-sm font-medium text-foreground">Add Custom Model</label>
                        <p className="text-xs text-muted-foreground">Download any GGUF model from a URL</p>

                        <div className="p-4 border border-border rounded-lg space-y-4">
                          <div className="space-y-2">
                            <label className="text-xs font-medium text-foreground">Model Name</label>
                            <input
                              type="text"
                              value={customModelName}
                              onChange={(e) => setCustomModelName(e.target.value)}
                              placeholder="my-custom-model"
                              className="w-full px-3 py-2 text-sm border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary/20"
                            />
                          </div>
                          <div className="space-y-2">
                            <label className="text-xs font-medium text-foreground">GGUF URL</label>
                            <input
                              type="url"
                              value={customModelUrl}
                              onChange={(e) => setCustomModelUrl(e.target.value)}
                              placeholder="https://huggingface.co/.../model.gguf"
                              className="w-full px-3 py-2 text-sm border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary/20"
                            />
                            <p className="text-xs text-muted-foreground">
                              Direct link to a .gguf file (e.g., from HuggingFace)
                            </p>
                          </div>

                          {customDownloadError && (
                            <div className="flex items-center gap-2 text-sm text-destructive">
                              <AlertCircle className="h-4 w-4" />
                              <span>{customDownloadError}</span>
                            </div>
                          )}

                          <Button
                            onClick={async () => {
                              setCustomDownloadError(null)
                              setIsDownloadingCustom(true)
                              try {
                                await downloadCustomModel(customModelName, customModelUrl)
                                // Clear inputs on success (download started)
                                setCustomModelName('')
                                setCustomModelUrl('')
                              } catch (err) {
                                setCustomDownloadError(String(err))
                              } finally {
                                setIsDownloadingCustom(false)
                              }
                            }}
                            disabled={!customModelName.trim() || !customModelUrl.trim() || isDownloadingCustom || downloadingModelId !== null}
                            className="w-full"
                          >
                            {isDownloadingCustom ? (
                              <><Loader2 className="h-4 w-4 mr-2 animate-spin" /> Starting Download...</>
                            ) : (
                              <><Download className="h-4 w-4 mr-2" /> Download Model</>
                            )}
                          </Button>
                        </div>

                        {/* Show download progress for custom models */}
                        {downloadingModelId && !downloadableModels.some(m => m.id === downloadingModelId) && (
                          <div className="p-4 border border-border rounded-lg bg-muted/50 space-y-3">
                            <div className="flex items-center justify-between">
                              <div className="space-y-1">
                                <div className="flex items-center gap-2">
                                  <span className="font-medium text-foreground">{downloadingModelId}</span>
                                  <Badge variant="secondary">Downloading</Badge>
                                </div>
                                <p className="text-xs text-muted-foreground">Custom model download in progress</p>
                              </div>
                              <Button variant="ghost" size="sm" onClick={() => cancelDownload(downloadingModelId)}>
                                <X className="h-4 w-4" />
                              </Button>
                            </div>
                            <div className="space-y-2">
                              <Progress value={llmDownloadProgress[downloadingModelId]?.percent || 0} className="w-full" />
                              <div className="flex justify-between text-xs text-muted-foreground">
                                <span>
                                  {llmDownloadProgress[downloadingModelId]?.status === 'Verifying' ? 'Verifying...' : 'Downloading...'}
                                </span>
                                <span>
                                  {llmDownloadProgress[downloadingModelId]?.total_bytes
                                    ? `${((llmDownloadProgress[downloadingModelId]?.downloaded_bytes || 0) / 1_000_000_000).toFixed(2)} / ${((llmDownloadProgress[downloadingModelId]?.total_bytes || 0) / 1_000_000_000).toFixed(2)} GB`
                                    : `${((llmDownloadProgress[downloadingModelId]?.downloaded_bytes || 0) / 1_000_000).toFixed(0)} MB`
                                  }
                                </span>
                              </div>
                            </div>
                          </div>
                        )}
                      </div>

                      {/* Custom Downloaded Models */}
                      {localModelsInfo.filter(m => !m.is_curated).length > 0 && (
                        <>
                          <Separator />
                          <div className="space-y-3">
                            <label className="text-sm font-medium text-foreground">Custom Models</label>
                            <p className="text-xs text-muted-foreground">Models you've downloaded from custom URLs</p>

                            {localModelsInfo.filter(m => !m.is_curated).map((model) => {
                              const isDownloading = downloadingModelId === model.id
                              const progress = llmDownloadProgress[model.id]
                              const progressPercent = progress?.percent || 0
                              const isDefault = defaultModel?.provider_type === 'embedded' && defaultModel?.model_id === model.id

                              return (
                                <div key={model.id} className="p-4 border border-border rounded-lg space-y-3">
                                  <div className="flex items-start justify-between">
                                    <div className="space-y-1">
                                      <div className="flex items-center gap-2">
                                        <span className="font-medium text-foreground">{model.name}</span>
                                        <Badge variant="outline" className="text-green-600 border-green-600">
                                          <Check className="h-3 w-3 mr-1" />Downloaded
                                        </Badge>
                                        {currentLlmModel === model.id && (
                                          <Badge variant="default" className="text-xs">Loaded</Badge>
                                        )}
                                        {isDefault && (
                                          <Badge variant="outline" className="text-xs text-green-600 border-green-600">Default</Badge>
                                        )}
                                      </div>
                                      <p className="text-xs text-muted-foreground">
                                        {(model.size_bytes / 1_000_000_000).toFixed(2)} GB • Custom model
                                      </p>
                                    </div>
                                    <div className="flex items-center gap-2">
                                      {/* Tool support toggle for custom models */}
                                      <div className="flex items-center gap-1 mr-2">
                                        <button
                                          onClick={async () => {
                                            const currentSupport = model.has_native_tool_support ?? false
                                            await setModelToolSupport(model.id, !currentSupport)
                                          }}
                                          className={`flex items-center gap-1 px-2 py-1 text-xs rounded-md border transition-colors ${
                                            model.has_native_tool_support
                                              ? 'bg-blue-500/20 text-blue-600 border-blue-500/30 hover:bg-blue-500/30'
                                              : 'bg-muted text-muted-foreground border-border hover:bg-muted/80'
                                          }`}
                                          title={model.has_native_tool_support
                                            ? 'Model supports native function calling (click to disable)'
                                            : 'Model uses simulated tool calling (click to enable native tools)'}
                                        >
                                          <Wrench className="h-3 w-3" />
                                          {model.has_native_tool_support ? 'Tools' : 'No Tools'}
                                        </button>
                                      </div>
                                      {!isDefault && (
                                        <Button
                                          variant="ghost"
                                          size="sm"
                                          onClick={async () => {
                                            setIsSavingDefault(true)
                                            try {
                                              await saveDefaultModel('embedded', model.id)
                                            } finally {
                                              setIsSavingDefault(false)
                                            }
                                          }}
                                          disabled={isSavingDefault}
                                          className="text-xs"
                                        >
                                          Set Default
                                        </Button>
                                      )}
                                      <Button
                                        variant={currentLlmModel === model.id ? 'secondary' : 'outline'}
                                        size="sm"
                                        onClick={() => initializeLlmModel(model.id)}
                                        disabled={isLoadingLlmModel || currentLlmModel === model.id}
                                      >
                                        {isLoadingLlmModel ? (
                                          <Loader2 className="h-4 w-4 animate-spin" />
                                        ) : currentLlmModel === model.id ? (
                                          <><Check className="h-4 w-4 mr-1" /> Loaded</>
                                        ) : (
                                          'Load'
                                        )}
                                      </Button>
                                      <Button
                                        variant="ghost"
                                        size="sm"
                                        onClick={() => deleteLlmModel(model.id)}
                                        className="text-destructive hover:text-destructive hover:bg-destructive/10"
                                      >
                                        <Trash2 className="h-4 w-4" />
                                      </Button>
                                    </div>
                                  </div>

                                  {/* Download Progress (for re-downloads) */}
                                  {isDownloading && (
                                    <div className="space-y-2">
                                      <Progress value={progressPercent} className="w-full" />
                                      <div className="flex justify-between text-xs text-muted-foreground">
                                        <span>
                                          {progress?.status === 'Verifying' ? 'Verifying...' : 'Downloading...'}
                                        </span>
                                        <span>{progressPercent.toFixed(1)}%</span>
                                      </div>
                                    </div>
                                  )}
                                </div>
                              )
                            })}
                          </div>
                        </>
                      )}
                    </div>
                  )}

                  {/* Ollama Provider UI */}
                  {activeProvider !== 'embedded' && (
                    <>
                      {/* Connection Status */}
                      <div className="space-y-3">
                        <label className="text-sm font-medium text-foreground">Connection Status</label>
                        <div className="flex items-center justify-between p-4 bg-muted/50 rounded-lg">
                          <div className="flex items-center gap-3">
                            <div className={`h-3 w-3 rounded-full ${ollamaConnected ? 'bg-green-500' : 'bg-red-500'}`} />
                            <div>
                              <p className="text-sm font-medium text-foreground">
                                {ollamaConnected ? 'Connected to Ollama' : 'Not Connected'}
                              </p>
                              <p className="text-xs text-muted-foreground">
                                {ollamaConnected ? 'Ready to use AI features' : 'Install and run Ollama from ollama.ai'}
                              </p>
                            </div>
                          </div>
                          <Button variant="outline" size="sm" onClick={handleTestOllamaConnection} disabled={isTestingConnection}>
                            {isTestingConnection ? <Loader2 className="h-4 w-4 animate-spin" /> : 'Test Connection'}
                          </Button>
                        </div>
                      </div>

                      {ollamaConnected && (
                        <>
                          <Separator />

                          {/* Model Selection */}
                          <div className="space-y-3">
                            <label className="text-sm font-medium text-foreground">Available Models</label>
                            {llmModels.length === 0 ? (
                              <div className="text-center py-6 border border-dashed border-border rounded-lg">
                                <Bot className="h-8 w-8 mx-auto text-muted-foreground mb-2" />
                                <p className="text-sm text-muted-foreground">No models found in Ollama</p>
                                <p className="text-xs text-muted-foreground mt-1">Run: ollama pull llama3.2</p>
                              </div>
                            ) : (
                              <div className="space-y-2">
                                {llmModels.map((model) => {
                                  const isDefault = defaultModel?.provider_type === 'ollama' && defaultModel?.model_id === model.id
                                  const llmRecommendation = hardwareInfo
                                    ? getOllamaModelRecommendation(model.id, hardwareInfo)
                                    : null
                                  return (
                                    <div key={model.id} className="flex items-center justify-between p-3 border border-border rounded-lg hover:bg-muted/50 transition-colors">
                                      <div>
                                        <div className="flex items-center gap-2 flex-wrap">
                                          <span className="text-sm font-medium text-foreground">{model.name}</span>
                                          {currentLlmModel === model.id && (
                                            <Badge variant="default" className="text-xs">Active</Badge>
                                          )}
                                          {isDefault && (
                                            <Badge variant="outline" className="text-xs text-green-600 border-green-600">Default</Badge>
                                          )}
                                          {llmRecommendation?.recommendation === 'Recommended' && (
                                            <Badge className="bg-green-500/20 text-green-600 border-green-500/30 text-xs hover:bg-green-500/20 hover:text-green-600">
                                              Best for you
                                            </Badge>
                                          )}
                                          {llmRecommendation?.recommendation === 'NotRecommended' && (
                                            <Badge variant="outline" className="text-yellow-600 border-yellow-500/50 text-xs hover:text-yellow-600">
                                              May be slow
                                            </Badge>
                                          )}
                                          {llmRecommendation?.recommendation === 'TooHeavy' && (
                                            <Badge variant="outline" className="text-red-600 border-red-500/50 text-xs hover:text-red-600">
                                              Too demanding
                                            </Badge>
                                          )}
                                        </div>
                                        {model.description && (
                                          <p className="text-xs text-muted-foreground">{model.description}</p>
                                        )}
                                        {llmRecommendation?.reason && (
                                          <p className="text-xs text-muted-foreground">{llmRecommendation.reason}</p>
                                        )}
                                      </div>
                                      <div className="flex items-center gap-2">
                                        {!isDefault && (
                                          <Button
                                            variant="ghost"
                                            size="sm"
                                            onClick={async () => {
                                              setIsSavingDefault(true)
                                              try {
                                                await saveDefaultModel('ollama', model.id)
                                              } finally {
                                                setIsSavingDefault(false)
                                              }
                                            }}
                                            disabled={isSavingDefault}
                                            className="text-xs"
                                          >
                                            Set Default
                                          </Button>
                                        )}
                                        <Button
                                          variant={currentLlmModel === model.id ? 'secondary' : 'outline'}
                                          size="sm"
                                          onClick={() => handleLoadLlmModel(model.id)}
                                          disabled={isLoadingLlmModel || currentLlmModel === model.id}
                                        >
                                          {isLoadingLlmModel ? (
                                            <Loader2 className="h-4 w-4 animate-spin" />
                                          ) : currentLlmModel === model.id ? (
                                            <><Check className="h-4 w-4 mr-1" /> Active</>
                                          ) : (
                                            'Select'
                                          )}
                                        </Button>
                                      </div>
                                    </div>
                                  )
                                })}
                              </div>
                            )}
                          </div>
                        </>
                      )}
                    </>
                  )}
                </div>
              </Card>
            </TabsContent>

            {/* Speakers Tab */}
            <TabsContent value="speakers" className="space-y-6">
              <Card className="p-6">
                <div className="flex items-center gap-3 mb-6">
                  <div className="p-2 rounded-lg bg-primary/10">
                    <Users className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <h2 className="font-semibold text-foreground">Speaker Diarization</h2>
                    <p className="text-sm text-muted-foreground">Identify who said what in recordings</p>
                  </div>
                </div>

                <div className="space-y-6">
                  <div className="space-y-2">
                    <label className="text-sm font-medium text-foreground">Diarization Engine</label>
                    <Select value={diarizationProvider} onValueChange={(value: 'pyannote' | 'sortformer') => setDiarizationProvider(value)}>
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder="Select diarization engine" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="pyannote">PyAnnote - Unlimited speakers</SelectItem>
                        <SelectItem value="sortformer">Sortformer v2 - Up to 4 speakers</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>

                  <Separator />

                  <div className="space-y-3">
                    <label className="text-sm font-medium text-foreground">
                      {diarizationProvider === 'pyannote' ? 'PyAnnote Models' : 'Sortformer Model'}
                    </label>

                    {diarizationProvider === 'pyannote' ? (
                      <>
                        {diarizationModelsInfo.map((model) => (
                          <div key={model.name} className="flex items-center justify-between p-3 border border-border rounded-lg">
                            <div>
                              <span className="text-sm font-medium text-foreground">{model.name}</span>
                              <p className="text-xs text-muted-foreground">{model.size_mb}MB</p>
                            </div>
                            {model.is_downloaded ? (
                              <Badge variant="outline" className="text-green-600 border-green-600">
                                <Check className="h-3 w-3 mr-1" />Downloaded
                              </Badge>
                            ) : (
                              <Badge variant="secondary">Not Downloaded</Badge>
                            )}
                          </div>
                        ))}
                        {!diarizationModelsReady && (
                          <Button onClick={downloadDiarizationModels} disabled={isDiarizationDownloading} className="w-full">
                            {isDiarizationDownloading ? (
                              <><Loader2 className="h-4 w-4 mr-2 animate-spin" />Downloading... {diarizationDownloadProgress?.progress || 0}%</>
                            ) : (
                              <><Download className="h-4 w-4 mr-2" />Download PyAnnote Models (~32MB)</>
                            )}
                          </Button>
                        )}
                      </>
                    ) : (
                      <>
                        <div className="flex items-center justify-between p-3 border border-border rounded-lg">
                          <div>
                            <span className="text-sm font-medium text-foreground">Sortformer v2 ONNX</span>
                            <p className="text-xs text-muted-foreground">~25MB</p>
                          </div>
                          {sortformerModelReady ? (
                            <Badge variant="outline" className="text-green-600 border-green-600">
                              <Check className="h-3 w-3 mr-1" />Downloaded
                            </Badge>
                          ) : (
                            <Badge variant="secondary">Not Downloaded</Badge>
                          )}
                        </div>
                        {!sortformerModelReady && (
                          <Button onClick={handleDownloadSortformer} disabled={sortformerDownloading} className="w-full">
                            {sortformerDownloading ? (
                              <><Loader2 className="h-4 w-4 mr-2 animate-spin" />Downloading... {sortformerDownloadProgress}%</>
                            ) : (
                              <><Download className="h-4 w-4 mr-2" />Download Sortformer Model</>
                            )}
                          </Button>
                        )}
                        {sortformerDownloading && <Progress value={sortformerDownloadProgress} className="w-full" />}
                      </>
                    )}
                  </div>

                  <Separator />

                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <label className="text-sm font-medium text-foreground flex items-center gap-2">
                        <Users className="h-4 w-4" />Live Speaker Detection
                      </label>
                      <p className="text-xs text-muted-foreground">Identify speakers during live recording</p>
                    </div>
                    <Switch
                      checked={liveDiarizationEnabled}
                      onCheckedChange={handleLiveDiarizationChange}
                      disabled={diarizationProvider === 'pyannote' ? !diarizationModelsReady : !sortformerModelReady}
                    />
                  </div>

                  {diarizationProvider === 'pyannote' && (
                    <>
                      <Separator />
                      <div className="space-y-3">
                        <div className="flex items-center justify-between">
                          <label className="text-sm font-medium text-foreground">Registered Speakers</label>
                          <Badge variant="secondary">{registeredSpeakers.length} registered</Badge>
                        </div>
                        {registeredSpeakers.length === 0 ? (
                          <div className="text-center py-6 border border-dashed border-border rounded-lg">
                            <UserPlus className="h-8 w-8 mx-auto text-muted-foreground mb-2" />
                            <p className="text-sm text-muted-foreground">No speakers registered yet</p>
                          </div>
                        ) : (
                          <div className="space-y-2">
                            {registeredSpeakers.map((speaker) => (
                              <div key={speaker.id} className="flex items-center justify-between p-3 border border-border rounded-lg">
                                <div>
                                  <span className="text-sm font-medium text-foreground">{speaker.name}</span>
                                  <p className="text-xs text-muted-foreground">
                                    {speaker.sample_count} samples • Added {new Date(speaker.created_at).toLocaleDateString()}
                                  </p>
                                </div>
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  onClick={() => handleDeleteSpeaker(speaker.id)}
                                  disabled={isDeletingSpeaker === speaker.id}
                                  className="text-destructive hover:text-destructive hover:bg-destructive/10"
                                >
                                  {isDeletingSpeaker === speaker.id ? <Loader2 className="h-4 w-4 animate-spin" /> : <Trash2 className="h-4 w-4" />}
                                </Button>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    </>
                  )}
                </div>
              </Card>
            </TabsContent>

            {/* Audio Tab */}
            <TabsContent value="audio" className="space-y-6">
              <Card className="p-6">
                <div className="flex items-center gap-3 mb-6">
                  <div className="p-2 rounded-lg bg-primary/10">
                    <Mic className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <h2 className="font-semibold text-foreground">Audio Devices</h2>
                    <p className="text-sm text-muted-foreground">Configure default input and output devices</p>
                  </div>
                </div>

                <div className="space-y-6">
                  <div className="space-y-2">
                    <label className="text-sm font-medium text-foreground">Microphone</label>
                    <Select value={selectedMic} onValueChange={setSelectedMic}>
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder="Select microphone" />
                      </SelectTrigger>
                      <SelectContent>
                        {devices.microphones.map((device) => (
                          <SelectItem key={device.name} value={device.name}>{device.name}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>

                  <Separator />

                  <div className="space-y-2">
                    <label className="text-sm font-medium text-foreground">System Audio</label>
                    <Select value={selectedSystem} onValueChange={setSelectedSystem}>
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder="Select system audio" />
                      </SelectTrigger>
                      <SelectContent>
                        {devices.speakers.map((device) => (
                          <SelectItem key={device.name} value={device.name}>{device.name}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <p className="text-xs text-muted-foreground">Capture audio from other meeting participants through system audio.</p>
                  </div>
                </div>
              </Card>
            </TabsContent>

            {/* General Tab */}
            <TabsContent value="general" className="space-y-6">
              {/* System Information Card */}
              {hardwareInfo && (
                <Card className="p-6">
                  <div className="flex items-center gap-3 mb-4">
                    <div className="p-2 rounded-lg bg-primary/10">
                      <Cpu className="h-5 w-5 text-primary" />
                    </div>
                    <div>
                      <h2 className="font-semibold text-foreground">System Information</h2>
                      <p className="text-sm text-muted-foreground">Detected hardware capabilities</p>
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground">CPU</p>
                      <p className="text-sm font-medium text-foreground">{hardwareInfo.hardware.cpu_cores} Cores</p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground">Memory</p>
                      <p className="text-sm font-medium text-foreground">{hardwareInfo.hardware.memory_gb} GB RAM</p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground">GPU</p>
                      <p className="text-sm font-medium text-foreground">{hardwareInfo.hardware.has_gpu ? hardwareInfo.hardware.gpu_type : 'CPU Only'}</p>
                    </div>
                    <div className="space-y-1">
                      <p className="text-xs text-muted-foreground">Performance Tier</p>
                      <p className="text-sm font-medium text-foreground">{hardwareInfo.hardware.performance_tier}</p>
                    </div>
                  </div>

                  <p className="text-xs text-muted-foreground mt-4">{hardwareInfo.hardware.tier_description}</p>
                </Card>
              )}

              <Card className="p-6">
                <div className="flex items-center gap-3 mb-6">
                  <div className="p-2 rounded-lg bg-primary/10">
                    <FolderOpen className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <h2 className="font-semibold text-foreground">Storage</h2>
                    <p className="text-sm text-muted-foreground">Configure where recordings are saved</p>
                  </div>
                </div>

                <div className="space-y-4">
                  <div className="flex items-center justify-between p-4 bg-muted/50 rounded-lg">
                    <div>
                      <p className="text-sm font-medium text-foreground">Recordings Folder</p>
                      <p className="text-xs text-muted-foreground mt-1">~/Documents/Meeting Local/Recordings</p>
                    </div>
                    <Button variant="outline" size="sm">Change</Button>
                  </div>
                  <div className="flex items-center justify-between p-4 bg-muted/50 rounded-lg">
                    <div>
                      <p className="text-sm font-medium text-foreground">Transcripts Folder</p>
                      <p className="text-xs text-muted-foreground mt-1">~/Documents/Meeting Local/Transcripts</p>
                    </div>
                    <Button variant="outline" size="sm">Change</Button>
                  </div>
                </div>
              </Card>

              <Card className="p-6">
                <div className="flex items-center gap-3 mb-6">
                  <div className="p-2 rounded-lg bg-primary/10">
                    <Info className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <h2 className="font-semibold text-foreground">About</h2>
                    <p className="text-sm text-muted-foreground">Application information</p>
                  </div>
                </div>

                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Version</span>
                    <Badge variant="secondary">0.1.0</Badge>
                  </div>
                  <Separator />
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Whisper Engine</span>
                    <Badge variant="outline">whisper.cpp</Badge>
                  </div>
                  <Separator />
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Platform</span>
                    <Badge variant="outline">Tauri 2.x</Badge>
                  </div>
                </div>
              </Card>
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </>
  )
}
