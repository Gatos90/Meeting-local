// Database types for Meeting-Local
// These types match the Rust database models

export interface Recording {
  id: string
  title: string
  created_at: string
  completed_at: string | null
  duration_seconds: number | null
  status: string
  audio_file_path: string | null
  meeting_folder_path: string | null
  microphone_device: string | null
  system_audio_device: string | null
  sample_rate: number
  transcription_model: string | null
  language: string | null
  diarization_provider: string | null
}

export interface Category {
  id: string
  name: string
  color: string | null
  is_system: boolean
}

export interface Tag {
  id: string
  name: string
  color: string | null
  usage_count: number
}

export interface RecordingWithMetadata {
  recording: Recording
  categories: Category[]
  tags: Tag[]
  transcript_count: number
}

export interface TranscriptSegment {
  id: string
  recording_id: string
  text: string
  audio_start_time: number
  audio_end_time: number
  duration: number
  display_time: string
  confidence: number
  sequence_id: number
  // Speaker diarization fields (optional)
  speaker_id?: string | null
  speaker_label?: string | null
  is_registered_speaker?: boolean
}

// Speaker colors for visual differentiation
export const SPEAKER_COLORS = [
  '#3B82F6', // blue
  '#10B981', // green
  '#F59E0B', // amber
  '#EF4444', // red
  '#8B5CF6', // purple
  '#EC4899', // pink
  '#06B6D4', // cyan
  '#F97316', // orange
] as const

// Get color for a speaker based on their ID
export function getSpeakerColor(speakerId: string | null | undefined): string {
  if (!speakerId) return '#6B7280' // gray for unknown
  // Use a simple hash to get consistent colors for the same speaker
  let hash = 0
  for (let i = 0; i < speakerId.length; i++) {
    hash = speakerId.charCodeAt(i) + ((hash << 5) - hash)
  }
  return SPEAKER_COLORS[Math.abs(hash) % SPEAKER_COLORS.length]
}

export interface SearchFilters {
  category_ids?: string[]
  tag_ids?: string[]
  date_from?: string
  date_to?: string
  search_transcripts: boolean
}

export interface SearchResult {
  recording: Recording
  matched_text: string
  categories: Category[]
  tags: Tag[]
}

export interface AllSettings {
  language: string | null
  mic_rnnoise: boolean
  mic_highpass: boolean
  mic_normalizer: boolean
  sys_rnnoise: boolean
  sys_highpass: boolean
  sys_normalizer: boolean
  last_microphone: string | null
  last_system_audio: string | null
  recordings_folder: string | null
  current_model: string | null
}

// Camelcase versions for frontend use
export interface SettingsState {
  language: string | null
  micRnnoise: boolean
  micHighpass: boolean
  micNormalizer: boolean
  sysRnnoise: boolean
  sysHighpass: boolean
  sysNormalizer: boolean
  lastMicrophone: string | null
  lastSystemAudio: string | null
  recordingsFolder: string | null
  currentModel: string | null
}

// Recording update payload
export interface RecordingUpdate {
  title?: string
  status?: string
  completed_at?: string
  duration_seconds?: number
  audio_file_path?: string
  meeting_folder_path?: string
  transcription_model?: string
  diarization_provider?: string
}
