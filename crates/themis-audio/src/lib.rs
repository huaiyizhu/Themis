mod source;
pub(crate) mod stub;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(windows)]
mod windows;

pub use source::{AudioSource, LoopbackSource};

pub fn create_loopback(
    sample_rate: u32,
    channels: u16,
) -> anyhow::Result<Box<dyn AudioSource + Send>> {
    #[cfg(windows)]
    {
        Ok(Box::new(windows::WasapiLoopback::new(
            sample_rate,
            channels,
        )?))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::CoreAudioLoopback::new(
            sample_rate,
            channels,
        )?))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Ok(Box::new(stub::StubAudioSource::new(sample_rate, channels)))
    }
}
