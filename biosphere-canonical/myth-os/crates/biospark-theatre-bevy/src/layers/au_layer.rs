// THEATRE-AU: Generative ambient audio layer via cpal.
//
// Phase 5 ships an additive sine synthesiser (A2 drone + perfect fifth + octave)
// as the mock audio effect, proving the pipeline:
//   AuLayer::new()  →  cpal output stream (audio thread)
//                   →  AtomicU32 shared state (lock-free, no latency)
//   sync_au_state() →  writes beat / tempo / level each Bevy tick
//   audio callback  →  reads shared state, fills PCM buffer
//
// The drone is quiet ambient (−20 dBFS-ish): a slow A2 pedal + E3 fifth + A3
// octave, LFO-modulated harmonic balance, with a soft swell on each downbeat.
//
// TODO(phase-5-glyphs): Accept user audio code from myth-vault (e.g. a p5/WebAudio
//   sketch rendered inside a WebView) or replace DroneGen with a sample player.

use std::f32::consts::TAU;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

use bevy::ecs::system::NonSend;
use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use myth_wire::ChannelId;

use crate::compositor::{TheatreMixer, TheatreState};

// ── Shared state (main thread ↔ audio callback) ───────────────────────────────

/// Lock-free values shared between Bevy systems and the cpal audio callback.
///
/// All floats are stored as their bit-pattern in an `AtomicU32`.  This is
/// well-defined: `f32::to_bits` / `f32::from_bits` round-trip losslessly.
/// `Relaxed` ordering is sufficient — these are advisory values (beat, level),
/// not synchronisation primitives.
pub struct AuShared {
    /// Beat position [0.0, 1.0) from `TheatreState`.
    pub beat:    AtomicU32,
    /// Tempo in BPM from `TheatreState`.
    pub tempo:   AtomicU32,
    /// Channel fader level [0.0, 1.0] from `TheatreMixer`.
    pub level:   AtomicU32,
    /// Channel mute state.
    pub muted:   AtomicBool,
}

impl Default for AuShared {
    fn default() -> Self {
        Self {
            beat:  AtomicU32::new(0_f32.to_bits()),
            tempo: AtomicU32::new(120_f32.to_bits()),
            level: AtomicU32::new(1_f32.to_bits()),
            muted: AtomicBool::new(false),
        }
    }
}

impl AuShared {
    #[inline] pub fn beat(&self)  -> f32 { f32::from_bits(self.beat.load(Ordering::Relaxed)) }
    #[inline] pub fn tempo(&self) -> f32 { f32::from_bits(self.tempo.load(Ordering::Relaxed)) }
    #[inline] pub fn level(&self) -> f32 { f32::from_bits(self.level.load(Ordering::Relaxed)) }
    #[inline] pub fn muted(&self) -> bool { self.muted.load(Ordering::Relaxed) }
}

// ── Additive drone synthesiser ────────────────────────────────────────────────

/// Generates PCM samples: A2 pedal drone + E3 fifth + A3 octave.
///
/// Total peak level before `level` scaling ≈ −20 dBFS.
/// Soft-clips via `tanh`, so no hard clipping even at level = 1.0.
struct DroneGen {
    sample_rate: f32,
    channels:    usize,
    /// Phase accumulators for each oscillator (radians, wrapped to [0, TAU)).
    phases: [f32; 3],
    /// Base frequencies: A2 = 110 Hz, E3 = 165 Hz, A3 = 220 Hz.
    freqs:  [f32; 3],
    /// Relative amplitudes of each oscillator.
    amps:   [f32; 3],
    /// Slow LFO phase (0.05 Hz) — modulates harmonic balance.
    lfo:    f32,
}

impl DroneGen {
    fn new(sample_rate: u32, channels: usize) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            channels,
            phases: [0.0; 3],
            freqs:  [110.0, 165.0, 220.0], // root, fifth, octave
            amps:   [0.45,  0.30,  0.20 ],
            lfo:    0.0,
        }
    }

    /// Fill `output` with interleaved PCM samples (all channels get the same
    /// mono signal — Theatre audio is ambient mono-to-stereo).
    fn fill<T>(&mut self, output: &mut [T], beat: f32, level: f32)
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        let sr = self.sample_rate;

        // Slow LFO [0, 1]: modulates harmonic partial balance
        let lfo_val = self.lfo.sin() * 0.5 + 0.5;

        // Downbeat accent: brief +40% swell on the first 8% of each beat,
        // fading to zero over that window.
        let accent = if beat < 0.08 {
            (1.0 - beat / 0.08) * 0.40
        } else {
            0.0
        };

        for frame in output.chunks_mut(self.channels) {
            // Additive synthesis — three sine partials
            let mut mono = 0.0_f32;
            for i in 0..3 {
                // Upper partials get a tiny LFO-driven detuning for warmth
                let freq_factor = if i > 0 {
                    1.0 + lfo_val * 0.0015 * i as f32
                } else {
                    1.0
                };
                // Harmonic amplitude boosted slightly by LFO on partials 1 & 2
                let amp_mod = 1.0 + lfo_val * 0.35 * i as f32;
                mono += self.phases[i].sin() * self.amps[i] * amp_mod;

                // Advance phase
                self.phases[i] += TAU * self.freqs[i] * freq_factor / sr;
                if self.phases[i] >= TAU { self.phases[i] -= TAU; }
            }

            // Soft saturation via tanh, then scale to quiet ambient level
            mono = mono.tanh() * 0.80;
            mono *= level * (1.0 + accent) * 0.10; // ≈ −20 dBFS at level = 1.0

            let sample = T::from_sample(mono);
            for s in frame.iter_mut() { *s = sample; }

            // Advance LFO (0.05 Hz)
            self.lfo += TAU * 0.05 / sr;
            if self.lfo >= TAU { self.lfo -= TAU; }
        }
    }
}

