//! Original procedural score and cartoon foley. No recordings or voice model.
use std::f32::consts::TAU;
pub const RATE: usize = 22_050;
pub const NAMES: [&str; 24] = [
    "swing",
    "heavy_swing",
    "impact",
    "flesh_cut",
    "blood_spray",
    "pain_low",
    "pain_high",
    "scream",
    "death",
    "rock_throw",
    "rock_hit",
    "blessing",
    "slurp",
    "blop",
    "gurgle",
    "splat",
    "male_pain",
    "female_pain",
    "male_fear",
    "female_fear",
    "male_victory",
    "female_victory",
    "male_scream",
    "female_scream",
];
struct Random(u64);
impl Random {
    fn new(seed: u64) -> Self {
        Self((seed ^ 0x9e3779b97f4a7c15).max(1))
    }
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1_u32 << 24) as f32
    }
}
fn hz(note: i32) -> f32 {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}
fn envelope(t: f32, duration: f32) -> f32 {
    (t / 0.008).min(1.0) * ((duration - t) / 0.025).clamp(0.0, 1.0)
}
fn note(samples: &mut [f32], at: f32, duration: f32, pitch: i32, gain: f32, bass: bool) {
    let start = (at * RATE as f32) as usize;
    let count = (duration * RATE as f32) as usize;
    let frequency = hz(pitch);
    for i in 0..count {
        let t = i as f32 / RATE as f32;
        let p = TAU * frequency * t;
        let voice = if bass {
            p.sin() + 0.22 * (p * 2.0).sin()
        } else {
            p.sin() + 0.25 * (p * 2.0).sin() + 0.10 * (p * 3.0).sin()
        };
        let decay = (-t * if bass { 5.0 } else { 11.0 }).exp();
        let len = samples.len();
        samples[(start + i) % len] += voice * gain * decay * envelope(t, duration);
    }
}
/// Sixteen bars of a mischievous minor-key waltz. Chord tones and rhythmic
/// variations are chosen from a local seed, independent of simulation RNG.
pub fn music(seed: u64) -> Vec<u8> {
    let beat = 60.0 / 116.0;
    let mut samples = vec![0.0; (48.0 * beat * RATE as f32) as usize];
    let mut rng = Random::new(seed);
    let chords = [[57, 60, 64], [53, 57, 60], [50, 53, 57], [52, 56, 59]];
    for bar in 0..16 {
        let chord = chords[bar % 4];
        let at = bar as f32 * 3.0 * beat;
        note(&mut samples, at, beat * 0.8, chord[0] - 24, 0.27, true);
        for step in 1..3 {
            for pitch in chord {
                note(
                    &mut samples,
                    at + step as f32 * beat,
                    beat * 0.45,
                    pitch,
                    0.065,
                    false,
                );
            }
        }
        for step in 0..6 {
            if rng.next() < 0.25 && step % 2 == 1 {
                continue;
            }
            let choice = (rng.next() * 3.0) as usize;
            let pitch = chord[choice.min(2)] + 12 + if bar % 8 == 7 && step == 5 { 12 } else { 0 };
            note(
                &mut samples,
                at + step as f32 * beat * 0.5,
                beat * 0.42,
                pitch,
                0.13,
                false,
            );
        }
    }
    // Preserve the original melody and add a broad brass-like pad, octave
    // support and downbeat drums. All tails wrap into the start of the loop.
    let mut drums = Random::new(seed.wrapping_add(81));
    for bar in 0..16 {
        let chord = chords[bar % 4];
        let start = bar as f32 * 3.0 * beat;
        for pitch in [chord[0] - 12, chord[1] - 12, chord[2] - 12] {
            for i in 0..(3.1 * beat * RATE as f32) as usize {
                let t = i as f32 / RATE as f32;
                let duration = 3.1 * beat;
                let env = (t / 0.18).min(1.0) * ((duration - t) / 0.35).clamp(0.0, 1.0);
                let phase = TAU * hz(pitch) * t;
                let brass = phase.sin() + 0.22 * (phase * 2.0).sin() + 0.10 * (phase * 3.0).sin();
                let index = ((start * RATE as f32) as usize + i) % samples.len();
                samples[index] += brass * env * 0.055;
            }
        }
        for step in 0..3 {
            let at = start + step as f32 * beat;
            for i in 0..(0.23 * RATE as f32) as usize {
                let t = i as f32 / RATE as f32;
                let pulse = if step == 0 {
                    (TAU * (70.0 * t + 1.8 * (1.0 - (-t * 35.0).exp()))).sin()
                        * 0.22
                        * (-t * 19.0).exp()
                } else {
                    (drums.next() * 2.0 - 1.0) * 0.045 * (-t * 32.0).exp()
                };
                let index = ((at * RATE as f32) as usize + i) % samples.len();
                samples[index] += pulse * envelope(t, 0.23);
            }
        }
    }
    wav(&samples)
}
pub fn effect(kind: usize) -> Vec<u8> {
    let durations = [
        0.22, 0.34, 0.20, 0.32, 0.68, 0.34, 0.32, 0.78, 0.70, 0.26, 0.24, 0.75, 0.48, 0.25, 0.75,
        0.30, 0.50, 0.50, 0.80, 0.80, 0.90, 0.90, 0.95, 0.95,
    ];
    let duration = durations[kind];
    let mut rng = Random::new(kind as u64 + 145);
    let mut low = 0.0;
    let mut phase = 0.0;
    let mut samples = Vec::with_capacity((duration * RATE as f32) as usize);
    for i in 0..(duration * RATE as f32) as usize {
        let t = i as f32 / RATE as f32;
        let u = t / duration;
        let noise = rng.next() * 2.0 - 1.0;
        low += 0.12 * (noise - low);
        let value = match kind {
            0 | 1 | 9 => {
                let sweep = (std::f32::consts::PI * u).sin().powi(2);
                (noise * 0.40 + low * 0.55) * sweep * if kind == 1 { 1.0 } else { 0.75 }
            }
            2 | 10 => {
                let f = if kind == 10 { 390.0 } else { 95.0 };
                ((TAU * (f * t - 35.0 * t * t)).sin() * 0.55 + noise * 0.35) * (-t * 24.0).exp()
            }
            3 => {
                (noise * 0.38 + low * 0.85 + ((TAU * 260.0 * t) * (-t * 7.0).exp()).sin() * 0.12)
                    * (-t * 9.0).exp()
            }
            4 => {
                let pulse = (TAU * (15.0 * t + 5.0 * t * t)).sin().max(0.0);
                (low * 0.85 + noise * 0.15) * pulse * (1.0 - u)
            }
            5..=8 => {
                let base = [155.0, 245.0, 310.0, 125.0][kind - 5];
                let f = base * (1.0 + 0.17 * (TAU * 6.0 * t).sin()) * (1.15 - 0.55 * u);
                phase += TAU * f / RATE as f32;
                let buzz = phase.sin() + 0.35 * (phase * 2.0).sin() + 0.17 * (phase * 3.0).sin();
                let vowel = 0.60 + 0.20 * (TAU * 4.5 * t).sin();
                (buzz * 0.35 * vowel + noise * 0.07) * (std::f32::consts::PI * u).sin().powf(0.6)
            }
            12 => {
                let suction = (TAU * (9.0 * t + 16.0 * t * t)).sin().abs();
                let bubble = (TAU * (210.0 * t - 130.0 * t * t)).sin();
                (low * 1.4 + bubble * 0.24) * suction * (std::f32::consts::PI * u).sin()
            }
            13 => {
                let bubble = (TAU * (230.0 * t - 250.0 * t * t)).sin();
                (bubble * 0.62 + low * 0.30) * (-t * 17.0).exp()
            }
            14 => {
                let bubbles = (TAU * (21.0 * t + 2.5 * (TAU * 1.7 * t).sin()))
                    .sin()
                    .max(0.0);
                (low * 1.25 + (TAU * (110.0 * t + 1.8 * (TAU * 5.0 * t).sin())).sin() * 0.22)
                    * bubbles
                    * (std::f32::consts::PI * u).sin()
            }
            15 => {
                (low * 1.6 + noise * 0.22)
                    * (0.5 + 0.5 * (TAU * 37.0 * t).sin())
                    * (-t * 12.0).exp()
            }
            16..=23 => {
                let female = kind % 2 == 1;
                let feeling = (kind - 16) / 2;
                let base = if female { 235.0 } else { 142.0 };
                let contour = match feeling {
                    0 => 1.25 - 0.5 * u,
                    1 => 1.0 + 0.6 * u,
                    2 => 0.8 + 0.5 * (std::f32::consts::PI * u).sin(),
                    _ => 1.6 - 0.5 * u,
                };
                let fundamental = base * contour * (1.0 + 0.04 * (TAU * 7.0 * t).sin());
                phase += TAU * fundamental / RATE as f32;
                let mut voice = 0.0;
                for harmonic in 1..=24 {
                    let f = fundamental * harmonic as f32;
                    let vowel = (-((f - 780.0) / 260.0).powi(2)).exp()
                        + 0.7 * (-((f - 1350.0) / 400.0).powi(2)).exp()
                        + 0.25 * (-((f - 2600.0) / 600.0).powi(2)).exp();
                    voice +=
                        (phase * harmonic as f32).sin() * (0.09 + vowel) / (harmonic as f32).sqrt();
                }
                let tremolo = if feeling == 2 {
                    (TAU * 3.3 * t).sin().abs() * 0.55 + 0.45
                } else {
                    1.0
                };
                (voice * 0.25 + noise * 0.04)
                    * tremolo
                    * (std::f32::consts::PI * u).sin().powf(0.65)
            }
            _ => [523.25, 659.25, 783.99]
                .iter()
                .map(|f| (TAU * f * t).sin() * 0.16 * (-t * 4.0).exp())
                .sum::<f32>(),
        };
        samples.push(value * envelope(t, duration));
    }
    wav(&samples)
}
fn wav(samples: &[f32]) -> Vec<u8> {
    let bytes = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + bytes as usize);
    out.extend(b"RIFF");
    out.extend((36 + bytes).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16_u32.to_le_bytes());
    out.extend(1_u16.to_le_bytes());
    out.extend(1_u16.to_le_bytes());
    out.extend((RATE as u32).to_le_bytes());
    out.extend((RATE as u32 * 2).to_le_bytes());
    out.extend(2_u16.to_le_bytes());
    out.extend(16_u16.to_le_bytes());
    out.extend(b"data");
    out.extend(bytes.to_le_bytes());
    for sample in samples {
        out.extend((((sample.tanh() * 0.82) * i16::MAX as f32) as i16).to_le_bytes());
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_audio_is_bounded_non_silent_pcm_and_music_varies_by_seed() {
        let first = music(42);
        assert_eq!(first, music(42));
        assert_ne!(first, music(43));
        for bytes in std::iter::once(first).chain((0..NAMES.len()).map(effect)) {
            assert_eq!(&bytes[..4], b"RIFF");
            assert_eq!(&bytes[8..12], b"WAVE");
            assert_eq!(
                u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize,
                bytes.len() - 44
            );
            let samples = bytes[44..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>();
            assert!(samples.iter().any(|v| v.unsigned_abs() > 1000));
            assert!(samples.iter().all(|v| v.unsigned_abs() < 30_000));
            if samples.len() < RATE * 2 {
                assert!(samples[0].abs() < 10 && samples.last().unwrap().abs() < 100);
            } else {
                assert!((samples[0] as i32 - *samples.last().unwrap() as i32).abs() < 2000);
            }
        }
    }
}
