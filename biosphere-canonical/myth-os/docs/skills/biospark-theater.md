---
name: biospark-theater
description: The BioSpark Theater System — a decoupled composite renderer for the Quantum Ecosystem. Load this skill before building anything in the `theater` crate or any output adapter. Defines what the Theater is, what a Channel is, how the compositor works, the Rust trait interfaces, Cargo feature gates for optional adapters, the build sequence, and what must never go inside it. TRIGGER: any time someone mentions the Theater System, channels, output adapters, the compositor, or asks how modules connect to a renderer.
---

# BioSpark Theater System

The Theater is the output layer of the BioSpark / myth-os ecosystem. It is a
decoupled composite renderer — a typed signal router and compositor that sits
between the narrative engine (mythos) and whatever is displaying, playing, or
recording the output.

The Theater does not care about content. It cares about signal type.
The Theater does not run the simulation. It observes it.
The Theater can be swapped entirely without touching a single module.

---

## The Mental Model

Think of **Adobe After Effects**, not a TV remote.

In After Effects:
- There are layers — each layer is independent
- Multiple layers can be active simultaneously
- Layers composite on top of each other in z-order
- You can solo a layer, mute a layer, toggle visibility
- The composition still exists whether any layer is visible or not

The BioSpark Theater works the same way, except the "layers" are **Channels**,
and instead of video tracks they carry typed wire packets from Quantum modules.

```
THEATER COMPOSITION
─────────────────────────────────────────────────────
Ch 1  [TERRAIN/ATLAS]    ███████████████  ACTIVE   SPA
Ch 2  [STORY/LOOM]       ░░░░░░░░░░░░░░░  MUTED    NAR
Ch 3  [AUDIO/COMPOSER]   ███████████████  ACTIVE   AUD
Ch 4  [BEHAVIOR]         ░░░░░░░░░░░░░░░  INACTIVE BHV
Ch 5  [RACK/EGUI]        ███████████████  ACTIVE   CTL
Ch 6  [WORD PROCESSOR]   ░░░░░░░░░░░░░░░  CONNECTED NAR
─────────────────────────────────────────────────────
COMPOSITE OUTPUT → [Bevy 3D] + [Audio] + [egui overlay]
```

Flipping Ch 2 to ACTIVE does not restart anything. The narrative simulation
has been running the whole time. You just turned on the display for it.

---

## Three Processes, One System

The BioSpark Theater System runs as three separate binaries communicating over
TCP. They are logically and physically separated. They can run on the same
machine (localhost) or across a network — the code is identical either way.

```
engine           binds  TCP localhost:7700   (or any host:port)
                 streams wire packets to all connected clients
                 keeps running if clients disconnect

instruments      connects to engine TCP
                 egui rack UI — humans control the simulation
                 sends CTL wire packets back to engine
                 receives state snapshots for display

theater-bin      connects to engine TCP
                 receives wire packets, routes to channel handlers
                 composites active channels, renders output
                 can export .qgcp when triggered
```

**Transport rule:** TCP only. No shared memory. No in-process shortcuts.
Moving to a home network or the cloud is a config change — just change the
IP address. Adding TLS is a wrapper around the TCP stream. Nothing else changes.

---

## The Theater as a Library Crate

The Theater is a **library crate** first. The binary (`theater-bin`) is a thin
wrapper that imports it. This means any BioSpark project can import just the
compositor logic without pulling in renderers they don't need.

```toml
# crates/theater/Cargo.toml
[package]
name = "biospark-theater"
version = "0.1.0"
edition = "2021"

[lib]
name = "theater"

[features]
default = []
bevy  = ["dep:bevy"]
egui  = ["dep:egui", "dep:eframe"]
audio = ["dep:cpal"]

[dependencies]
myth-wire = { path = "../myth-wire" }         # or git/crates.io when published
serde     = { version = "1", features = ["derive"] }
bincode   = "2"
tokio     = { version = "1", features = ["full"] }

# Optional — only compiled when features are enabled
bevy  = { version = "0.14", optional = true }
egui  = { version = "0.28", optional = true }
eframe = { version = "0.28", optional = true }
cpal  = { version = "0.15", optional = true }
```

**Usage examples:**

```toml
# A 3D world viewer — needs Bevy and audio, no egui rack
biospark-theater = { version = "0.1", features = ["bevy", "audio"] }

# A novel-writing tool — needs egui panels only, no 3D, no audio
biospark-theater = { version = "0.1", features = ["egui"] }

# A headless exporter — no renderer at all
biospark-theater = { version = "0.1" }

# The full theater binary — everything
biospark-theater = { version = "0.1", features = ["bevy", "egui", "audio"] }
```

**The progression:**

```
Phase 1 — workspace path deps (development, no publishing friction)
          myth-wire = { path = "../myth-wire" }

Phase 2 — git deps (share across projects, pin versions)
          myth-wire = { git = "https://github.com/biospark/myth-wire", tag = "v0.1.0" }

Phase 3 — crates.io (when the API is stable enough to commit to publicly)
          myth-wire = "0.1"
```

