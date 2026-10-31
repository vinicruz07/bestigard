//! Driver físico de saída de áudio e sintetizador procedural de ondas sonoras em Safe Rust.
//!
//! Conecta o fluxo de `AudioEvent` emitido pelo `AudioController` a um sintetizador polifônico
//! com geradores de onda (senoidal, quadrada, triangular e ruído branco), envelopes ADSR,
//! sequenciador melódico de BGM, limitador de pico e codificador PCM16 WAV sem dependências C externas.
#![forbid(unsafe_code)]

use crate::audio::{AudioController, AudioEvent, MusicPlaybackState, MusicTrack, SoundCue};
use std::f32::consts::TAU;

/// Formas de onda suportadas pelo sintetizador procedural.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveformKind {
    /// Onda senoidal pura e suave (ideal para sinos, magia e ambientes serenos).
    Sine,
    /// Onda quadrada rica em harmônicos ímpares (ideal para UI, moedas e fanfarras).
    Square,
    /// Onda triangular suave (ideal para flautas, colheita e melodia de fundo).
    Triangle,
    /// Ruído branco pseudo-aleatório determinístico (ideal para impactos percussivos e limpeza de pedras/arbustos).
    WhiteNoise,
}

/// Envelope de modulação de amplitude ADSR (Attack, Decay, Sustain, Release).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsrEnvelope {
    pub attack_secs: f32,
    pub decay_secs: f32,
    pub sustain_level: f32,
    pub release_secs: f32,
}

impl AdsrEnvelope {
    pub const fn new(attack_secs: f32, decay_secs: f32, sustain_level: f32, release_secs: f32) -> Self {
        Self {
            attack_secs,
            decay_secs,
            sustain_level,
            release_secs,
        }
    }

    pub const PERCUSSIVE: Self = Self::new(0.005, 0.04, 0.35, 0.06);
    pub const CHIME: Self = Self::new(0.008, 0.08, 0.55, 0.14);
    pub const PAD: Self = Self::new(0.03, 0.10, 0.70, 0.12);

    /// Calcula o ganho do envelope no instante `t` (em segundos) dentro de uma nota de duração `total_duration`.
    pub fn amplitude_at(&self, t: f32, total_duration: f32) -> f32 {
        if t < 0.0 || t >= total_duration || total_duration <= 0.0 {
            return 0.0;
        }

        let attack = self.attack_secs.max(0.0001);
        let decay = self.decay_secs.max(0.0001);
        let sustain = self.sustain_level.clamp(0.0, 1.0);
        let release = self.release_secs.max(0.0001);

        let release_start = (total_duration - release).max(0.0);

        if t >= release_start {
            let rel_progress = ((t - release_start) / release).clamp(0.0, 1.0);
            return sustain * (1.0 - rel_progress);
        }

        if t < attack {
            return (t / attack).clamp(0.0, 1.0);
        }

        let after_attack = t - attack;
        if after_attack < decay {
            let dec_progress = (after_attack / decay).clamp(0.0, 1.0);
            return 1.0 - (1.0 - sustain) * dec_progress;
        }

        sustain
    }
}

/// Especificação de um segmento tonal ou varredura de frequência (glissando).
#[derive(Debug, Clone, Copy)]
pub struct SynthToneSpec {
    pub waveform: WaveformKind,
    pub start_freq_hz: f32,
    pub end_freq_hz: f32,
    pub duration_secs: f32,
    pub envelope: AdsrEnvelope,
    pub gain: f32,
}

/// Sintetizador procedural de efeitos sonoros e trilhas musicais em Safe Rust.
pub struct ProceduralSynthesizer;

impl ProceduralSynthesizer {
    pub const DEFAULT_SAMPLE_RATE: u32 = 22_050;

