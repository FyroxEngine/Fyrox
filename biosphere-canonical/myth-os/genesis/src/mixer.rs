use bevy::prelude::*;
use crossbeam_channel::{bounded, Receiver, Sender};
use midir::{MidiInput, MidiInputConnection};
use mythos::quantum_module::Department;
use std::sync::Mutex;
use tracing::{info, warn};

// ── Mixable trait ─────────────────────────────────────────────────────────

/// Any Genesis component that can be controlled by a MIDI fader/knob.
/// Implement this on every module component to make it Traktor-addressable.
pub trait Mixable: Send + Sync + 'static {
    fn parameters(&self) -> Vec<&'static str>;
    fn set_parameter(&mut self, param: &str, value: f32); // value is 0.0–1.0 normalized
    fn get_parameter(&self, param: &str) -> f32;
}

// ── InstrumentControl (Bevy ECS component) ────────────────────────────────

/// Attached to any entity that should respond to MIDI CC input.
#[derive(Component, Debug, Clone)]
pub struct InstrumentControl {
    pub module_id: String,
    pub midi_cc: u8,
    pub parameter: String,
    pub bus_channel: BusChannel,
    pub scale_min: f32,
    pub scale_max: f32,
}

impl InstrumentControl {
    pub fn new(
        module_id: impl Into<String>,
        midi_cc: u8,
        parameter: impl Into<String>,
        bus: BusChannel,
    ) -> Self {
        Self {
            module_id: module_id.into(),
            midi_cc,
            parameter: parameter.into(),
            bus_channel: bus,
            scale_min: 0.0,
            scale_max: 1.0,
        }
    }

    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.scale_min = min;
        self.scale_max = max;
        self
    }
}

// ── Bus channels (map to Traktor channels 1–4) ────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BusChannel {
    Structure  = 1, // Ch1: Terrain + Architect
    Entities   = 2, // Ch2: Modeling + Behavior
    Atmosphere = 3, // Ch3: Environment + Story
    Dynamics   = 4, // Ch4: Lighting + Sound
}

impl BusChannel {
    pub fn from_department(dept: &Department) -> Self {
        match dept {
            Department::Structure  => Self::Structure,
            Department::Entities   => Self::Entities,
            Department::Atmosphere => Self::Atmosphere,
            Department::Dynamics   => Self::Dynamics,
        }
    }

    pub fn from_traktor_channel(ch: u8) -> Option<Self> {
        match ch {
            1 => Some(Self::Structure),
            2 => Some(Self::Entities),
            3 => Some(Self::Atmosphere),
            4 => Some(Self::Dynamics),
            _ => None,
        }
    }
}

// ── MIDI event ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Event)]
pub struct MidiCcEvent {
    pub channel: u8,  // 0-indexed Traktor channel
    pub cc: u8,
    pub value: u8,    // 0–127
}

impl MidiCcEvent {
    pub fn normalized(&self) -> f32 {
        self.value as f32 / 127.0
    }
}

// ── MIDI receiver resource ────────────────────────────────────────────────

#[derive(Resource)]
pub struct MidiReceiver {
    rx: Receiver<MidiCcEvent>,
    // Mutex makes MidiInputConnection Sync so we can store it as a Bevy Resource.
    _conn: Mutex<Option<MidiInputConnection<()>>>,
}

impl MidiReceiver {
    pub fn open(device_hint: &str) -> Self {
        let (tx, rx) = bounded::<MidiCcEvent>(1024);
        let conn = Self::try_connect(device_hint, tx);
        Self { rx, _conn: Mutex::new(conn) }
    }

    fn try_connect(hint: &str, tx: Sender<MidiCcEvent>) -> Option<MidiInputConnection<()>> {
        let input = MidiInput::new("genesis-in").ok()?;
        let ports = input.ports();
        if ports.is_empty() {
            warn!("No MIDI input ports found");
            return None;
        }

        let port = ports
            .iter()
            .find(|p| input.port_name(p).map(|n| n.contains(hint)).unwrap_or(false))
            .or_else(|| ports.first())?;

        let name = input.port_name(port).unwrap_or_default();
        info!(port = %name, "MIDI input connected");

        let conn = input
            .connect(
                port,
                "genesis-recv",
                move |_stamp, msg, _| {
                    // CC messages: status 0xBn, cc, value
                    if msg.len() == 3 && (msg[0] & 0xF0) == 0xB0 {
                        let _ = tx.send(MidiCcEvent {
                            channel: msg[0] & 0x0F,
                            cc: msg[1],
                            value: msg[2],
                        });
                    }
                },
                (),
            )
            .ok()?;

        Some(conn)
    }

    pub fn drain(&self) -> Vec<MidiCcEvent> {
        self.rx.try_iter().collect()
    }
}

// ── Traktor mixer Bevy plugin ─────────────────────────────────────────────

pub struct TraktorMixerPlugin {
    pub device_hint: String,
}

impl Default for TraktorMixerPlugin {
    fn default() -> Self {
        Self { device_hint: "Traktor".into() }
    }
}

impl Plugin for TraktorMixerPlugin {
    fn build(&self, app: &mut App) {
        let receiver = MidiReceiver::open(&self.device_hint);
        app.insert_resource(receiver)
            .add_event::<MidiCcEvent>()
            .add_systems(Update, pump_midi_events)
            .add_systems(Update, log_unhandled_cc);
    }
}

/// Pull raw MIDI bytes from the background thread into Bevy events each frame.
fn pump_midi_events(
    receiver: Res<MidiReceiver>,
    mut writer: EventWriter<MidiCcEvent>,
) {
    for event in receiver.drain() {
        writer.send(event);
    }
}

/// Generic system: routes CC events to InstrumentControl components.
/// Call `apply_midi_to::<YourComponent>` in your plugin's build() to wire a module.
pub fn apply_midi_to<T: Component + Mixable>(
    mut events: EventReader<MidiCcEvent>,
    mut query: Query<(&mut T, &InstrumentControl)>,
) {
    for ev in events.read() {
        for (mut component, ctrl) in query.iter_mut() {
            if ev.cc == ctrl.midi_cc {
                let normalized = ev.normalized();
                let scaled = ctrl.scale_min + normalized * (ctrl.scale_max - ctrl.scale_min);
                component.set_parameter(&ctrl.parameter, scaled);
            }
        }
    }
}

fn log_unhandled_cc(mut events: EventReader<MidiCcEvent>) {
    for ev in events.read() {
        tracing::trace!(ch = ev.channel, cc = ev.cc, val = ev.value, "MIDI CC");
    }
}
