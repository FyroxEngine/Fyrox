---
name: myth-os-architecture
description: The foundational architecture law for the myth-os / BioSpark Quantum Ecosystem. Load this skill before designing any new crate, binary, module, or feature. Defines the narrative-engine-first principle, three-layer architecture, library-crate model, Cargo feature gates, workspace layout, crate ownership rules, module independence law, and the anti-patterns that have caused full restarts. TRIGGER: any time new code is being planned, any new crate or module is being added, any time Bevy or a renderer is discussed as a dependency, or any time the workspace structure is being changed.
---

# myth-os Architecture Law

This is the north star document for the myth-os / BioSpark Quantum Ecosystem.
Read it entirely before writing any code.

Violations of these rules have caused full project restarts.
The rules exist to prevent that from happening again.

---

## The Prime Law

**Genesis is a narrative engine. Rendering is a consequence.**

The simulation runs — actors make decisions, relationships form, events cascade,
world state evolves — in pure Rust, with no renderer, no window, no GPU.
Run the full simulation headless from the command line and the world still
exists, changes, and produces events.

Bevy, egui, audio, text export — these are ways of **observing** what the
simulation produced. They are output adapters. They are optional. They are
the last layer, not the first.

**If you are reaching for Bevy to make something work, stop.
Put the logic in `mythos` first. Wire Bevy to it second.**

---

## The Three Layers

```
┌──────────────────────────────────────────────────────────────┐
│  LAYER 1 — NARRATIVE ENGINE                                  │
│  crates: myth-wire, myth-qgcp, myth-vault                    │
│                                                              │
│  Pure Rust. Zero renderer deps. Runs headless.               │
│  Owns: wire protocol, genesis container types, vault I/O.    │
│  myth-qgcp: WorldGenesis, MediaGenesis, ActorGenesis,        │
│             UIGenesis, MythosModule, Container, Capsule.      │
│  myth-vault: seal, lineage, BLAKE3 content addressing.       │
│                                                              │
│  Compile check: cargo build -p myth-qgcp                     │
│  must succeed with ZERO renderer crates in the dep tree.     │
└──────────────────────────────────────────────────────────────┘
           │  emits WirePackets, exposes WorldState
           ▼
┌──────────────────────────────────────────────────────────────┐
│  LAYER 2 — PLUGIN SYSTEM + MODULES                           │
│  crates: myth-plugin (+ per-module crates)                   │
│                                                              │
│  myth-plugin: PluginRegistry, MythPlugin trait, MythAddon,   │
│               LayoutRequest / LayoutGrant negotiation.       │
│  Module crates: standalone library, one per Quantum module.  │
│  Depends on: myth-wire + myth-qgcp + domain libs only.       │
│  Communicates with other modules via WirePackets ONLY.       │
│  Never imports another module crate directly.                │
└──────────────────────────────────────────────────────────────┘
           │  plugins emit/consume WirePackets
           ▼
┌──────────────────────────────────────────────────────────────┐
│  LAYER 3 — OUTPUT ADAPTERS                                   │
│  crate: biospark-theater (with Cargo feature gates)          │
│  bins:  engine, instruments, theater-bin                     │
│  examples: plugin-foundry                                    │
│                                                              │
│  Adapters read world state and module output.                │
│  They have NO write access to the narrative engine.          │
│  They are entirely optional — the world runs without them.   │
│  Renderer deps live HERE only (bevy, egui, cpal).            │
└──────────────────────────────────────────────────────────────┘
```

---

## Library Crates First

Everything that could be useful to more than one binary is a **library crate**.
Binaries are thin wrappers — 30–100 lines of wiring.

```
myth-wire          types only (WireType, WirePacket, BDna, MythId)
                   deps: serde, bincode, blake3, uuid — nothing else

myth-qgcp          Genesis Container types (WorldGenesis, MediaGenesis,
                   ActorGenesis, UIGenesis, MythosModule, Container, Capsule)
                   deps: myth-wire, serde, serde_json, bincode, blake3, hex, chrono, uuid
                   NO renderer. NO tokio. NO Bevy.

myth-vault         Sealed vault storage — persists and retrieves Genesis Containers
                   deps: myth-wire, myth-qgcp, serde, blake3
                   NO renderer.

myth-plugin        Plugin + Addon trait surface, PluginRegistry, layout negotiation
                   deps: myth-wire, myth-vault, thiserror, tracing, uuid
                   NO renderer. NO Bevy. NO tokio.

biospark-theater   compositor + router (library)
                   deps: myth-wire, tokio
                   renderer deps behind Cargo feature flags only
```

**Cargo features for optional adapters:**

```toml
# biospark-theater/Cargo.toml
[features]
default = []
bevy  = ["dep:bevy"]
egui  = ["dep:egui", "dep:eframe"]
audio = ["dep:cpal"]
```

A project that only needs prose output:
```toml
biospark-theater = { version = "0.1", features = ["egui"] }
```
No Bevy is compiled. No audio. Only what is needed.

