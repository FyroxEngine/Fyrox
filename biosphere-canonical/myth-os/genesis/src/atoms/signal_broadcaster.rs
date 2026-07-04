// GEN-ATOM-13: Physical Signal Broadcaster
// Streams simulation events to MIDI, DMX (serial), and GPIO.

use bevy::prelude::*;
use crossbeam_channel::{bounded, Receiver, Sender};
use midir::{MidiOutput, MidiOutputConnection};
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

// ── MIDI ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MidiEvent {
    pub channel: u8, // 0-15
    pub note: u8,
    pub velocity: u8,
    pub on: bool,
}

pub struct MidiBroadcaster {
    tx: Sender<MidiEvent>,
}

impl MidiBroadcaster {
    /// Spawns a background thread that owns the MIDI connection.
    /// Returns None if no MIDI output port is available.
    pub fn open(port_name_hint: &str) -> Option<Self> {
        let output = MidiOutput::new("genesis-midi").ok()?;
        let ports = output.ports();
        if ports.is_empty() {
            warn!("No MIDI output ports found");
            return None;
        }

        let port = ports
            .iter()
            .find(|p| {
                output
                    .port_name(p)
                    .map(|n| n.contains(port_name_hint))
                    .unwrap_or(false)
            })
            .or_else(|| ports.first())?;

        let port_name = output.port_name(port).unwrap_or_default();
        let conn = output.connect(port, "genesis-out").ok()?;
        let conn: Arc<Mutex<MidiOutputConnection>> = Arc::new(Mutex::new(conn));

        let (tx, rx): (Sender<MidiEvent>, Receiver<MidiEvent>) = bounded(256);

        std::thread::spawn(move || {
            info!(port = %port_name, "MIDI broadcaster running");
            for event in rx {
                let status = if event.on {
                    0x90 | (event.channel & 0x0F) // Note On
                } else {
                    0x80 | (event.channel & 0x0F) // Note Off
                };
                let msg = [status, event.note & 0x7F, event.velocity & 0x7F];
                if let Err(e) = conn.lock().unwrap().send(&msg) {
                    error!("MIDI send error: {:?}", e);
                }
            }
        });

        Some(Self { tx })
    }

    pub fn send(&self, event: MidiEvent) {
        let _ = self.tx.send(event);
    }

    pub fn note_on(&self, channel: u8, note: u8, velocity: u8) {
        self.send(MidiEvent { channel, note, velocity, on: true });
    }

    pub fn note_off(&self, channel: u8, note: u8) {
        self.send(MidiEvent { channel, note, velocity: 0, on: false });
    }
}

// ── DMX (serial) ──────────────────────────────────────────────────────────

/// 512-channel DMX universe, serialized over RS-485/serial.
pub struct DmxBroadcaster {
    tx: Sender<Box<[u8; 512]>>,
}

impl DmxBroadcaster {
    pub fn open(port: &str, baud: u32) -> Option<Self> {
        let mut serial = serialport::new(port, baud)
            .timeout(std::time::Duration::from_millis(100))
            .open()
            .ok()?;

        let (tx, rx): (Sender<Box<[u8; 512]>>, Receiver<Box<[u8; 512]>>) = bounded(64);
        let port_owned = port.to_string();

        std::thread::spawn(move || {
            info!(port = %port_owned, baud = %baud, "DMX broadcaster running");
            for frame in rx {
                // DMX512 break + MAB + start byte (0x00) + 512 channels
                let mut packet = Vec::with_capacity(514);
                packet.push(0x00); // start code
                packet.extend_from_slice(&*frame);
                if let Err(e) = serial.write_all(&packet) {
                    error!("DMX serial write error: {:?}", e);
                }
            }
        });

        Some(Self { tx })
    }

    /// Send a full 512-channel frame.
    pub fn send_frame(&self, channels: Box<[u8; 512]>) {
        let _ = self.tx.send(channels);
    }

    /// Set a single channel (1-indexed, DMX standard).
    pub fn set_channel(&self, channel: u16, value: u8) {
        let mut frame = Box::new([0u8; 512]);
        let idx = (channel.saturating_sub(1) as usize).min(511);
        frame[idx] = value;
        self.send_frame(frame);
    }
}

// ── Bevy resource wrappers ─────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct PhysicalSignalState {
    pub midi: Option<MidiBroadcaster>,
    pub dmx: Option<DmxBroadcaster>,
}

/// Bevy event: trigger a MIDI note from simulation logic.
#[derive(Event)]
pub struct SimMidiTrigger {
    pub channel: u8,
    pub note: u8,
    pub velocity: u8,
    pub on: bool,
}

/// Bevy event: set a DMX channel from simulation logic.
#[derive(Event)]
pub struct SimDmxSet {
    pub channel: u16,
    pub value: u8,
}

pub struct SignalBroadcasterPlugin {
    pub midi_port_hint: String,
    pub dmx_port: Option<String>,
}

impl Default for SignalBroadcasterPlugin {
    fn default() -> Self {
        Self {
            midi_port_hint: String::new(),
            dmx_port: None,
        }
    }
}

impl Plugin for SignalBroadcasterPlugin {
    fn build(&self, app: &mut App) {
        let midi = MidiBroadcaster::open(&self.midi_port_hint);
        if midi.is_some() {
            info!("MIDI broadcaster attached");
        }

        let dmx = self.dmx_port.as_deref().and_then(|p| {
            let d = DmxBroadcaster::open(p, 250_000);
            if d.is_some() { info!("DMX broadcaster attached on {}", p); }
            d
        });

        app.insert_resource(PhysicalSignalState { midi, dmx })
            .add_event::<SimMidiTrigger>()
            .add_event::<SimDmxSet>()
            .add_systems(Update, (dispatch_midi, dispatch_dmx));
    }
}

fn dispatch_midi(
    state: Res<PhysicalSignalState>,
    mut events: EventReader<SimMidiTrigger>,
) {
    if let Some(midi) = &state.midi {
        for ev in events.read() {
            midi.send(MidiEvent {
                channel: ev.channel,
                note: ev.note,
                velocity: ev.velocity,
                on: ev.on,
            });
        }
    }
}

fn dispatch_dmx(
    state: Res<PhysicalSignalState>,
    mut events: EventReader<SimDmxSet>,
) {
    if let Some(dmx) = &state.dmx {
        for ev in events.read() {
            dmx.set_channel(ev.channel, ev.value);
        }
    }
}