You stay in Phase 1 the entire time you are actively developing. Publishing
is just a decision, not a refactor.

---

## What the Theater IS

- A typed signal router — routes wire type packets to registered output handlers
- A channel compositor — manages which channels are active and layers their output
- A connection registry — knows what modules are feeding what channels
- A pure Rust library crate, zero renderer dependencies in the base feature set

## What the Theater IS NOT

- Not the simulation (mythos owns that)
- Not a module (it has no wire type outputs of its own)
- Not a renderer (it delegates to output adapters behind feature flags)
- Not a window manager (the adapter owns the window)
- Not a Bevy plugin, egui app, or anything renderer-specific in its base form

---

## Channels

A **Channel** is a named, persistent routing configuration. It is not a TV
channel you flip to exclusively — it is an After Effects layer. Multiple channels
can be active simultaneously. Their outputs composite.

```rust
pub struct Channel {
    pub id:         ChannelId,
    pub name:       String,
    pub wire_types: Vec<WireType>,
    pub state:      ChannelState,
    pub z_order:    u32,
}
```

### Channel States

```
Disconnected   No output handler registered. Signal flows nowhere.
               The simulation still runs. Data is just not displayed.

Connected      Output handler registered, not yet active.

Active         Live. Wire packets routed to handler. Output composited.

Muted          Handler receives packets but does not render visibly.
               Useful for recording without display.

Solo           Only this channel's visual output composites.
               All others effectively muted for display.

Recording      Active AND writing output to file/stream.
               Can combine with Active, Muted, or Solo.
```

### Channel Presets (Canonical 16)

| Ch | Name | Modules | Wire Types | Default Output |
|----|------|---------|------------|----------------|
| 01 | Atlas — Terrain | GEN-01 | SPA | Bevy 3D mesh |
| 02 | Genesis — World | GEN-01..08 | SPA, VIS, BHV | Bevy 3D full scene |
| 03 | Environment | GEN-02, GEN-04 | SPA, VIS | Bevy 3D sky/fog |
| 04 | Entities | GEN-05..07 | VIS, BHV | Bevy 3D actors |
| 05 | Society | GEN-08 | SOC | egui graph view |
| 06 | Loom — Word Processor | GEN-10, GEN-11 | NAR | text editor UI |
| 07 | Chronicle — Timeline | GEN-09 | TMP | timeline UI |
| 08 | Memory | GEN-11 | NAR | memory viewer |
| 09 | Logic | GEN-13 | LGC | node graph UI |
| 10 | Simulation | GEN-14 | DAT | data dashboard |
| 11 | Sound | GEN-12 | AUD | waveform / audio |
| 12 | Rack / Instruments | all | CTL | egui rack panels |
| 13 | Forge — Assets | GEN-15 | AST | asset browser |
| 14 | Network | GEN-16 | EVT | network monitor |
| 15 | Identity / BDna | all | IDN | lineage viewer |
| 16 | Record / Playback | all | DAT | file I/O |

---

## Rust Trait Interfaces

### WirePacket — the signal unit

```rust
/// Must always be Serialize + Deserialize — it crosses process boundaries.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WirePacket {
    pub wire_type: WireType,
    pub source_id: ModuleId,
    pub tick:      u64,
    pub payload:   WirePayload,
}
```

### TheaterModule — how a module plugs in

```rust
pub trait TheaterModule: Send + Sync {
    fn id(&self) -> ModuleId;
    fn wire_outputs(&self) -> &[WireType];
    fn wire_inputs(&self)  -> &[WireType] { &[] }
    fn tick(&mut self, world: &WorldState, dt: f64) -> Vec<WirePacket>;
    fn receive(&mut self, _packet: &WirePacket) {}
    fn is_standalone(&self) -> bool { true }
}
```

### OutputHandler — how an adapter plugs in

```rust
pub trait OutputHandler: Send + Sync {
    fn name(&self) -> &str;
    fn accepts(&self) -> &[WireType];
    fn handle(&mut self, packet: &WirePacket, channel: &Channel);
    fn composite(&self) -> Option<Frame> { None }
    fn on_state_change(&mut self, _new_state: ChannelState) {}
}
```

### EngineTransport — behind a trait, swappable

```rust
pub trait EngineTransport: Send + Sync {
    fn bind(addr: &str)    -> Result<Self, TransportError> where Self: Sized;
    fn connect(addr: &str) -> Result<Self, TransportError> where Self: Sized;
    fn send(&mut self, packet: &WirePacket) -> Result<(), TransportError>;
    fn recv(&mut self)     -> Result<WirePacket, TransportError>;
}

/// Default: TCP stream, bincode framing.
/// localhost → home network → cloud: change the addr string, nothing else.
pub struct TcpTransport { /* tokio TcpStream */ }
impl EngineTransport for TcpTransport { ... }
```

---

## Build Sequence — Step by Step

Each step compiles and has passing tests before the next begins.

