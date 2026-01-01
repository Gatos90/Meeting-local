// Hardware detection types and API for model recommendations
import { invoke } from '@tauri-apps/api/core';

export type PerformanceTier = 'Low' | 'Medium' | 'High' | 'Ultra';
export type GpuType = 'None' | 'Metal' | 'Cuda' | 'Vulkan' | 'OpenCL';
export type RecommendationLevel = 'Recommended' | 'Compatible' | 'NotRecommended' | 'TooHeavy';

export interface HardwareProfileInfo {
  cpu_cores: number;
  has_gpu: boolean;
  gpu_type: string;
  memory_gb: number;
  performance_tier: PerformanceTier;
  tier_description: string;
}

export interface ModelRecommendation {
  model_name: string;
  recommendation: RecommendationLevel;
  reason: string;
}

export interface HardwareRecommendations {
  hardware: HardwareProfileInfo;
  whisper_models: ModelRecommendation[];
  llm_models: ModelRecommendation[];
  best_whisper_model: string;
  best_llm_model: string | null;
}

// Cached recommendations to avoid repeated backend calls
let cachedRecommendations: HardwareRecommendations | null = null;

/**
 * Get hardware-based model recommendations from the backend
 * Results are cached for the session since hardware doesn't change
 */
export async function getHardwareRecommendations(): Promise<HardwareRecommendations> {
  if (cachedRecommendations) {
    return cachedRecommendations;
  }

  try {
    cachedRecommendations = await invoke<HardwareRecommendations>('get_hardware_recommendations');
    return cachedRecommendations;
  } catch (error) {
    console.error('Failed to get hardware recommendations:', error);
    // Return default recommendations for Medium tier
    return getDefaultRecommendations();
  }
}

/**
 * Clear cached recommendations (for testing or refresh)
 */
export function clearRecommendationsCache(): void {
  cachedRecommendations = null;
}

/**
 * Get recommendation for a specific whisper model
 */
export function getWhisperRecommendation(
  modelName: string,
  recommendations: HardwareRecommendations
): ModelRecommendation | undefined {
  return recommendations.whisper_models.find(m => m.model_name === modelName);
}

/**
 * Get recommendation for a specific LLM model
 */
export function getLlmRecommendation(
  modelName: string,
  recommendations: HardwareRecommendations
): ModelRecommendation | undefined {
  return recommendations.llm_models.find(m => m.model_name === modelName);
}

/**
 * Get recommendation for an Ollama model by fuzzy matching
 * Maps Ollama IDs like "llama3.2" to backend names like "llama-3.2-1b-instruct"
 */
export function getOllamaModelRecommendation(
  ollamaModelId: string,
  recommendations: HardwareRecommendations
): ModelRecommendation | undefined {
  const id = ollamaModelId.toLowerCase();

  // Try exact match first
  let match = recommendations.llm_models.find(m =>
    m.model_name.toLowerCase() === id
  );
  if (match) return match;

  // Fuzzy matching patterns: [pattern, array of backend model names to try]
  const patterns: [RegExp, string[]][] = [
    [/llama.*3.*2.*1b/i, ['llama-3.2-1b-instruct']],
    [/llama.*3.*2.*3b/i, ['llama-3.2-3b-instruct']],
    [/llama.*3.*2/i, ['llama-3.2-1b-instruct', 'llama-3.2-3b-instruct']],
    [/phi.*3.*5|phi-3|phi3/i, ['phi-3.5-mini']],
    [/mistral.*7b|^mistral$/i, ['mistral-7b-instruct']],
    [/qwen.*2.*5.*7b|qwen.*7b/i, ['qwen-2.5-7b-instruct']],
  ];

  for (const [pattern, modelNames] of patterns) {
    if (pattern.test(id)) {
      match = recommendations.llm_models.find(m =>
        modelNames.includes(m.model_name)
      );
      if (match) return match;
    }
  }

  // If no fuzzy match, calculate based on estimated size
  const estimatedSize = estimateModelSizeGB(ollamaModelId);
  return calculateModelRecommendation(estimatedSize, recommendations.hardware.performance_tier, 'llm');
}

/**
 * Estimate model size in GB from model ID/name
 * Parses common patterns like "llama3.2:3b", "mistral:7b", etc.
 */