    /// Avalia uma amostra normalizada `[-1.0, 1.0]` para a forma de onda e fase `[0.0, 1.0)`.
    #[inline]
    pub fn sample_waveform(waveform: WaveformKind, phase: f32, noise_seed: &mut u32) -> f32 {
        let wrapped = phase.rem_euclid(1.0);
        match waveform {
            WaveformKind::Sine => (wrapped * TAU).sin(),
            WaveformKind::Square => {
                if wrapped < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            WaveformKind::Triangle => 1.0 - 4.0 * (wrapped - 0.5).abs(),
            WaveformKind::WhiteNoise => {
                // Gerador congruente linear determinístico rápido (LCG) em Safe Rust
                *noise_seed = noise_seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let normalized = (*noise_seed >> 8) as f32 / ((1u32 << 24) as f32);
                normalized * 2.0 - 1.0
            }
        }
    }

    /// Sintetiza um segmento tonal completo em amostras PCM `f32` (`[-1.0, 1.0]`).
    pub fn synthesize_tone(spec: &SynthToneSpec, sample_rate: u32) -> Vec<f32> {
        let sr = sample_rate.max(1000) as f32;
        let total_samples = ((spec.duration_secs.max(0.005)) * sr).round() as usize;
        let mut samples = Vec::with_capacity(total_samples);
        let mut phase = 0.0f32;
        let mut noise_seed = 0xBEEF_CAFEu32;

        for i in 0..total_samples {
            let t = i as f32 / sr;
            let progress = (t / spec.duration_secs.max(0.0001)).clamp(0.0, 1.0);
            let freq = spec.start_freq_hz + (spec.end_freq_hz - spec.start_freq_hz) * progress;
            let env = spec.envelope.amplitude_at(t, spec.duration_secs);
            let raw = Self::sample_waveform(spec.waveform, phase, &mut noise_seed);
            samples.push((raw * env * spec.gain).clamp(-1.0, 1.0));
            phase = (phase + freq.max(20.0) / sr).rem_euclid(1.0);
        }

        samples
    }

    /// Sintetiza as amostras PCM completas para qualquer `SoundCue` do jogo com modulação de `pitch`.
    pub fn synthesize_sfx(cue: SoundCue, pitch: f32, sample_rate: u32) -> Vec<f32> {
        let p = pitch.clamp(0.25, 4.0);
        let segments: Vec<SynthToneSpec> = match cue {
            SoundCue::UiClick => vec![SynthToneSpec {
                waveform: WaveformKind::Triangle,
                start_freq_hz: 680.0 * p,
                end_freq_hz: 920.0 * p,
                duration_secs: 0.045,
                envelope: AdsrEnvelope::PERCUSSIVE,
                gain: 0.65,
            }],
            SoundCue::GoldCollect => vec![
                SynthToneSpec {
                    waveform: WaveformKind::Square,
                    start_freq_hz: 987.77 * p, // B5
                    end_freq_hz: 987.77 * p,
                    duration_secs: 0.06,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.55,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Square,
                    start_freq_hz: 1318.51 * p, // E6
                    end_freq_hz: 1318.51 * p,
                    duration_secs: 0.12,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.60,
                },
            ],
            SoundCue::FoodHarvest => vec![SynthToneSpec {
                waveform: WaveformKind::Triangle,
                start_freq_hz: 392.0 * p, // G4
                end_freq_hz: 784.0 * p,   // G5
                duration_secs: 0.14,
                envelope: AdsrEnvelope::CHIME,
                gain: 0.65,
            }],
            SoundCue::FeedCreature => vec![
                SynthToneSpec {
                    waveform: WaveformKind::Triangle,
                    start_freq_hz: 330.0 * p,
                    end_freq_hz: 494.0 * p,
                    duration_secs: 0.07,
                    envelope: AdsrEnvelope::PERCUSSIVE,
                    gain: 0.60,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Sine,
                    start_freq_hz: 523.25 * p,
                    end_freq_hz: 659.25 * p,
                    duration_secs: 0.09,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.65,
                },
            ],
            SoundCue::CreatureEvolve => vec![
                SynthToneSpec {
                    waveform: WaveformKind::Triangle,
                    start_freq_hz: 523.25 * p, // C5
                    end_freq_hz: 523.25 * p,
                    duration_secs: 0.08,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.65,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Triangle,
                    start_freq_hz: 659.25 * p, // E5
                    end_freq_hz: 659.25 * p,
                    duration_secs: 0.08,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.65,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Square,
                    start_freq_hz: 783.99 * p, // G5
                    end_freq_hz: 1046.50 * p,  // C6
                    duration_secs: 0.18,
                    envelope: AdsrEnvelope::PAD,
                    gain: 0.60,
                },
            ],
            SoundCue::EggDeposit => vec![SynthToneSpec {
                waveform: WaveformKind::Sine,
                start_freq_hz: 440.0 * p,
                end_freq_hz: 587.33 * p,
                duration_secs: 0.12,
                envelope: AdsrEnvelope::CHIME,
                gain: 0.65,
            }],
            SoundCue::EggHatch => vec![
                SynthToneSpec {
                    waveform: WaveformKind::WhiteNoise,
                    start_freq_hz: 500.0,
                    end_freq_hz: 900.0,
                    duration_secs: 0.05,
                    envelope: AdsrEnvelope::PERCUSSIVE,
                    gain: 0.45,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Triangle,
                    start_freq_hz: 587.33 * p,
                    end_freq_hz: 1174.66 * p,
                    duration_secs: 0.16,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.70,
                },
            ],
            SoundCue::BuildingPlaced => vec![
                SynthToneSpec {
                    waveform: WaveformKind::Triangle,
                    start_freq_hz: 180.0 * p,
                    end_freq_hz: 120.0 * p,
                    duration_secs: 0.07,
                    envelope: AdsrEnvelope::PERCUSSIVE,
                    gain: 0.75,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Square,
                    start_freq_hz: 360.0 * p,
                    end_freq_hz: 540.0 * p,
                    duration_secs: 0.09,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.50,
                },
            ],
            SoundCue::ObstacleCleared => vec![
                SynthToneSpec {
                    waveform: WaveformKind::WhiteNoise,
                    start_freq_hz: 400.0,
                    end_freq_hz: 200.0,
                    duration_secs: 0.09,
                    envelope: AdsrEnvelope::PERCUSSIVE,
                    gain: 0.60,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Sine,
                    start_freq_hz: 440.0 * p,
                    end_freq_hz: 660.0 * p,
                    duration_secs: 0.08,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.55,
                },
            ],
            SoundCue::CombatAttackHit => vec![SynthToneSpec {
                waveform: WaveformKind::Square,
                start_freq_hz: 280.0 * p,
                end_freq_hz: 95.0 * p,
                duration_secs: 0.09,
                envelope: AdsrEnvelope::PERCUSSIVE,
                gain: 0.75,
            }],
            SoundCue::CombatSuperEffective => vec![
                SynthToneSpec {
                    waveform: WaveformKind::Square,
                    start_freq_hz: 320.0 * p,
                    end_freq_hz: 140.0 * p,
                    duration_secs: 0.07,
                    envelope: AdsrEnvelope::PERCUSSIVE,
                    gain: 0.75,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Square,
                    start_freq_hz: 520.0 * p,
                    end_freq_hz: 960.0 * p,
                    duration_secs: 0.12,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.70,
                },
            ],
            SoundCue::CombatVictory => vec![
                SynthToneSpec {
                    waveform: WaveformKind::Square,
                    start_freq_hz: 523.25 * p,
                    end_freq_hz: 523.25 * p,
                    duration_secs: 0.08,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.60,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Square,
                    start_freq_hz: 659.25 * p,
                    end_freq_hz: 659.25 * p,
                    duration_secs: 0.08,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.60,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Square,
                    start_freq_hz: 783.99 * p,
                    end_freq_hz: 1046.50 * p,
                    duration_secs: 0.18,
                    envelope: AdsrEnvelope::PAD,
                    gain: 0.65,
                },
            ],
            SoundCue::CombatDefeat => vec![
                SynthToneSpec {
                    waveform: WaveformKind::Triangle,
                    start_freq_hz: 392.0 * p,
                    end_freq_hz: 349.23 * p,
                    duration_secs: 0.12,
                    envelope: AdsrEnvelope::PAD,
                    gain: 0.60,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Triangle,
                    start_freq_hz: 311.13 * p,
                    end_freq_hz: 261.63 * p,
                    duration_secs: 0.18,
                    envelope: AdsrEnvelope::PAD,
                    gain: 0.60,
                },
            ],
            SoundCue::MilestoneClaim => vec![
                SynthToneSpec {
                    waveform: WaveformKind::Sine,
                    start_freq_hz: 659.25 * p,
                    end_freq_hz: 987.77 * p,
                    duration_secs: 0.10,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.65,
                },
                SynthToneSpec {
                    waveform: WaveformKind::Sine,
                    start_freq_hz: 1318.51 * p,
                    end_freq_hz: 1318.51 * p,
                    duration_secs: 0.16,
                    envelope: AdsrEnvelope::CHIME,
                    gain: 0.70,
                },
            ],
        };

        let mut output = Vec::new();
        for seg in &segments {
            output.extend_from_slice(&Self::synthesize_tone(seg, sample_rate));
        }
        output
    }

    /// Retorna a partitura cíclica `(frequência_hz, duração_segundos)` e forma de onda de cada `MusicTrack`.
    pub fn bgm_sequence_notes(track: MusicTrack) -> (&'static [(f32, f32)], WaveformKind) {
        const SANCTUARY_NOTES: &[(f32, f32)] = &[
            (261.63, 0.35), // C4
            (329.63, 0.35), // E4
            (392.00, 0.35), // G4
            (493.88, 0.35), // B4
            (523.25, 0.45), // C5
            (392.00, 0.35), // G4
            (329.63, 0.35), // E4
            (293.66, 0.45), // D4
        ];

        const ARENA_NOTES: &[(f32, f32)] = &[
            (220.00, 0.18), // A3
            (220.00, 0.18), // A3
            (261.63, 0.18), // C4
            (293.66, 0.18), // D4
            (329.63, 0.24), // E4
            (293.66, 0.18), // D4
            (261.63, 0.18), // C4
            (196.00, 0.28), // G3
        ];

        const VICTORY_NOTES: &[(f32, f32)] = &[
            (523.25, 0.14), // C5
            (659.25, 0.14), // E5
            (783.99, 0.14), // G5
            (1046.50, 0.42), // C6
        ];

        match track {
            MusicTrack::SanctuaryTheme => (SANCTUARY_NOTES, WaveformKind::Triangle),
            MusicTrack::ArenaBattleTheme => (ARENA_NOTES, WaveformKind::Square),
            MusicTrack::VictoryFanfare => (VICTORY_NOTES, WaveformKind::Square),
        }
    }

    /// Codifica um slice de amostras normalizadas `f32` (`[-1.0, 1.0]`) em um container WAV PCM 16-bit mono válido.
    pub fn encode_wav_pcm16(samples: &[f32], sample_rate: u32) -> Vec<u8> {
        let data_bytes_len = (samples.len() * 2) as u32;
        let riff_chunk_size = 36 + data_bytes_len;
        let byte_rate = sample_rate * 2;
        let block_align: u16 = 2;
        let bits_per_sample: u16 = 16;

        let mut wav = Vec::with_capacity(44 + samples.len() * 2);
        // Header RIFF
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&riff_chunk_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        // Subchunk fmt
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // PCM chunk size
        wav.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat = 1 (PCM)
        wav.extend_from_slice(&1u16.to_le_bytes()); // NumChannels = 1 (Mono)
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());
        // Subchunk data
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_bytes_len.to_le_bytes());

