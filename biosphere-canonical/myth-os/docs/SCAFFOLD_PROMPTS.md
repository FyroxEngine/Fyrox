# myth-os — Scaffold Node Prompts
> Every modular part of the workspace, organized by layer, with a prompt for how to build or complete it.
> Use these prompts verbatim (or near-verbatim) when starting a new Claude Code session on that component.
> Updated: 2026-06-21

---

## HOW TO USE THIS FILE

Each section below is one **node** — a discrete modular unit of myth-os.
Each node has:
- **What it is** — one-line description
- **Status** — LIVE / STUB / PARKED / PLANNED
- **Files** — where it lives
- **Scaffold Prompt** — paste this into a new session to build or finish it

Nodes are organized bottom-up: foundation first, renderer last, apps at the top.

---

---

# LAYER 0 — FOUNDATION PROTOCOL
## The wire types, identity, and packet envelope that everything else depends on.

---

### NODE: WireType
**What it is:** The 17 closed canonical signal types that are the only legal interface between all modules.
**Status:** ✅ LIVE — complete, tested, no work needed.
**Files:** `crates/myth-wire/src/wire_type.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-wire/src/wire_type.rs.
The WireType enum has 17 variants (DAT/CTL/AUD/NAR/TMP/AGT/VIS/SPA/BHV/SOC/ENR/IDN/EVT/AST/MET/LGC/RES).
This set is CLOSED — do not add variants. Extend DAT payload schemas instead.
Task: [describe what you need to add or fix here]
```

---

### NODE: WirePacket
**What it is:** The single legal message type between all systems. Every cross-crate communication is a WirePacket.
**Status:** ✅ LIVE
**Files:** `crates/myth-wire/src/packet.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-wire/src/packet.rs.
WirePacket { wire_type: WireType, source_id: MythId, tick: u64, payload: Vec<u8> }
with encode<T: Serialize> / decode<T: DeserializeOwned> helpers using bincode.
All cross-system communication is WirePacket only. No raw structs cross crate boundaries.
Task: [describe what you need — adding a packet field, adding encode variants, etc.]
```

---

### NODE: MythId / Blake3Hash / ChannelId
**What it is:** UUID-based entity identifier, BLAKE3 content fingerprint, Theater channel ID.
**Status:** ✅ LIVE
**Files:** `crates/myth-wire/src/ids.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-wire/src/ids.rs.
MythId wraps uuid::Uuid. Blake3Hash wraps 32 bytes (blake3 output). ChannelId wraps u32.
Dep rule: myth-wire depends only on serde, bincode, uuid, blake3, thiserror. NO renderer deps here.
Task: [describe new identity type needed, e.g. a VaultId newtype]
```

---

### NODE: BDna
**What it is:** 64-bit deterministic identity vector (Vec<bool>) — the DNA fingerprint of every entity.
**Status:** ✅ LIVE
**Files:** `crates/myth-wire/src/bdna.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-wire/src/bdna.rs.
BDna is a Vec<bool> of length 64 (BDNA_LENGTH const). from_seed(bytes) uses blake3 to produce
deterministic 64-bool vectors. Same seed always produces same BDna. Used in ActorGenesis.bdna_signature
and WorldGenesis identity. Deception mechanics can temporarily alter BDna at runtime.
Task: [e.g. add BDna::xor_blend for mixing two BDna identities]
```

---

### NODE: Sigil
**What it is:** Routing identity for CELL capabilities — parallel to Glyph but for actors.
**Status:** ✅ LIVE — added 2026-06-21
**Files:** `crates/myth-wire/src/sigil.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-wire/src/sigil.rs.
Sigil { symbol: String, tier: u8, cooldown_ms: u64, energy_cost: f32 }
Used by CELLs inside ActorGenesis. Tier 0 = passive sensor, 1–16 = action tiers.
Parallel to Glyph (which CAPSULEs use), but Sigils belong to CELLs.
Task: [e.g. add Sigil::can_activate(current_energy: f32) helper]
```

---

---

# LAYER 1 — GENESIS CONTAINERS (QGCP)
## The canonical data hierarchy: WorldGenesis → 16 Mythos → 16 Containers → 16 Capsules.

---

### NODE: WorldGenesis
**What it is:** The headless world-state container. The root of all simulation state.
**Status:** ✅ LIVE
**Files:** `crates/myth-qgcp/src/genesis.rs`, `crates/myth-qgcp/src/lib.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-qgcp/src/genesis.rs and lib.rs.
WorldGenesis is the root container. Hierarchy: 1 WorldGenesis → 16 MythosModules →
16 Containers → 16 Capsules = 4,096 total capsule slots. Constants: MAX_MYTHOS=16,
MAX_CONTAINERS=16, MAX_CAPSULES=16, TOTAL_CAPACITY=4096.
File extension: .worldgenesis. Short alias: WorldGen.
Dep rule: myth-qgcp depends on myth-wire, serde, blake3, chrono, uuid only. NO renderer.
Task: [e.g. add WorldGenesis::merge for combining two world states]
```

---

### NODE: MythosModule
**What it is:** One of 16 named slots in a WorldGenesis or ActorGenesis. Contains Containers.
**Status:** ✅ LIVE
**Files:** `crates/myth-qgcp/src/mythos.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-qgcp/src/mythos.rs.
MythosModule holds up to MAX_CONTAINERS (16) Container items. It is the second tier
of the 16×16×16 hierarchy. Each of the 16 BSQM instrument modules maps to one
MythosModule slot in the WorldGenesis.
Task: [e.g. add a MythosModule::find_container_by_symbol helper]
```

---

### NODE: Capsule
**What it is:** A data payload flowing through the ATOM graph. Uses Glyphs for routing identity.
**Status:** ✅ LIVE
**Files:** `crates/myth-qgcp/src/capsule.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-qgcp/src/capsule.rs.
Capsule { id, name, wire_type: WireType, payload: serde_json::Value, lineage_hash,
author_dna, tags, created_at }. Uses Glyphs for routing identity (not Sigils — those
belong to CELLs). CAPSULEs are the data packets flowing through the ATOM graph.
Think of them like audio samples: discrete, typed, content-addressed.
Task: [e.g. add Capsule::fork that duplicates with a new lineage_hash]
```

---

