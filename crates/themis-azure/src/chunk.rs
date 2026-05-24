//! REST STT chunking: how much audio to accumulate before each Azure request.

pub const SAMPLE_RATE: u32 = 16_000;
/// Seconds of audio per REST chunk (lower = faster subtitles, more API calls).
pub const CHUNK_SECS: u32 = 2;
pub const CHUNK_SAMPLES: usize = (SAMPLE_RATE as usize) * (CHUNK_SECS as usize);
/// 1s overlap between chunks to avoid cutting words at boundaries.
pub const OVERLAP_SAMPLES: usize = SAMPLE_RATE as usize;

pub fn chunk_buffer_ms() -> u32 {
    CHUNK_SECS * 1000
}
