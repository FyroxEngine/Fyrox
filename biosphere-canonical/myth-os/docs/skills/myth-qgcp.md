---
name: myth-qgcp
description: The myth-qgcp crate — Quantum Genesis Container Protocol. Defines all Genesis Container types (WorldGenesis, MediaGenesis, ActorGenesis, UIGenesis), their Rust structs, the Law of 16 hierarchy, seal/lineage mechanics, and what each type still needs. TRIGGER: any time someone mentions genesis containers, capsules, QGCP, WorldGenesis, ActorGenesis, UIGenesis, MediaGenesis, .worldgenesis/.mediagenesis/.actorgenesis/.uigenesis files, or asks how world state is stored or structured.
---

# myth-qgcp — Quantum Genesis Container Protocol

The canonical sealed container format for the BioSpark ecosystem. All world
state, media assets, actor definitions, and UI layouts live in Genesis
Containers. This crate defines the types; the Vault stores and retrieves them;
plugins read capsule data to operate.

---

## The Four Genesis Types

| Type | Alias | File Ext | genesis_id Prefix | Purpose |
|------|-------|----------|-------------------|---------|
| `WorldGenesis` | `WorldGen` | `.worldgenesis` | `world_` | Headless world state — server AND client agents |
| `MediaGenesis` | `MediaGen` | `.mediagenesis` | `media_` | Binary asset bundle — client Theater only |
| `ActorGenesis` | `ActorGen` | `.actorgenesis` | `actor_` | Agent/character container |
| `UIGenesis`    | `UIGen`    | `.uigenesis`    | `ui_`    | UI layout slot topology |

A world ships as a directory:
```
worlds/kasmir-delta/
    kasmir-delta.worldgenesis   ← simulation state (headless — server + agents)
    kasmir-delta.mediagenesis   ← assets: GLBs, audio, images (client only)
    meta.toml                   ← name, bDNA, checksums
```

MediaGenesis is the "DVD" — it's the WorldGenesis with all the binary assets
attached. Think of WorldGenesis as the CD (pure data) and MediaGenesis as the
DVD (data + media). The headless server and agents only need the CD.

---

## The Law of 16 — Capacity Hierarchy

Every Genesis type obeys the same hierarchy:

```
Genesis Container
  └── 16 Mythos (modules / departments / layout regions)
        └── 16 Containers (sub-modules / slot groups)
              └── 16 Capsules (atomic parameters / slot definitions)

Total: 16 × 16 × 16 = 4,096 capsule slots per genesis container
```

Constants enforced everywhere:
```rust
pub const MAX_MYTHOS:     usize = 16;
pub const MAX_CONTAINERS: usize = 16;
pub const MAX_CAPSULES:   usize = 16;
pub const TOTAL_CAPACITY: usize = 4096;
```

Attempting to add a 17th Mythos returns `QgcpError::MythosOverflow`.

---

## The BSQM Relationship

**BSQM-MODULES-GENESIS-V1.0** (`docs/BSQM-MODULES-GENESIS-V1.0.json`) is the
base module library Genesis Container. It is NOT a WorldGenesis — it is the
canonical specification of the 16 Quantum Modules with all their sub-containers
and atomic capsule parameters (4,096 total).

A WorldGenesis `depends_on` the BSQM. When a world loads, the system must
verify the BSQM is present and its lineage hash matches.

The 16 modules in BSQM, organized by department:

| # | Module | Crest | Department |
|---|--------|-------|------------|
| 01 | Terrain | Atlas | WorldConstruction |
| 02 | Environment | Mythos | WorldConstruction |
| 03 | Architect | Architect | WorldConstruction |
| 04 | Lighting | Prism | WorldConstruction |
| 05 | Modeling | Animus | EntitySystems |
| 06 | Choreography | Loom | EntitySystems |
| 07 | Behavior | (new art) | EntitySystems |
| 08 | Society | Order | EntitySystems |
| 09 | Sequencer | Chronicle | NarrativeSystems |
| 10 | Story | Quill | NarrativeSystems |
| 11 | Memory | Codex | NarrativeSystems |
| 12 | Sound | Composer | NarrativeSystems |
| 13 | Logic | (new art) | PipelineSystems |
| 14 | Simulation | (new art) | PipelineSystems |
| 15 | Forge | Forge | PipelineSystems |
| 16 | Network | Nexus | PipelineSystems |

