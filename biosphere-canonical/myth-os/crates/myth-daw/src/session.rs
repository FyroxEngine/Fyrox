use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::clip::{Clip, ClipState};
use crate::track::Track;

/// The Session (clip launcher) — the grid of clips and scenes.
///
/// Rows = Scenes, Columns = Tracks.
/// Any scene can be triggered to launch all its clips simultaneously.
/// This is the Ableton Session View equivalent — for improvisation and
/// non-linear performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub tracks: Vec<Track>,
    pub scenes: Vec<Scene>,
    /// clips[scene_idx][track_idx] — None means empty slot.
    pub clips:  Vec<Vec<Option<Clip>>>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            scenes: Vec::new(),
            clips:  Vec::new(),
        }
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
        // Add an empty slot for this track in every existing scene.
        for row in &mut self.clips {
            row.push(None);
        }
    }

    pub fn add_scene(&mut self, name: impl Into<String>) {
        self.scenes.push(Scene::new(name));
        // Add a row of empty slots for every existing track.
        self.clips.push(vec![None; self.tracks.len()]);
    }

    pub fn set_clip(&mut self, scene: usize, track: usize, clip: Clip) {
        if let Some(row) = self.clips.get_mut(scene) {
            if let Some(slot) = row.get_mut(track) {
                *slot = Some(clip);
            }
        }
    }

    pub fn clear_slot(&mut self, scene: usize, track: usize) {
        if let Some(row) = self.clips.get_mut(scene) {
            if let Some(slot) = row.get_mut(track) {
                *slot = None;
            }
        }
    }

    /// Trigger all clips in a scene — queues them to start together.
    pub fn trigger_scene(&mut self, scene: usize) {
        if let Some(row) = self.clips.get_mut(scene) {
            for slot in row.iter_mut().flatten() {
                slot.state = ClipState::Queued;
            }
        }
        if let Some(s) = self.scenes.get_mut(scene) {
            s.playing = true;
        }
    }

    /// Stop all playing clips.
    pub fn stop_all(&mut self) {
        for row in &mut self.clips {
            for slot in row.iter_mut().flatten() {
                if slot.state == ClipState::Playing {
                    slot.state = ClipState::Stopping;
                }
            }
        }
        for scene in &mut self.scenes {
            scene.playing = false;
        }
    }

    /// Advance queued clips to playing on a quantize boundary.
    pub fn commit_queued(&mut self) {
        for row in &mut self.clips {
            for slot in row.iter_mut().flatten() {
                if slot.state == ClipState::Queued  { slot.state = ClipState::Playing; }
                if slot.state == ClipState::Stopping { slot.state = ClipState::Idle; }
            }
        }
    }

    pub fn track_count(&self) -> usize { self.tracks.len() }
    pub fn scene_count(&self) -> usize { self.scenes.len() }
}

impl Default for Session {
    fn default() -> Self { Self::new() }
}

/// A horizontal row in the session grid — triggers all its clips together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id:      Uuid,
    pub name:    String,
    pub color:   Option<[u8; 3]>,
    pub playing: bool,
    /// Tempo override while this scene is active. None = use master BPM.
    pub bpm:     Option<f64>,
}

impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id:      Uuid::new_v4(),
            name:    name.into(),
            color:   None,
            playing: false,
            bpm:     None,
        }
    }
}
