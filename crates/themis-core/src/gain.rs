//! PCM gain helpers — loopback level follows the Windows mix; quiet system volume
//! yields quiet samples. Normalizing before STT avoids "no speech" on low volume.

/// Returns peak sample magnitude before gain (0..32767).
pub fn normalize_pcm16(pcm: &mut [i16], target_peak: i16, max_gain: f32) -> u32 {
    if pcm.is_empty() {
        return 0;
    }
    let peak: u32 = pcm
        .iter()
        .map(|s| s.unsigned_abs() as u32)
        .max()
        .unwrap_or(0);
    if peak == 0 {
        return 0;
    }
    let target = target_peak.max(1) as u32;
    if peak < target {
        let gain = (target as f32 / peak as f32).min(max_gain.max(1.0));
        for s in pcm.iter_mut() {
            let v = (*s as f32 * gain).round();
            *s = v.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        }
    }
    peak
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boosts_quiet_signal() {
        let mut pcm = vec![100i16; 100];
        let peak = normalize_pcm16(&mut pcm, 8000, 20.0);
        assert_eq!(peak, 100);
        assert!(pcm[0] > 1000);
    }
}
