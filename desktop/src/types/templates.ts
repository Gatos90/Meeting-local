// Types for prompt templates

export interface PromptTemplate {
  id: string
  name: string
  description?: string
  prompt: string
  icon?: string
  is_builtin: boolean
  sort_order: number
  created_at: string
}

export interface CreatePromptTemplate {
  name: string
  prompt: string
  description?: string
  icon?: string
  sort_order?: number
}

export interface UpdatePromptTemplate {
  name?: string
  prompt?: string
  description?: string
  icon?: string
  sort_order?: number
}
