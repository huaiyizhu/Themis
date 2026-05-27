# Platform Notes

## Windows

- **System audio output**: WASAPI **loopback** on the default **playback (render)** endpoint — the digital mix of all apps Windows sends to that output (headphones, HDMI, USB audio, virtual devices). Not the microphone; not tied to physical speakers or volume sliders. Optional `THEMIS_AUDIO_OUTPUT_DEVICE` in `.env` (friendly name substring or endpoint ID) if you route audio through a specific virtual device. Falls back to a stub tone when loopback is unavailable (dev/CI).
- **Service**: Install with `themis-cli service install` (Administrator). See [packaging/windows/themis-service.md](../packaging/windows/themis-service.md).
- **Hotkey**: `Ctrl+Shift+T` toggles capture.
- **Overlay adaptive contrast**: samples desktop pixels behind the overlay (`Ctrl+Shift+A`).

## macOS

- **System audio**: Apple does not expose a public loopback API for all apps. This build captures the **default input device** via Core Audio / cpal. For true system-playback capture, route output through a virtual device and select it as input.

### BlackHole setup (recommended)

1. Install [BlackHole 2ch](https://existential.audio/blackhole/) (free; reboot if the installer asks).
2. **System Settings → Sound → Output**: choose **BlackHole 2ch** while capturing (apps will play into BlackHole).
3. **System Settings → Sound → Input**: choose **BlackHole 2ch** (Themis reads this device).
4. **Optional — hear audio while capturing**: open **Audio MIDI Setup** (`/Applications/Utilities/Audio MIDI Setup.app`) → **+** → **Create Multi-Output Device** → check **BlackHole 2ch** and your headphones/built-in speakers → set macOS **Output** to this Multi-Output Device.

### Permissions

- macOS may prompt for **Microphone** access when capture starts (input-device path). Allow for Terminal, iTerm, or the built `themis-tray` app.
- Check **System Settings → Privacy & Security → Microphone** if `probe` shows `frames = 0`.

### Development

- Scripts: `./scripts/themis.sh` (or `./dev.sh`, `./tray.sh`). Logs: `~/Library/Logs/Themis`.
- First Tauri build needs `icon.icns`: `./scripts/themis.sh icons` (or run `tray`, which auto-generates).
- `THEMIS_AUDIO_CAPTURE_MODE` / `THEMIS_AUDIO_OUTPUT_DEVICE` in `.env` are **ignored** on macOS.
- **Hotkey**: `Cmd+Shift+T` toggles capture.
- **Overlay adaptive contrast**: not available (no desktop sampling API wired yet); style presets still work (`Cmd+Shift+S`).

### LaunchAgent

`themis-cli agent install` then `themis-cli agent start`. See [packaging/macos/com.themis.agent.plist](../packaging/macos/com.themis.agent.plist).

## iOS (deferred)

Capturing audio *played by the device* is restricted on iOS:

| Approach | App Store | Notes |
|----------|-----------|-------|
| Microphone | Yes | Captures ambient sound, not isolated system mix |
| ReplayKit broadcast | Limited | User must start screen/system broadcast; heavy UX |
| System loopback | No | Not available to third-party apps |

Planned milestone: ReplayKit Broadcast Upload Extension feeding the same Azure pipeline.

## Linux (deferred)

PipeWire/PulseAudio `module-loopback` or `pw-record --target` would be the likely approach.
