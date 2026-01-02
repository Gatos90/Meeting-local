'use client'

import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { AllSettings, SettingsState } from '@/types/database'

// Map from snake_case (database) to camelCase (frontend state)
const snakeToCamelMap: Record<string, keyof SettingsState> = {
  'language': 'language',
  'mic_rnnoise': 'micRnnoise',
  'mic_highpass': 'micHighpass',
  'mic_normalizer': 'micNormalizer',
  'sys_rnnoise': 'sysRnnoise',
  'sys_highpass': 'sysHighpass',
  'sys_normalizer': 'sysNormalizer',
  'last_microphone': 'lastMicrophone',
  'last_system_audio': 'lastSystemAudio',
  'recordings_folder': 'recordingsFolder',
  'current_model': 'currentModel',
}

// Convert snake_case settings from Rust to camelCase for frontend
function convertToCamelCase(settings: AllSettings): SettingsState {
  return {
    language: settings.language,
    micRnnoise: settings.mic_rnnoise,
    micHighpass: settings.mic_highpass,
    micNormalizer: settings.mic_normalizer,
    sysRnnoise: settings.sys_rnnoise,
    sysHighpass: settings.sys_highpass,
    sysNormalizer: settings.sys_normalizer,
    lastMicrophone: settings.last_microphone,
    lastSystemAudio: settings.last_system_audio,
    recordingsFolder: settings.recordings_folder,
    currentModel: settings.current_model,
  }
}

// Default settings
const defaultSettings: SettingsState = {
  language: null,
  micRnnoise: false,
  micHighpass: false,
  micNormalizer: false,
  sysRnnoise: false,
  sysHighpass: false,
  sysNormalizer: false,
  lastMicrophone: null,
  lastSystemAudio: null,
  recordingsFolder: null,
  currentModel: null,
}

export function useSettings() {
  const [settings, setSettings] = useState<SettingsState>(defaultSettings)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Load settings on mount
  const loadSettings = useCallback(async () => {
    try {
      setLoading(true)
      setError(null)
      const allSettings = await invoke<AllSettings>('db_load_settings_on_startup')
      setSettings(convertToCamelCase(allSettings))
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to load settings: ${errorMessage}`)
      console.error('Load settings error:', err)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    loadSettings()
  }, [loadSettings])

  // Update a string setting
  const updateSetting = useCallback(async (key: string, value: string | null) => {
    try {
      setError(null)
      await invoke('db_set_setting', {
        key,
        value: value ?? '',
        valueType: 'string'
      })

      // Update local state using camelCase key
      const camelKey = snakeToCamelMap[key] || key
      setSettings(prev => ({
        ...prev,
        [camelKey]: value,
      }))

      console.log(`Setting saved: ${key} = ${value}`)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to save setting: ${errorMessage}`)
      console.error('Save setting error:', err)
      throw err
    }
  }, [])

  // Update a boolean setting (stored as "true"/"false" strings)
  const updateBoolSetting = useCallback(async (key: string, value: boolean) => {
    try {
      setError(null)
      await invoke('db_set_setting', {
        key,
        value: value ? 'true' : 'false',
        valueType: 'boolean'
      })

      // Update local state using camelCase key
      const camelKey = snakeToCamelMap[key] || key
      setSettings(prev => ({
        ...prev,
        [camelKey]: value,
      }))

      console.log(`Setting saved: ${key} = ${value}`)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to save setting: ${errorMessage}`)
      console.error('Save setting error:', err)
      throw err
    }
  }, [])

  // Convenience methods for audio processing settings
  const setMicRnnoise = useCallback((enabled: boolean) =>
    updateBoolSetting('mic_rnnoise', enabled), [updateBoolSetting])

  const setMicHighpass = useCallback((enabled: boolean) =>
    updateBoolSetting('mic_highpass', enabled), [updateBoolSetting])

  const setMicNormalizer = useCallback((enabled: boolean) =>
    updateBoolSetting('mic_normalizer', enabled), [updateBoolSetting])

  const setSysRnnoise = useCallback((enabled: boolean) =>
    updateBoolSetting('sys_rnnoise', enabled), [updateBoolSetting])

  const setSysHighpass = useCallback((enabled: boolean) =>
    updateBoolSetting('sys_highpass', enabled), [updateBoolSetting])

  const setSysNormalizer = useCallback((enabled: boolean) =>
    updateBoolSetting('sys_normalizer', enabled), [updateBoolSetting])

  // Save last used devices
  const setLastMicrophone = useCallback((device: string | null) =>
    updateSetting('last_microphone', device), [updateSetting])

  const setLastSystemAudio = useCallback((device: string | null) =>
    updateSetting('last_system_audio', device), [updateSetting])

  // Save other settings
  const setRecordingsFolder = useCallback((folder: string | null) =>
    updateSetting('recordings_folder', folder), [updateSetting])

  const setCurrentModel = useCallback((model: string | null) =>
    updateSetting('current_model', model), [updateSetting])

  const setLanguage = useCallback((language: string | null) =>
    updateSetting('language', language), [updateSetting])

  return {
    settings,
    loading,
    error,
    refresh: loadSettings,
    // Generic setters
    updateSetting,
    updateBoolSetting,
    // Convenience setters
    setMicRnnoise,
    setMicHighpass,
    setMicNormalizer,
    setSysRnnoise,
    setSysHighpass,
    setSysNormalizer,
    setLastMicrophone,
    setLastSystemAudio,
    setRecordingsFolder,
    setCurrentModel,
    setLanguage,
  }
}
