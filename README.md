# Themis

**Project Name:** Themis  
**项目名称：** Themis（忒弥斯）

This project captures real-time audio played on laptops, mobile phones and other devices. The collected audio data is transmitted to large AI models, enabling instant speech-to-text conversion, content analysis and intelligent feedback generation. It delivers one-stop real-time processing including sound capture, semantic interpretation and smart response output.

本项目可实时捕获电脑、手机等终端设备播放音频，将声源数据同步接入大模型智能体系。依托 AI 能力快速完成语音转文字解析，同时实时分析音频内容逻辑与信息内核，即时输出研判结论、应答反馈，实现声息采集、语义拆解、智能答复一站式实时处理。

## Features (v0.1)

- **Windows**: System tray app, floating transcript overlay, `Ctrl+Shift+T` hotkey, optional Windows Service
- **macOS**: Menu bar tray, `Cmd+Shift+T`, LaunchAgent for background daemon
- **Azure Speech**: Streaming transcription via REST (chunked); mock mode when keys are absent
- **gRPC IPC**: Lightweight local API between UI and `themis-service`
- **CI/CD**: GitHub Actions build/test on Windows and macOS; release artifacts on version tags

## Requirements

| Platform | Version |
|----------|---------|
| Windows | 10 or later |
| macOS | 12 or later |
| Rust | stable (see `rust-toolchain.toml`) |
| Node.js | 20+ (for Tauri frontend) |

## Quick start

### 1. Azure Speech resource

1. Create a [Speech resource](https://portal.azure.com/#create/Microsoft.CognitiveServicesSpeechServices) in Azure Portal.
2. Copy **Key** and **Region** (e.g. `eastus`).
3. Copy `.env.example` to `.env` and set:

```env
AZURE_SPEECH_KEY=your_key
AZURE_SPEECH_REGION=eastus
```

Without keys, the service uses **mock transcription** (useful for development).

### 2. Build from source

```bash
# Rust binaries
cargo build --release -p themis-service -p themis-cli

# Tray app (install Node 20+ first)
cd apps/themis-tray
npm install
npm run tauri dev
```

### 3. Run

**Development (two terminals)**

```bash
# Terminal 1 — backend (keep running)
cd D:\learning\Themis   # project root
cargo run -p themis-service

# Terminal 2 — tray UI
cd apps/themis-tray
npm run tauri dev
```

The overlay should show `Status: idle — …` (not `Service offline`).  
If you only run `tauri dev` without the service, the dark overlay correctly shows **Service offline** — that is expected.

**No transcript text while “capturing”?**

- You configured **Azure Speech** in `.env`: the app captures **speaker output** (loopback), not the microphone.
- Set **`AZURE_SPEECH_LANGUAGE`** to match the video: `en-US` for English, `zh-CN` for Chinese.
- After fixing audio format on Windows, restart `themis-service` so loopback PCM is correct.
- For a quick UI test without Azure, set `THEMIS_USE_MOCK_SPEECH=true` in `.env` and restart `themis-service`.
- After `Ctrl+Shift+T` to start capture, wait a few seconds; partial text should appear under the status line.

**Portable mode (release build)**

1. `cargo build --release -p themis-service`
2. `cd apps/themis-tray && npm run tauri dev`  
   The tray tries to spawn `target/debug` or `target/release/themis-service` automatically.

**Windows Service (admin)**

```powershell
themis-cli service install
themis-cli service start
```

**macOS LaunchAgent**

```bash
themis-cli agent install
themis-cli agent start
```

### 4. Shortcuts

| Platform | Action |
|----------|--------|
| Windows | `Ctrl+Shift+T` — toggle capture |
| macOS | `Cmd+Shift+T` — toggle capture |
| Tray | Left-click — show/hide overlay; menu — toggle / quit |

### 5. Diagnostics

```bash
themis-cli doctor
themis-cli status
```

## Repository layout

```
crates/themis-{core,audio,azure,ipc,service,cli}  # Rust workspace
apps/themis-tray                                     # Tauri tray + overlay
packaging/                                           # Service / LaunchAgent templates
docs/                                                # Architecture & platform notes
```

See [docs/architecture.md](docs/architecture.md) and [docs/platform-notes.md](docs/platform-notes.md).

## iOS roadmap

iOS system-audio capture is **not included in v0.1** due to platform restrictions. See [docs/platform-notes.md](docs/platform-notes.md) for ReplayKit and App Store constraints.

## License

MIT