### NODE: Cell
**What it is:** The atomic capability unit of an Actor's Genesis Container. Mix of runtime ATOM + CAPSULE. Uses Sigils. Enables actors to DO things.
**Status:** ✅ LIVE — added 2026-06-21
**Files:** `crates/myth-qgcp/src/cell.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-qgcp/src/cell.rs and crates/myth-wire/src/sigil.rs.
Cell { cell_id, name, sigil: Sigil, wire_type: WireType, payload: serde_json::Value,
lineage_hash, action_atom: Option<String>, tags, created_at }.
CELLs are the leaf units of ActorContainers. They enable actors to DO things (cast spells,
pick locks, erupt volcanoes). action_atom references a runtime ATOM node ID. None = data-only cell.
Task: [e.g. build a Cell::activate that checks sigil.cooldown_ms and energy_cost]
```

---

### NODE: ActorContainer
**What it is:** A named group of up to 16 Cells inside an ActorGenesis.
**Status:** ✅ LIVE — added 2026-06-21
**Files:** `crates/myth-qgcp/src/actor_container.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-qgcp/src/actor_container.rs.
ActorContainer { id, name, wire_out: WireType, description: Option<String>, cells: Vec<Cell> }
MAX_CELLS = 16. Up to 16 ActorContainers per ActorGenesis (mirroring the 16-container law).
ActorContainers are the capability groups of an actor — "Combat Actions", "Memory Shards", etc.
Task: [e.g. add ActorContainer::active_cells that returns cells whose sigil.tier > 0]
```

---

### NODE: ActorGenesis
**What it is:** An agent or character Genesis Container. Any active/reactive simulation entity.
**Status:** ✅ LIVE — updated 2026-06-21 to add actor_containers
**Files:** `crates/myth-qgcp/src/actor.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-qgcp/src/actor.rs.
ActorGenesis has: genesis_id, home_world_id, actor_name, archetype_id, bdna_signature: BDna,
heraldry_symbol, intelligence_tier: u8 (0–16), mythos: Vec<MythosModule> (legacy/compatibility),
actor_containers: Vec<ActorContainer> (NEW — up to 16, each holds up to 16 Cells),
lifecycle: String ("draft"|"active"|"sealed"), sealed: bool, seal: Option<SealBlock>.
ACTORS are any active/reactive entity: NPCs, volcanoes, alarm systems. Not just agents.
Task: [e.g. implement ActorGenesis::can_perform(sigil_symbol) checking cooldowns]
```

---

### NODE: MediaGenesis
**What it is:** Binary asset bundle for the BioSpark Theater. Client-side only.
**Status:** ✅ LIVE
**Files:** `crates/myth-qgcp/src/media.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-qgcp/src/media.rs.
MediaGenesis holds AssetRef items (GLB, PNG, WAV, etc.) keyed by MythId.
File extension: .mediagenesis. Short alias: MediaGen.
AssetRef { id: MythId, media_type: MediaType, vault_path: String, size_bytes: u64 }.
Task: [e.g. add MediaGenesis::total_size_mb helper]
```

---

### NODE: UIGenesis
**What it is:** UI layout slot topology container — describes the instrument panel layout.
**Status:** ✅ LIVE
**Files:** `crates/myth-qgcp/src/ui.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-qgcp/src/ui.rs.
UIGenesis holds SlotDefinition items that describe named layout slots.
LayoutRegion { x, y, width, height } — normalized 0.0–1.0 coords.
Used by MythPlugin::layout_request() and negotiated by PluginRegistry.
Task: [e.g. add UIGenesis::slot_at_point(x, y) for click-target resolution]
```

---

### NODE: SealBlock
**What it is:** Immutable content-addressed seal applied to a Genesis container when finalized.
**Status:** ✅ LIVE
**Files:** `crates/myth-qgcp/src/seal.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-qgcp/src/seal.rs.
SealBlock { lineage_hash: String (blake3 hex), sealed_by: String, sealed_at: i64 }.
Applied by WorldGenesis::seal() and ActorGenesis::seal(). Once sealed, no mutations.
All Genesis types check self.sealed before accepting mutations.
Task: [e.g. add SealBlock::verify that re-hashes content and compares lineage_hash]
```

---

---

# LAYER 2 — CONTENT STORAGE (VAULT)

---

### NODE: VaultRegistry
**What it is:** Content-addressable asset storage. All GLB, PNG, WAV, WASM, MOLECULE files are stored here keyed by MythId (blake3 hash).
**Status:** ✅ LIVE
**Files:** `crates/myth-vault/src/lib.rs`, `crates/myth-vault/src/registry.rs`, `crates/myth-vault/src/atoms/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-vault/src/lib.rs and registry.rs.
VaultRegistry is content-addressable: store(bytes) → MythId (blake3 hash), fetch(id) → bytes.
All assets are keyed by their blake3 hash. The vault is fully relocatable via MYTH_VAULT_ROOT env var.
Portal system: child vaults reference master vault assets without copying bytes.
The vault has 17 ATOM sub-modules (blob_storage, fingerprinter, dedup_engine, etc.).
Task: [e.g. implement VaultRegistry::import_directory scanning a folder of GLB files]
```

---

### NODE: Master Vault MCP Server
**What it is:** MCP (Model Context Protocol) server that exposes the vault to Claude and other LLM tools via vault_fetch, vault_ingest, vault_list, vault_search, vault_info.
**Status:** ✅ LIVE — confirmed working
**Files:** `bins/myth-vault-mcp/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read bins/myth-vault-mcp/src/main.rs, tools.rs, server.rs.
The MCP server exposes 5 tools: vault_fetch (get asset by MythId), vault_ingest (add bytes),
vault_list (enumerate stored assets), vault_search (by tag/type), vault_info (registry stats).
Runs as a stdio MCP server. Configured via MYTH_VAULT_ROOT env var.
Task: [e.g. add vault_ingest_path tool that ingests all files in a directory tree]
```

---

---

# LAYER 3 — PLUGIN / INSTRUMENT SYSTEM

---

### NODE: MythPlugin Trait
**What it is:** The trait every instrument (built-in and external) implements. The registry cannot tell them apart.
**Status:** ✅ LIVE
**Files:** `crates/myth-plugin/src/plugin.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-plugin/src/plugin.rs.
MythPlugin trait: id(), name(), version(), wire_in(), wire_out(), heraldry_symbol(),
on_attach(vault), on_detach(), process(packet) → Vec<WirePacket>, tick(delta_ms) → Vec<WirePacket>,
layout_request() → LayoutRequest.
CLOSED SYSTEM: plugins interact with the world ONLY through WirePackets. No direct mutation.
Core instruments (built-in) and external plugins both implement this trait — registry sees no difference.
Task: [e.g. add a priority: u8 field to the trait for ordering when multiple plugins handle the same wire type]
```

