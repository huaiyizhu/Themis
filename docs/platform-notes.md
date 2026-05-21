# Platform Notes

## Windows

- **Loopback**: Uses CPAL/WASAPI against the default render device. If loopback is unavailable, the engine falls back to a stub tone source (for dev/CI).
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
