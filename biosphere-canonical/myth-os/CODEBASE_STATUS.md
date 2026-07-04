# Codebase Status
> Last updated: 2026-06-21. UPDATE THIS FILE at the end of every session.
> This is the authoritative source for what is real, what is stubbed, and what is dead.
> Read this FIRST before touching anything. Do not create duplicates of canonical crates.

---

## CANONICAL CRATES — These are the real ones. Do not duplicate.

| Crate | Path | Status | Notes |
|-------|------|--------|-------|
| `myth-wire` | `crates/myth-wire` | ✅ LIVE | WireType, MythId, WirePacket, **Sigil** (new). Foundation of everything. |
| `myth-vault` | `crates/myth-vault` | ✅ LIVE | Content-addressable storage. VaultProfile added 2026-06-15. |
| `myth-qgcp` | `crates/myth-qgcp` | ✅ LIVE | Genesis Container hierarchy. **Cell, ActorContainer** added 2026-06-21. ActorGenesis now has `actor_containers`. |
| `myth-plugin` | `crates/myth-plugin` | ✅ LIVE | MythPlugin trait, MythAddon trait. |
| `myth-plugin-registry` | `crates/myth-plugin-registry` | ✅ LIVE | Certifies .wasm plugins, stamps heraldry glyphs. |
| `myth-wasm-host` | `crates/myth-wasm-host` | 🔶 STUB | Wasmtime ABI written, transport not wired yet. |
| `myth-vault-mcp` | `bins/myth-vault-mcp` | ✅ LIVE | MCP server, confirmed working 2026-06-15. vault_ingest_path needs rebuild after next restart. |
| `myth-controls` | `crates/myth-controls` | ✅ LIVE | **NEW 2026-06-21.** ControlDef, ControlType, TaperCurve. ATOM panel widget definitions. |

## MODULE CRATES — All 16 instruments. Types real, logic stubbed.

All live at `modules/myth-<name>/`. All compile. All have real types in `types.rs`.
`plugin.rs` `process()` returns empty vec in all of them — no logic yet.

| Module | Department | Types done | Logic done |
|--------|-----------|------------|------------|
| `myth-atlas` | WorldConstruction | ✅ Full (256 ATOMs + `control: Option<ControlDef>` on SubModuleSpec, sim params, NRPN) | ❌ Stub |
| `myth-mythos` | WorldConstruction | ✅ Basic | ❌ Stub |
| `myth-architect` | WorldConstruction | ✅ Basic | ❌ Stub |
| `myth-prism` | WorldConstruction | ✅ Basic | ❌ Stub |
| `myth-animus` | EntitySystems | ✅ Basic | ❌ Stub |
| `myth-loom` | EntitySystems | ✅ Basic | ❌ Stub |
| `myth-instinct` | EntitySystems | ✅ Basic | ❌ Stub |
| `myth-order` | EntitySystems | ✅ Basic | ❌ Stub |
| `myth-chronicle` | NarrativeSystems | ✅ Basic | ❌ Stub |
| `myth-quill` | NarrativeSystems | ✅ Basic | ❌ Stub |
| `myth-codex` | NarrativeSystems | ✅ Basic | ❌ Stub |
| `myth-composer` | NarrativeSystems | ✅ Basic | ❌ Stub |
| `myth-axiom` | PipelineSystems/Universal | ✅ Basic | ❌ Stub |
| `myth-continuum` | PipelineSystems | ✅ Basic | ❌ Stub |
| `myth-forge` | PipelineSystems | ✅ Basic | ❌ Stub |
| `myth-nexus` | PipelineSystems | ✅ Basic | ❌ Stub |

## ADAPTERS

| Adapter | Path | Status |
|---------|------|--------|
| `myth-atlas-bevy` | `adapters/myth-atlas-bevy` | 🔶 STUB | Terrain mesh grid exists, no data flows into it yet. Bevy wiring pending. |

## MOLECULES

| Molecule | Path | Status |
|----------|------|--------|
| `myth-molecule-inference-router` | `molecules/inference-router` | 🔶 STUB | All 4 LLM backends return stub errors. Transport not wired. |