---

## Current Rust Structs

### WorldGenesis

```rust
pub struct WorldGenesis {
    pub genesis_id:    String,          // "world_<uuid>"
    pub package_id:    String,          // e.g. "pkg.biospark.kasmir-delta"
    pub name:          String,
    pub domain:        String,          // "world" | "agent-memory" | "project"
    pub description:   Option<String>,
    pub lifecycle:     String,          // "draft" | "active" | "sealed"
    pub sealed:        bool,
    pub mythos:        Vec<MythosModule>,
    pub seal:          Option<SealBlock>,
    pub created_at:    i64,
    pub schema_version: String,
}
```

### MediaGenesis

```rust
pub struct MediaGenesis {
    pub genesis_id:       String,       // "media_<uuid>"
    pub world_genesis_id: String,       // companion WorldGenesis ID
    pub name:             String,
    pub assets:           Vec<AssetRef>,
    pub lifecycle:        String,
    pub sealed:           bool,
    pub seal:             Option<SealBlock>,
    pub created_at:       i64,
    pub schema_version:   String,
}

pub struct AssetRef {
    pub asset_id:        String,
    pub name:            String,
    pub media_type:      MediaType,     // Audio | Image | Video | Model | Texture | Skybox | Font
    pub path:            String,
    pub size_bytes:      u64,
    pub blake3_checksum: String,
    pub wire_hint:       Option<String>,
    pub tags:            Vec<String>,
}
```

### ActorGenesis

```rust
pub struct ActorGenesis {
    pub genesis_id:        String,      // "actor_<uuid>"
    pub home_world_id:     String,
    pub actor_name:        String,
    pub archetype_id:      String,      // ⚠ untyped string — see gaps below
    pub bdna_signature:    BDna,        // deterministic from world+name
    pub heraldry_symbol:   String,
    pub intelligence_tier: u8,          // 0–16, capped at 16
    pub mythos:            Vec<MythosModule>,
    pub lifecycle:         String,
    pub sealed:            bool,
    pub seal:              Option<SealBlock>,
    pub created_at:        i64,
    pub schema_version:    String,
}
```

### UIGenesis

```rust
pub struct UIGenesis {
    pub genesis_id:      String,        // "ui_<uuid>"
    pub name:            String,
    pub description:     Option<String>,
    pub target_world_id: Option<String>,  // None = universal layout
    pub theme_hint:      String,        // "futuristic-archivist" | "dark" | "light"
    pub mythos:          Vec<MythosModule>,  // 16 LayoutRegions as Mythos slots
    pub lifecycle:       String,
    pub sealed:          bool,
    pub seal:            Option<SealBlock>,
    pub created_at:      i64,
    pub schema_version:  String,
}
```

The 16 `LayoutRegion` variants (UIGenesis Mythos = one region per slot):
`HeaderLeft`, `HeaderCenter`, `HeaderRight`, `CanvasToolbar`, `CanvasLeft`,
`CanvasMain`, `CanvasRight`, `CanvasStatusBar`, `FooterLeft`, `FooterCenter`,
`FooterRight`, `OverlayModal`, `OverlayDrawer`, `OverlayTooltip`,
`Notification`, `ContextMenu`

UIGenesis default layout: `UIGenesis::default_layout()` builds all 16 regions
pre-populated with `SlotDefinition` capsules.

### MythosModule

```rust
pub struct MythosModule {
    pub module_id:   String,
    pub name:        String,
    pub wire_type:   WireType,          // primary wire type for this module
    pub containers:  Vec<Container>,
}
```

