'use client'

import { useState, useEffect } from 'react'
import { useTemplates } from '@/hooks/useTemplates'
import { useLlm, ProviderType } from '@/hooks/useLlm'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import { Card } from '@/components/ui/card'
import { MarkdownPreview } from '@/components/ui/markdown-preview'
import { LlmProviderSelector, useLlmSelection } from '@/components/ui/llm-provider-selector'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Plus,
  Copy,
  Pencil,
  Trash2,
  FileText,
  List,
  CheckSquare,
  MessageSquare,
  Sparkles,
  ClipboardList,
  Users,
  Target,
  Calendar,
  Lightbulb,
  Loader2,
  LucideIcon,
  RefreshCw,
  Eye,
  Code,
  Filter,
} from 'lucide-react'
import type { PromptTemplate, CreatePromptTemplate, UpdatePromptTemplate } from '@/types/templates'

// Available icons for templates
const availableIcons = [
  { value: 'FileText', label: 'Document', icon: FileText },
  { value: 'List', label: 'List', icon: List },
  { value: 'CheckSquare', label: 'Checklist', icon: CheckSquare },
  { value: 'MessageSquare', label: 'Message', icon: MessageSquare },
  { value: 'Sparkles', label: 'Sparkles', icon: Sparkles },
  { value: 'ClipboardList', label: 'Clipboard', icon: ClipboardList },
  { value: 'Users', label: 'Users', icon: Users },
  { value: 'Target', label: 'Target', icon: Target },
  { value: 'Calendar', label: 'Calendar', icon: Calendar },
  { value: 'Lightbulb', label: 'Lightbulb', icon: Lightbulb },
]

// Map icon names to components
const iconMap: Record<string, LucideIcon> = {
  FileText,
  List,
  CheckSquare,
  MessageSquare,
  Sparkles,
  ClipboardList,
  Users,
  Target,
  Calendar,
  Lightbulb,
}

type FilterType = 'all' | 'builtin' | 'custom'