export function estimateModelSizeGB(modelId: string): number {
  const id = modelId.toLowerCase();

  // Check for explicit size indicators
  if (id.includes('70b')) return 40.0;
  if (id.includes('34b') || id.includes('33b')) return 20.0;
  if (id.includes('13b') || id.includes('14b')) return 8.0;
  if (id.includes('8b')) return 5.0;
  if (id.includes('7b')) return 4.5;
  if (id.includes('3b') || id.includes('4b')) return 2.0;
  if (id.includes('1b') || id.includes('2b')) return 0.8;

  // Check for known model families
  if (id.includes('phi') || id.includes('gemma')) return 2.5;
  if (id.includes('mistral') || id.includes('llama')) return 4.5;
  if (id.includes('qwen')) return 4.5;

  // Default estimate for unknown models
  return 3.0;
}

/**
 * Calculate recommendation for any model based on size and hardware tier
 * This is the core logic for determining if a model will work well
 */
export function calculateModelRecommendation(
  modelSizeGB: number,
  performanceTier: PerformanceTier,
  modelType: 'whisper' | 'llm'
): ModelRecommendation {
  // Size thresholds per tier (in GB)
  const thresholds = {
    whisper: {
      Low: { recommended: 0.1, compatible: 0.3, notRecommended: 0.5 },
      Medium: { recommended: 0.5, compatible: 1.0, notRecommended: 1.5 },
      High: { recommended: 1.5, compatible: 2.5, notRecommended: 3.0 },
      Ultra: { recommended: 10, compatible: 10, notRecommended: 10 },
    },
    llm: {
      Low: { recommended: 1, compatible: 2, notRecommended: 3 },
      Medium: { recommended: 2.5, compatible: 4, notRecommended: 5 },
      High: { recommended: 4.5, compatible: 6, notRecommended: 8 },
      Ultra: { recommended: 10, compatible: 15, notRecommended: 20 },
    }
  };

  const limits = thresholds[modelType][performanceTier];

  if (modelSizeGB <= limits.recommended) {
    return {
      model_name: '',
      recommendation: 'Recommended',
      reason: `Great fit for your hardware (${modelSizeGB.toFixed(1)}GB)`
    };
  } else if (modelSizeGB <= limits.compatible) {
    return {
      model_name: '',
      recommendation: 'Compatible',
      reason: `Will work but may be slower (${modelSizeGB.toFixed(1)}GB)`
    };
  } else if (modelSizeGB <= limits.notRecommended) {
    return {
      model_name: '',
      recommendation: 'NotRecommended',
      reason: `May cause performance issues (${modelSizeGB.toFixed(1)}GB)`
    };
  } else {
    return {
      model_name: '',
      recommendation: 'TooHeavy',
      reason: `Too large for your hardware (${modelSizeGB.toFixed(1)}GB)`
    };
  }
}

/**
 * Get badge color for recommendation level
 */
export function getRecommendationBadgeColor(level: RecommendationLevel): string {
  switch (level) {
    case 'Recommended':
      return 'green';
    case 'Compatible':
      return 'gray';
    case 'NotRecommended':
      return 'yellow';
    case 'TooHeavy':
      return 'red';
    default:
      return 'gray';
  }
}

/**
 * Get badge label for recommendation level
 */
export function getRecommendationBadgeLabel(level: RecommendationLevel): string {
  switch (level) {
    case 'Recommended':
      return 'Best for your PC';
    case 'Compatible':
      return 'Compatible';
    case 'NotRecommended':
      return 'May be slow';
    case 'TooHeavy':
      return 'Not recommended';
    default:
      return '';
  }
}

/**
 * Get performance tier display string
 */
export function getPerformanceTierDisplay(tier: PerformanceTier): string {
  switch (tier) {
    case 'Low':
      return 'Basic';
    case 'Medium':
      return 'Moderate';
    case 'High':
      return 'High Performance';
    case 'Ultra':
      return 'High-End';
    default:
      return tier;
  }
}

/**
 * Default recommendations for when backend call fails
 */
function getDefaultRecommendations(): HardwareRecommendations {
  return {
    hardware: {
      cpu_cores: 4,
      has_gpu: false,
      gpu_type: 'None',
      memory_gb: 8,
      performance_tier: 'Medium',
      tier_description: 'Moderate hardware - most models will work',
    },
    whisper_models: [
      { model_name: 'base', recommendation: 'Recommended', reason: 'Default recommendation' },
      { model_name: 'small', recommendation: 'Recommended', reason: 'Good for most systems' },
      { model_name: 'base-q5_1', recommendation: 'Recommended', reason: 'Optimized for your hardware' },
    ],
    llm_models: [
      { model_name: 'llama-3.2-1b-instruct', recommendation: 'Recommended', reason: 'Lightweight model' },
      { model_name: 'llama-3.2-3b-instruct', recommendation: 'Recommended', reason: 'Good balance' },
    ],
    best_whisper_model: 'base',
    best_llm_model: 'llama-3.2-1b-instruct',
  };
}
