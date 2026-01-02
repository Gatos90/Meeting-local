'use client'

import { useState, useEffect, ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Cpu, Mic, Volume2, Globe, X, Loader2, ChevronDown, ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { useSettings } from '@/hooks/useSettings'

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

// Whisper supported languages
const LANGUAGES = [
  { code: 'auto', name: 'Auto Detect' },
  { code: 'auto-translate', name: 'Auto Detect (Translate)' },
  { code: 'en', name: 'English' },
  { code: 'zh', name: 'Chinese' },
  { code: 'de', name: 'German' },
  { code: 'es', name: 'Spanish' },
  { code: 'ru', name: 'Russian' },
  { code: 'ko', name: 'Korean' },
  { code: 'fr', name: 'French' },
  { code: 'ja', name: 'Japanese' },
  { code: 'pt', name: 'Portuguese' },
  { code: 'tr', name: 'Turkish' },
  { code: 'pl', name: 'Polish' },
  { code: 'nl', name: 'Dutch' },
  { code: 'ar', name: 'Arabic' },
  { code: 'sv', name: 'Swedish' },
  { code: 'it', name: 'Italian' },
  { code: 'hi', name: 'Hindi' },
]

interface SettingsPopoverProps {
  children: ReactNode
  open: boolean
  onOpenChange: (open: boolean) => void
  micDevice?: string
  systemDevice?: string
  onMicChange: (device: string) => void
  onSystemChange: (device: string) => void
  microphones: Array<{ name: string }>
  speakers: Array<{ name: string }>
}

export function SettingsPopover({
  children,
  open,
  onOpenChange,
  micDevice,
  systemDevice,
  onMicChange,
  onSystemChange,
  microphones,
  speakers,
}: SettingsPopoverProps) {
  // Use the settings hook for persistent storage
  const {
    settings,
    setMicRnnoise,
    setMicHighpass,
    setMicNormalizer,
    setSysRnnoise,
    setSysHighpass,
    setSysNormalizer,
    setLastMicrophone,
    setLastSystemAudio,
    setLanguage: saveLanguage,
    setCurrentModel: saveCurrentModel,
  } = useSettings()

  const [currentModel, setCurrentModel] = useState<string>('base')
  const [language, setLanguage] = useState<string>('auto')
  const [availableModels, setAvailableModels] = useState<ModelInfo[]>([])
  const [isLoadingModel, setIsLoadingModel] = useState(false)

  // Expandable sections state
  const [micExpanded, setMicExpanded] = useState(false)
  const [sysExpanded, setSysExpanded] = useState(false)

  // Sync local state with persisted settings
  useEffect(() => {
    if (settings.currentModel) {
      setCurrentModel(settings.currentModel)
    }
    if (settings.language) {
      setLanguage(settings.language)
    }
  }, [settings.currentModel, settings.language])

  // Load model-related settings on mount (these still use original Tauri commands)
  useEffect(() => {
    const loadModelSettings = async () => {
      // Load available models
      try {
        const models = await invoke<ModelInfo[]>('whisper_get_available_models')
        const available = models.filter(m => m.status === 'Available')
        setAvailableModels(available)
      } catch (err) {
        console.error('Failed to load available models:', err)
      }

      // Load current model from Whisper engine
      try {
        const model = await invoke<string | null>('whisper_get_current_model')
        if (model) setCurrentModel(model)
      } catch (err) {
        console.error('Failed to load model:', err)
      }

      // Load language preference from Whisper engine
      try {
        const lang = await invoke<string | null>('get_language_preference')
        if (lang) setLanguage(lang)
      } catch (err) {
        console.error('Failed to load language:', err)
      }
    }

    if (open) {
      loadModelSettings()
    }
  }, [open])

  const handleModelChange = async (modelName: string) => {
    setIsLoadingModel(true)
    try {
      await invoke('whisper_load_model', { modelName })
      setCurrentModel(modelName)
      // Save to database for persistence
      await saveCurrentModel(modelName)
    } catch (err) {
      console.error('Failed to load model:', err)
    } finally {
      setIsLoadingModel(false)
    }
  }

  const handleLanguageChange = async (value: string) => {
    setLanguage(value)
    try {
      await invoke('set_language_preference', { language: value })
      // Save to database for persistence
      await saveLanguage(value)
    } catch (err) {
      console.error('Failed to save language:', err)
    }
  }

  // Device change handlers - save to database
  const handleMicChange = async (device: string) => {
    onMicChange(device)
    try {
      await setLastMicrophone(device)
    } catch (err) {
      console.error('Failed to save last microphone:', err)
    }
  }

  const handleSystemChange = async (device: string) => {
    onSystemChange(device)
    try {
      await setLastSystemAudio(device)
    } catch (err) {
      console.error('Failed to save last system audio:', err)
    }
  }

  // Microphone processing handlers - save to database AND update runtime setting
  const handleMicRnnoiseChange = async (enabled: boolean) => {
    try {
      await invoke('set_mic_rnnoise_enabled', { enabled })
      await setMicRnnoise(enabled)
    } catch (err) {
      console.error('Failed to save mic rnnoise setting:', err)
    }
  }

  const handleMicHighpassChange = async (enabled: boolean) => {
    try {
      await invoke('set_mic_highpass_enabled', { enabled })
      await setMicHighpass(enabled)
    } catch (err) {
      console.error('Failed to save mic highpass setting:', err)
    }
  }

  const handleMicNormalizerChange = async (enabled: boolean) => {
    try {
      await invoke('set_mic_normalizer_enabled', { enabled })
      await setMicNormalizer(enabled)
    } catch (err) {
      console.error('Failed to save mic normalizer setting:', err)
    }
  }

  // System audio processing handlers - save to database AND update runtime setting
  const handleSysRnnoiseChange = async (enabled: boolean) => {
    try {
      await invoke('set_sys_rnnoise_enabled', { enabled })
      await setSysRnnoise(enabled)
    } catch (err) {
      console.error('Failed to save sys rnnoise setting:', err)
    }
  }

  const handleSysHighpassChange = async (enabled: boolean) => {
    try {
      await invoke('set_sys_highpass_enabled', { enabled })
      await setSysHighpass(enabled)
    } catch (err) {
      console.error('Failed to save sys highpass setting:', err)
    }
  }

  const handleSysNormalizerChange = async (enabled: boolean) => {
    try {
      await invoke('set_sys_normalizer_enabled', { enabled })
      await setSysNormalizer(enabled)
    } catch (err) {
      console.error('Failed to save sys normalizer setting:', err)
    }
  }

  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <PopoverTrigger asChild>
        {children}
      </PopoverTrigger>
      <PopoverContent
        className="w-80 p-4 bg-white rounded-2xl border border-border shadow-xl max-h-[85vh] overflow-y-auto"
        side="top"
        align="start"
        sideOffset={16}
      >
        <div className="flex justify-between items-center mb-4">
          <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Configuration
          </span>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={() => onOpenChange(false)}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

        <div className="space-y-4">
          {/* Model Selection */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-foreground flex items-center gap-2">
              <Cpu className="h-3 w-3" /> Model
              {isLoadingModel && <Loader2 className="h-3 w-3 animate-spin" />}
            </label>
            <Select
              value={currentModel}
              onValueChange={handleModelChange}
              disabled={isLoadingModel || availableModels.length === 0}
            >
              <SelectTrigger className="bg-muted/50 border-border">
                <SelectValue placeholder={availableModels.length === 0 ? "No models available" : "Select model"} />
              </SelectTrigger>
              <SelectContent>
                {availableModels.map((model) => (
                  <SelectItem key={model.name} value={model.name}>
                    {model.name} ({model.speed})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Language Selection */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-foreground flex items-center gap-2">
              <Globe className="h-3 w-3" /> Language
            </label>
            <Select value={language} onValueChange={handleLanguageChange}>
              <SelectTrigger className="bg-muted/50 border-border">
                <SelectValue placeholder="Select language" />
              </SelectTrigger>
              <SelectContent>
                {LANGUAGES.map((lang) => (
                  <SelectItem key={lang.code} value={lang.code}>
                    {lang.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Microphone Section */}
          <div className="space-y-2 pt-2 border-t border-border">
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-foreground flex items-center gap-2">
                <Mic className="h-3 w-3" /> Input Device
              </label>
              <Select value={micDevice || ''} onValueChange={handleMicChange}>
                <SelectTrigger className="bg-muted/50 border-border">
                  <SelectValue placeholder="Select microphone" />
                </SelectTrigger>
                <SelectContent>
                  {microphones.map((mic) => (
                    <SelectItem key={mic.name} value={mic.name}>
                      {mic.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Expandable Processing Options */}
            <button
              onClick={() => setMicExpanded(!micExpanded)}
              className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors"
            >
              {micExpanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
              Processing Options
            </button>

            {micExpanded && (
              <div className="pl-4 space-y-1.5 animate-in slide-in-from-top-1 duration-200">
                <div className="flex items-center justify-between py-0.5">
                  <span className="text-[11px] text-muted-foreground">Noise Suppression</span>
                  <Switch
                    checked={settings.micRnnoise}
                    onCheckedChange={handleMicRnnoiseChange}
                    className="scale-75"
                  />
                </div>
                <div className="flex items-center justify-between py-0.5">
                  <span className="text-[11px] text-muted-foreground">High-pass Filter</span>
                  <Switch
                    checked={settings.micHighpass}
                    onCheckedChange={handleMicHighpassChange}
                    className="scale-75"
                  />
                </div>
                <div className="flex items-center justify-between py-0.5">
                  <span className="text-[11px] text-muted-foreground">Loudness Normalizer</span>
                  <Switch
                    checked={settings.micNormalizer}
                    onCheckedChange={handleMicNormalizerChange}
                    className="scale-75"
                  />
                </div>
              </div>
            )}
          </div>

          {/* System Audio Section */}
          <div className="space-y-2 pt-2 border-t border-border">
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-foreground flex items-center gap-2">
                <Volume2 className="h-3 w-3" /> Output Device
              </label>
              <Select value={systemDevice || ''} onValueChange={handleSystemChange}>
                <SelectTrigger className="bg-muted/50 border-border">
                  <SelectValue placeholder="Select system audio" />
                </SelectTrigger>
                <SelectContent>
                  {speakers.map((speaker) => (
                    <SelectItem key={speaker.name} value={speaker.name}>
                      {speaker.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Expandable Processing Options */}
            <button
              onClick={() => setSysExpanded(!sysExpanded)}
              className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors"
            >
              {sysExpanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
              Processing Options
            </button>

            {sysExpanded && (
              <div className="pl-4 space-y-1.5 animate-in slide-in-from-top-1 duration-200">
                <div className="flex items-center justify-between py-0.5">
                  <span className="text-[11px] text-muted-foreground">Noise Suppression</span>
                  <Switch
                    checked={settings.sysRnnoise}
                    onCheckedChange={handleSysRnnoiseChange}
                    className="scale-75"
                  />
                </div>
                <div className="flex items-center justify-between py-0.5">
                  <span className="text-[11px] text-muted-foreground">High-pass Filter</span>
                  <Switch
                    checked={settings.sysHighpass}
                    onCheckedChange={handleSysHighpassChange}
                    className="scale-75"
                  />
                </div>
                <div className="flex items-center justify-between py-0.5">
                  <span className="text-[11px] text-muted-foreground">Loudness Normalizer</span>
                  <Switch
                    checked={settings.sysNormalizer}
                    onCheckedChange={handleSysNormalizerChange}
                    className="scale-75"
                  />
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="mt-4 pt-3 border-t border-border">
          <p className="text-[10px] text-muted-foreground text-center">
            Changes apply to next recording
          </p>
        </div>
      </PopoverContent>
    </Popover>
  )
}
