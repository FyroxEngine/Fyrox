use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::clip::Clip;
use crate::track::Track;

/// The Arrangement — linear timeline, the traditional DAW view.
///
/// Tracks run horizontally in time. Clips sit at specific beat positions.
/// Automation curves live in lanes below each track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arrangement {
    pub tracks:     Vec<ArrangementTrack>,
    /// Total length of the arrangement in beats. Expands automatically.
    pub length:     f64,
}

impl Arrangement {
    pub fn new() -> Self {
        Self { tracks: Vec::new(), length: 128.0 }
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(ArrangementTrack::new(track));
    }

    pub fn place_clip(&mut self, track_id: Uuid, mut clip: Clip, start_beat: f64) {
        if let Some(t) = self.tracks.iter_mut().find(|t| t.track.id == track_id) {
            clip.start_beat = start_beat;
            // Expand arrangement if the clip extends past current length.
            if let Some(end) = clip.end_beat() {
                if end > self.length { self.length = end + 4.0; }
            }
            t.clips.push(clip);
            t.clips.sort_by(|a, b| a.start_beat.partial_cmp(&b.start_beat).unwrap());
        }
    }

    pub fn remove_clip(&mut self, clip_id: Uuid) {
        for t in &mut self.tracks {
            t.clips.retain(|c| c.id != clip_id);
        }
    }

    /// All clips active at a given beat position, across all tracks.
    pub fn active_clips_at(&self, beat: f64) -> Vec<(&Track, &Clip)> {
        self.tracks.iter()
            .flat_map(|t| t.clips.iter()
                .filter(move |c| c.is_active_at(beat))
                .map(move |c| (&t.track, c)))
            .collect()
    }
}

impl Default for Arrangement {
    fn default() -> Self { Self::new() }
}

/// A track's presence in the arrangement — its clips and automation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrangementTrack {
    pub track:      Track,
    pub clips:      Vec<Clip>,
    pub automation: Vec<AutomationLane>,
}

impl ArrangementTrack {
    pub fn new(track: Track) -> Self {
        Self { track, clips: Vec::new(), automation: Vec::new() }
    }

    pub fn add_automation(&mut self, param: impl Into<String>) -> &mut AutomationLane {
        self.automation.push(AutomationLane::new(param));
        self.automation.last_mut().unwrap()
    }
}

/// A single automation lane — a parameter varying over time as breakpoints.
///
/// Matches the spec's "draw automation curves / breakpoints for precise control."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationLane {
    pub id:          Uuid,
    /// Parameter name — e.g. "tension", "volume", "filter_cutoff".
    pub param:       String,
    /// Sorted list of (beat, value 0.0–1.0) breakpoints.
    pub breakpoints: Vec<(f64, f64)>,
    pub enabled:     bool,
}

impl AutomationLane {
    pub fn new(param: impl Into<String>) -> Self {
        Self {
            id:          Uuid::new_v4(),
            param:       param.into(),
            breakpoints: Vec::new(),
            enabled:     true,
        }
    }

    pub fn add_point(&mut self, beat: f64, value: f64) {
        let value = value.clamp(0.0, 1.0);
        // Keep sorted by beat.
        let pos = self.breakpoints.partition_point(|(b, _)| *b < beat);
        self.breakpoints.insert(pos, (beat, value));
    }

    pub fn remove_near(&mut self, beat: f64, tolerance: f64) {
        self.breakpoints.retain(|(b, _)| (b - beat).abs() > tolerance);
    }

    /// Linear interpolation between breakpoints at a given beat.
    pub fn value_at(&self, beat: f64) -> f64 {
        if self.breakpoints.is_empty() { return 0.0; }
        let pts = &self.breakpoints;

        // Before first point.
        if beat <= pts[0].0 { return pts[0].1; }
        // After last point.
        if beat >= pts[pts.len() - 1].0 { return pts[pts.len() - 1].1; }

        // Find surrounding pair and lerp.
        let idx = pts.partition_point(|(b, _)| *b < beat);
        let (b0, v0) = pts[idx - 1];
        let (b1, v1) = pts[idx];
        let t = (beat - b0) / (b1 - b0);
        v0 + t * (v1 - v0)
    }
}