### Container

```rust
pub struct Container {
    pub container_id: String,
    pub name:         String,
    pub wire_type:    WireType,
    pub capsules:     Vec<Capsule>,
}
```

### Capsule

```rust
pub struct Capsule {
    pub capsule_id:   String,           // hex content-addressed ID
    pub name:         String,
    pub wire_type:    WireType,
    pub payload:      serde_json::Value,
    pub tags:         Vec<String>,
    pub lineage_hash: Option<String>,   // BLAKE3 over payload — detects drift
}
```

### SealBlock

```rust
pub struct SealBlock {
    pub lineage_hash: String,           // BLAKE3 hex over sealed mythos
    pub sealed_by:    String,           // who/what sealed it
    pub sealed_at:    i64,
}
```

---

## Gaps — What Needs to Be Added

These fields exist in the BSQM JSON or were identified in architecture review.
They are not yet in the Rust structs. Add in priority order.

### Priority 1 — WorldGenesis: `bdna_signature`

Every world needs a deterministic identity fingerprint, derived from
`world_name + domain + seed`. ActorGenesis already has `bdna_signature` (a
`BDna`). WorldGenesis has nothing.

```rust
// Add to WorldGenesis:
pub bdna_signature: Option<String>,   // hex, deterministic from name+domain+seed
```

### Priority 2 — MythosModule: `crest`, `color`, `primary_wire_out`, `status`

The BSQM has `"crest": "Atlas"` on each module. Our `MythosModule` has no
`crest` field. This breaks the heraldry system — a plugin says
`"Glyph:Erosion↑Atlas"` but the system has no declared source of truth for
which crest belongs to which module. These belong on `MythosModule`.

```rust
// Add to MythosModule:
pub crest:            String,         // heraldry crest name e.g. "Atlas"
pub color:            Option<String>, // hex color for UI e.g. "#1e8cff"
pub primary_wire_out: Option<String>, // "SPA" | "VIS" | "AUD" etc.
pub status:           String,         // "draft" | "built" | "planned" | "in_progress"
```

### Priority 3 — Container: `symbol`

BSQM containers have a 3-letter routing shortcode: `"WOA"` (World Origin
Anchor), `"HGM"` (Hex Grid Mapper). Used in ATOM node graphs as the node label.
Without it every node just shows a long name.

```rust
// Add to Container:
pub symbol: Option<String>,           // e.g. "WOA", "HGM"
```

### Priority 4 — Capsule: `value_type` + `value_range`

**This is the untyped capsule problem.** Every capsule in BSQM has this payload:
```json
{ "param_index": 5, "default_val": 0.584 }
```
They are all raw floats. The system must *know from code* what index 5 means.
There is no way to display, validate, or safely read a capsule without knowing
its type. Our `payload: serde_json::Value` is flexible but semantically opaque.

```rust
// Add to Capsule:
pub value_type:  String,              // "f32" | "f64" | "bool" | "u8" | "u16" |
                                      // "u32" | "i32" | "vec3" | "color" |
                                      // "string" | "enum"
pub value_range: Option<[f64; 2]>,   // [min, max] for numeric types
```

This was the original reason the BSQM capsules were only strings/LLM-interpreted.
The module system was built precisely to move beyond that — adding value_type
completes that move.

### Priority 5 — WorldGenesis: `seed_answers`

The creation ritual. Questions answered at world-birth that get locked into the
lineage. Once sealed, they are part of the seal hash.

```rust
// New type:
pub struct SeedAnswer {
    pub id:        String,
    pub question:  String,
    pub answer:    String,
    pub locked_at: i64,
}

// Add to WorldGenesis:
pub seed_answers: Vec<SeedAnswer>,
```

### Priority 6 — WorldGenesis: `depends_on`

Which module genesis (BSQM) this world was built from. The system needs this
to verify the module base is present before loading a world.

