'use client'

import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface AudioDevice {
  name: string
  is_default: boolean
  device_type: string
}

interface DeviceList {
  microphones: AudioDevice[]
  speakers: AudioDevice[]
}

export function useAudioDevices() {
  const [devices, setDevices] = useState<DeviceList>({
    microphones: [],
    speakers: [],
  })
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchDevices = useCallback(async () => {
    try {
      setLoading(true)
      setError(null)

      const audioDevices = await invoke<AudioDevice[]>('get_audio_devices')

      // Separate devices by type
      const microphones = audioDevices.filter(
        (d) => d.device_type === 'input' || d.device_type === 'microphone'
      )
      const speakers = audioDevices.filter(
        (d) => d.device_type === 'output' || d.device_type === 'speaker' || d.device_type === 'system'
      )

      setDevices({ microphones, speakers })
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(`Failed to fetch audio devices: ${errorMessage}`)
      console.error('Fetch devices error:', err)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchDevices()
  }, [fetchDevices])

  return {
    devices,
    loading,
    error,
    refresh: fetchDevices,
  }
}
