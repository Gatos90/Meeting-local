//! LLM Model Registry - Available models for download

use super::types::DownloadableModel;

/// Get list of available models for download
pub fn available_models() -> Vec<DownloadableModel> {
    // Curated list of recommended GGUF models
    // Each model includes the HuggingFace repo ID for tokenizer loading
    vec![
        DownloadableModel {
            id: "llama-3.2-3b-instruct".to_string(),
            name: "Llama 3.2 3B Instruct".to_string(),
            description: "Meta's latest small model, great for summarization and chat.".to_string(),
            size_bytes: 2_000_000_000, // ~2 GB
            hf_repo: "bartowski/Llama-3.2-3B-Instruct-GGUF".to_string(),
            gguf_file: "Llama-3.2-3B-Instruct-Q4_K_M.gguf".to_string(),
            url: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf".to_string(),
            sha256: None,
            context_length: 131072,
            recommended_for: vec!["summarization".to_string(), "chat".to_string()],
            has_native_tool_support: false, // Llama 3.2 3B doesn't have native function calling
        },
        DownloadableModel {
            id: "llama-3.2-1b-instruct".to_string(),
            name: "Llama 3.2 1B Instruct".to_string(),
            description: "Smallest Llama model, fast and lightweight.".to_string(),
            size_bytes: 800_000_000, // ~0.8 GB
            hf_repo: "bartowski/Llama-3.2-1B-Instruct-GGUF".to_string(),
            gguf_file: "Llama-3.2-1B-Instruct-Q4_K_M.gguf".to_string(),
            url: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf".to_string(),
            sha256: None,
            context_length: 131072,
            recommended_for: vec!["quick".to_string()],
            has_native_tool_support: false, // Llama 3.2 1B doesn't have native function calling
        },
        DownloadableModel {
            id: "mistral-7b-instruct".to_string(),
            name: "Mistral 7B Instruct".to_string(),
            description: "High-quality 7B model with excellent instruction following.".to_string(),
            size_bytes: 4_370_000_000, // ~4.4 GB
            hf_repo: "bartowski/Mistral-7B-Instruct-v0.3-GGUF".to_string(),
            gguf_file: "Mistral-7B-Instruct-v0.3-Q4_K_M.gguf".to_string(),
            url: "https://huggingface.co/bartowski/Mistral-7B-Instruct-v0.3-GGUF/resolve/main/Mistral-7B-Instruct-v0.3-Q4_K_M.gguf".to_string(),
            sha256: None,
            context_length: 32768,
            recommended_for: vec!["summarization".to_string(), "chat".to_string()],
            has_native_tool_support: true, // Mistral Instruct v0.3 supports function calling
        },
        DownloadableModel {
            id: "qwen-2.5-7b-instruct".to_string(),
            name: "Qwen 2.5 7B Instruct".to_string(),
            description: "Alibaba's latest model, strong multilingual support.".to_string(),
            size_bytes: 4_500_000_000, // ~4.5 GB
            hf_repo: "Qwen/Qwen2.5-7B-Instruct-GGUF".to_string(),
            gguf_file: "qwen2.5-7b-instruct-q4_k_m.gguf".to_string(),
            url: "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf".to_string(),
            sha256: None,
            context_length: 32768,
            recommended_for: vec!["multilingual".to_string(), "chat".to_string()],
            has_native_tool_support: true, // Qwen 2.5 supports function calling
        },
        DownloadableModel {
            id: "phi-3.5-mini".to_string(),
            name: "Phi 3.5 Mini".to_string(),
            description: "Microsoft's efficient small model.".to_string(),
            size_bytes: 2_200_000_000, // ~2.2 GB
            hf_repo: "bartowski/Phi-3.5-mini-instruct-GGUF".to_string(),
            gguf_file: "Phi-3.5-mini-instruct-Q4_K_M.gguf".to_string(),
            url: "https://huggingface.co/bartowski/Phi-3.5-mini-instruct-GGUF/resolve/main/Phi-3.5-mini-instruct-Q4_K_M.gguf".to_string(),
            sha256: None,
            context_length: 131072,
            recommended_for: vec!["suggestions".to_string()],
            has_native_tool_support: false, // Phi 3.5 Mini doesn't have native function calling
        },
    ]
}

/// Get the HuggingFace repo ID for a model
/// This is used by the sidecar provider to fetch the tokenizer
pub fn get_hf_repo_for_model(model_id: &str) -> Option<String> {
    available_models()
        .into_iter()
        .find(|m| m.id == model_id)
        .map(|m| m.hf_repo)
}
