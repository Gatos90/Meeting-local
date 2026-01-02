'use client'

import { useState, useCallback, KeyboardEvent } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Send, Square, Loader2 } from 'lucide-react'

interface ChatInputProps {
  onSend: (message: string) => void
  onCancel: () => void
  isProcessing: boolean
  disabled?: boolean
  placeholder?: string
}

export function ChatInput({
  onSend,
  onCancel,
  isProcessing,
  disabled = false,
  placeholder = 'Ask a question about this recording...'
}: ChatInputProps) {
  const [input, setInput] = useState('')

  const handleSend = useCallback(() => {
    const trimmed = input.trim()
    if (trimmed && !isProcessing && !disabled) {
      onSend(trimmed)
      setInput('')
    }
  }, [input, isProcessing, disabled, onSend])

  const handleKeyDown = useCallback((e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }, [handleSend])

  return (
    <div className="flex gap-2">
      <Input
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        disabled={disabled || isProcessing}
        className="flex-1"
      />

      {isProcessing ? (
        <Button
          variant="destructive"
          size="icon"
          onClick={onCancel}
          title="Cancel"
        >
          <Square className="w-4 h-4" />
        </Button>
      ) : (
        <Button
          size="icon"
          onClick={handleSend}
          disabled={!input.trim() || disabled}
          title="Send"
        >
          <Send className="w-4 h-4" />
        </Button>
      )}
    </div>
  )
}