        for &s in samples {
            let clamped = s.clamp(-1.0, 1.0);
            let pcm16 = (clamped * i16::MAX as f32) as i16;
            wav.extend_from_slice(&pcm16.to_le_bytes());
        }

        wav
    }
}

/// Voz polifônica ativa de efeito sonoro no mixer físico.
#[derive(Debug, Clone)]
pub struct ActiveVoice {
    pub cue: SoundCue,
    pub samples: Vec<f32>,
    pub cursor: usize,
    pub volume: f32,
}

impl ActiveVoice {
    #[inline]
    pub fn is_finished(&self) -> bool {
        self.cursor >= self.samples.len()
    }
}

/// Driver de saída de áudio físico multiplataforma em Safe Rust.
///
/// Consome os `AudioEvent`s do `AudioController`, gerencia polifonia de SFX com *voice stealing*,
/// sintetiza trilhas de BGM em tempo real e produz buffers PCM prontos para o dispositivo de saída.
#[derive(Debug, Clone)]
pub struct AudioOutputDriver {
    sample_rate: u32,
    max_voices: usize,
    active_voices: Vec<ActiveVoice>,
    active_bgm: Option<MusicTrack>,
    bgm_state: MusicPlaybackState,
    bgm_looping: bool,
    bgm_volume: f32,
    bgm_note_index: usize,
    bgm_note_elapsed_secs: f32,
    bgm_phase: f32,
    noise_seed: u32,
    events_processed: u64,
    total_samples_rendered: u64,
    last_peak_amplitude: f32,
}