---

### NODE: MythAddon Trait
**What it is:** Cross-instrument packet filter/augmenter. Hooks into a plugin's output stream after process() runs.
**Status:** ✅ LIVE
**Files:** `crates/myth-plugin/src/addon.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-plugin/src/addon.rs.
MythAddon: id(), target_plugin() (or "*" for any), heraldry_symbol(),
on_output(source_packet, plugin_output) → Vec<WirePacket>,
on_tick_output(delta_ms, tick_output) → Vec<WirePacket>.
Addons run in registration order. Each receives the output of the previous one.
Addons cannot access Vault directly, cannot call process(), cannot mutate plugin state.
Task: [e.g. build a LoggingAddon that prints every packet an instrument emits to tracing::debug]
```

---

### NODE: PluginRegistry
**What it is:** Certifies plugins, manages registration, heraldry glyph stamping, layout negotiation.
**Status:** ✅ LIVE
**Files:** `crates/myth-plugin-registry/src/lib.rs`, `certificate.rs`, `manifest.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-plugin-registry/src/lib.rs and certificate.rs.
PluginRegistry: register(plugin) → stamps Glyph heraldry on the plugin, certifies it,
maps its wire_in() types to the routing table. negotiate_layout(plugin) checks UIGenesis
and either grants or denies each LayoutRequest slot.
Heraldry format: "Glyph:<symbol>↑<parent-crest>", e.g. "Glyph:Erosion↑Atlas".
Task: [e.g. implement dispatch(packet) that routes a WirePacket to all matching plugins]
```

---

### NODE: WASM Plugin Host
**What it is:** Sandboxed wasmtime runtime for external/user plugins. ABI defined, transport not yet wired.
**Status:** 🔶 STUB
**Files:** `crates/myth-wasm-host/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-wasm-host/src/abi.rs, host.rs, sandbox.rs.
WasmHost loads a .wasm file implementing the MythPlugin ABI. The ABI is defined in abi.rs.
Sandbox.rs sets up the wasmtime Store with memory limits and capability restrictions.
Current status: ABI written, host::load() stubbed, process() not wired through wasmtime.
Task: implement WasmHost::process(packet) — serialize WirePacket to JSON, call guest's 
process() export, deserialize the returned Vec<WirePacket> back from guest memory.
Constraint: guest WASM must not be able to escape the sandbox (no arbitrary FS/net access).
```

---

### NODE: ControlDef (myth-controls)
**What it is:** ATOM panel widget definition — instructions for building a Fader/Knob/XYPad/etc. on the Instrument Vault panel.
**Status:** ✅ LIVE — created 2026-06-21
**Files:** `crates/myth-controls/src/lib.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-controls/src/lib.rs.
ControlDef { control_type: ControlType, range_min, range_max, default_value, taper: TaperCurve,
unit_label: Option<&'static str>, llm_prompt: Option<&'static str> }.
ControlType: Fader, Knob, Button, Toggle, XYPad, StepSequencer, Meter, Display.
TaperCurve: Linear, Logarithmic, Exponential, SCurve.
ControlDef is stored as Option<ControlDef> on SubModuleSpec (the ATOM definition).
None = passive processor with no panel widget. All 256 ATOMs per module carry one optional ControlDef.
Task: [e.g. implement ControlDef::normalize(raw_value) → f32 using TaperCurve math]
```

---

### NODE: Plugin Template / Addon Template
**What it is:** Minimal working templates for building new instruments and addons. Copy-to-start.
**Status:** ✅ LIVE (compile-only stubs)
**Files:** `templates/plugin-template/src/lib.rs`, `templates/addon-template/src/lib.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read templates/plugin-template/src/lib.rs.
This is the minimum viable MythPlugin implementation. All methods stubbed with correct return types.
To build a new instrument: copy this template to modules/myth-<name>/, update Cargo.toml,
add the path to workspace Cargo.toml, implement types.rs, wire into plugin.rs.
Task: build myth-<name> instrument by fleshing out this template.
Wire type this module emits: <WireType>. Department: <WorldConstruction|EntitySystems|NarrativeSystems|PipelineSystems>.
```

---

---

# LAYER 4 — THE 16 INSTRUMENT MODULES (BSQM)
## All 16 live at `modules/myth-<name>/`. Pattern: types.rs (domain types) + plugin.rs (MythPlugin impl) + lib.rs (re-exports).

---

### NODE: myth-atlas (MYTH-01 — Terrain / WorldConstruction)
**What it is:** Terrain, coordinates, pathfinding, zone transitions, geographic intelligence.
**Status:** ✅ TYPES LIVE (256 ATOMs + ControlDef field), ❌ Logic stubbed
**Files:** `modules/myth-atlas/src/types.rs` (556 lines — the only fully-specified module)

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-atlas/src/types.rs (the full file).
Atlas has 16 containers × 16 sub-modules = 256 ATOMs, all defined in atlas_module_spec().
SubModuleSpec { name, symbol, wire_out: WireType, control: Option<ControlDef> }.
4 layers: Coordinate (containers 1–4), Cartographic (5–8), Navigation (9–12), Intelligence (13–16).
Also has: AtlasSimParams (gravity/precipitation/seed/time), AtlasConfig (terrain gen params with NRPN),
BiomeType enum, TerrainChunk / SpawnPoint / PathMesh packet types.
Task: implement AtlasInstrument::process() — receive a CTL packet with AtlasConfig,
generate a TerrainChunk using the configured noise params, emit it as a SPA packet.
```

---

### NODE: myth-mythos (MYTH-02 — Environment / WorldConstruction)
**What it is:** Weather, atmosphere, day/night cycle, sky systems.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-mythos/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-mythos/src/types.rs and plugin.rs.
myth-mythos is MYTH-02 (Environment/Mythos department: WorldConstruction).
Primary wire_out: SPA (spatial environment data). BSQM symbol: MYT.
Current types.rs has basic domain types but NOT the full 256-ATOM SubModuleSpec table yet.
Task: build the full ContainerSpec/SubModuleSpec table for myth-mythos following the same
pattern as modules/myth-atlas/src/types.rs. 16 containers, 16 sub-modules each.
Domain: weather systems, atmospheric simulation, day/night cycle, sky rendering data,
precipitation, wind, fog, lightning. Each ATOM should have an appropriate wire_out and
control: Option<ControlDef> (add ControlDef for any ATOM that has a tuneable parameter).
```

