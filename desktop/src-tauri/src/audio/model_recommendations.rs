//! Model Recommendations based on Hardware Profile
//!
//! Provides recommendations for which AI models will work best
//! on the user's current hardware configuration.

use serde::Serialize;
use super::hardware_detector::{HardwareProfile, PerformanceTier, GpuType};

/// Recommendation level for a model
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum RecommendationLevel {
    /// Model is recommended for this hardware - will run great
    Recommended,
    /// Model is compatible but not optimal
    Compatible,
    /// Model may be slow on this hardware
    NotRecommended,
    /// Model is too demanding for this hardware
    TooHeavy,
}

/// Individual model recommendation
#[derive(Debug, Clone, Serialize)]
pub struct ModelRecommendation {
    pub model_name: String,
    pub recommendation: RecommendationLevel,
    pub reason: String,
}

/// Hardware profile info for frontend display
#[derive(Debug, Clone, Serialize)]
pub struct HardwareProfileInfo {
    pub cpu_cores: u8,
    pub has_gpu: bool,
    pub gpu_type: String,
    pub memory_gb: u8,
    pub performance_tier: String,
    pub tier_description: String,
}

/// Complete recommendations response
#[derive(Debug, Clone, Serialize)]
pub struct HardwareRecommendations {
    pub hardware: HardwareProfileInfo,
    pub whisper_models: Vec<ModelRecommendation>,
    pub llm_models: Vec<ModelRecommendation>,
    pub best_whisper_model: String,
    pub best_llm_model: Option<String>,
}

impl HardwareProfile {
    /// Get model recommendations based on detected hardware
    pub fn get_model_recommendations(&self) -> HardwareRecommendations {
        let hardware_info = self.to_info();
        let whisper_models = self.get_whisper_recommendations();
        let llm_models = self.get_llm_recommendations();

        let best_whisper = whisper_models
            .iter()
            .find(|m| m.recommendation == RecommendationLevel::Recommended)
            .map(|m| m.model_name.clone())
            .unwrap_or_else(|| "base-q5_1".to_string());

        let best_llm = llm_models
            .iter()
            .find(|m| m.recommendation == RecommendationLevel::Recommended)
            .map(|m| m.model_name.clone());

        HardwareRecommendations {
            hardware: hardware_info,
            whisper_models,
            llm_models,
            best_whisper_model: best_whisper,
            best_llm_model: best_llm,
        }
    }

    fn to_info(&self) -> HardwareProfileInfo {
        let gpu_type_str = match &self.gpu_type {
            GpuType::None => "None",
            GpuType::Metal => "Apple Metal",
            GpuType::Cuda => "NVIDIA CUDA",
            GpuType::Vulkan => "Vulkan",
            GpuType::OpenCL => "OpenCL",
        };

        let (tier_str, tier_desc) = match &self.performance_tier {
            PerformanceTier::Low => ("Low", "Basic hardware - use lightweight models"),
            PerformanceTier::Medium => ("Medium", "Moderate hardware - most models will work"),
            PerformanceTier::High => ("High", "Powerful hardware - large models supported"),
            PerformanceTier::Ultra => ("Ultra", "High-end hardware - all models supported"),
        };

        HardwareProfileInfo {
            cpu_cores: self.cpu_cores,
            has_gpu: self.has_gpu_acceleration,
            gpu_type: gpu_type_str.to_string(),
            memory_gb: self.memory_gb,
            performance_tier: tier_str.to_string(),
            tier_description: tier_desc.to_string(),
        }
    }