export default function TemplatesPage() {
  const {
    templates,
    builtinTemplates,
    customTemplates,
    isLoading,
    error,
    createTemplate,
    updateTemplate,
    deleteTemplate,
    duplicateTemplate,
  } = useTemplates()

  const {
    complete,
    activeProvider,
    currentModel,
    selectProvider,
    initializeModel,
    providers,
    ollamaConnected,
  } = useLlm()

  // Use the reusable LLM selection hook
  const llmSelection = useLlmSelection()

  // Filter state
  const [filterType, setFilterType] = useState<FilterType>('all')

  // Dialog state
  const [isEditorOpen, setIsEditorOpen] = useState(false)
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false)
  const [editingTemplate, setEditingTemplate] = useState<PromptTemplate | null>(null)
  const [templateToDelete, setTemplateToDelete] = useState<PromptTemplate | null>(null)
  const [isSaving, setIsSaving] = useState(false)

  // Form state
  const [formName, setFormName] = useState('')
  const [formDescription, setFormDescription] = useState('')
  const [formPrompt, setFormPrompt] = useState('')
  const [formIcon, setFormIcon] = useState('MessageSquare')

  // AI generation state
  const [isGenerating, setIsGenerating] = useState(false)
  const [generationError, setGenerationError] = useState<string | null>(null)
  const [showRawEditor, setShowRawEditor] = useState(false)

  // Reset LLM selection when dialog opens for new template
  useEffect(() => {
    if (isEditorOpen && !editingTemplate) {
      llmSelection.reset()
    }
  }, [isEditorOpen, editingTemplate])

  // Check if any providers are available
  const hasProviders = providers.some(p => {
    if (p.provider_type === 'ollama') return ollamaConnected
    if (p.provider_type === 'embedded') return true
    return p.is_available
  })

  // Get icon component
  const getIcon = (iconName?: string): LucideIcon => {
    if (iconName && iconMap[iconName]) {
      return iconMap[iconName]
    }
    return MessageSquare
  }

  // Filter templates based on selected filter
  const filteredTemplates = filterType === 'all'
    ? templates
    : filterType === 'builtin'
      ? builtinTemplates
      : customTemplates

  // Check if we can generate (provider and model selected)
  const canGenerate = llmSelection.hasSelection && formDescription.trim()

  // Generate template with AI
  const handleGenerateWithAI = async () => {
    if (!formDescription.trim() || !llmSelection.selectedProvider || !llmSelection.selectedModel) return

    try {
      setIsGenerating(true)
      setGenerationError(null)

      // Switch provider if needed
      if (llmSelection.selectedProvider !== activeProvider) {
        await selectProvider(llmSelection.selectedProvider)
      }

      // Initialize model if needed
      if (llmSelection.selectedModel !== currentModel) {
        await initializeModel(llmSelection.selectedModel)
      }

      const systemPrompt = `You are an expert at creating prompt templates for analyzing meeting transcripts.
Create clear, well-structured markdown prompts that guide an AI to extract useful information from meetings.

Your templates should:
- Use clear markdown headers (##) to organize sections
- Include specific instructions for what to extract
- Specify output formats (bullet lists, numbered lists, etc.)
- Be focused and actionable
- Be comprehensive but not overly long

Return ONLY the prompt template text in markdown format. Do not include any explanations or meta-commentary.`

      const userPrompt = `Create a prompt template for analyzing meeting transcripts based on this description:

"${formDescription}"

The template should help extract relevant information from the meeting transcript. Generate a well-structured markdown prompt.`

      const response = await complete({
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: userPrompt }
        ],
        max_tokens: 1500,
        temperature: 0.7,
      })

      setFormPrompt(response.content)
    } catch (err) {
      console.error('Failed to generate template:', err)
      setGenerationError(String(err))
    } finally {
      setIsGenerating(false)
    }
  }

  // Open editor for creating new template
  const handleCreate = () => {
    setEditingTemplate(null)
    setFormName('')
    setFormDescription('')
    setFormPrompt('')
    setFormIcon('MessageSquare')
    setGenerationError(null)
    setShowRawEditor(false)
    setIsEditorOpen(true)
  }

  // Open editor for editing existing template
  const handleEdit = (template: PromptTemplate) => {
    setEditingTemplate(template)
    setFormName(template.name)
    setFormDescription(template.description || '')
    setFormPrompt(template.prompt)
    setFormIcon(template.icon || 'MessageSquare')
    setGenerationError(null)
    setShowRawEditor(true) // Show raw editor when editing existing
    setIsEditorOpen(true)
  }

  // Handle duplicate
  const handleDuplicate = async (template: PromptTemplate) => {
    try {
      setIsSaving(true)
      await duplicateTemplate(template.id)
    } finally {
      setIsSaving(false)
    }
  }

  // Confirm delete
  const handleDeleteClick = (template: PromptTemplate) => {
    setTemplateToDelete(template)
    setIsDeleteDialogOpen(true)
  }

  // Execute delete
  const handleConfirmDelete = async () => {
    if (!templateToDelete) return
    try {
      setIsSaving(true)
      await deleteTemplate(templateToDelete.id)
    } finally {
      setIsSaving(false)
      setIsDeleteDialogOpen(false)
      setTemplateToDelete(null)
    }
  }

  // Save template (create or update)
  const handleSave = async () => {
    if (!formName.trim() || !formPrompt.trim()) return

    try {
      setIsSaving(true)
      if (editingTemplate) {
        // Update existing template
        const updates: UpdatePromptTemplate = {
          name: formName,
          description: formDescription || undefined,
          prompt: formPrompt,
          icon: formIcon,
        }
        await updateTemplate(editingTemplate.id, updates)
      } else {
        // Create new template
        const input: CreatePromptTemplate = {
          name: formName,
          description: formDescription || undefined,
          prompt: formPrompt,
          icon: formIcon,
        }
        await createTemplate(input)
      }
      setIsEditorOpen(false)
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <>
      {/* Header */}
      <header className="flex h-16 items-center justify-between border-b border-border bg-background/50 backdrop-blur-sm px-8">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-semibold text-foreground">Prompt Templates</h1>
          <Badge variant="secondary">
            {isLoading ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              `${templates.length} templates`
            )}
          </Badge>
        </div>

        <Button onClick={handleCreate}>
          <Plus className="w-4 h-4 mr-2" />
          Create Template
        </Button>
      </header>

      {/* Main Content */}
      <div className="flex-1 overflow-y-auto p-8">
        <div className="mx-auto max-w-6xl space-y-6">
          {/* Filter Buttons */}
          <div className="flex items-center gap-2 flex-wrap">
            <Filter className="h-4 w-4 text-muted-foreground" />
            <Button
              variant={filterType === 'all' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilterType('all')}
              className="h-7"
            >
              All
            </Button>
            <Button
              variant={filterType === 'builtin' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilterType('builtin')}
              className="h-7"
            >
              Built-in
            </Button>
            <Button
              variant={filterType === 'custom' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilterType('custom')}
              className="h-7"
            >
              Custom
            </Button>
          </div>

          {/* Error State */}
          {error && (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <div className="p-4 rounded-full bg-destructive/10 mb-4">
                <FileText className="h-8 w-8 text-destructive" />
              </div>
              <h3 className="text-lg font-medium text-foreground">
                Error loading templates
              </h3>
              <p className="text-muted-foreground mt-1">{error}</p>
            </div>
          )}

          {/* Loading State */}
          {isLoading && !error && (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mb-4" />
              <p className="text-muted-foreground">Loading templates...</p>
            </div>
          )}

          {/* Empty State */}
          {!isLoading && !error && filteredTemplates.length === 0 && (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <div className="p-4 rounded-full bg-muted mb-4">
                <FileText className="h-8 w-8 text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium text-foreground">
                {filterType === 'custom' ? 'No custom templates yet' : 'No templates found'}
              </h3>
              <p className="text-muted-foreground mt-1">
                {filterType === 'custom'
                  ? 'Create your first custom template to get started'
                  : 'Try adjusting your filter'}
              </p>
            </div>
          )}

          {/* Template Grid */}
          {!isLoading && !error && filteredTemplates.length > 0 && (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              {filteredTemplates.map((template) => {
                const Icon = getIcon(template.icon || undefined)
                return (
                  <Card
                    key={template.id}
                    className="p-4 hover:shadow-md transition-shadow cursor-pointer group"
                    onClick={() => !template.is_builtin && handleEdit(template)}
                  >
                    <div className="flex items-start gap-3">
                      <div className="flex-shrink-0 w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                        <Icon className="w-5 h-5 text-primary" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <h3 className="font-medium truncate">{template.name}</h3>
                          {template.is_builtin && (
                            <Badge variant="secondary" className="flex-shrink-0 text-xs">
                              Built-in
                            </Badge>
                          )}
                        </div>
                        {template.description && (
                          <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
                            {template.description}
                          </p>
                        )}
                        <p className="text-xs text-muted-foreground/60 mt-2 line-clamp-2 font-mono">
                          {template.prompt.slice(0, 100)}...
                        </p>
                      </div>
                    </div>

                    {/* Action Buttons */}
                    <div className="flex items-center justify-end gap-1 mt-4 pt-3 border-t opacity-0 group-hover:opacity-100 transition-opacity">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation()
                          handleDuplicate(template)
                        }}
                        disabled={isSaving}
                      >
                        <Copy className="w-4 h-4 mr-1" />
                        Duplicate
                      </Button>
                      {!template.is_builtin && (
                        <>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation()
                              handleEdit(template)
                            }}
                            disabled={isSaving}
                          >
                            <Pencil className="w-4 h-4 mr-1" />
                            Edit
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation()
                              handleDeleteClick(template)
                            }}
                            disabled={isSaving}
                            className="text-destructive hover:text-destructive"
                          >
                            <Trash2 className="w-4 h-4 mr-1" />
                            Delete
                          </Button>
                        </>
                      )}
                    </div>
                  </Card>
                )
              })}
            </div>
          )}
        </div>

        {/* Template Editor Dialog */}
        <Dialog open={isEditorOpen} onOpenChange={setIsEditorOpen}>
          <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
            <DialogHeader>
              <DialogTitle>
                {editingTemplate ? 'Edit Template' : 'Create Template'}
              </DialogTitle>
              <DialogDescription>
                {editingTemplate
                  ? 'Update your custom template'
                  : 'Describe what you want and let AI generate the prompt template'
                }
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              {/* Name field */}
              <div className="space-y-2">
                <Label htmlFor="name">Name</Label>
                <Input
                  id="name"
                  value={formName}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) => setFormName(e.target.value)}
                  placeholder="e.g., Weekly Team Meeting"
                />
              </div>

              {/* Description / AI generation input */}
              <div className="space-y-2">
                <Label htmlFor="description">
                  {editingTemplate ? 'Description (optional)' : 'Describe what you want'}
                </Label>
                <Textarea
                  id="description"
                  value={formDescription}
                  onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setFormDescription(e.target.value)}
                  placeholder={editingTemplate
                    ? "Brief description of this template"
                    : "e.g., I want a template for weekly team meetings that captures decisions, action items, blockers, and follow-ups for each team member..."
                  }
                  rows={3}
                />
                {!editingTemplate && (
                  <div className="space-y-3">
                    {/* Provider and Model Selection - Using reusable component */}
                    <LlmProviderSelector
                      selectedProvider={llmSelection.selectedProvider}
                      selectedModel={llmSelection.selectedModel}
                      onProviderChange={llmSelection.setSelectedProvider}
                      onModelChange={llmSelection.setSelectedModel}
                      disabled={isGenerating}
                      compact
                    />

                    {/* Generate buttons */}
                    <div className="flex items-center gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleGenerateWithAI}
                        disabled={!canGenerate || isGenerating}
                      >
                        {isGenerating ? (
                          <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                        ) : (
                          <Sparkles className="w-4 h-4 mr-2" />
                        )}
                        {isGenerating ? 'Generating...' : 'Generate with AI'}
                      </Button>
                      {formPrompt && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={handleGenerateWithAI}
                          disabled={!canGenerate || isGenerating}
                        >
                          <RefreshCw className="w-4 h-4 mr-2" />
                          Regenerate
                        </Button>
                      )}
                    </div>

                    {!hasProviders && (
                      <p className="text-xs text-muted-foreground">
                        Configure an AI provider in Settings to enable generation
                      </p>
                    )}
                  </div>
                )}
                {generationError && (
                  <p className="text-sm text-destructive">{generationError}</p>
                )}
              </div>

              {/* Generated prompt preview/edit */}
              {(formPrompt || editingTemplate) && (
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label>Prompt Template</Label>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setShowRawEditor(!showRawEditor)}
                    >
                      {showRawEditor ? (
                        <>
                          <Eye className="w-4 h-4 mr-2" />
                          Preview
                        </>
                      ) : (
                        <>
                          <Code className="w-4 h-4 mr-2" />
                          Edit
                        </>
                      )}
                    </Button>
                  </div>

                  {showRawEditor ? (
                    <Textarea
                      value={formPrompt}
                      onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setFormPrompt(e.target.value)}
                      placeholder="Enter your prompt template in markdown..."
                      rows={10}
                      className="font-mono text-sm"
                    />
                  ) : (
                    <div className="border rounded-md p-4 bg-muted/30 max-h-[300px] overflow-y-auto">
                      {formPrompt ? (
                        <MarkdownPreview content={formPrompt} />
                      ) : (
                        <p className="text-sm text-muted-foreground italic">
                          No prompt generated yet. Describe what you want above and click "Generate with AI".
                        </p>
                      )}
                    </div>
                  )}
                </div>
              )}

              {/* Icon selector */}
              <div className="space-y-2">
                <Label htmlFor="icon">Icon</Label>
                <Select value={formIcon} onValueChange={setFormIcon}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {availableIcons.map((icon) => {
                      const IconComponent = icon.icon
                      return (
                        <SelectItem key={icon.value} value={icon.value}>
                          <div className="flex items-center gap-2">
                            <IconComponent className="w-4 h-4" />
                            <span>{icon.label}</span>
                          </div>
                        </SelectItem>
                      )
                    })}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setIsEditorOpen(false)}
                disabled={isSaving}
              >
                Cancel
              </Button>
              <Button
                onClick={handleSave}
                disabled={isSaving || !formName.trim() || !formPrompt.trim()}
              >
                {isSaving && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                {editingTemplate ? 'Save Changes' : 'Create Template'}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Delete Confirmation Dialog */}
        <Dialog open={isDeleteDialogOpen} onOpenChange={setIsDeleteDialogOpen}>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Delete Template</DialogTitle>
              <DialogDescription>
                Are you sure you want to delete "{templateToDelete?.name}"? This action cannot be undone.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setIsDeleteDialogOpen(false)}
                disabled={isSaving}
              >
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={handleConfirmDelete}
                disabled={isSaving}
              >
                {isSaving && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                Delete
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </>
  )
}
