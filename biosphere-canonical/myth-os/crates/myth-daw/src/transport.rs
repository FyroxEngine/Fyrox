use serde::{Deserialize, Serialize};

/// The master clock and playback state for the DAW.
///
/// Everything that moves in myth-os is slaved to Transport. The Theater tick
/// counter advances by one per `tick()` call. MIDI, audio, and narrative
/// capsules all read from here — never from wall time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transport {
    pub state:       PlayState,
    pub bpm:         f64,
    /// Current position in beats (fractional).
    pub position:    f64,
    /// How many beats per bar (default 4).
    pub beats_per_bar: u32,
    /// Ticks per beat — the internal resolution.
    pub ppq:         u32,
    pub loop_region: Option<LoopRegion>,
    /// Raw frame counter — incremented by `tick()` every call.
    /// Named `frame` to avoid shadowing the `tick()` method.
    pub frame:       u64,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            state:         PlayState::Stopped,
            bpm:           120.0,
            position:      0.0,
            beats_per_bar: 4,
            ppq:           96,
            loop_region:   None,
            frame:         0,
        }
    }
}

impl Transport {
    /// Advance the transport by one frame at the given sample rate.
    /// Returns the new beat position.
    pub fn tick(&mut self, sample_rate: f64) -> f64 {
        if self.state == PlayState::Playing || self.state == PlayState::Recording {
            let beats_per_second = self.bpm / 60.0;
            let beats_per_frame  = beats_per_second / sample_rate;
            self.position += beats_per_frame;

            if let Some(lp) = &self.loop_region {
                if self.position >= lp.end {
                    self.position = lp.start;
                }
            }
        }
        self.frame += 1;
        self.position
    }

    pub fn play(&mut self) {
        if self.state == PlayState::Stopped || self.state == PlayState::Paused {
            self.state = PlayState::Playing;
        }
    }

    pub fn pause(&mut self) {
        if self.state == PlayState::Playing {
            self.state = PlayState::Paused;
        }
    }

    pub fn stop(&mut self) {
        self.state    = PlayState::Stopped;
        self.position = 0.0;
    }

    pub fn record(&mut self) {
        self.state = PlayState::Recording;
    }

    pub fn seek(&mut self, beat: f64) {
        self.position = beat.max(0.0);
    }

    /// Current bar (0-based) and beat within bar (0-based).
    /// 0-indexed so sync math against audio/video buffers is frame-accurate.
    pub fn bar_beat(&self) -> (u64, u32) {
        let bpb  = self.beats_per_bar as f64;
        let bar  = (self.position / bpb).floor() as u64;
        let beat = (self.position % bpb).floor() as u32;
        (bar, beat)
    }

    /// Position formatted as BAR.BEAT for the transport display.
    pub fn position_display(&self) -> String {
        let (bar, beat) = self.bar_beat();
        format!("{bar:03}.{beat}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayState {
    Stopped,
    Playing,
    Paused,
    Recording,
}

/// A looping region defined in beats.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LoopRegion {
    pub start: f64,
    pub end:   f64,
}

impl LoopRegion {
    pub fn new(start: f64, end: f64) -> Self {
        assert!(end > start, "loop end must be after start");
        Self { start, end }
    }

    pub fn length_beats(&self) -> f64 { self.end - self.start }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(position: f64) -> Transport {
        Transport { position, ..Default::default() }
    }

    // ── bar_beat display alignment ────────────────────────────────────────────

    #[test]
    fn position_zero_is_bar0_beat0() {
        // 0-indexed: position 0.0 = bar 0, beat 0 — aligns with audio/video buffer index 0.
        assert_eq!(at(0.0).bar_beat(), (0, 0));
        assert_eq!(at(0.0).position_display(), "000.0");
    }

    #[test]
    fn beat_boundaries_in_4_4() {
        assert_eq!(at(0.0).bar_beat(), (0, 0));
        assert_eq!(at(1.0).bar_beat(), (0, 1));
        assert_eq!(at(2.0).bar_beat(), (0, 2));
        assert_eq!(at(3.0).bar_beat(), (0, 3));
        // Bar rollover: position 4.0 = bar 1, beat 0.
        assert_eq!(at(4.0).bar_beat(), (1, 0));
        assert_eq!(at(5.0).bar_beat(), (1, 1));
        assert_eq!(at(8.0).bar_beat(), (2, 0));
    }

    #[test]
    fn fractional_position_stays_in_same_beat() {
        // Position 0.5 is still bar 0, beat 0.
        assert_eq!(at(0.5).bar_beat(), (0, 0));
        // Position 3.99 is still bar 0, beat 3 — not bar 1.
        assert_eq!(at(3.99).bar_beat(), (0, 3));
        // Position 4.0 flips to bar 1, beat 0.
        assert_eq!(at(4.0).bar_beat(), (1, 0));
    }

    #[test]
    fn position_display_format() {
        assert_eq!(at(0.0).position_display(),  "000.0");
        assert_eq!(at(4.0).position_display(),  "001.0");
        assert_eq!(at(8.0).position_display(),  "002.0");
        assert_eq!(at(16.0).position_display(), "004.0");
    }

    // ── transport state machine ───────────────────────────────────────────────

    #[test]
    fn play_pause_stop_cycle() {
        let mut t = Transport::default();
        assert_eq!(t.state, PlayState::Stopped);
        t.play();
        assert_eq!(t.state, PlayState::Playing);
        t.pause();
        assert_eq!(t.state, PlayState::Paused);
        t.play();
        assert_eq!(t.state, PlayState::Playing);
        t.stop();
        assert_eq!(t.state, PlayState::Stopped);
        assert_eq!(t.position, 0.0);
    }

    #[test]
    fn stopped_transport_does_not_advance() {
        let mut t = Transport::default(); // Stopped
        t.tick(44100.0);
        assert_eq!(t.position, 0.0);
        assert_eq!(t.frame, 1); // frame counter still advances
    }

    #[test]
    fn playing_transport_advances_position() {
        let mut t = Transport::default();
        t.play();
        // 120 BPM, 1 tick at 120fps = 1/60 sec = 2/60 beats
        let pos = t.tick(120.0);
        assert!(pos > 0.0);
        assert_eq!(t.frame, 1);
    }

    #[test]
    fn loop_region_wraps_position() {
        let mut t = Transport::default();
        t.play();
        t.loop_region = Some(LoopRegion::new(0.0, 4.0));
        // Seek past the loop end.
        t.seek(3.99);
        // A tick should wrap back to the start.
        t.tick(1.0); // 1fps so each tick = 2 beats at 120 BPM
        assert!(t.position < 4.0, "should have wrapped: pos={}", t.position);
    }

    // ── seek ─────────────────────────────────────────────────────────────────

    #[test]
    fn seek_negative_clamps_to_zero() {
        let mut t = Transport::default();
        t.seek(-5.0);
        assert_eq!(t.position, 0.0);
    }
}