// ── cpal stream construction ──────────────────────────────────────────────────

/// Build a typed output stream. Generic over sample type so the same audio
/// generation logic works regardless of what the OS device requires.
fn build_stream<T>(
    device:      &cpal::Device,
    config:      &cpal::StreamConfig,
    shared:      Arc<AuShared>,
    sample_rate: u32,
    channels:    usize,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32> + Send + 'static,
{
    let mut gen = DroneGen::new(sample_rate, channels);

    device
        .build_output_stream(
            config,
            move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
                if shared.muted() || shared.level() < 0.0001 {
                    output.fill(T::from_sample(0.0_f32));
                    return;
                }
                gen.fill(output, shared.beat(), shared.level());
            },
            |err| tracing::error!("AuLayer stream error: {err}"),
            None, // no timeout
        )
        .map_err(|e| format!("build_output_stream failed: {e}"))
}

// ── AuLayer ───────────────────────────────────────────────────────────────────

/// An audio channel that generates ambient audio via cpal.
///
/// The cpal `Stream` lives inside this struct and is kept alive for the
/// lifetime of the Theatre. Dropping `AuLayer` silently stops audio output.
pub struct AuLayer {
    pub channel_id: ChannelId,
    /// Shared state written by Bevy systems, read by the audio callback.
    pub shared: Arc<AuShared>,
    /// Kept alive — cpal stops the stream when this drops.
    _stream: cpal::Stream,
}

impl AuLayer {
    /// Open the default audio output device and start the ambient drone.
    ///
    /// Returns `Err` (with a descriptive message) if no audio output is
    /// available or the stream cannot be opened. The Theatre continues
    /// without audio — call site should log the error and skip `with_au_layer`.
    pub fn new(channel_id: ChannelId) -> Result<Self, String> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .ok_or_else(|| "no default audio output device found".to_string())?;

        let config = device
            .default_output_config()
            .map_err(|e| format!("default_output_config: {e}"))?;

        let sample_rate = config.sample_rate().0;
        let channels    = config.channels() as usize;
        let format      = config.sample_format();
        let stream_cfg  = cpal::StreamConfig::from(config.clone());

        let shared    = Arc::new(AuShared::default());
        let shared_cb = shared.clone();

        let stream = match format {
            cpal::SampleFormat::F32 =>
                build_stream::<f32>(&device, &stream_cfg, shared_cb, sample_rate, channels),
            cpal::SampleFormat::I16 =>
                build_stream::<i16>(&device, &stream_cfg, shared_cb, sample_rate, channels),
            cpal::SampleFormat::I32 =>
                build_stream::<i32>(&device, &stream_cfg, shared_cb, sample_rate, channels),
            cpal::SampleFormat::U16 =>
                build_stream::<u16>(&device, &stream_cfg, shared_cb, sample_rate, channels),
            other => Err(format!("sample format {other:?} not supported by AuLayer")),
        }?;

        stream
            .play()
            .map_err(|e| format!("stream.play(): {e}"))?;

        tracing::info!(
            channel    = channel_id.get(),
            device     = %device.name().unwrap_or_else(|_| "unknown".into()),
            sample_rate,
            channels,
            format     = ?format,
            "AuLayer: audio stream opened"
        );

        Ok(Self { channel_id, shared, _stream: stream })
    }
}

// ── Resources ─────────────────────────────────────────────────────────────────

/// Holds all active AU layers — keeps the cpal streams alive for the app lifetime.
///
/// `cpal::Stream` is `!Send + !Sync` (platform-generic, even on WASAPI where it is
/// actually safe).  We store this as a Bevy `NonSend` resource so it stays on the
/// main thread.  Systems that access it are automatically pinned to the main thread.
#[derive(Default)]
pub struct ActiveAuLayers(pub Vec<AuLayer>);

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct AuLayerPlugin;

impl Plugin for AuLayerPlugin {
    fn build(&self, app: &mut App) {
        // NonSend: cpal::Stream is !Send+!Sync; this resource lives on the main thread.
        app.init_non_send_resource::<ActiveAuLayers>()
            .add_systems(Update, sync_au_state);
    }
}

// ── Update system ─────────────────────────────────────────────────────────────

/// Push the current beat, tempo, and channel state to every AU layer's shared
/// state.  Runs every Bevy Update tick (~60 Hz).  The audio callback reads
/// these values lock-free at ~44 100 samples/sec.
fn sync_au_state(
    state:  Res<TheatreState>,
    mixer:  Res<TheatreMixer>,
    layers: NonSend<ActiveAuLayers>,
) {
    for layer in &layers.0 {
        let (level, muted) = match mixer.0.channel(layer.channel_id) {
            Some(ch) => (ch.level, ch.muted),
            None     => (0.0, true),
        };

        layer.shared.beat.store(state.beat.to_bits(),      Ordering::Relaxed);
        layer.shared.tempo.store(state.tempo_bpm.to_bits(), Ordering::Relaxed);
        layer.shared.level.store(level.to_bits(),           Ordering::Relaxed);
        layer.shared.muted.store(muted,                     Ordering::Relaxed);
    }
}
