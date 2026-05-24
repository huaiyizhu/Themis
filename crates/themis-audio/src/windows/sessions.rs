//! Enumerate Windows audio sessions with active playback (for process loopback).

use tracing::debug;
use windows::core::Interface;
use windows::Win32::Media::Audio::{
    eConsole, eRender, AudioSessionStateActive, IAudioSessionControl, IAudioSessionControl2,
    IAudioSessionEnumerator, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

/// PIDs that currently have an **active** audio session on the default console render device.
pub fn active_audio_session_pids() -> anyhow::Result<Vec<u32>> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device: IMMDevice = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;

        let session_manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
        let session_enum: IAudioSessionEnumerator = session_manager.GetSessionEnumerator()?;
        let count = session_enum.GetCount()?;

        let mut pids = Vec::new();
        for i in 0..count {
            let session: IAudioSessionControl = session_enum.GetSession(i)?;
            let state = session.GetState()?;
            if state != AudioSessionStateActive {
                continue;
            }
            let session2: IAudioSessionControl2 = session.cast()?;
            let pid = session2.GetProcessId()?;
            if pid != 0 {
                pids.push(pid);
            }
        }

        pids.sort_unstable();
        pids.dedup();
        debug!(count = pids.len(), ?pids, "active audio session pids");
        Ok(pids)
    }
}