---

### NODE: myth-architect (MYTH-03 — Architect / WorldConstruction)
**What it is:** Procedural building generation, settlement placement, structural systems.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-architect/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-architect/src/types.rs and plugin.rs.
myth-architect is MYTH-03 (Architect department: WorldConstruction).
Primary wire_out: SPA. BSQM symbol: ARC.
Task: build the full 256-ATOM SubModuleSpec table following myth-atlas as the pattern.
Domain: procedural building gen, floor plan layout, settlement placement, structural integrity,
material selection, architectural style, interior generation, ruin states, fortification.
Include ControlDef for tuneable parameters (density sliders, style knobs, etc.).
```

---

### NODE: myth-prism (MYTH-04 — Lighting / WorldConstruction)
**What it is:** Lighting systems, shadow volumes, emissive sources, light propagation.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-prism/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-prism/src/types.rs and plugin.rs.
myth-prism is MYTH-04 (Lighting/Prism department: WorldConstruction).
Primary wire_out: VIS. BSQM symbol: PRS.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: directional/point/spot lights, shadow cascade control, emissive surface management,
light propagation volumes, sky light sampling, god rays, light color temperature, bloom,
exposure control, HDR tone mapping. ControlDef for color pickers, intensity knobs, etc.
```

---

### NODE: myth-animus (MYTH-05 — Modeling / EntitySystems)
**What it is:** 3D model loading, LOD management, mesh deformation, body type assignment.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-animus/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-animus/src/types.rs and plugin.rs.
myth-animus is MYTH-05 (Modeling/Animus department: EntitySystems).
Primary wire_out: AST (asset references to GLB models). BSQM symbol: ANM.
Types already defined: LodStrategy enum (Distance/ScreenSize/Manual), BodyType enum
(Humanoid/Quadruped/Avian/Aquatic/Serpentine/Insectoid/Celestial/Abstract).
Task: build the full 256-ATOM SubModuleSpec table.
Domain: GLB/GLTF import, LOD switching, skinned mesh animation, morph targets,
procedural mesh deformation, vertex shader params, instancing, occlusion culling,
body-type to skeleton mapping, ragdoll physics attachment.
```

---

### NODE: myth-loom (MYTH-06 — Choreography / EntitySystems)
**What it is:** Animation state machines, blend trees, procedural motion, inverse kinematics.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-loom/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-loom/src/types.rs and plugin.rs.
myth-loom is MYTH-06 (Choreography/Loom department: EntitySystems).
Primary wire_out: BHV. BSQM symbol: LOM.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: animation state machine (idle/walk/run/attack states), blend tree weights,
root motion extraction, inverse kinematics (IK) for hand/foot placement,
procedural secondary motion (hair, cloth, tails), facial animation, lip sync,
crowd simulation, synchronized group choreography, emote system.
ControlDef for blend weight faders, IK target position knobs.
```

---

### NODE: myth-instinct (MYTH-07 — Behavior / EntitySystems)
**What it is:** AI behavior trees, GOAP planner, sensory systems, drive/need states.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-instinct/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-instinct/src/types.rs and plugin.rs.
myth-instinct is MYTH-07 (Behavior/Instinct department: EntitySystems).
Primary wire_out: BHV. BSQM symbol: INS.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: behavior tree node types (selector, sequence, condition, action), GOAP goal/action
definitions, blackboard memory, sensory cone (vision/hearing/smell), need states (hunger/fear/
aggression), personality trait modifiers, patrol patterns, alert/suspicious/hostile state machine,
group coordination, flee behavior, territory marking. ControlDef for aggression/fear sliders.
```

---

### NODE: myth-order (MYTH-08 — Society / EntitySystems)
**What it is:** Faction systems, relationship graphs, reputation, social drives, politics.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-order/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-order/src/types.rs and plugin.rs.
myth-order is MYTH-08 (Society/Order department: EntitySystems).
Primary wire_out: SOC. BSQM symbol: ORD.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: faction membership and standing, relationship graph (love/hate/trust/fear axes),
reputation propagation, political hierarchy, economic class, cultural norms and taboo,
social event triggers (weddings/funerals/coronations), gossip/rumor network,
alliance and betrayal mechanics, law enforcement, social class mobility.
ControlDef for reputation decay rate knob, faction bias sliders.
```

---

### NODE: myth-chronicle (MYTH-09 — Sequencer / NarrativeSystems)
**What it is:** Timeline sequencer, event scheduling, quest phase gating, temporal triggers.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-chronicle/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-chronicle/src/types.rs and plugin.rs.
myth-chronicle is MYTH-09 (Sequencer/Chronicle department: NarrativeSystems).
Primary wire_out: TMP. BSQM symbol: CHR.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: event timeline (past/present/future), scheduled trigger system, quest phase
state machine, calendar (in-world dates, seasons, moon phases), countdown timer,
narrative beat metronome, act break detection, scene pacing analyzer, time-of-day
event hooks, delayed consequence system, cooldown tracker.
ControlDef for BPM/tempo knobs for narrative pacing, trigger delay sliders.
```

---

### NODE: myth-quill (MYTH-10 — Story / NarrativeSystems)
**What it is:** Narrative structure, story arcs, lore generation, plot thread management.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-quill/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-quill/src/types.rs and plugin.rs.
myth-quill is MYTH-10 (Story/Quill department: NarrativeSystems).
Primary wire_out: NAR. BSQM symbol: QLL.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: story arc definition (setup/confrontation/resolution), plot thread tracking,
protagonist/antagonist arc intersection, Chekhov's gun registration, foreshadowing injector,
revelation timing, red herring seeder, narrative tension meter, chapter/scene manager,
dialogue tree branching, lore document generator, world-building fact registry.
ControlDef for tension/mystery knobs, pacing tempo controls.
```

---