**Publishing progression:**

```
Now    — workspace path deps   { path = "../myth-wire" }
Later  — git deps              { git = "...", tag = "v0.1" }
Later  — crates.io             "myth-wire" = "0.1"
```

Stay in path deps until the API is stable. Publishing is a decision, not a refactor.

---

## Three Separate Processes

Engine, Instruments, and Theater-bin are separate OS processes.
They communicate over TCP. Always. Even on localhost.

```
engine         binds  127.0.0.1:7700   headless, no window
instruments    client egui rack UI     sends CTL, receives state
theater-bin    client compositor       receives packets, renders
```

**Why TCP even on localhost:**
The discipline of using the network boundary guarantees that the separation
is real, not accidental. Moving to a home network is a config line.
Moving to the cloud is a config line + TLS wrapper. Nothing else changes.

**Never use shared memory between these three processes.**
`Arc<Mutex<>>` across binaries defeats the purpose entirely.

---

## Workspace Layout (Actual — as of 2026-06)

```
myth-os/
├── Cargo.toml              workspace root
│
├── crates/                 LIBRARY CRATES — importable, publishable
│   ├── myth-wire/          wire types, BDna, MythId (zero renderer deps)
│   ├── myth-qgcp/          Genesis Container types (.worldgenesis, .mediagenesis,
│   │                       .actorgenesis, .uigenesis)
│   ├── myth-vault/         sealed vault storage
│   ├── myth-plugin/        Plugin + Addon traits, PluginRegistry,
│   │                       layout negotiation (SlotType, LayoutRequest, LayoutGrant)
│   └── theater/            compositor lib (biospark-theater) — planned
│
├── modules/                MODULE CRATES — one per Quantum module (planned)
│   └── …                   GEN-01 through GEN-16 (not yet created)
│
├── adapters/               OUTPUT ADAPTER CRATES (planned)
│   ├── theater-bevy/       Bevy 3D handler
│   ├── theater-egui/       egui handler
│   └── theater-audio/      CPAL audio handler
│
├── bins/                   BINARY CRATES — thin wrappers (planned)
│   ├── engine/             headless TCP server
│   ├── instruments/        egui rack UI
│   └── theater-bin/        compositor binary
│
├── examples/               EXAMPLE APPLICATIONS (runnable now)
│   └── plugin-foundry/     Plugin Foundry — creates other plugins
│                           `cargo run -p plugin-foundry`
│
├── library/                Library UI (vault management — separate app, planned)
│
└── docs/
    ├── skills/             skill documents (source of truth)
    │   ├── myth-os-architecture.md
    │   ├── myth-qgcp.md
    │   ├── myth-plugin.md
    │   ├── biospark-theater.md
    │   └── …
    └── BSQM-MODULES-GENESIS-V1.0.json   BioSpark Quantum Modules genesis
```

### What Compiles Right Now

```
cargo build -p myth-wire       ✓ compiles, zero renderer deps
cargo build -p myth-qgcp       ✓ compiles, zero renderer deps
cargo build -p myth-vault      ✓ compiles, zero renderer deps
cargo build -p myth-plugin     ✓ compiles, zero renderer deps
cargo run   -p plugin-foundry  ✓ launches egui Plugin Foundry app
```

---

## Crate Ownership Rules

### `myth-wire`
```
✓ WireType enum (17 variants — never add more without a decision)
✓ WirePacket, WirePayload (must be Serialize + Deserialize always)
✓ BDna (Vec<bool>, exactly 64 elements — invariant, never relax)
✓ MythId (content-addressed BLAKE3 ID)
✗ NO logic, NO simulation, NO rendering
✗ NO async runtime
✗ deps beyond serde + bincode + blake3 + uuid are PROHIBITED
```

### `myth-qgcp`
```
✓ WorldGenesis, MediaGenesis, ActorGenesis, UIGenesis structs
✓ MythosModule, Container, Capsule, SealBlock
✓ Law of 16: MAX_MYTHOS=16, MAX_CONTAINERS=16, MAX_CAPSULES=16
✓ LayoutRegion (16 variants) and SlotDefinition for UIGenesis
✓ seal(), verify_seal(), lineage_hash (BLAKE3)
✗ NO renderer, NO tokio, NO Bevy, NO egui
✗ NO layout negotiation (that is myth-plugin's job)
```

### `myth-vault`
```
✓ Persists and retrieves sealed Genesis Containers
✓ VaultRegistry — the handle plugins receive in on_attach()
✓ Content-addressed storage via BLAKE3
✗ NO renderer, NO tokio
```

### `myth-plugin`
```
✓ MythPlugin trait (id, name, version, wire_in, wire_out, process, tick,
                    heraldry_symbol, layout_request)
✓ MythAddon trait (id, target_plugin ["*" = wildcard], on_output, on_tick_output)
✓ PluginRegistry (route, tick, negotiate_layout, two-phase borrow pattern)
✓ SlotType, Visibility, LayoutRequest, SlotRequest, LayoutGrant, DeniedSlot
✗ NO renderer, NO egui, NO Bevy, NO tokio
✗ Plugins needing egui take it as THEIR OWN dep — not myth-plugin's
```

