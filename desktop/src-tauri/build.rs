use std::path::PathBuf;

fn main() {
    // GPU Acceleration Detection and Build Guidance
    detect_and_report_gpu_capabilities();

    // Build and copy the LLM sidecar for bundling
    build_and_copy_sidecar();

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=Cocoa");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
    tauri_build::build()
}

/// Copy the LLM sidecar to the binaries folder for Tauri bundling
/// Note: The sidecar should be built first via beforeDevCommand/beforeBuildCommand
fn build_and_copy_sidecar() {
    let target = std::env::var("TARGET").unwrap_or_else(|_| "x86_64-pc-windows-msvc".to_string());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let binaries_dir = manifest_dir.join("binaries");

    // Create binaries directory if it doesn't exist
    std::fs::create_dir_all(&binaries_dir).ok();

    // Determine source and destination paths
    let sidecar_name = if cfg!(windows) { "llm-sidecar.exe" } else { "llm-sidecar" };
    let target_sidecar_name = if cfg!(windows) {
        format!("llm-sidecar-{}.exe", target)
    } else {
        format!("llm-sidecar-{}", target)
    };

    // Source: target/{profile}/llm-sidecar[.exe]
    let source = manifest_dir.join("target").join(&profile).join(sidecar_name);
    let dest = binaries_dir.join(&target_sidecar_name);

    // Copy sidecar only if source is newer than dest (avoid rebuild loop)
    if source.exists() {
        let should_copy = if dest.exists() {
            // Compare modification times
            let source_mtime = std::fs::metadata(&source).and_then(|m| m.modified()).ok();
            let dest_mtime = std::fs::metadata(&dest).and_then(|m| m.modified()).ok();
            match (source_mtime, dest_mtime) {
                (Some(s), Some(d)) => s > d,
                _ => true, // Copy if we can't determine times
            }
        } else {
            true // Dest doesn't exist, need to copy
        };

        if should_copy {
            if let Err(e) = std::fs::copy(&source, &dest) {
                println!("cargo:warning=Failed to copy sidecar: {}", e);
            } else {
                println!("cargo:warning=Copied LLM sidecar to binaries/");
            }
        }
    } else {
        println!("cargo:warning=LLM sidecar not found at: {}", source.display());
        println!("cargo:warning=The sidecar is built automatically via beforeDevCommand");
    }

    println!("cargo:rerun-if-changed=llm-sidecar/src/main.rs");
    println!("cargo:rerun-if-changed=llm-sidecar/Cargo.toml");
}

/// Detects GPU acceleration capabilities and provides build guidance
fn detect_and_report_gpu_capabilities() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    println!("cargo:warning=Building Meeting-Local for: {}", target_os);

    match target_os.as_str() {
        "macos" => {
            println!("cargo:warning=macOS: Metal GPU acceleration ENABLED by default");
            #[cfg(feature = "coreml")]
            println!("cargo:warning=CoreML acceleration ENABLED");
        }
        "windows" => {
            if cfg!(feature = "cuda") {
                println!("cargo:warning=Windows: CUDA GPU acceleration ENABLED");
            } else if cfg!(feature = "vulkan") {
                println!("cargo:warning=Windows: Vulkan GPU acceleration ENABLED");
            } else {
                println!("cargo:warning=Windows: Using CPU-only mode");
                println!("cargo:warning=For NVIDIA GPU: cargo build --release --features cuda");
                println!("cargo:warning=For AMD/Intel GPU: cargo build --release --features vulkan");

                // Try to detect NVIDIA GPU
                if which::which("nvidia-smi").is_ok() {
                    println!("cargo:warning=NVIDIA GPU detected! Consider rebuilding with --features cuda");
                }
            }
        }
        "linux" => {
            if cfg!(feature = "cuda") {
                println!("cargo:warning=Linux: CUDA GPU acceleration ENABLED");
            } else if cfg!(feature = "vulkan") {
                println!("cargo:warning=Linux: Vulkan GPU acceleration ENABLED");
            } else if cfg!(feature = "hipblas") {
                println!("cargo:warning=Linux: AMD ROCm (HIP) acceleration ENABLED");
            } else {
                println!("cargo:warning=Linux: Using CPU-only mode");
                println!("cargo:warning=For NVIDIA GPU: cargo build --release --features cuda");
                println!("cargo:warning=For AMD GPU: cargo build --release --features hipblas");

                if which::which("nvidia-smi").is_ok() {
                    println!("cargo:warning=NVIDIA GPU detected! Consider rebuilding with --features cuda");
                }
                if which::which("rocm-smi").is_ok() {
                    println!("cargo:warning=AMD GPU detected! Consider rebuilding with --features hipblas");
                }
            }
        }
        _ => {
            println!("cargo:warning=Unknown platform: {}", target_os);
        }
    }
}
