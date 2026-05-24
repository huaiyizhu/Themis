# Platform Notes

## Windows

- **System audio output**: WASAPI **loopback** on the default **playback (render)** endpoint — the digital mix of all apps Windows sends to that output (headphones, HDMI, USB audio, virtual devices). Not the microphone; not tied to physical speakers or volume sliders. Optional `THEMIS_AUDIO_OUTPUT_DEVICE` in `.env` (friendly name substring or endpoint ID) if you route audio through a specific virtual device. Falls back to a stub tone when loopback is unavailable (dev/CI).
- **Service**: Install with `themis-cli service install` (Administrator). See [packaging/windows/themis-service.md](../packaging/windows/themis-service.md).
- **Hotkey**: `Ctrl+Shift+T` toggles capture.

## macOS

- **System audio**: Apple does not expose a public loopback API for all apps. This build captures the **default input device**. For true system audio, route output through [BlackHole](https://existential.audio/blackhole/) and select it as the Mac input source.
- **LaunchAgent**: `themis-cli agent install` then `themis-cli agent start`.
- **Hotkey**: `Cmd+Shift+T`.

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