    fn get_whisper_recommendations(&self) -> Vec<ModelRecommendation> {
        // Whisper models with their size in MB
        let models: Vec<(&str, u32)> = vec![
            // Quantized models (smallest first)
            ("tiny-q5_1", 32),
            ("base-q5_1", 60),
            ("small-q5_1", 190),
            ("tiny-q8_0", 44),
            ("base-q8_0", 82),
            ("small-q8_0", 264),
            ("medium-q5_0", 539),
            ("large-v3-turbo-q5_0", 574),
            ("medium-q8_0", 823),
            ("large-v3-turbo-q8_0", 874),
            ("large-v3-q5_0", 1080),
            // Standard models
            ("tiny", 78),
            ("base", 148),
            ("small", 488),
            ("tiny.en", 78),
            ("base.en", 148),
            ("small.en", 488),
            ("medium", 1530),
            ("medium.en", 1530),
            ("large-v3-turbo", 1620),
            ("large-v3", 3100),
        ];

        models.iter().map(|(name, size_mb)| {
            let (recommendation, reason) = self.recommend_whisper_model(name, *size_mb);
            ModelRecommendation {
                model_name: name.to_string(),
                recommendation,
                reason,
            }
        }).collect()
    }

    fn recommend_whisper_model(&self, name: &str, size_mb: u32) -> (RecommendationLevel, String) {
        match self.performance_tier {
            PerformanceTier::Low => {
                // Low tier: Only tiny and base quantized models
                if name.contains("tiny-q") || name == "tiny" {
                    (RecommendationLevel::Recommended, "Lightweight model, works well on basic hardware".to_string())
                } else if name.contains("base-q") {
                    (RecommendationLevel::Recommended, "Good balance for basic hardware".to_string())
                } else if name == "base" || name == "tiny.en" || name == "base.en" {
                    (RecommendationLevel::Compatible, "May work but quantized versions are faster".to_string())
                } else if size_mb > 500 {
                    (RecommendationLevel::TooHeavy, "Too demanding for this hardware".to_string())
                } else {
                    (RecommendationLevel::NotRecommended, "May be slow on basic hardware".to_string())
                }
            }
            PerformanceTier::Medium => {
                // Medium tier: Up to small models, quantized preferred
                if name.contains("small-q") || name.contains("base-q") {
                    (RecommendationLevel::Recommended, "Optimized for your hardware".to_string())
                } else if name == "base" || name == "small" || name.contains("tiny") {
                    (RecommendationLevel::Recommended, "Works well on your hardware".to_string())
                } else if name.contains("base.en") || name.contains("small.en") {
                    (RecommendationLevel::Recommended, "English-optimized, good performance".to_string())
                } else if name.contains("medium-q") {
                    (RecommendationLevel::Compatible, "Will work but may be slower".to_string())
                } else if size_mb > 1500 {
                    (RecommendationLevel::TooHeavy, "Too demanding for this hardware".to_string())
                } else {
                    (RecommendationLevel::NotRecommended, "May cause performance issues".to_string())
                }
            }
            PerformanceTier::High => {
                // High tier: Most models work, large-v3 may be slow
                if name.contains("large-v3-turbo") || name.contains("medium") || name.contains("small") {
                    (RecommendationLevel::Recommended, "Great choice for your hardware".to_string())
                } else if name.contains("base") || name.contains("tiny") {
                    (RecommendationLevel::Compatible, "Will work fast, but larger models offer better accuracy".to_string())
                } else if name == "large-v3" {
                    (RecommendationLevel::NotRecommended, "Very large model, may be slow".to_string())
                } else if name.contains("large-v3-q") {
                    (RecommendationLevel::Recommended, "Quantized large model, good balance".to_string())
                } else {
                    (RecommendationLevel::Compatible, "Compatible with your hardware".to_string())
                }
            }
            PerformanceTier::Ultra => {
                // Ultra tier: All models work great
                if name == "large-v3" || name == "large-v3-turbo" {
                    (RecommendationLevel::Recommended, "Best accuracy, your hardware handles it well".to_string())
                } else if name.contains("medium") || name.contains("large") {
                    (RecommendationLevel::Recommended, "Great performance on your hardware".to_string())
                } else {
                    (RecommendationLevel::Compatible, "Works well, but larger models offer better accuracy".to_string())
                }
            }
        }
    }

