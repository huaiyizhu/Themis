# Platform Notes

## Windows

- **System audio output**: WASAPI **loopback** on the default **playback (render)** endpoint — the digital mix of all apps Windows sends to that output (headphones, HDMI, USB audio, virtual devices). Not the microphone; not tied to physical speakers or volume sliders. Optional `THEMIS_AUDIO_OUTPUT_DEVICE` in `.env` (friendly name substring or endpoint ID) if you route audio through a specific virtual device. Falls back to a stub tone when loopback is unavailable (dev/CI).
- **Voice/video calls (dual capture)**: When `THEMIS_AUDIO_CAPTURE_MODE=auto` and a known call app is running, Themis captures **output loopback + microphone** mixed for STT. Force with `call` or `dual`. Optional `THEMIS_AUDIO_INPUT_DEVICE` for mic selection.
- **Service**: Install with `themis-cli service install` (Administrator). See [packaging/windows/themis-service.md](../packaging/windows/themis-service.md).
- **Hotkey**: `Ctrl+Shift+T` toggles capture.
- **Overlay adaptive contrast**: samples desktop pixels behind the overlay (`Ctrl+Shift+A`).

## macOS

- **System audio (recommended, macOS 14.2+)**: **Core Audio Process Tap** — captures all apps’ playback without BlackHole. Enabled when `THEMIS_AUDIO_CAPTURE_MODE` is `auto` (default) or `process_tap` / `tap` / `process`. Requires **System Audio Recording** permission (prompt on first capture).
- **Voice/video calls (dual capture)**: When `THEMIS_AUDIO_CAPTURE_MODE=auto` and a known call app is running (Zoom, Teams, FaceTime, Discord, etc.), Themis captures **both** system playback (process tap) **and** the microphone, mixed into one stream for STT. Force dual anytime with `call` or `dual`. Optional `THEMIS_AUDIO_INPUT_DEVICE` selects the mic (name substring).
- **Fallback — input device**: `THEMIS_AUDIO_CAPTURE_MODE=input` uses the default **microphone / input** via cpal (same as older builds). Optional `THEMIS_AUDIO_INPUT_DEVICE=BlackHole` if you use a virtual device.

### Process tap (no BlackHole)

1. Use macOS **14.2+** (you are on 26.x — supported).
2. Keep `THEMIS_AUDIO_CAPTURE_MODE=auto` in `.env` (or set `process_tap`).
3. `./scripts/themis.sh restart` then `tray` → **Cmd+Shift+T**.
4. On first capture, allow **System Audio Recording** when macOS prompts (also check **System Settings → Privacy & Security** if `probe` shows no signal).
5. If probe fails with `themis_tap_start failed ('nope')`: rebuild and let the script sign the binary (`./scripts/themis.sh probe` runs `codesign` on macOS), or manually: `codesign --force --sign - target/debug/themis-cli target/debug/themis-service`
6. Status should show `capture=process_tap` and rising `peak` / `frames` while audio is playing.

### BlackHole setup (optional legacy)

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
- `THEMIS_AUDIO_OUTPUT_DEVICE` is **Windows-only** (WASAPI endpoint hint).
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
