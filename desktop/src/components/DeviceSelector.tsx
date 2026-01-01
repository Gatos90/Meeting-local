'use client'

interface Device {
  name: string
  is_default: boolean
}

interface DeviceSelectorProps {
  label: string
  devices: Device[]
  selectedDevice: string
  onSelect: (device: string) => void
  disabled?: boolean
}

export function DeviceSelector({
  label,
  devices,
  selectedDevice,
  onSelect,
  disabled = false,
}: DeviceSelectorProps) {
  return (
    <div className="mb-4">
      <label className="block text-sm font-medium text-gray-700 mb-2">
        {label}
      </label>
      <select
        value={selectedDevice}
        onChange={(e) => onSelect(e.target.value)}
        disabled={disabled}
        className="w-full px-4 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed"
      >
        <option value="">Select {label.toLowerCase()}...</option>
        {devices.map((device) => (
          <option key={device.name} value={device.name}>
            {device.name} {device.is_default ? '(Default)' : ''}
          </option>
        ))}
      </select>
    </div>
  )
}