```rust
// Add to WorldGenesis:
pub depends_on: Vec<String>,          // genesis IDs e.g. ["BSQM-20240418-001"]
```

### Priority 7 — ActorGenesis: `EntityArchetype` + `EntityCapabilities`

`archetype_id: String` is completely untyped. "humanoid" and "creature" are
the same to the type system. Nothing enforces what an archetype can do.

```rust
pub enum EntityArchetype {
    Humanoid,     // sentient biped
    Creature,     // non-humanoid fauna
    Flora,        // plant life (can still have behavior)
    Construct,    // built / mechanical
    Celestial,    // stars, planets, moons at cosmic scale
    Faction,      // organization / civilization
    Spirit,       // non-physical / narrative entity
    Phenomenon,   // weather, fire, disease — emergent
}

pub struct EntityCapabilities {
    pub can_move:         bool,
    pub can_communicate:  bool,
    pub can_reproduce:    bool,
    pub has_sentience:    bool,
    pub can_die:          bool,
    pub can_form_groups:  bool,
    pub has_memory:       bool,
    pub can_use_tools:    bool,
}

// Replace in ActorGenesis:
// pub archetype_id: String  →
pub archetype:      EntityArchetype,
pub capabilities:   EntityCapabilities,
```

### Priority 8 — WorldGenesis: `wire_types` manifest

Fast routing hint — which wire types are active in this world — without
deserializing 4,096 capsules. Derived from the capsule scan but stored at top
level for startup performance.

```rust
// Add to WorldGenesis:
pub wire_types: Vec<String>,          // e.g. ["SPA", "ENR", "VIS", "BHV"]
```

### Priority 9 — Genesis-level `assets`

Icon, banner, seal art, crest SVG for display in the Library UI and Theater.

```rust
pub struct GenesisAssets {
    pub icon:     Option<String>,     // path to icon image
    pub banner:   Option<String>,     // path to banner image
    pub seal_art: Option<String>,     // path to seal artwork
    pub crest_svg: Option<String>,    // path to crest SVG
}

// Add to WorldGenesis, MythosModule:
pub assets: Option<GenesisAssets>,
```

---

## Seal and Lineage

A sealed container is **immutable**. Any mutation attempt returns
`QgcpError::Sealed`. The seal carries a `lineage_hash` (BLAKE3 over the
serialized mythos) so drift is cryptographically detectable.

```
seal() flow:
  1. validate_capacity()       — enforces Law of 16
  2. verify_integrity()        — checks all capsule lineage hashes
  3. serde_json::to_string(mythos)
  4. blake3::hash(content)
  5. lifecycle → "sealed", sealed → true
  6. SealBlock { lineage_hash, sealed_by, sealed_at }

verify_seal() flow:
  1. Re-hash current mythos content
  2. Compare against stored seal.lineage_hash
  → false = container has drifted after sealing
```

---

## Crate Dependency Rule

```
myth-qgcp depends on: myth-wire, serde, serde_json, bincode, blake3, hex, chrono, uuid
NO renderer deps. NO tokio. NO Bevy. NO egui.
```

This never changes. myth-qgcp must compile in any environment including headless
servers with no graphics hardware.

---

## What myth-qgcp Does NOT Cover

- **Vault storage** — how genesis containers are persisted to disk is
  `myth-vault`'s concern. myth-qgcp only defines the types.
- **Plugin reading of capsules** — how instruments access capsule data at
  runtime is a `myth-plugin` concern (via `VaultRegistry`).
- **BSQM module specs** — the 16 Quantum Module JSON files and their I/O
  contracts are in `docs/BSQM-MODULES-GENESIS-V1.0.json` and the
  `quantum-modules` skill (Google Drive). The Rust structs here just need to
  be able to hold what those specs define.
- **UIGenesis layout negotiation** — that is `myth-plugin`'s
  `negotiate_layout()` and `LayoutRequest` / `LayoutGrant` types. UIGenesis is
  the data format; myth-plugin is the negotiation protocol.
