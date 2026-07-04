# MYTH-OS — MASTER BLUEPRINT
## Quantum Quill Narrative Operating System

**Version:** 0.3.0
**Last Updated:** 2026-06-13
**Status:** Active Development — Foundation Phase
**Workspace Root:** `J:\myth-os\`

> **For Claude:** Read this at the start of every session. Update status markers
> after completing any task. If something here conflicts with the actual code,
> trust the code and update this file.

---

## STATUS KEY

| Symbol | Meaning |
|--------|---------|
| ✅ | Built & tested |
| 🔄 | In progress / partial |
| 🏗️ | Stubbed / skeleton only |
| 📋 | Planned — design exists |
| 💡 | Idea — needs design |
| ❌ | Blocked or abandoned |
| 🔒 | Depends on something not built yet |

---

## PART I — CRATE STATUS MAP

### Foundation Layer (no renderer deps)

| Crate | Canonical Name | Status | Notes |
|-------|---------------|--------|-------|
| `crates/myth-wire` | **Myth Wire** | ✅ | WirePacket, MythId, WireType (TMP/EVT/CTL/DAT/BHV/SPA/NAR/AUD) |
| `crates/myth-core` | **Myth Core** | 🏗️ | Skeleton only — needs clock loop, bus, OctaveEnforcer |
| `crates/myth-vault` | **Quantum Vault** | 🏗️ | Skeleton only — content-addressable storage |
| `crates/myth-quill` | **Quantum Quill** | 🏗️ | Skeleton only — capsules, containers, heraldry |
| `crates/myth-daw` | **Myth DAW** | ✅ | Transport (0-indexed, tested), Session, Arrangement, Mixer, Clips, Tracks, Wire packets |
| `crates/myth-stencil` | **Myth Stencil** | ✅ | Recursive BSP panel layout, MIDI CC binding, lerp animation, .stencil file format |

### Renderer Layer

| Crate | Canonical Name | Status | Notes |
|-------|---------------|--------|-------|
| `crates/myth-forge` | **Myth Forge** | 🔄 | egui rack UI — uses myth-stencil for layout, design system needs update |
| `crates/biospark-theatre` | **BioSpark Theatre** | 🏗️ | Typed signal router — 10-step build sequence documented, not started |
| `crates/biospark-theatre-bevy` | **BioSpark Theatre (Bevy)** | 🏗️ | Bevy adapter for Theatre — depends on Theatre |

### Executables

| Binary | Status | Notes |
|--------|--------|-------|
| `bins/opalis` | 🔄 | Vault card UI + egui-snarl node graph — ported from H drive, egui 0.33 isolated, needs wiring |

### Legacy Root Crates (pre-consolidation, do not extend)

`mythos` `vault` `core` `genesis` `library` `asset-forge` `genesis-collider-mandelbulb` `void-sculptor` `viewer`

---

## PART II — OS SERVICES (PLANNED)

These are the "one thing only" substrate crates everything else sits on top of.
Think microservices, but as Rust crates with Myth Wire packets as the IPC.

| Crate | Purpose | Status | Depends On |
|-------|---------|--------|------------|
| `services/myth-clock` | Master tick source — all crates slave to this | 📋 | myth-wire |
| `services/myth-bus` | In-process message router (crossbeam channels) | 📋 | myth-wire |
| `services/myth-cfg` | Persistent settings / preferences (TOML) | 📋 | — |
| `services/myth-fs` | Virtual filesystem / asset path resolver | 📋 | myth-vault |
| `services/myth-log` | Structured event log (tracing + sqlite) | 📋 | — |
| `services/myth-ipc` | gRPC bridge if anything goes out-of-process | 📋 | tonic/prost |
| `services/myth-auth` | Identity / permissions (local, no cloud) | 💡 | — |
| `services/myth-net` | Peer sync / collaboration transport | 💡 | — |

**Build order:** `myth-clock` → `myth-bus` → `myth-cfg` → `myth-fs` → rest

---

## PART III — THE 16 MODULES (Quantum Quill Law of 16)

Managed by OctaveEnforcer in myth-core. Each module is a crate in `modules/`.
All communicate via myth-wire packets.

| # | Module | Wire Type | Status | Notes |
|---|--------|-----------|--------|-------|
| 01 | Character / Persona | BHV | 📋 | DNA traits, arc tracking |
| 02 | Emotion | BHV | 📋 | Mood state, bloom effects |
| 03 | Environment / Spatial | SPA | 📋 | Location, atmosphere |
| 04 | Event | EVT | 📋 | Trigger system |
| 05 | Narrative | NAR | 📋 | Plot structure, act breaks |
| 06 | Audio | AUD | 📋 | Capsule→sound mapping |
| 07 | Effect / Modulation | CTL | 📋 | Tension, mystery, pacing knobs |
| 08 | Logic | CTL | 📋 | Conditionals, rule engine |
| 09 | Memory Matrix | DAT | 📋 | Associative event memory |
| 10 | Voxel World | SPA | 📋 | 3D coordinate grid |
| 11 | Faction / Social | BHV | 📋 | Relationship networks |
| 12 | Digital DNA | DAT | 📋 | Genetic trait sequencer |
| 13 | Prophecy / Foreshadow | NAR | 📋 | Pattern recognition, hints |
| 14 | Procedural Gen | DAT | 📋 | Names, locations, events |
| 15 | Export / Publish | DAT | 📋 | Manuscript, screenplay, game formats |
| 16 | Analytics | DAT | 📋 | Story structure analysis |

---

## PART IV — VAULT SYSTEM

The Vault is the project container — everything lives inside one.
The Master Vault acts like Godot's asset store: validation, media, game server.

### Vault Types

| Type | Purpose | Status |
|------|---------|--------|
| **Novel Vault** | Long-form prose fiction | 📋 |
| **Screenplay Vault** | Film/TV visual storytelling | 📋 |
| **Library Vault** | Master asset repository (hub) | 📋 |
| **Playground Vault** | Risk-free experimentation | 📋 |
| **Collaboration Vault** | Multi-user team projects | 💡 |
| **Master Vault** | Asset store + validation + media server | 📋 |

### Portal System

Portals are live connections between vaults for resource sharing.

| Feature | Status | Notes |
|---------|--------|-------|
| Read-only portal | 📋 | Pull assets from another vault |
| Read-write portal | 📋 | Bi-directional sync |
| Portal conflict resolution | 📋 | Auto-merge / manual |
| Portal health monitor | 📋 | Status display |
| Portal analytics | 💡 | Usage tracking |

### Vault UI (opalis)

| Screen | Status | Notes |
|--------|--------|-------|
| Splash / loading | ✅ | Ported from H drive |
| Vault card browser | ✅ | Click to enter vault |
| Inside vault view | ✅ | Plugin load/unload |
| Plugin system | 🔄 | Skeleton — needs myth-wire wiring |
| Node graph (egui-snarl) | ✅ | Working in opalis |

---

## PART V — THEATER SYSTEM (BioSpark Theatre)

The Theatre is a decoupled composite renderer. Bevy is an adapter, not the host.
Channels = After Effects layers (not TV channels).

### 10-Step Build Sequence

| Step | Component | Status |
|------|-----------|--------|
| 1 | `ChannelId` newtype + `ChannelKind` enum | 🏗️ |
| 2 | `Layer` struct (transform, visibility, z-order) | 🏗️ |
| 3 | `Compositor` — owns Vec<Layer>, no renderer | 🏗️ |
| 4 | `Signal` router — myth-wire packets in | 🏗️ |
| 5 | `RenderTarget` trait (renderer-agnostic) | 🏗️ |
| 6 | Bevy adapter implements `RenderTarget` | 🏗️ |
| 7 | egui adapter implements `RenderTarget` | 🏗️ |
| 8 | myth-daw Transport → Theatre tick | 🏗️ |
| 9 | Layer automation via myth-daw AutomationLane | 🏗️ |
| 10 | myth-wire packet bus connects all modules | 🏗️ |

---

## PART VI — MYTH-DAW DETAIL

The DAW is the primary testbed for Theater and the wire bus.

### Transport

| Feature | Status | Notes |
|---------|--------|-------|
| Play / Pause / Stop / Record | ✅ | Tested |
| BPM, position, beats_per_bar | ✅ | |
| `bar_beat()` — **0-indexed** | ✅ | Matches audio/video buffer index 0 |
| `position_display()` | ✅ | Format: `000.0` |
| Loop region wrap | ✅ | Tested |
| Seek + negative clamp | ✅ | Tested |
| Wire to myth-core clock | 🔒 | Needs myth-clock service |

### Session (Clip Launcher)

| Feature | Status |
|---------|--------|
| Track + Scene grid | ✅ |
| `trigger_scene()` → Queued | ✅ |
| `commit_queued()` → Playing | ✅ |
| `stop_all()` | ✅ |

### Arrangement (Timeline)

| Feature | Status |
|---------|--------|
| ArrangementTrack + clips | ✅ |
| `active_clips_at(beat)` | ✅ |
| AutomationLane breakpoints | ✅ |
| Linear interpolation | ✅ |

### Mixer

| Feature | Status |
|---------|--------|
| Fader / Mute / Solo / Arm | ✅ |
| `effective_level()` solo logic | ✅ |
| Sends | ✅ |
| Peak metering | 📋 |

### Wire Packets

| Packet | Wire Type | Status |
|--------|-----------|--------|
| TransportTick | TMP | ✅ |
| ClipEvent | EVT | ✅ |
| MixerLevel | CTL | ✅ |
| AutomationValue | DAT | ✅ |

### Demo

`cargo run --example session_demo -p myth-daw`
Outputs 120 BPM, 2-bar run with tension automation and wire packet log.

---

## PART VII — MYTH-STENCIL DETAIL

BSP recursive panel layout for any UI — MIDI-controllable in real time.

| Feature | Status | Notes |
|---------|--------|-------|
| `PanelNode` recursive split | ✅ | Horizontal / Vertical |
| `flex` + `flex_target` | ✅ | Lerp animation per tick |
| `apply_midi(ch, cc, val)` | ✅ | Walks tree, sets flex_target |
| `tick(dt, speed)` | ✅ | Lerps all nodes |
| `layout_normalized()` | ✅ | 0.0–1.0 coords, no egui dep |
| egui layout (feature flag) | ✅ | `egui-layout` feature |
| `.stencil` file save/load | ✅ | bincode |
| myth-forge integration | ✅ | Re-exports from scene.rs |

---

## PART VIII — BLUEPRINT SYSTEM (THIS FILE)

The Blueprint is the shared project brain. Rules:

1. **Claude reads this first** every session before touching code
2. **Update status** after completing any task — don't batch updates
3. **If code disagrees with this file, trust the code** and fix this file
4. **Add rescued ideas** to Part IX — nothing gets lost
5. **Add external project notes** to Part X before migrating

### Sync Check Phrases

| Phrase | Meaning |
|--------|---------|
| `sync check` | Quick verify — what do you think is built? |
| `reality check` | Deep verify — read the actual files first |
| `blueprint check` | Is BLUEPRINT.md up to date? |
| `where are we` | Summarize current state and next priority |

---

## PART IX — RESCUED IDEAS (nothing gets lost)

Ideas from old projects and brainstorms that haven't become crates yet.

### From old Google AI Studio sessions

- **Scrollforge Parser** — NLP that reads brainstorm text and extracts containers/capsules/traits into CSV for batch creation. Lives as a myth-quill plugin.
- **Digital DNA Sequencer** — Genetic trait system for characters. Dominant/recessive traits, inheritance, mutation. Feeds into Character module (Module 01).
- **Bloom Effects** — Capsules evolve over time. A Sorrow capsule can bloom into Love. Cross-bloom interactions. Lives in Emotion module (Module 02).
- **Memory Matrix** — Organic event memory with spatial-temporal anchoring, significance scoring, temporal decay. Module 09.
- **Orbital Effects** — Characters can have circular movement patterns with radius/speed. Good for environmental simulation in voxel world.
- **Prophecy Injector** — Mystic Weaver agent plants foreshadowing, tracks fulfillment, generates red herrings. Module 13.
- **Invisible Agents** — Plot-influencing entities the user doesn't see directly. Fate mechanics, destiny threads.
- **16-Channel Narrative Mixer** — Audio-style mixer for narrative "channels" (tension, mystery, romance, pacing). These are the modulation parameters mapped to MIDI.

### Xyrona Prime Profession UI Themes (50 total — DO NOT LOSE THESE)

Each profession is a full UI personality — not just colors but layout metaphors,
visual language, and interaction style. These are the themes for agents, vaults,
and modules. The user has the full 50 in their notes.

**Mapping to myth-os modules and tools:**

| Profession | Maps To |
|-----------|---------|
| Neural Narrative Designer | Quantum Quill (modules 01–05) |
| Harmonic Resonance Engineer | myth-daw, 16-channel mixer |
| Dream Sequencer | Session clip launcher / arrangement |
| Eldritch Archivist | Quantum Vault / Library Vault |
| Chronomancer | Transport / timeline scrubber |
| Terraformer | Voxel World (Module 10) |
| Synesthetic Architect | BioSpark Theatre |
| Echo Psychologist | Emotion module (Module 02) |
| Fractal Artisan | myth-stencil / mandelbulb / procedural gen |
| Quantum Architect | myth-core / OctaveEnforcer |
| Neural Data Analyst | Analytics (Module 16) / memory matrix |
| Eldritch / Mystic | Prophecy & Foreshadow (Module 13) |
| Void Hunter | Debug / system monitor |
| Arcane Weather Weaver | Environment / atmosphere (Module 03) |

**Design principle:** Each vault type ships with a matching profession theme.
Novel Vault → Neural Narrative Designer. Analytics view → Neural Data Analyst HUD.
The theme tells you *what mode you're in*, not just what color the buttons are.

**Implementation plan (future):**
- `myth-theme` crate — `ThemeProfile` struct with colors, fonts, layout metaphors, particle effects
- Themes stored in myth-vault as assets
- User can assign any theme to any vault
- Agent windows get their profession's theme automatically
- 50 base themes from Xyrona Prime, user-extensible

### From stencil brainstorm

- **Stencil as live performance tool** — S4 jog wheels control panel sizes in real time. Already has MIDI CC binding. Just needs the MIDI loop from myth-controller.
- **Named slot system** — Panel leaves get `slot_id` so Theater can route capsule visuals to specific screen regions by name.

### From opalis vault system

- **Vault Analytics dashboard** — Content statistics, agent activity, productivity patterns, health check. Already designed in the old doc.
- **Vault Cloning / Branching** — Clone a vault for major revisions. Branch like git for alternative endings.
- **Vault Templates** — Start new projects from pre-configured structures (Novel, Screenplay, Playground, etc.)
- **Conditional Portals** — Portal only activates on weekdays, or only when editing character containers, etc.

### Hardware ideas

- **Traktor S4 jog wheels → scrub timeline** — Jog wheel MIDI → Transport seek
- **S4 crossfader → scene transition** — Crossfade between two active scenes
- **S4 sample pads → clip launcher** — 4×4 pads trigger Session clips directly
- **DMX → Theater lighting layer** — Emotion module drives DMX values via RS-485

---

## PART X — EXTERNAL PROJECT MIGRATION LOG

Projects on `H:\000-CLAUDE-CODE\rust_projects` — status of each.

| Project | Status | Action |
|---------|--------|--------|
| `stencil-crate` | ✅ Migrated | Became `crates/myth-stencil` |
| `opalis-prism-software-sequencer` | ✅ Migrated | Lives at `bins/opalis` (egui 0.33 isolated) |
| `atomic-genesis` (atomic-mind/soul/body) | 📋 Pending | Review before migrating |
| `crest-forge` | 📋 Pending | Review before migrating |
| `prime-ds` | 📋 Pending | Design system? Check against Procedural_Rack_Design_System |
| `nucleus-engine` | 📋 Pending | Review before migrating |
| `xyrona` (theater skeleton) | 📋 Pending | Diff against biospark-theatre before deciding |
| `cartographer` | ❌ Empty | Skeleton only, nothing to migrate |
| `condition-engine` | ❌ Empty | Skeleton only |
| `qgcp` | ❌ Empty | Skeleton only |
| `quill-translator` | ❌ Empty | Skeleton only |

**React projects** (Loom, Atlas) — finished and look good, will need egui ports eventually. Do not rush — port only when we have the UI substrate ready.

---

## PART XI — KNOWN BUGS & ISSUES

| # | Location | Description | Status |
|---|----------|-------------|--------|
| 1 | `bins/opalis` | Not wired to myth-wire — standalone only | 📋 Fix when myth-bus ready |
| 2 | `crates/myth-forge` | Design system not fully applied (see memory) | 📋 |
| 3 | `examples/session_demo.rs` | Unused variable warnings (demo scaffolding) | 💡 Low priority |
| 4 | `bins/opalis` | Genesis procedure fires automatically on new vault — should be opt-in, user may already have a world to load | 📋 |
| 5 | `bins/opalis` | Vault cards are too wide — height is correct, width needs to shrink (more square) | 📋 |
| 6 | `bins/opalis` | Two instrument windows AND two mandelbulb windows open on launch — duplicates | 📋 |
| 7 | `bins/opalis` | Mandelbulb windows open empty / blank | 📋 |
| 8 | `bins/opalis` | All instrument views are the same and you have to scroll through all — want one active at a time with row of buttons (top or bottom) to switch | 📋 |
| 9 | `bins/opalis` | No visible UI for loading/adding plugins from inside a vault | 📋 |

---

## PART XII — WHAT TO BUILD NEXT

Ordered by dependency — can't build layer N without layer N-1.

### Tier 1 — Unblock everything else
- [ ] `services/myth-clock` — master tick, replaces Transport::tick() as the heartbeat
- [ ] `services/myth-bus` — crossbeam channel router so crates can talk

### Tier 2 — First real integration
- [ ] Wire myth-daw Transport into myth-clock
- [ ] Wire myth-daw WirePackets through myth-bus
- [ ] BioSpark Theatre Steps 1–4 (ChannelId, Layer, Compositor, Signal router)

### Tier 3 — Vault foundation
- [ ] myth-vault: content-addressable store with MythId keys
- [ ] Wire opalis vault UI to myth-vault for real load/save
- [ ] myth-cfg: TOML settings that persist between runs

### Tier 4 — First module
- [ ] Module 02 Emotion — simplest stateful module, good test of the bus
- [ ] Module 01 Character — depends on Emotion for bloom effects

### Tier 5 — Run it
- [ ] myth-daw session_demo wired through Theatre → renders to egui panel
- [ ] MIDI loop from myth-controller → myth-stencil flex_target
- [ ] S4 pads → Session clip launcher

---

## PART XIII — INTERSTELLAR TOUR (First Show Spec)

The Theater's first production. A live MMORPG concert event in the metaverse.
Target: 100–500 attendees. City-block scale space. Planetarium dome venue.

### Concept

The Order of the Quantum Quill discovers artists across the multiverse.
Some of those artists are getting popular. Now they tour.
Attendees log in like an MMORPG, represent their world, and experience the show together.

### Pre-Show Flow

| Stage | Description | Tech |
|-------|-------------|------|
| **Login** | MMORPG-style login screen | myth-vault identity |
| **Avatar Select** | Choose from any represented world's species (e.g. Hydralis for Xyrona Prime, Venturan, etc.) | vault character system |
| **The Space** | City-block sized wandering area — Order easter eggs hidden throughout, lore embedded in environment | voxel world / Module 10 |
| **The Door** | Venturan ticket agents working the entrance. Flirty. Will let you jump the line. | NPC agents / Sociomind |
| **Merch Booths** | Concert t-shirts and items — purchasable online, same object in physical world | `/biospark-splash-creator` integration |
| **Seat Assignment** | If you don't find a seat, one finds you | Theater seating system |

### The Show Begins

- House lights dim
- Audience locked in place — can look anywhere (full dome) but cannot move
- Enforced stillness after the wandering chaos — directorial choice, makes first note hit harder
- Total darkness
- One star appears. Then another.
- Aurae Nurei's voice enters before any visuals: *"The moon and the stars and you, take my breath awayyyyyyyyyyy"*
- Then the show begins

### Why Planetarium Dome

- **360° visual field** — procedural shader art / mandelbulb fills the entire sky
- **No stage direction** — the audience IS surrounded by the performance, not watching it from one angle
- **Star field as opening** — darkness + stars is the natural state of a dome, the show grows out of the space itself
- **Scale feels cosmic** — even a city-block sized dome feels infinite when the ceiling is the universe
- **Algorithmic art perfect fit** — generative visuals (void-sculptor, genesis-collider-mandelbulb) were made for curved projection
- **Intimacy at small scale** — 100-500 people in a dome is a ritual, not a stadium show

### Artists & Worlds (Order Discoveries)

- **Aurae Nurei** — from Xyrona Prime (confirmed headliner, first show)
- 15–20 total artists discovered so far
- First show: handful of artists, intimate
- Open to human Suno creators joining with their own artist/world
- Visual artists for procedural shader art welcome — each artist gets their own visual world

### Tech Stack for the Show

| Component | Handles |
|-----------|---------|
| BioSpark Theatre | The venue / compositor — this IS the planetarium |
| myth-daw Transport | Show runs on the clock — lighting, transitions, cues |
| myth-daw 16-channel mixer | Live mix of the show (S4 as DJ booth) |
| Traktor S4 | Live performance control — crossfader, pads, jogs |
| void-sculptor / mandelbulb | Dome visuals — procedural shader art per artist |
| Module 03 Environment | Atmosphere, spatial audio zones |
| Module 02 Emotion | Crowd emotional state feeds back into visuals |
| Sociomind agents | Venturans, NPCs, Order members in the space |
| biospark-splash-creator | Merch design → physical purchase pipeline |
| myth-vault | Identity, avatar, world representation |

### Open Collaboration

- Suno creators can submit artists from their own worlds
- Shader/algorithmic visual artists can design a world's dome visuals
- Each represented world gets its own aesthetic zone in the pre-show space
- The Order's agents are the connective tissue between all worlds

### Notes

- The Venturans at the door are the first NPC interaction — sets the tone that this world is alive
- Merch bridge (digital → physical) makes the event real beyond the screen
- The forced stillness during the opening is intentional — it's directing, not a limitation
- First show small by design — the people there when Aurae Nurei's voice first appears in the dark will remember it

---

## APPENDIX A — RUN COMMANDS

```powershell
# Run the DAW demo (2-bar wire packet output)
cargo run --example session_demo -p myth-daw

# Run tests for myth-daw (9 transport tests)
cargo test -p myth-daw

# Run tests for all crates
cargo test --workspace

# Build everything (check for compile errors)
cargo build --workspace

# Run opalis vault UI
cargo run -p opalis
```

---

## APPENDIX B — KEY FILE LOCATIONS

| File | Purpose |
|------|---------|
| `J:\myth-os\Cargo.toml` | Workspace root — member list and shared deps |
| `crates/myth-wire/src/lib.rs` | WirePacket, WireType, MythId |
| `crates/myth-daw/src/transport.rs` | Transport + 9 unit tests |
| `crates/myth-stencil/src/node.rs` | PanelNode BSP tree |
| `crates/myth-stencil/src/midi.rs` | MidiBinding flex_from_cc |
| `crates/myth-forge/src/scene.rs` | Re-exports myth-stencil, panel_bg_color |
| `bins/opalis/src/main.rs` | Vault card app entry point |
| `C:\Users\phant\.claude\projects\J--myth-os\memory\` | Claude's persistent memory |

---

*This document is the source of truth for project state. Keep it honest.*
