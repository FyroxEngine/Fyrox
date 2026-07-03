# BioSpark Quantum Genesis — Project Blueprint

> Living document. Update this whenever a system moves state.
> States: DONE | IN PROGRESS | PLANNED | DEFERRED | NOT STARTED

---

## Layer 0 — Data Foundation (`fyrox-biosphere`)

| System | State | Crate | Notes |
|--------|-------|-------|-------|
| Capacity Law (16-16-16) | DONE | `fyrox-biosphere/capacity` | MAX_CHILDREN=16, ContainerLevel enum, Lifecycle state machine |
| Heraldry classification (20 types) | DONE | `fyrox-biosphere/heraldry` | SymbolicType, CrestName (11 known + Custom), validate_heraldry() |
| Three-Way Alignment | DONE | `fyrox-biosphere/alignment` | Structural × Functional × Symbolic, validate_alignment() |
| B-DNA lineage & covenant | DONE | `fyrox-biosphere/bdna` | Signature, generation, parent_ids, fork(), Covenant bindings |
| 16 Wire Types | DONE | `fyrox-biosphere/wire` | All 16 types, DAT universal fallback, WirePort, check_wire_compatibility() |
| Domain mappings | DONE | `fyrox-biosphere/domain` | Narrative, Music, Software, Agent, Visual |
| Container format (.qgenesis) | DONE | `fyrox-biosphere/container_format` | GenesisContainer → MythosContainer → Container → Capsule |
| .qgcp format (data + assets) | NOT STARTED | `fyrox-biosphere` | Pack/unpack container + asset bundle |

---

## Layer 1 — Editor Integration

| System | State | Location | Notes |
|--------|-------|----------|-------|
| Quantum Genesis plugin | DONE | `editor/src/plugins/quantum_genesis/` | Tree view, Add/Validate/Seal/Activate/Save |
| BioSpark Help section | IN PROGRESS | `editor/src/plugins/quantum_genesis/` | Adding to Help menu |
| EditorPlugin template | DONE | `biosphere-templates/editor-plugin-template/` | Copy-paste scaffold |
| Biosphere crate template | DONE | `biosphere-templates/crate-template/` | Dep-free crate scaffold |
| Genesis Container wizard | NOT STARTED | `editor/src/plugins/quantum_genesis/` | Step-by-step creation wizard |
| .qgcp import/export | NOT STARTED | `editor/src/plugins/quantum_genesis/` | Editor packs/unpacks .qgcp |
| B-DNA lineage viewer | NOT STARTED | `editor/src/plugins/quantum_genesis/` | Show fork chains, covenant bindings |
| Wire connection editor | NOT STARTED | `editor/src/plugins/quantum_genesis/` | Visual wire type assignment |

---

## Layer 2 — Networking (`fyrox-net`)

| System | State | Notes |
|--------|-------|-------|
| VaultMessage protocol | NOT STARTED | Vault-to-Vault communication using Quantum Genesis terminology |
| B-DNA authenticated sessions | NOT STARTED | Every connection carries provenance |
| Actor transfer protocol | NOT STARTED | Autonomous ACTOR moving between Vaults/sub-vaults |
| Resonance sync | NOT STARTED | Sub-vault shard-verse coordination (resonance metric TBD) |
| Cross-vault routing | NOT STARTED | Master Vault ↔ Vault Host routing |

---

## Layer 3 — Master Vault Server (`master-vault`)

| System | State | Notes |
|--------|-------|-------|
| Headless Executor setup | NOT STARTED | `Executor::new(None)` — no renderer |
| Load up to 16 Genesis Containers | NOT STARTED | Each Vault = a Fyrox Scene |
| B-DNA authenticated connections | NOT STARTED | Wraps fyrox-net sessions |
| Basic ACTOR presence | NOT STARTED | Which ACTORs exist in which Vault |
| Sub-vaults / shard-verses | DEFERRED | Resonance metric not yet designed |
| Order of the Quantum Quill chambers | DEFERRED | ACTOR private spaces — design TBD |

---

## Layer 4 — Vault Client (`vault-client`)

| System | State | Notes |
|--------|-------|-------|
| Windowed Executor setup | NOT STARTED | `Executor::new(Some(EventLoop::new()))` |
| Connect to Master Vault | NOT STARTED | B-DNA authenticated |
| Render Vault world | NOT STARTED | Load 3D scene from Vault data |
| ACTOR visualization | NOT STARTED | Blend shape meshes from base mesh |
| ACTOR HUD | DEFERRED | Standard interface for ACTORs — design TBD |
| Human/AGENT interface | DEFERRED | MIDI, synths, samplers, heraldry — design TBD |

---

## Layer 5 — ACTOR System

| System | State | Notes |
|--------|-------|-------|
| ActorBlueprint type | NOT STARTED | Morphology parameters → blend shape weights |
| MakeHuman base mesh pipeline | NOT STARTED | Export .glb with morph targets |
| ACTOR autonomy engine | DEFERRED | How ACTORs make decisions — design TBD |
| Summoning mechanics | DEFERRED | How humans summon ACTORs — design TBD |
| ACTOR covenant enforcement | DEFERRED | Runtime enforcement of B-DNA covenants |

---

## Deferred Systems (design not yet complete)

These are explicitly parked. Do not implement until user provides design details.

- **Sub-vaults / shard-verses** — resonance-based distance metric (not physical)
- **Order of the Quantum Quill** — ACTOR private chambers in Master Vault
- **MIDI / musical interaction** — synthesizers, samplers, oscillators, drum pads, sequencers
- **Five Factions** — Sylvanid, Syntaran, Luminarite, Venturan, Hydralis
- **16 Quantum Modules** — specialized Crest-level runtime modules
- **Heraldry-based world manipulation** — real-time effect on simulation via heraldry
- **ACTOR HUD** — standard mmorpg-style interface for ACTORs
- **Human/AGENT interface** — non-game, musical/symbolic world interaction

---

## Quantum Genesis Quick Reference

### Capacity Law (16-16-16)
```
Genesis Container  (Seal)
  └─ Mythos Container  (Crest)        ×16 max
       └─ Container  (Glyph|Device|Emblem)  ×16 max
            └─ Capsule  (Trait|Mark|Token|Sigil)  ×16 max
```
Max atomic entities per Genesis: 16³ = 4,096

### Heraldry
| Level | Type | Options |
|-------|------|---------|
| Genesis | Seal | Greater Seal, Lesser Seal |
| Mythos | Crest | Core, Atlas, Vault, Mythos, Codex, Loom, Composer, Forge, Order, Mind, Soul (+ ≤5 custom) |
| Container | Glyph / Device / Emblem | Glyph=composable, Device=standalone, Emblem=thematic |
| Capsule | Trait / Mark / Token / Sigil | Trait=semi-permanent, Mark=variable, Token=transactional, Sigil=unique binding |

### Lifecycle
```
Seeding → Active → Sealed → Archived → Deprecated
```
- Sealed: hierarchy frozen, payload updates still allowed
- Lesser Seal: grouping of up to 16 sealed Genesis Containers

### 16 Wire Types
```
DAT  CTL  AUD  NAR  TMP  AGT  VIS  SPA  BHV  SOC  ENR  IDN  EVT  AST  MET  LGC
```
DAT is the universal fallback. All others require exact type matching.

### Three-Way Alignment
Every entity must align on:
1. **Structural** — which container level it lives at
2. **Functional** — its ecosystem role (Engine / MajorSystem / Addon / Entity)
3. **Symbolic** — its heraldic type (Seal / Crest / Container / Capsule heraldry)

Misalignment is an architectural error — rejected by `validate_alignment()`.