## ASSETS

| Asset | Path | Status |
|-------|------|--------|
| `atlas-terrain.vert.glsl` | `assets/shaders/` | ✅ Written | fBm simplex noise, gravity/precip uniforms |
| `atlas-terrain.frag.glsl` | `assets/shaders/` | ✅ Written | Biome colour ramp |
| `atlas-terrain.wgsl` | `assets/shaders/` | ✅ Written | Bevy splatmap shader |
| Master Vault | `master-vault/` | ✅ LIVE | Empty. MCP server confirmed. Ready for assets. |

---

## LEGACY / DEAD — Do not build on these. Do not duplicate them.

These exist from earlier sessions or experiments. They are NOT canonical.
If something here overlaps with a canonical crate above, the canonical one wins.

| Path | Why it exists | What to do |
|------|--------------|------------|
| `vault/` | Earlier vault attempt | DEAD — `crates/myth-vault` is canonical |
| `mythos/` | Earlier module attempt | DEAD — `modules/myth-*` are canonical |
| `core/` | Earlier core attempt | DEAD — `crates/myth-*` are canonical |
| `genesis/` | Earlier genesis attempt | DEAD — `crates/myth-qgcp` is canonical |
| `library/` | Unknown | INVESTIGATE before touching |
| `crates/myth-core` | Duplicate of `core/` | DEAD |
| `crates/myth-daw` | Audio DAW experiment | PARKED — revisit when myth-composer has logic |
| `crates/myth-stencil` | UI windowing experiment | PARKED — Plugin Foundry will supersede |
| `crates/myth-controller` | egui rack UI | PARKED — Layer 1 DJ controller, not started properly yet |
| `crates/biospark-theatre` | Theater system pure Rust | PARKED — real but not wired to anything yet |
| `crates/biospark-theatre-bevy` | Theater Bevy renderer | PARKED — real but not wired |
| `bins/opalis` | Node graph app (older) | INVESTIGATE — may have real node graph code worth salvaging |
| `viewer/` | OBJ viewer with orbit cam | INVESTIGATE — may actually render, could be quick win |
| `void-sculptor/` | Marching cubes | PARKED — useful later for terrain |
| `asset-forge/` | Asset pipeline tool | PARKED |
| `genesis-collider-mandelbulb/` | Mandelbulb experiment | PARKED |
| `services/myth-clock/` | Tick clock service | PARKED — needed eventually for simulation loop |

---

## PLANNED CRATES (Phase 2 / 3 — not yet created)

| Crate | Path | Role |
|-------|------|------|
| `myth-fyrox` | `adapters/myth-fyrox` | Fyrox bridge — WirePacket ↔ Fyrox scene/ECS |
| `myth-instrument-vault` | `bins/myth-instrument-vault` | Fyrox app — 4-layer UI (Reason → VCV → Actor Node → Actor Internals) |

## WHAT TO BUILD NEXT (priority order)

1. **Phase 1 complete** — myth-controls, Sigil, Cell, ActorContainer, SubModuleSpec.control all done (2026-06-21).
2. **Phase 2: myth-fyrox adapter** — `adapters/myth-fyrox/` bridge crate then `bins/myth-instrument-vault/` Fyrox app with 4-layer UI.
3. **Phase 3: Core Server actor loop** — activate `services/myth-clock`, wire actor simulation.
4. **Investigate `viewer/`** — may already render GLBs, cheapest path to something visible.
5. **Wire biospark-theatre** — Theater is written, just not connected.

---

## RULES FOR FUTURE SESSIONS

1. **Read this file first.** Before writing any code.
2. **Update this file last.** Before ending any session.
3. **Never create a new crate** without checking this file for duplicates first.
4. **If you find a crate not listed here**, add it to LEGACY or CANONICAL before touching it.
5. **Stubs are not done.** A crate that compiles with empty `process()` is scaffolding, not a feature.
6. **The Atlas whitepaper data is in** `modules/myth-atlas/src/types.rs` — all 256 ATOMs, sim params, NRPN mappings. That work is done and real.
