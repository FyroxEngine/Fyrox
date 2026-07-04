use serde::{Deserialize, Serialize};

/// A single heartbeat from the OS master clock.
///
/// Every subscriber receives one of these per frame. All timing in the system
/// is derived from `frame` and `elapsed_secs` — never from wall time directly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tick {
    /// Absolute frame counter since clock start. Never resets.
    /// This is the Genesis Protocol's sequence number — tick 0 is the Big Bang.
    pub frame: u64,

    /// Seconds elapsed since clock start.
    pub elapsed_secs: f64,

    /// Duration of this frame in seconds (1.0 / sample_rate).
    pub delta_secs: f64,

    /// The clock's configured sample rate (frames per second).
    pub sample_rate: f64,

    /// Temperature — starts at 1.0 (hot/fluid), cools toward 0.0 (crystallised).
    /// Mirrors the ATOMS engine: high temp = bonding phase, low = stable structure.
    /// Genesis Protocol uses this to know when the world has crystallised.
    pub temperature: f64,

    /// Which phase the clock is in.
    pub phase: ClockPhase,
}

impl Tick {
    /// Beats elapsed at a given BPM.
    pub fn beats_at_bpm(&self, bpm: f64) -> f64 {
        self.elapsed_secs * (bpm / 60.0)
    }

    /// True on the first tick of each new beat at the given BPM.
    pub fn is_beat_boundary(&self, bpm: f64) -> bool {
        let beats_now  = self.beats_at_bpm(bpm);
        let beats_prev = (self.elapsed_secs - self.delta_secs) * (bpm / 60.0);
        beats_now.floor() > beats_prev.floor()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClockPhase {
    /// Clock is warming up — subscribers registering, resources loading.
    Booting,
    /// Clock is running — simulation active.
    Running,
    /// Clock is cooling — world crystallising (Genesis Protocol settling).
    Crystallising,
    /// Clock is paused — simulation suspended.
    Paused,
    /// Clock has stopped — clean shutdown.
    Stopped,
}