    fn get_llm_recommendations(&self) -> Vec<ModelRecommendation> {
        // LLM models with their size in GB
        let models: Vec<(&str, f32)> = vec![
            ("llama-3.2-1b-instruct", 0.8),
            ("llama-3.2-3b-instruct", 2.0),
            ("phi-3.5-mini", 2.2),
            ("mistral-7b-instruct", 4.4),
            ("qwen-2.5-7b-instruct", 4.5),
        ];

        models.iter().map(|(name, size_gb)| {
            let (recommendation, reason) = self.recommend_llm_model(name, *size_gb);
            ModelRecommendation {
                model_name: name.to_string(),
                recommendation,
                reason,
            }
        }).collect()
    }

    fn recommend_llm_model(&self, name: &str, size_gb: f32) -> (RecommendationLevel, String) {
        let ram_gb = self.memory_gb as f32;

        match self.performance_tier {
            PerformanceTier::Low => {
                // Low tier: Only smallest model
                if name.contains("1b") {
                    (RecommendationLevel::Recommended, "Lightweight model for basic hardware".to_string())
                } else if size_gb <= 2.5 {
                    (RecommendationLevel::NotRecommended, "May be slow on basic hardware".to_string())
                } else {
                    (RecommendationLevel::TooHeavy, "Too demanding for this hardware".to_string())
                }
            }
            PerformanceTier::Medium => {
                // Medium tier: Up to 3B models
                if name.contains("1b") || name.contains("3b") {
                    (RecommendationLevel::Recommended, "Good fit for your hardware".to_string())
                } else if name.contains("phi-3.5") {
                    (RecommendationLevel::Compatible, "Works but may be slower".to_string())
                } else {
                    (RecommendationLevel::TooHeavy, format!("Requires more RAM (you have {}GB)", ram_gb as u8))
                }
            }
            PerformanceTier::High => {
                // High tier: Most models up to ~4GB
                if size_gb <= 4.0 {
                    (RecommendationLevel::Recommended, "Great choice for your hardware".to_string())
                } else {
                    (RecommendationLevel::Compatible, "Will work but may be slower".to_string())
                }
            }
            PerformanceTier::Ultra => {
                // Ultra tier: All models
                if size_gb >= 4.0 {
                    (RecommendationLevel::Recommended, "Best quality, your hardware handles it well".to_string())
                } else {
                    (RecommendationLevel::Recommended, "Fast and efficient on your hardware".to_string())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_tier_recommendations() {
        let profile = HardwareProfile {
            cpu_cores: 4,
            has_gpu_acceleration: false,
            gpu_type: GpuType::None,
            memory_gb: 8,
            performance_tier: PerformanceTier::Low,
        };

        let recommendations = profile.get_model_recommendations();
        assert_eq!(recommendations.hardware.performance_tier, "Low");

        // Check that tiny models are recommended
        let tiny = recommendations.whisper_models.iter()
            .find(|m| m.model_name == "tiny-q5_1")
            .unwrap();
        assert_eq!(tiny.recommendation, RecommendationLevel::Recommended);

        // Check that large models are too heavy
        let large = recommendations.whisper_models.iter()
            .find(|m| m.model_name == "large-v3")
            .unwrap();
        assert_eq!(large.recommendation, RecommendationLevel::TooHeavy);
    }

    #[test]
    fn test_ultra_tier_recommendations() {
        let profile = HardwareProfile {
            cpu_cores: 16,
            has_gpu_acceleration: true,
            gpu_type: GpuType::Cuda,
            memory_gb: 32,
            performance_tier: PerformanceTier::Ultra,
        };

        let recommendations = profile.get_model_recommendations();
        assert_eq!(recommendations.hardware.performance_tier, "Ultra");

        // Check that large models are recommended
        let large = recommendations.whisper_models.iter()
            .find(|m| m.model_name == "large-v3")
            .unwrap();
        assert_eq!(large.recommendation, RecommendationLevel::Recommended);
    }
}
