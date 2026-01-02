// Chat types for AI conversations about recordings

export type ChatRole = 'system' | 'user' | 'assistant'

export type ChatMessageStatus = 'pending' | 'streaming' | 'complete' | 'cancelled' | 'error'

export interface ChatMessage {
  id: string
  recording_id: string
  session_id?: string
  role: ChatRole
  content: string
  created_at: string
  sequence_id: number
  status: ChatMessageStatus
  error_message?: string
  provider_type?: string
  model_id?: string
}

export interface ChatSession {
  id: string
  recording_id: string
  title: string
  created_at: string
  provider_type?: string
  model_id?: string
}

export interface ChatConfig {
  provider_type?: string
  model_id?: string
}

export interface DefaultLlmConfig {
  provider_type?: string
  model_id?: string
}

export interface SendMessageResponse {
  user_message_id: string
  assistant_message_id: string
}

export interface ChatMessageStatusResponse {
  message_id: string
  status: string
  content: string
  error_message?: string
}

export interface ChatStreamEvent {
  message_id: string
  token: string
  content: string
}

export interface ChatCompleteEvent {
  message_id: string
  status: 'complete' | 'error'
  error?: string
}
