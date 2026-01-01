<div align="center">

# Meeting-Local

### Your meetings. Your data. Your machine.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Windows](https://img.shields.io/badge/Windows-0078D6?logo=windows&logoColor=white)](https://github.com)
[![macOS](https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white)](https://github.com)
[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)

**Privacy-first meeting transcription that runs 100% locally.**
No cloud. No subscriptions. No data leaves your device.

[Download](#download) · [Features](#features) · [Build from Source](#building-from-source)

</div>

---

<!--
## Screenshots

Add your screenshots here:
![Meeting-Local Dashboard](docs/screenshots/dashboard.png)
![Recording in Action](docs/screenshots/recording.png)
-->

## Features

| Feature | Description |
|---------|-------------|
| **Real-time Transcription** | Powered by OpenAI Whisper — from tiny (fast) to large-v3 (accurate) |
| **Speaker Diarization** | Automatically identify who said what with AI speaker detection |
| **Auto Translation** | Select your preferred language and transcripts are automatically translated — speak German, get English notes |
| **Local LLM Chat** | Ask questions about your meetings using local AI models |
| **GPU Accelerated** | CUDA (NVIDIA), Metal (Apple), Vulkan (AMD/Intel) support |
| **100% Private** | Everything runs locally — your recordings never leave your machine |
| **Multi-language** | Supports 15+ languages with automatic detection |
| **Noise Suppression** | Built-in audio processing for cleaner transcriptions |
| **Search & Organize** | Full-text search, categories, and tags for all recordings |

---

## How It Works

1. **Record** — Capture microphone and system audio simultaneously
2. **Transcribe** — Real-time speech-to-text as you record
3. **Translate** — Automatic translation to your preferred language (speak any language, get notes in yours)
4. **Identify** — AI detects and labels different speakers
5. **Analyze** — Chat with local LLMs to summarize and extract insights

---

## Download

> **Coming Soon** — Release builds will be available here.

| Platform | GPU | Download |
|----------|-----|----------|
| Windows | NVIDIA (CUDA) | [meeting-local-CUDA.exe](#) |
| Windows | AMD/Intel (Vulkan) | [meeting-local-Vulkan.exe](#) |
| macOS | Apple Silicon (Metal) | [meeting-local-Metal.dmg](#) |
| macOS | Intel (Metal) | [meeting-local-Metal.dmg](#) |

---

## Tech Stack

<table>
<tr>
<td width="50%">

**Frontend**
- Next.js 14
- React 18
- TypeScript
- Tailwind CSS
- Radix UI

</td>
<td width="50%">

**Backend**
- Rust
- Tauri 2.x
- SQLite
- Tokio async runtime

</td>
</tr>
<tr>
<td width="50%">

**AI/ML**
- whisper-rs (transcription)
- mistral.rs (local LLM)
- pyannote-rs (diarization)
- ONNX Runtime

</td>
<td width="50%">

**Audio**
- cpal (capture)
- nnnoiseless (noise suppression)
- ebur128 (loudness normalization)
- FFmpeg (encoding)

</td>
</tr>
</table>

---

## Building from Source

### Prerequisites

| Requirement | Windows | macOS | Notes |
|-------------|:-------:|:-----:|-------|
| **Rust** | ✓ | ✓ | [rustup.rs](https://rustup.rs) |
| **Node.js** | ✓ | ✓ | v18+ recommended |
| **pnpm** | ✓ | ✓ | `npm install -g pnpm` |
| **Visual Studio** | ✓ | — | "Desktop development with C++" workload |
| **CUDA Toolkit** | ✓* | — | *For NVIDIA GPUs only ([download](https://developer.nvidia.com/cuda-downloads)) |
| **Vulkan SDK** | ✓* | — | *For AMD/Intel GPUs only ([download](https://vulkan.lunarg.com/sdk/home)) |
| **Xcode CLI** | — | ✓ | `xcode-select --install` |

### Clone & Install

```bash
git clone https://github.com/YOUR_USERNAME/Meeting-Local.git
cd Meeting-Local/desktop
pnpm install
```

### Development

```bash
# Auto-detect GPU and run
pnpm run tauri:dev

# Or specify GPU backend:
pnpm run tauri:dev:cuda    # NVIDIA GPU
pnpm run tauri:dev:vulkan  # AMD/Intel GPU
pnpm run tauri:dev:metal   # macOS
pnpm run tauri:dev:cpu     # CPU only (fallback)
```

### Production Build

Use the build scripts — they check prerequisites automatically:

<details>
<summary><b>Windows (NVIDIA GPU)</b></summary>

```powershell
cd desktop
./scripts/build-cuda.ps1
```

**Requirements:**
- CUDA Toolkit 12.x installed
- `CUDA_PATH` environment variable set
- CMake installed
- Visual Studio with C++ tools

</details>

<details>
<summary><b>Windows (AMD/Intel GPU)</b></summary>

```powershell
cd desktop
./scripts/build-vulkan.ps1
```

**Requirements:**
- Vulkan SDK installed
- `VULKAN_SDK` environment variable set
- Visual Studio with C++ tools

</details>

<details>
<summary><b>macOS (Apple Silicon / Intel)</b></summary>

```bash
cd desktop
./scripts/build-metal.sh
```

**Requirements:**
- Xcode Command Line Tools
- Rust with appropriate target

</details>

### Build Output

| Platform | Location |
|----------|----------|
| Windows | `desktop/src-tauri/target/release/bundle/nsis/*.exe` |
| macOS | `desktop/src-tauri/target/release/bundle/dmg/*.dmg` |

---

## Troubleshooting

<details>
<summary><b>CUDA build errors</b></summary>

1. Verify CUDA is installed: `nvidia-smi`
2. Check environment variable: `echo %CUDA_PATH%`
3. Ensure Visual Studio C++ tools are installed

</details>

<details>
<summary><b>Build fails with missing modules</b></summary>

Try a clean build:
```bash
cd desktop/src-tauri
cargo clean
cd ..
pnpm run tauri:dev
```

</details>

<details>
<summary><b>Audio devices not detected</b></summary>

- Windows: Check privacy settings for microphone access
- macOS: Grant microphone permission when prompted

</details>

---

## License

This project is licensed under the **GNU General Public License v3.0** — see the [LICENSE](LICENSE) file for details.

This means you can use, modify, and distribute this software, but any derivative work must also be open source under the same license.

---

<div align="center">

**Built with Rust, TypeScript, and a commitment to privacy.**

</div>