### Module crates (e.g., `terrain`, `loom`) — planned
```
✓ Implements MythPlugin from myth-plugin
✓ Defines its WireType inputs and outputs
✓ Can run standalone: cargo run -p terrain
✓ deps: myth-wire + myth-plugin + domain-specific libs
✗ Does NOT depend on other module crates
✗ Does NOT depend on Bevy unless inherently spatial/visual
✗ Does NOT share mutable state with other modules
```

### `biospark-theater` — planned
```
✓ Channel, ChannelState, compositor logic
✓ TheaterModule + OutputHandler traits
✓ EngineTransport trait + TcpTransport impl
✓ Renderer deps behind Cargo features ONLY
✗ Does NOT run the simulation
✗ Does NOT own world state
✗ Does NOT inspect WirePacket payloads for routing
   (route by WireType only — content is the handler's concern)
```

### Binaries (`engine`, `instruments`, `theater-bin`) — planned
```
✓ 30–100 lines of wiring
✓ Imports library crates, connects them, starts the process
✗ Contains NO business logic
✗ Contains NO simulation code
✗ Contains NO rendering code
```

---

## Module Independence Law

Every module MUST be able to run with none of the others loaded.

Test: `cargo run -p terrain` must do something meaningful
with no other modules active.

If it can't — the module is not independent. Fix it before continuing.

Modules communicate ONLY via WirePackets through the Theater.
They never import each other's types or call each other's functions.

---

## What We Carry Forward From Previous Work

These are correct and move into the new structure:

| Artifact | New Location | Note |
|----------|-------------|------|
| WireType (17 variants) | crates/myth-wire | correct |
| BDna (Vec<bool>, 64) | crates/myth-wire | correct |
| QuantumModule manifest types | crates/mythos | correct |
| 16 module JSON manifests | assets/modules/ | correct data model |
| scanner.rs ModuleRegistry | bins/engine (thin) | pure Rust, keep |
| fBm/hash noise math | modules/terrain | extract from topology.rs |
| Library crate (vault UI) | library/ | already separate |
| Theme tokens | library/src/ui/ | correct layer |
| Camera orbit/fly math | adapters/theater-bevy | correct layer |

These are discarded — the logic may be right but the coupling is wrong:

| Artifact | Problem |
|----------|---------|
| genesis/src/atoms/*.rs | Bevy Plugin wrappers — logic moves to mythos/modules |
| genesis/src/main.rs | Bevy as host — becomes bins/engine (~60 lines) |
| genesis/src/containers.rs | Flat loader — replaced by crates/qgcp |
| genesis/src/mixer.rs | MIDI logic fine, Bevy coupling wrong |

---

## Anti-Patterns — Do Not Repeat

### The Bevy Host Trap
Simulation logic in `impl Plugin for X { fn build() }`.
Bevy systems are renderers. Not the simulation.
Fix: Logic in `mythos` or module crates. Bevy reads state. Bevy never writes simulation.

### The Monolith
One binary loads everything. User wants to write a novel and boots a 3D physics engine.
Fix: Each module is standalone. Each binary composes only what it needs via Cargo features.

### Module Coupling
Module A imports Module B's types and calls its functions.
Fix: WirePackets only. Modules are strangers to each other.

### Shared Memory Across Processes
`Arc<Mutex<>>` or channels spanning what should be separate binaries.
Fix: TCP. Always. Even localhost. The boundary is the point.

### Fat Binary
All logic in `main.rs`. Library crate is empty scaffolding.
Fix: Library crate holds everything. Binary is wiring only.

### Premature Publishing
Pushing to crates.io before the API is stable, then having to make breaking changes.
Fix: Path deps during active development. Publish when the API is settled.

### Renderer Dep Creep
A "small" renderer import sneaking into `mythos` or a module crate.
Fix: Run `cargo tree -p mythos | grep bevy` after every change.
If bevy appears — revert immediately.

---

## Pre-Flight Checklist

Before writing any new code in myth-os:

- [ ] Which layer? (myth-qgcp / myth-plugin / adapter / binary / example)
- [ ] Is this a library crate or a binary? (default to library)
- [ ] Does `cargo build -p myth-qgcp` still show zero renderer deps?
- [ ] Does `cargo build -p myth-plugin` still show zero renderer deps?
- [ ] Can this module run standalone? (`cargo run -p <module>`)
- [ ] Are modules communicating via WirePackets only?
- [ ] Did a renderer dep sneak into a library crate?
- [ ] Is the binary under 100 lines? (if not, move logic to library)
- [ ] Is every new type consistent with the myth-qgcp skill?
- [ ] Does every Capsule have a lineage_hash?
- [ ] Does every new plugin implement `layout_request()` (even if it returns default)?
- [ ] If adding a field to WorldGenesis/ActorGenesis — is it in the myth-qgcp gap list first?