### NODE: myth-codex (MYTH-11 — Memory / NarrativeSystems)
**What it is:** Organic event memory with spatial-temporal anchoring, significance scoring, temporal decay.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-codex/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-codex/src/types.rs and plugin.rs.
myth-codex is MYTH-11 (Memory/Codex department: NarrativeSystems).
Primary wire_out: DAT. BSQM symbol: CDX.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: associative event memory (spatial-temporal anchoring), memory significance scoring,
temporal decay curve, memory consolidation (short→long term), trauma imprinting,
collective memory (faction-level shared memories), memory distortion over time,
flashback trigger, déjà vu system, forgotten memory recovery, written record generation.
ControlDef for decay rate knob, consolidation threshold slider.
```

---

### NODE: myth-composer (MYTH-12 — Sound / NarrativeSystems)
**What it is:** Procedural audio, music systems, adaptive soundtrack, spatial sound, Capsule→sound mapping.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-composer/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-composer/src/types.rs and plugin.rs.
myth-composer is MYTH-12 (Sound/Composer department: NarrativeSystems).
Primary wire_out: AUD. BSQM symbol: CMP.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: adaptive music layer mixer (tension/calm/combat/ambient channels), procedural
melody generator, harmonic resonance field (emotional overtones), spatial audio emitter
placement, reverb/echo zone definitions, Capsule→sound trigger mapping, stinger library,
silence as a tool, crowd/ambient sound generation, instrument layer automation.
ControlDef for all 16 mixer channel faders, reverb wet/dry knob, etc.
```

---

### NODE: myth-axiom (MYTH-13 — Logic / PipelineSystems)
**What it is:** Boolean rule engine, conditional logic, procedural generator, computation graph.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-axiom/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-axiom/src/types.rs and plugin.rs.
myth-axiom is MYTH-13 (Logic/Axiom department: PipelineSystems/Universal).
Primary wire_out: LGC. BSQM symbol: AXM.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: boolean expression evaluator, AND/OR/NOT/XOR gate ATOMs, conditional branch router,
comparison operators (=, >, <, !=, in-range), pattern matcher, regex filter, probability gate
(stochastic branching), procedural name/location/event generator, rule engine (if-then-else),
mathematical expression evaluator, signal combiner, thresholding.
ControlDef for probability sliders, threshold knobs.
```

---

### NODE: myth-continuum (MYTH-14 — Simulation / PipelineSystems)
**What it is:** Physics simulation, fluid dynamics, thermodynamics, voxel destruction, sim loops.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-continuum/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-continuum/src/types.rs and plugin.rs.
myth-continuum is MYTH-14 (Simulation/Continuum department: PipelineSystems).
Primary wire_out: ENR. BSQM symbol: CNT.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: rigid body physics, soft body deformation, fluid simulation (FFT-based ocean, rivers),
thermodynamic heat transfer, combustion propagation, electrical circuit simulation,
voxel destruction/fracture, material stress fracture, crowd pressure simulation,
weather simulation (pressure systems), gravity field modulation, time dilation zone.
ControlDef for gravity multiplier knob, viscosity slider, time dilation control.
```

---