impl Default for AudioOutputDriver {
    fn default() -> Self {
        Self::new(ProceduralSynthesizer::DEFAULT_SAMPLE_RATE, 16)
    }
}

impl AudioOutputDriver {
    /// Inicializa o driver físico com a taxa de amostragem e limite de polifonia especificados.
    pub fn new(sample_rate: u32, max_voices: usize) -> Self {
        Self {
            sample_rate: sample_rate.max(8_000),
            max_voices: max_voices.max(1),
            active_voices: Vec::with_capacity(max_voices.max(1)),
            active_bgm: None,
            bgm_state: MusicPlaybackState::Stopped,
            bgm_looping: true,
            bgm_volume: 0.8,
            bgm_note_index: 0,
            bgm_note_elapsed_secs: 0.0,
            bgm_phase: 0.0,
            noise_seed: 0x1234_5678,
            events_processed: 0,
            total_samples_rendered: 0,
            last_peak_amplitude: 0.0,
        }
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    pub fn active_voice_count(&self) -> usize {
        self.active_voices.len()
    }

    #[inline]
    pub fn events_processed(&self) -> u64 {
        self.events_processed
    }

    #[inline]
    pub fn total_samples_rendered(&self) -> u64 {
        self.total_samples_rendered
    }

    #[inline]
    pub fn last_peak_amplitude(&self) -> f32 {
        self.last_peak_amplitude
    }

    #[inline]
    pub fn bgm_state(&self) -> MusicPlaybackState {
        self.bgm_state
    }

    #[inline]
    pub fn active_bgm(&self) -> Option<MusicTrack> {
        self.active_bgm
    }

    /// Consome uma lista de `AudioEvent`s e dispara a síntese de vozes SFX ou transições de BGM.
    pub fn consume_events(&mut self, events: &[AudioEvent]) {
        for event in events {
            self.events_processed += 1;
            match *event {
                AudioEvent::PlaySound { cue, volume, pitch } => {
                    if volume <= 0.0001 {
                        continue;
                    }
                    let samples =
                        ProceduralSynthesizer::synthesize_sfx(cue, pitch, self.sample_rate);
                    if self.active_voices.len() >= self.max_voices {
                        // Voice stealing seguro: remove a voz mais antiga em reprodução
                        self.active_voices.remove(0);
                    }
                    self.active_voices.push(ActiveVoice {
                        cue,
                        samples,
                        cursor: 0,
                        volume: volume.clamp(0.0, 1.0),
                    });
                }
                AudioEvent::PlayMusic {
                    track,
                    loop_playback,
                    volume,
                } => {
                    self.active_bgm = Some(track);
                    self.bgm_state = MusicPlaybackState::Playing;
                    self.bgm_looping = loop_playback;
                    self.bgm_volume = volume.clamp(0.0, 1.0);
                    self.bgm_note_index = 0;
                    self.bgm_note_elapsed_secs = 0.0;
                    self.bgm_phase = 0.0;
                }
                AudioEvent::PauseMusic => {
                    if self.bgm_state == MusicPlaybackState::Playing {
                        self.bgm_state = MusicPlaybackState::Paused;
                    }
                }
                AudioEvent::ResumeMusic => {
                    if self.bgm_state == MusicPlaybackState::Paused {
                        self.bgm_state = MusicPlaybackState::Playing;
                    }
                }
                AudioEvent::StopMusic => {
                    self.bgm_state = MusicPlaybackState::Stopped;
                    self.active_bgm = None;
                    self.bgm_note_index = 0;
                    self.bgm_note_elapsed_secs = 0.0;
                }
            }
        }
    }

    /// Drena eventos pendentes do `AudioController` e sincroniza o volume de BGM em tempo real.
    pub fn sync_with_controller(&mut self, controller: &mut AudioController) {
        let events = controller.drain_events();
        if !events.is_empty() {
            self.consume_events(&events);
        }
        self.bgm_volume = controller.effective_bgm_volume();
    }

    /// Renderiza e mixa `sample_count` amostras físicas de áudio (SFX polifônicos + BGM sequenciado).
    pub fn render_samples(&mut self, sample_count: usize) -> Vec<f32> {
        let mut buffer = vec![0.0f32; sample_count];
        let sr = self.sample_rate as f32;
        let inv_sr = 1.0 / sr;
        let mut peak = 0.0f32;

        for out_sample in &mut buffer {
            let mut mixed = 0.0f32;

            // 1. Mixa todas as vozes de efeitos sonoros (SFX) ativas
            for voice in &mut self.active_voices {
                if voice.cursor < voice.samples.len() {
                    mixed += voice.samples[voice.cursor] * voice.volume;
                    voice.cursor += 1;
                }
            }

            // 2. Mixa o sintetizador sequencial de música de fundo (BGM)
            if self.bgm_state == MusicPlaybackState::Playing {
                if let Some(track) = self.active_bgm {
                    let (notes, waveform) = ProceduralSynthesizer::bgm_sequence_notes(track);
                    if !notes.is_empty() {
                        if self.bgm_note_index >= notes.len() {
                            if self.bgm_looping {
                                self.bgm_note_index = 0;
                            } else {
                                self.bgm_state = MusicPlaybackState::Stopped;
                            }
                        }

                        if self.bgm_state == MusicPlaybackState::Playing {
                            let (freq, duration) = notes[self.bgm_note_index];
                            let env = AdsrEnvelope::PAD.amplitude_at(
                                self.bgm_note_elapsed_secs,
                                duration.max(0.05),
                            );
                            let wave = ProceduralSynthesizer::sample_waveform(
                                waveform,
                                self.bgm_phase,
                                &mut self.noise_seed,
                            );
                            // Ganho suave para música de fundo não sobrepor os efeitos sonoros
                            mixed += wave * env * self.bgm_volume * 0.28;

                            self.bgm_phase = (self.bgm_phase + freq * inv_sr).rem_euclid(1.0);
                            self.bgm_note_elapsed_secs += inv_sr;
                            if self.bgm_note_elapsed_secs >= duration {
                                self.bgm_note_elapsed_secs = 0.0;
                                self.bgm_note_index += 1;
                            }
                        }
                    }
                }
            }

            // Limitador físico suave (soft-clip / clamp) para evitar distorção digital nos alto-falantes
            let clamped = mixed.clamp(-1.0, 1.0);
            let abs = clamped.abs();
            if abs > peak {
                peak = abs;
            }
            *out_sample = clamped;
        }

        // Remove vozes de SFX que já terminaram sua reprodução
        self.active_voices.retain(|v| !v.is_finished());

        self.total_samples_rendered += sample_count as u64;
        self.last_peak_amplitude = peak;
        buffer
    }

    /// Executa um passo completo do driver para um intervalo `dt_secs`:
    /// sincroniza com o `AudioController` e sintetiza o bloco correspondente de amostras PCM.
    pub fn step_audio(&mut self, dt_secs: f32, controller: &mut AudioController) -> Vec<f32> {
        self.sync_with_controller(controller);
        let sample_count = ((dt_secs.clamp(0.0, 0.25)) * self.sample_rate as f32).round() as usize;
        self.render_samples(sample_count.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adsr_envelope_and_waveforms() {
        let env = AdsrEnvelope::new(0.1, 0.1, 0.5, 0.1);
        assert_eq!(env.amplitude_at(-0.1, 1.0), 0.0);
        assert!((env.amplitude_at(0.05, 1.0) - 0.5).abs() < 1e-3);
        assert!((env.amplitude_at(0.1, 1.0) - 1.0).abs() < 1e-3);
        assert!((env.amplitude_at(0.5, 1.0) - 0.5).abs() < 1e-3);
        assert!((env.amplitude_at(0.95, 1.0) - 0.25).abs() < 1e-3);
        assert_eq!(env.amplitude_at(1.0, 1.0), 0.0);

        let mut seed = 42u32;
        for i in 0..100 {
            let phase = i as f32 / 100.0;
            for wf in [
                WaveformKind::Sine,
                WaveformKind::Square,
                WaveformKind::Triangle,
                WaveformKind::WhiteNoise,
            ] {
                let s = ProceduralSynthesizer::sample_waveform(wf, phase, &mut seed);
                assert!((-1.0..=1.0).contains(&s));
            }
        }
    }

    #[test]
    fn test_synthesize_all_sfx_cues_and_wav_encoding() {
        let all_cues = [
            SoundCue::UiClick,
            SoundCue::GoldCollect,
            SoundCue::FoodHarvest,
            SoundCue::FeedCreature,
            SoundCue::CreatureEvolve,
            SoundCue::EggDeposit,
            SoundCue::EggHatch,
            SoundCue::BuildingPlaced,
            SoundCue::ObstacleCleared,
            SoundCue::CombatAttackHit,
            SoundCue::CombatSuperEffective,
            SoundCue::CombatVictory,
            SoundCue::CombatDefeat,
            SoundCue::MilestoneClaim,
        ];

        for cue in all_cues {
            let pcm = ProceduralSynthesizer::synthesize_sfx(cue, 1.0, 22_050);
            assert!(!pcm.is_empty(), "Cue {:?} deve gerar amostras PCM", cue);
            assert!(
                pcm.iter().any(|&s| s.abs() > 0.05),
                "Cue {:?} deve possuir energia acústica não-nula",
                cue
            );
        }

        let click_pcm = ProceduralSynthesizer::synthesize_sfx(SoundCue::UiClick, 1.0, 22_050);
        let wav_bytes = ProceduralSynthesizer::encode_wav_pcm16(&click_pcm, 22_050);
        assert_eq!(&wav_bytes[0..4], b"RIFF");
        assert_eq!(&wav_bytes[8..12], b"WAVE");
        assert_eq!(wav_bytes.len(), 44 + click_pcm.len() * 2);
    }

    #[test]
    fn test_audio_output_driver_polyphony_and_bgm_mixing() {
        let mut controller = AudioController::new();
        let mut driver = AudioOutputDriver::new(22_050, 4);

        controller.play_music(MusicTrack::SanctuaryTheme, true);
        // Enfileira 6 efeitos simultâneos para testar voice stealing seguro com limite de 4 vozes
        for _ in 0..6 {
            controller.play_sound(SoundCue::GoldCollect);
        }

        let buffer = driver.step_audio(1.0 / 60.0, &mut controller);
        assert!(!buffer.is_empty());
        assert_eq!(driver.active_voice_count(), 4);
        assert_eq!(driver.events_processed(), 7);
        assert!(driver.last_peak_amplitude() > 0.01);
        assert_eq!(driver.bgm_state(), MusicPlaybackState::Playing);
        assert_eq!(driver.active_bgm(), Some(MusicTrack::SanctuaryTheme));

        // Avança tempo suficiente para concluir todos os SFX curtos
        let _ = driver.render_samples(22_050 / 2);
        assert_eq!(driver.active_voice_count(), 0);
    }
}

