use serde::{Deserialize, Serialize};
use myth_wire::{MythId, WirePacket, WireType};
use uuid::Uuid;

use crate::transport::{PlayState, Transport};

// ── DAW source ID ─────────────────────────────────────────────────────────────

fn daw_source() -> MythId {
    MythId::from_uuid(uuid::uuid!("da000000-0000-0000-0000-000000000001"))
}

// ── Transport packets (TMP wire type) ─────────────────────────────────────────

/// Emitted every tick while playing. Theater uses this to align renders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportTick {
    pub state:    PlayState,
    pub beat:     f64,
    pub bpm:      f64,
    pub bar:      u64,
    pub beat_num: u32,
}

impl TransportTick {
    pub fn from_transport(t: &Transport) -> Self {
        let (bar, beat_num) = t.bar_beat();
        Self {
            state:    t.state,
            beat:     t.position,
            bpm:      t.bpm,
            bar,
            beat_num,
        }
    }

    pub fn to_packet(&self, frame: u64) -> WirePacket {
        WirePacket::encode(WireType::Temporal, daw_source(), frame, self)
            .expect("TransportTick serialization is infallible")
    }
}

// ── Clip event packets (EVT wire type) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipEvent {
    Started { clip_id: Uuid, capsule_id: Uuid, track_name: String },
    Stopped { clip_id: Uuid },
    Looped  { clip_id: Uuid },
}

impl ClipEvent {
    pub fn to_packet(&self, tick: u64) -> WirePacket {
        WirePacket::encode(WireType::Event, daw_source(), tick, self)
            .expect("ClipEvent serialization is infallible")
        }
}

// ── Mixer level packets (CTL wire type) ───────────────────────────────────────

/// Sent when a fader value changes — Theater uses this to weight channel output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerLevel {
    pub channel_id: Uuid,
    pub level:      f64, // effective level after mute/solo
}

impl MixerLevel {
    pub fn to_packet(&self, tick: u64) -> WirePacket {
        WirePacket::encode(WireType::Control, daw_source(), tick, self)
            .expect("MixerLevel serialization is infallible")
    }
}

// ── Automation value packets (DAT wire type) ──────────────────────────────────

/// Emitted each tick for every active automation lane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationValue {
    pub track_id:  Uuid,
    pub param:     String,
    pub value:     f64,
}

impl AutomationValue {
    pub fn to_packet(&self, tick: u64) -> WirePacket {
        WirePacket::encode(WireType::Data, daw_source(), tick, self)
            .expect("AutomationValue serialization is infallible")
    }
}