### NODE: myth-forge (MYTH-15 — Forge / PipelineSystems)
**What it is:** Asset pipeline tool, procedural asset generation, material synthesis, export.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-forge/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-forge/src/types.rs and plugin.rs.
myth-forge is MYTH-15 (Forge department: PipelineSystems).
Primary wire_out: AST. BSQM symbol: FRG.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: procedural texture synthesis, PBR material parameter generator, GLB/GLTF export,
asset LOD baker, texture atlas packer, normal map generator, ambient occlusion baker,
mesh optimization (decimation), UV unwrapping, asset variant generator, preview renderer,
catalogue manifest writer, asset tag system, quality tier selector.
ControlDef for roughness/metallic knobs, resolution dropdown, quality tier selector.
```

---

### NODE: myth-nexus (MYTH-16 — Network / PipelineSystems)
**What it is:** Network transport, peer sync, multi-user session management, collaboration protocol.
**Status:** ✅ TYPES BASIC, ❌ Logic stubbed, ❌ SubModuleSpec table not yet built
**Files:** `modules/myth-nexus/src/types.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read modules/myth-nexus/src/types.rs and plugin.rs.
myth-nexus is MYTH-16 (Network/Nexus department: PipelineSystems).
Primary wire_out: IDN. BSQM symbol: NXS.
Task: build the full 256-ATOM SubModuleSpec table.
Domain: peer discovery, session handshake, WirePacket serialization over UDP/TCP,
authority delegation (who owns which world region), conflict resolution (last-write-wins vs CRDTs),
bandwidth throttle/priority, packet loss recovery, player presence protocol,
latency compensation, anti-cheat anchor (BDna identity verification), graceful disconnect.
ControlDef for bandwidth limit slider, update rate knob, authority zone painter.
```

---

---

# LAYER 5 — SERVICES (OS SUBSTRATE)

---

### NODE: myth-clock
**What it is:** Master tick source. All subsystems slave to this. Implements Genesis Protocol cooling sequence.
**Status:** ✅ WRITTEN but PARKED (not yet wired to anything)
**Files:** `services/myth-clock/src/clock.rs`, `tick.rs`, `subscriber.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read services/myth-clock/src/clock.rs, tick.rs, subscriber.rs.
MythClock: sample_rate (fps), frame counter, elapsed_secs, temperature (1.0→0.0 cooling),
ClockPhase (Booting/Running/Crystallising/Paused/Stopped).
Tick broadcast: every subscriber gets Tick { frame, elapsed_secs, delta_secs, sample_rate, temperature, phase }.
Genesis Protocol: begin_genesis() starts cooling. At temp ≤ 0.38 → Crystallising phase.
ClockSubscriber receives via crossbeam bounded channel (buffer=4).
Task: wire myth-clock into the Core Server binary as the simulation heartbeat.
All module instruments should subscribe to myth-clock and use Tick.frame as their tick source.
Run pattern: spawn std::thread running clock.run_blocking() at 60.0 fps.
```

---

### NODE: myth-bus (PLANNED)
**What it is:** In-process message router — crossbeam channels connecting all modules via WireType subscription.
**Status:** 📋 PLANNED — not yet created
**Files:** planned at `services/myth-bus/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Dependency: crates/myth-wire, crossbeam-channel.
Build services/myth-bus as a new workspace member.
MythBus: a broker that owns a HashMap<WireType, Vec<Sender<WirePacket>>>.
subscribe(wire_type) → Receiver<WirePacket>. publish(packet) → fans out to all subscribers.
All plugins publish their output packets through MythBus. The registry routes by wire_type.
Dep rule: myth-bus depends on myth-wire and crossbeam-channel ONLY. No tokio, no renderer.
Start with: Cargo.toml, src/lib.rs with MythBus struct, subscribe(), publish(), unsubscribe().
Add unit test: two subscribers on WireType::Spatial, publish one packet, both receive it.
```

---

---

# LAYER 6 — MOLECULES (REUSABLE ATOM SUBGRAPHS)

---

### NODE: InferenceRouter MOLECULE
**What it is:** Pre-wired ATOM subgraph routing LLM inference requests. Priority: Ollama → Claude → Gemini → OpenAI → Error.
**Status:** 🔶 STUB — backends return stub errors, transport not wired
**Files:** `molecules/inference-router/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read molecules/inference-router/src/lib.rs, router.rs, atoms.rs, capsule.rs.
InferenceRouter: probe() checks which backends are alive (Ollama at localhost:11434 health check first),
route(capsule: InferenceCapsule) → InferenceOutput. InferenceCapsule { prompt, output_type: OutputType, context }.
OutputType: Text, Json, Embedding, Code.
Backends are ATOMs: OllamaAtom, ClaudeAtom, GeminiAtom, OpenAiAtom — each stubs out with Err today.
Task: implement OllamaAtom::generate(capsule) — POST to http://localhost:11434/api/generate,
stream the response, return InferenceOutput::Text. Test with a simple prompt.
Then wire InferenceRouter into the plugin system so any instrument can emit an AGT packet
and get an inference response back via WireType::Agent.
```

---

---

# LAYER 7 — ADAPTERS

---

### NODE: myth-atlas-bevy (Bevy Terrain Adapter)
**What it is:** Bevy adapter for Atlas terrain — renders TerrainChunk packets as a 3D mesh in Bevy.
**Status:** 🔶 STUB — mesh grid exists, no data flows in yet
**Files:** `adapters/myth-atlas-bevy/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read adapters/myth-atlas-bevy/src/terrain.rs, splatmap.rs, foliage.rs.
myth-atlas-bevy is the Bevy renderer adapter for myth-atlas terrain data.
TerrainChunk packets (SPA wire type) from myth-atlas should drive mesh generation here.
terrain.rs: has mesh grid scaffold. splatmap.rs: 4-channel biome blend texture.
foliage.rs: instanced grass/tree placement.
Task: implement terrain.rs::spawn_chunk(commands, chunk: &TerrainChunk, materials) —
decode TerrainChunk.heightmap into a Bevy Mesh, apply biome splatmap from TerrainChunk.biome_map,
spawn as a PbrBundle entity.
Dep: bevy 0.14 (workspace dep), myth-wire, myth-atlas.
```

---

### NODE: myth-fyrox (PLANNED — Primary 3D Engine)
**What it is:** Fyrox bridge crate — WirePacket ↔ Fyrox scene/ECS. The engine that hosts the Instrument Vault client app.
**Status:** 📋 PLANNED — not yet created
**Files:** planned at `adapters/myth-fyrox/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read CODEBASE_STATUS.md for planned crates.
Build adapters/myth-fyrox as a new workspace member.
Fyrox (https://fyrox.rs) is the primary 3D engine for Instrument Vaults. It uses a hybrid
ECS/OOP architecture. Fyrox has a built-in editor (Fyroxed). Bevy stays ONLY for the BioSpark
Theater system. Fyrox owns all Instrument Vault client rendering.
Cargo.toml deps: fyrox = "0.34", myth-wire = { path = "../../crates/myth-wire" }.
Structure:
  src/lib.rs — FyroxBridge struct
  src/scene.rs — Fyrox scene graph integration
  src/wire_bridge.rs — WirePacket → Fyrox scene mutations
  src/plugin_host.rs — runs MythPlugin instruments inside Fyrox game loop
Design: FyroxBridge::process_packet(packet: &WirePacket) routes to the correct scene handler
based on WireType (SPA → terrain, BHV → actor animation, VIS → visual effects, etc.).
```

---

### NODE: BioSpark Theater (biospark-theatre)
**What it is:** Decoupled composite renderer. Channels = After Effects layers (not TV channels). Bevy is an adapter, not the host.
**Status:** 🔶 PARKED — written but not wired to anything
**Files:** `crates/biospark-theatre/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/biospark-theatre/src/lib.rs, channel.rs, layer.rs,
compositor.rs, glyph.rs, routing.rs.
BioSpark Theater is the composite renderer for The Interstellar Tour and all visual output.
10-step build sequence: ChannelId → Layer → Compositor → Signal router → RenderTarget trait
→ Bevy adapter → egui adapter → myth-daw Transport tick → AutomationLane → myth-wire bus.
Current status: channel.rs and layer.rs have types, compositor has Vec<Layer>, routing stubs exist.
Task: implement the RenderTarget trait (step 5) and wire myth-clock Tick → compositor.tick().
The compositor should call RenderTarget::composite(layers) every tick.
No renderer code in this crate — it is renderer-agnostic. Bevy and egui adapters are separate.
```

---

### NODE: myth-daw
**What it is:** Audio-style DAW infrastructure. Transport, Session clip launcher, Arrangement timeline, Mixer, AutomationLane. Fully tested.
**Status:** ✅ LIVE and tested (9 transport tests pass)
**Files:** `crates/myth-daw/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-daw/src/transport.rs, session.rs, arrangement.rs, mixer.rs.
myth-daw Transport: play/pause/stop/record, BPM, bar_beat() (0-indexed), loop_region, seek.
Session: track+scene clip grid, trigger_scene(), commit_queued() → Playing.
Arrangement: ArrangementTrack, active_clips_at(beat), AutomationLane breakpoints + lerp.
Mixer: Fader/Mute/Solo/Arm, effective_level() solo logic, sends.
Wire packets: TransportTick(TMP), ClipEvent(EVT), MixerLevel(CTL), AutomationValue(DAT).
Task: wire myth-daw Transport to myth-clock — Transport.tick() should consume a Tick from
ClockSubscriber instead of advancing internally. Transport.position_beats should slave to
Tick.elapsed_secs * (bpm / 60.0). Run demo: cargo run --example session_demo -p myth-daw
```

---

---

# LAYER 8 — APPLICATIONS (BINARIES)

---

### NODE: myth-instrument-vault (PLANNED — Fyrox Client App)
**What it is:** The primary user-facing application. Fyrox 3D app. 4-layer drill-down UI for controlling the simulation.
**Status:** 📋 PLANNED — not yet created
**Files:** planned at `bins/myth-instrument-vault/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read CODEBASE_STATUS.md, BLUEPRINT.md Part III module list,
and crates/myth-controls/src/lib.rs and crates/myth-qgcp/src/actor_container.rs.
Build bins/myth-instrument-vault as a Fyrox application (Cargo.toml dep: fyrox = "0.34").
4-layer drill-down UI (bidirectional reactive binding — all layers subscribe to same CONTROL values):
  L1: Propellerhead Reason-style instrument rack — 16 module instruments as rack units
  L2: VCV Rack Eurorack-style module panel — each instrument's 16 containers as Eurorack modules
  L3: Actor Node graph — actors as nodes with typed wire connections between them
  L4: Actor internals — double-click an actor node to enter: see its ATOMs, CELLs, CAPSULEs, CONTROLs
CONTROL values are authoritative on Core Server. All layers subscribe via WireType::Control packets.
When a user rotates a Knob (ControlDef) the new value emits as a WireType::Control WirePacket
to the Core Server. All UI layers receive the updated value back via subscription diff.
Start with: L1 rack rendering myth-atlas through myth-nexus as 16 rack units.
```

---

### NODE: Opalis (Vault Browser UI)
**What it is:** The vault card browser and node graph app. egui-based. The first working UI.
**Status:** 🔄 PARKED — working UI but not wired to myth-wire
**Files:** `bins/opalis/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read bins/opalis/src/main.rs, graph.rs, nodes.rs, vault.rs.
Opalis is an egui 0.33 app (egui isolated from workspace main version to avoid conflicts).
Working: vault card browser, inside-vault view, egui-snarl node graph.
Known bugs (from BLUEPRINT.md Part XI): duplicate windows on launch, blank mandelbulb windows,
all instrument views identical, no plugin load/unload UI, vault cards too wide.
Not wired: myth-wire packets not flowing through the node graph, vault.rs not using VaultRegistry.
Task: [pick one bug from the list and fix it — or wire vault.rs to VaultRegistry::fetch/store].
```

---

### NODE: Core Server (PLANNED — Headless Simulation)
**What it is:** The headless server that owns the simulation state. Independent of any renderer. Instrument Vaults are clients to this.
**Status:** 📋 PLANNED
**Files:** planned at `bins/myth-core-server/` (or activate `core/src/main.rs`)

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read services/myth-clock/src/clock.rs, crates/myth-plugin-registry/src/lib.rs,
crates/myth-qgcp/src/genesis.rs and actor.rs.
Build the Core Server as a headless Rust binary. It owns:
  - One WorldGenesis (the simulation state)
  - One MythClock (the heartbeat — 60fps tick loop)
  - One PluginRegistry (all 16 instruments registered as plugins)
  - One MythBus (WirePacket router connecting all instruments)
  - One VaultRegistry (content storage)
  - WireType::Control authority — all ControlDef values live here, broadcast to clients
The Core Server runs independently of Fyrox. Instrument Vaults (Fyrox clients) connect to it
via gRPC (myth-nexus / tonic) or in-process channels for local dev.
Actor simulation loop: each tick, iterate ActorGenesis records, call actor_containers cells,
emit WirePackets to MythBus, let instruments process them.
Start simple: single-process in-memory (bus is crossbeam channels, no network yet).
```

---

---

# LAYER 9 — ASSET PIPELINE & GEOMETRY

---

### NODE: asset-forge
**What it is:** Asset pipeline tool — ingests model spec TOMLs, generates manifests, names assets using naming convention.
**Status:** 🔄 PARKED — basic structure exists
**Files:** `asset-forge/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read asset-forge/src/main.rs, manifest.rs, pipeline.rs, tokens.rs.
asset-forge reads .toml specs (see asset-forge/examples/) and generates JSON manifests
following the naming convention: CATEGORY_NAME_STYLE_SIZE_VIEW_SHADER_VARIANT.
See examples: cave_entrance.toml, luminarite_spire.toml, xyrona_guardian.toml.
The catalogue.catalogue.json is the master index of all generated assets.
Task: implement pipeline.rs::run() — for each .toml in the input dir, parse the spec,
generate a manifest JSON, write to manifests/<category>/<name>/<style>/<full_name>.json,
and update catalogue.catalogue.json with a new entry.
```

---

### NODE: void-sculptor (Marching Cubes)
**What it is:** Marching cubes mesh generator from signed-distance-field volumes. Used for procedural cave/organic geometry.
**Status:** 🔶 PARKED — complete in isolation, not wired to myth-os
**Files:** `void-sculptor/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read void-sculptor/src/marching_cubes.rs, field_reader.rs, mesh.rs.
void-sculptor takes a signed distance field (3D f32 grid) and runs marching cubes to produce
a triangle mesh. Params: void-sculptor/0x7E2-ALPHA_params.json.
Currently standalone — not connected to myth-atlas or myth-architect.
Task: create a MythPlugin wrapper around void-sculptor so it can receive ENR packets
(field parameters) and emit AST packets (mesh bytes as GLB). Wire into myth-forge.
The void-sculptor output (GLB) should be stored in VaultRegistry and referenced as an AssetRef.
```

---

### NODE: genesis-collider-mandelbulb
**What it is:** Mandelbulb and Mandelbox 3D fractal renderer / SDF generator. For dome visuals in the Interstellar Tour.
**Status:** 🔶 PARKED — complete in isolation
**Files:** `genesis-collider-mandelbulb/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read genesis-collider-mandelbulb/src/mandelbulb.rs, field.rs, params.rs.
Generates 3D mandelbulb/mandelbox fields (signed distance functions). Params-driven.
Currently standalone output goes to viewer_output/.
For the Interstellar Tour: this feeds BioSpark Theatre as a VIS packet source.
Task: build a MythPlugin wrapper — receive VIS control packets (rotation/zoom/color params),
render one frame of the mandelbulb field, encode as PNG/EXR bytes, emit as WireType::Visual packet.
Plugin id: "genesis.mandelbulb". The Theatre Layer picks up VIS packets and renders to dome.
```

---

### NODE: viewer (GLB Viewer)
**What it is:** OBJ/mesh viewer with orbit camera. May already render GLBs.
**Status:** 🔶 INVESTIGATE — may already render, quick win
**Files:** `viewer/src/`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read viewer/src/main.rs, orbit.rs, field.rs, generate.rs, obj.rs.
This is a standalone viewer with orbit camera. Check if it currently compiles and runs.
Run: cargo run -p viewer (check Cargo.toml for the package name first).
Task: if it runs, wire it to VaultRegistry so it can browse and open any stored GLB by MythId.
If it loads OBJ only, add GLB support via the gltf crate.
This is the fastest path to "see something on screen" — prioritize getting it running first.
```

---

---

# LAYER 10 — DESIGN SYSTEM & UI THEME

---

### NODE: Design System
**What it is:** The Procedural Rack Design System — colour palette, visual language, UI personality.
**Status:** ✅ DOCUMENTED (HTML), 🔶 not yet fully applied to egui
**Files:** memory at `C:\Users\phant\.claude\projects\J--myth-os\memory\project_design_system.md`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read C:\Users\phant\.claude\projects\J--myth-os\memory\project_design_system.md.
The Design System defines the visual language for all myth-os UI:
color palette, panel types, rack unit aesthetics, control widget styles, typography.
Task: create crates/myth-theme/src/lib.rs with:
  - MYTH_COLORS const struct (all colors from the design system as [u8; 3] or egui::Color32)
  - ThemeProfile struct { name, colors, font_sizes, panel_style }
  - fn apply_to_egui(ctx: &egui::Context, profile: &ThemeProfile)
  - 3 preset profiles: RackDark (default), AtlasBlue (#1e8cff), ForgeOrange
Dep: egui (workspace). No renderer other than egui.
```

---

### NODE: Xyrona Profession Themes (50 themes)
**What it is:** 50 full UI personalities (one per profession) — layout metaphors, interaction style, visual language.
**Status:** 💡 IDEA — documented in BLUEPRINT.md Part IX
**Files:** BLUEPRINT.md Part IX

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read BLUEPRINT.md Part IX section "Xyrona Prime Profession UI Themes".
50 profession themes — each is a full UI personality, not just colors.
The 14 mapped professions are: Neural Narrative Designer → Quantum Quill, Harmonic Resonance Engineer
→ myth-daw mixer, Dream Sequencer → Session launcher, Eldritch Archivist → Vault, etc.
Task: create the first 3 profession themes as ThemeProfile instances in crates/myth-theme:
  1. NeuralNarrativeDesigner — organic flowing curves, dark purple/gold, biomorphic panels
  2. HarmonicResonanceEngineer — audio waveform aesthetics, oscilloscope green on black, rack UI
  3. ElitchArchivist — aged parchment + arcane sigils, ornate borders, tome-like UI
Each theme affects: egui visuals (rounding, spacing, colors), panel layout metaphors, particle hints.
```

---

---

# LAYER 11 — HARDWARE INTERFACES

---

### NODE: Traktor S4 / MIDI Integration
**What it is:** Native Instruments Traktor S4 → MIDI → myth-os. Jog wheels scrub timeline, crossfader transitions scenes, pads trigger clips.
**Status:** 💡 IDEA — midir dep exists, not wired
**Files:** `crates/myth-stencil/src/midi.rs` (existing MIDI binding), `crates/myth-daw/src/wire.rs`

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. Read crates/myth-stencil/src/midi.rs and crates/myth-daw/src/transport.rs.
myth-stencil already has: MidiBinding, flex_from_cc(), apply_midi(ch, cc, val).
myth-daw Transport already has: seek(beat), BPM control.
The Traktor S4 appears as a MIDI device via midir (workspace dep). S4 MIDI map:
  Jog wheel (left/right) → seek delta on Transport (scrub timeline)
  Crossfader (CC) → scene crossfade in Session clip launcher
  Sample pads (4×4) → trigger_scene() / trigger_clip()
  EQ knobs → MixerLevel packets (CTL wire type)
Task: build crates/myth-controller/src/s4.rs — open S4 MIDI port via midir, map incoming
CC messages to WirePackets (TMP for transport, EVT for clips, CTL for mixer levels),
emit to MythBus. Wire into the Core Server main loop.
```

---

### NODE: DMX / Lighting Hardware
**What it is:** RS-485 DMX output for live lighting rigs. Emotion module drives DMX values.
**Status:** 💡 IDEA — serialport dep exists
**Files:** no files yet; serialport in workspace deps

**Scaffold Prompt:**
```
myth-os workspace at J:\myth-os. serialport = "4" is already in workspace deps.
DMX512 protocol: 512 channels, RS-485 serial at 250kbaud. Each channel is 0–255.
myth-composer/myth-order (emotion/sound modules) emit ENR wire packets with lighting values.
Task: build crates/myth-dmx/src/lib.rs — DmxController wraps a serialport::SerialPort,
dmx_send(channels: [u8; 512]) formats the DMX512 frame (break, mark, start code, 512 bytes)
and writes to RS-485. MythPlugin wrapper: consume ENR packets, map payload values to
DMX channel assignments, emit dmx_send(). The venue lighting rig is controlled this way
in the Interstellar Tour.
```

---

---

# CROSS-CUTTING RULES (apply to every node)

```
NAMING: Rust identifiers are plain technical terms. Lore belongs ONLY in .name/.description
        display fields. Wrong: XyronaPlayer. Right: AudioPlayer.

DEPS:   myth-wire ← myth-qgcp ← myth-vault ← myth-plugin ← modules
        No renderer deps in foundation crates. No tokio in myth-wire.

ATOMS:  Every module has exactly 256 ATOMs (16 containers × 16 sub-modules). Fixed law.
        SubModuleSpec { name, symbol, wire_out, control: Option<ControlDef> }
        control: None = passive. control: Some(def) = has a panel widget.

WIRE:   All cross-boundary communication is WirePacket only.
        17 WireTypes — CLOSED SET. Do not add variants.

LAW 16: WorldGenesis → 16 MythosModules → 16 Containers → 16 Capsules = 4,096 slots.
        ActorGenesis → 16 ActorContainers → 16 Cells.
        Every module → 16 ContainerSpecs → 16 SubModuleSpecs.

UNITS:  ATOM = operation node. CAPSULE = data payload (uses Glyph). CELL = actor capability
        (uses Sigil). MOLECULE = saved ATOM subgraph. CONTROL = ATOM's panel widget.
        ACTOR = any active/reactive entity (NPC, volcano, alarm system).

CLOCK:  myth-clock is the ONE heartbeat. Nothing else keeps time independently.
        Tick.frame is sequence number. Tick.temperature drives Genesis Protocol.

STATUS: Read CODEBASE_STATUS.md first every session. Update it last.
        Never create a crate without checking it for duplicates.
```

---

*Generated 2026-06-21. Source of truth for scaffold prompts across all myth-os nodes.*
