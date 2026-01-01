use std::sync::OnceLock;
use log::info;
use serde::Serialize;
use sysinfo::System;

/// Hardware capabilities for audio processing optimization
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HardwareProfile {
    pub cpu_cores: u8,
    pub has_gpu_acceleration: bool,
    pub gpu_type: GpuType,
    pub memory_gb: u8,
    pub performance_tier: PerformanceTier,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum GpuType {
    None,
    Metal,      // Apple Silicon
    Cuda,       // NVIDIA
    Vulkan,     // AMD/Intel
    OpenCL,     // Generic GPU compute
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PerformanceTier {
    Low,      // CPU-only, limited resources
    Medium,   // CPU-only but powerful, or basic GPU
    High,     // Dedicated GPU with good compute
    Ultra,    // High-end hardware with fast GPU
}

/// Adaptive Whisper configuration based on hardware
#[derive(Debug, Clone)]
pub struct AdaptiveWhisperConfig {
    pub beam_size: usize,
    pub temperature: f32,
    pub use_gpu: bool,
    pub max_threads: Option<usize>,
    pub chunk_size_preference: ChunkSizePreference,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChunkSizePreference {
    Fast,       // Smaller chunks for responsiveness
    Balanced,   // Medium chunks for balance
    Quality,    // Larger chunks for accuracy
}

static HARDWARE_PROFILE: OnceLock<HardwareProfile> = OnceLock::new();

impl HardwareProfile {
    /// Get the detected hardware profile (cached after first call)
    pub fn detect() -> &'static HardwareProfile {
        HARDWARE_PROFILE.get_or_init(|| {
            let profile = Self::detect_hardware();
            info!("Detected hardware profile: {:?}", profile);
            profile
        })
    }

    /// Perform hardware detection
    fn detect_hardware() -> HardwareProfile {
        let cpu_cores = Self::detect_cpu_cores();
        let (has_gpu_acceleration, gpu_type) = Self::detect_gpu();
        let memory_gb = Self::detect_memory_gb();
        let performance_tier = Self::calculate_performance_tier(cpu_cores, &gpu_type, memory_gb);

        HardwareProfile {
            cpu_cores,
            has_gpu_acceleration,
            gpu_type,
            memory_gb,
            performance_tier,
        }
    }

    /// Detect number of CPU cores
    fn detect_cpu_cores() -> u8 {
        std::thread::available_parallelism()
            .map(|n| n.get().min(255) as u8)
            .unwrap_or(4) // Default to 4 cores
    }

    /// Detect GPU acceleration capabilities
    fn detect_gpu() -> (bool, GpuType) {
        // Check for Metal (Apple Silicon)
        #[cfg(target_os = "macos")]
        {
            if Self::has_metal_support() {
                return (true, GpuType::Metal);
            }
        }

        // Check for CUDA (NVIDIA)
        if Self::has_cuda_support() {
            return (true, GpuType::Cuda);
        }

        // Check for Vulkan (AMD/Intel/others)
        if Self::has_vulkan_support() {
            return (true, GpuType::Vulkan);
        }

        // Fallback to CPU-only
        (false, GpuType::None)
    }

    /// Detect available system memory in GB
    fn detect_memory_gb() -> u8 {
        let sys = System::new_all();
        let total_memory_bytes = sys.total_memory();
        // Convert bytes to GB (1 GB = 1,073,741,824 bytes)
        let memory_gb = (total_memory_bytes / 1_073_741_824) as u8;
        info!("Detected system memory: {} GB ({} bytes)", memory_gb, total_memory_bytes);
        memory_gb.max(1) // At least 1GB
    }

    /// Calculate performance tier based on hardware
    fn calculate_performance_tier(cpu_cores: u8, gpu_type: &GpuType, memory_gb: u8) -> PerformanceTier {
        match gpu_type {
            GpuType::Metal => {
                if memory_gb >= 16 && cpu_cores >= 8 {
                    PerformanceTier::Ultra
                } else {
                    PerformanceTier::High
                }
            }
            GpuType::Cuda => {
                if memory_gb >= 16 && cpu_cores >= 8 {
                    PerformanceTier::Ultra
                } else {
                    PerformanceTier::High
                }
            }
            GpuType::Vulkan | GpuType::OpenCL => {
                if memory_gb >= 12 && cpu_cores >= 6 {
                    PerformanceTier::High
                } else {
                    PerformanceTier::Medium
                }
            }
            GpuType::None => {
                if cpu_cores >= 8 && memory_gb >= 16 {
                    PerformanceTier::Medium
                } else {
                    PerformanceTier::Low
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn has_metal_support() -> bool {
        // Simple check for Apple Silicon (Metal is available on Intel Macs too, but less optimal for ML)
        std::env::consts::ARCH == "aarch64"
    }

    fn has_cuda_support() -> bool {
        // Check for CUDA environment variables
        if std::env::var("CUDA_PATH").is_ok() || std::env::var("CUDA_HOME").is_ok() {
            return true;
        }

        // Linux path
        if std::path::Path::new("/usr/local/cuda").exists() {
            return true;
        }

        // Windows: Check for NVIDIA driver/tools
        #[cfg(target_os = "windows")]
        {
            if std::path::Path::new("C:\\Program Files\\NVIDIA GPU Computing Toolkit").exists()
                || std::path::Path::new("C:\\Windows\\System32\\nvidia-smi.exe").exists()
                || std::path::Path::new("C:\\Windows\\System32\\nvcuda.dll").exists()
            {
                return true;
            }
        }

        false
    }

    fn has_vulkan_support() -> bool {
        // Check Vulkan SDK environment variable
        if std::env::var("VULKAN_SDK").is_ok() {
            return true;
        }

        // Linux paths
        if std::path::Path::new("/usr/lib/x86_64-linux-gnu/libvulkan.so").exists()
            || std::path::Path::new("/usr/lib/libvulkan.so").exists()
        {
            return true;
        }

        // Windows: Check for Vulkan runtime
        #[cfg(target_os = "windows")]
        {
            if std::path::Path::new("C:\\Windows\\System32\\vulkan-1.dll").exists() {
                return true;
            }
        }

        false
    }

    /// Generate adaptive Whisper configuration based on hardware
    /// Beam size tuned based on research: higher beam = better accuracy, slower processing
    /// Research shows beam_size 5-10 optimal for accuracy, with diminishing returns above 10
    pub fn get_whisper_config(&self) -> AdaptiveWhisperConfig {
        // Windows-specific override: Use beam size 3 for balance of stability and quality
        #[cfg(target_os = "windows")]
        {
            return AdaptiveWhisperConfig {
                beam_size: 3,  // Increased from 2 for better accuracy
                temperature: 0.2,
                use_gpu: self.has_gpu_acceleration,
                max_threads: Some(self.cpu_cores.min(8) as usize),
                chunk_size_preference: ChunkSizePreference::Balanced,
            };
        }

        // Platform-adaptive configuration for non-Windows systems
        // Beam sizes increased based on research for better transcription accuracy
        #[cfg(not(target_os = "windows"))]
        {
            match self.performance_tier {
                PerformanceTier::Ultra => AdaptiveWhisperConfig {
                    beam_size: 8,  // Maximum quality (research optimal: 5-10)
                    temperature: 0.1,
                    use_gpu: self.has_gpu_acceleration,
                    max_threads: Some(self.cpu_cores.min(8) as usize),
                    chunk_size_preference: ChunkSizePreference::Quality,
                },
                PerformanceTier::High => AdaptiveWhisperConfig {
                    beam_size: 5,  // High quality (research recommends 5+)
                    temperature: 0.2,
                    use_gpu: self.has_gpu_acceleration,
                    max_threads: Some(self.cpu_cores.min(6) as usize),
                    chunk_size_preference: ChunkSizePreference::Balanced,
                },
                PerformanceTier::Medium => AdaptiveWhisperConfig {
                    beam_size: 3,  // Balanced (increased from 2)
                    temperature: 0.3,
                    use_gpu: self.has_gpu_acceleration,
                    max_threads: Some(self.cpu_cores.min(4) as usize),
                    chunk_size_preference: ChunkSizePreference::Balanced,
                },
                PerformanceTier::Low => AdaptiveWhisperConfig {
                    beam_size: 1,  // Fast processing - greedy decoding
                    temperature: 0.4,
                    use_gpu: false, // Force CPU to avoid GPU overhead on weak hardware
                    max_threads: Some(2),
                    chunk_size_preference: ChunkSizePreference::Fast,
                },
            }
        }
    }

    /// Get recommended chunk duration in milliseconds based on performance tier
    pub fn get_recommended_chunk_duration_ms(&self) -> u32 {
        match self.performance_tier {
            PerformanceTier::Ultra => 25000,   // 25 seconds for maximum accuracy
            PerformanceTier::High => 20000,    // 20 seconds for high quality
            PerformanceTier::Medium => 15000,  // 15 seconds for balance
            PerformanceTier::Low => 10000,     // 10 seconds for responsiveness
        }
    }

    /// Check if hardware can handle real-time processing of given sample rate
    pub fn can_handle_realtime(&self, sample_rate: u32, channels: u16) -> bool {
        let data_rate = sample_rate * channels as u32;

        match self.performance_tier {
            PerformanceTier::Ultra => data_rate <= 192000, // Up to 192kHz stereo
            PerformanceTier::High => data_rate <= 96000,   // Up to 96kHz stereo or 192kHz mono
            PerformanceTier::Medium => data_rate <= 48000, // Up to 48kHz stereo
            PerformanceTier::Low => data_rate <= 22050,    // Up to 22kHz stereo or 48kHz mono
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_detection() {
        let profile = HardwareProfile::detect();
        assert!(profile.cpu_cores > 0);
        // Performance optimization: remove println! from tests
        log::debug!("Detected profile: {:?}", profile);
    }

    #[test]
    fn test_whisper_config_generation() {
        let profile = HardwareProfile::detect();
        let config = profile.get_whisper_config();

        assert!(config.beam_size >= 1 && config.beam_size <= 8);  // Updated: max is now 8 for Ultra tier
        assert!(config.temperature >= 0.0 && config.temperature <= 1.0);

        // Performance optimization: remove println! from tests
        log::debug!("Generated config: {:?}", config);
    }

    #[test]
    fn test_performance_tier_logic() {
        // Test different hardware combinations
        let low_tier = HardwareProfile::calculate_performance_tier(2, &GpuType::None, 4);
        assert_eq!(low_tier, PerformanceTier::Low);

        let high_tier = HardwareProfile::calculate_performance_tier(8, &GpuType::Metal, 16);
        assert_eq!(high_tier, PerformanceTier::Ultra);
    }
}