### Step 1 — `myth-wire` crate (foundation)
```
crates/myth-wire/
├── Cargo.toml    (serde, bincode only — zero other deps)
└── src/
    ├── lib.rs
    ├── wire_type.rs   (WireType enum, 17 variants)
    ├── packet.rs      (WirePacket, WirePayload)
    ├── bdna.rs        (BDna = Vec<bool>, 64-element invariant)
    └── ids.rs         (ModuleId, ChannelId, typed newtypes)
```
Deliverable: `cargo test -p myth-wire` — all 17 wire types round-trip
through bincode. No panics. No renderer deps anywhere in the tree.

### Step 2 — `biospark-theater` library crate (no features)
```
crates/theater/
├── Cargo.toml    (myth-wire, serde, bincode, tokio)
└── src/
    ├── lib.rs
    ├── channel.rs     (Channel, ChannelState, z_order)
    ├── traits.rs      (TheaterModule, OutputHandler)
    ├── transport.rs   (EngineTransport trait + TcpTransport)
    ├── frame.rs       (TheaterFrame, CompositeFrame)
    └── theater.rs     (Theater struct — router + compositor)
```
Deliverable: `cargo build -p biospark-theater` — zero warnings.
Zero renderer deps. Library only, no binary yet.

### Step 3 — NullOutput + serialization round-trip tests
`cargo test -p biospark-theater`
- WirePacket → bincode → WirePacket: all variants pass
- NullOutput receives packets correctly
- Channel state transitions all valid paths

### Step 4 — Engine binary (headless TCP server)
```
bins/engine/
├── Cargo.toml    (biospark-theater, qgcp)
└── src/main.rs   (~60 lines: bind TCP, tick loop, broadcast packets)
```
Deliverable: `cargo run -p engine` — starts silently, logs
"Engine listening on 127.0.0.1:7700". No window. No GPU.

### Step 5 — Instruments binary (egui, `egui` feature)
```
bins/instruments/
├── Cargo.toml    (biospark-theater features=["egui"])
└── src/main.rs   (connect to engine, draw rack panels, send CTL packets)
```
Deliverable: `cargo run -p instruments` — rack window opens.
Moving a fader logs a CTL packet in the engine terminal.

### Step 6 — Theater binary (first visual, `egui` feature)
```
bins/theater-bin/
├── Cargo.toml    (biospark-theater features=["egui"])
└── src/main.rs   (connect to engine, route packets, render data panels)
```
Deliverable: `cargo run -p theater-bin` — data dashboard opens.
Three terminals, three processes, all communicating.

### Step 7 — Bevy output handler (`bevy` feature)
Add `bevy` feature to theater. Implement `OutputHandler` for SPA/VIS.
Theater tick loop calls the handler — Bevy does NOT own the main loop.
Deliverable: terrain visible in the theater window.
Engine → TCP → Theater → Bevy. Fader in Instruments moves terrain.

### Step 8 — Audio output handler (`audio` feature)
Add `audio` feature. CPAL handler for AUD wire type.
Deliverable: `cargo run -p theater-bin -- --features audio` plays sound.

### Step 9 — Channel preset config
Channel configurations load from JSON. Saved between sessions.
Deliverable: edit `channels.json`, restart theater-bin, new layout loads.

### Step 10 — `.qgcp` export
Theater writes a fully-resolved `.qgcp` snapshot on demand.
Deliverable: `Ctrl+E` in theater-bin writes `output.qgcp`.

---

## Crate Dependency Graph (Target)

```
myth-wire                       (types only — serde, bincode)
    ↑
biospark-theater                (compositor lib — tokio, myth-wire)
    ├── feature: bevy  → bevy
    ├── feature: egui  → egui, eframe
    └── feature: audio → cpal
    ↑
bins/engine                     (headless TCP server)
bins/instruments                (egui rack — features=["egui"])
bins/theater-bin                (full output — features=["bevy","egui","audio"])
```

Other BioSpark projects import `biospark-theater` with only the features they need.
The simulation (`mythos`) and the theater are the same regardless of which features
are compiled in. Features only add output handlers.

---

## What the Theater Must Never Do

```
✗  Own the simulation tick (mythos owns that)
✗  Import from module crates directly
✗  Put renderer code in the base crate (features only)
✗  Use shared memory between engine / instruments / theater-bin
   (TCP even on localhost — the discipline is the point)
✗  Route packets by payload content (route by WireType only)
✗  Let adapters call modules directly
✗  Break the EngineTransport abstraction by using TcpTransport directly
   in calling code (always use the trait)
```

---

## Anti-Patterns

**Bevy as host:** Making `App::run()` the main loop.
Fix: Theater tick loop is the host. Bevy OutputHandler is called from it.

**Shared memory shortcut:** `Arc<Mutex<>>` between binaries because
"they're on the same machine anyway."
Fix: TCP even on localhost. This is what lets you move to the network for free.

**Monolithic feature set:** One feature that enables everything.
Fix: Granular features. Users compile only what they need.

**Payload inspection in router:** Pattern-matching packet content to decide routing.
Fix: Route by WireType. Content is the handler's concern.

**Fat binary:** All logic in `main.rs`, library crate is a shell.
Fix: Library crate holds all logic. Binary is 30-60 lines of wiring.
