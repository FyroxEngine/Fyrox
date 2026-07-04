# ADR-003: Performance Budget

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** BioSpark Studios / Quantum Genesis Engine  

---

## Context

The architectural target is a **1ms master event/render clock** processing
real-time audio, visuals, and hardware inputs simultaneously. This budget
must survive across all Genesis systems running concurrently in a single
Bevy `Update` schedule tick.

Without explicit limits, the following systems degrade non-linearly:
- `tick_souls` — O(n) over SoulStore size
- `wander_system` — O(n) over entities with WanderBrain
- `validate_room_adjacency` — O(n²) over placed tile count (runs once at PostStartup)
- `scan_dir` in AssetRegistry — O(files) at startup, not per-frame
- MIDI event dispatch — O(bindings × events) per frame

---

## Decision

### Per-frame system budgets (target, not enforced at runtime yet)

| System | Max entities/items | Target time |
|---|---|---|
| `tick_souls` | 1 000 souls | < 0.3 ms |
| `wander_system` | 1 000 actors | < 0.2 ms |
| `init_souls` (first-frame only) | 1 000 actors | < 5 ms total |
| `sync_midi_to_rack` | 128 CC events/frame | < 0.05 ms |
| `draw_rack` (egui) | 16 modules | < 1.0 ms |
| Studio Scribe write (Phase 5+) | all entities | < 0.2 ms |

**Hard limit: no single Update system may block for > 2ms.**  
Systems that exceed this must be moved to a Tokio async task or a
`FixedUpdate` schedule with a longer interval.

### Startup budgets (one-time, not per-frame)

| Operation | Limit |
|---|---|
| `scan_dir` (AssetRegistry) | < 500 ms, < 10 000 files |
| `ensure_module_manifests` | < 100 ms |
| `spawn_room_chain` | < 16 rooms default, < 64 rooms max |
| `validate_room_adjacency` | < 64 rooms (runs PostStartup) |
| `build_heightmap` | 100×100 grid (10 201 vertices) — fixed |

### SoulStore hard limits

| Metric | Limit | Rationale |
|---|---|---|
| Souls per Genesis instance | 1 000 | Keeps tick_souls < 0.3ms |
| Souls per vault | 10 000 | SoulStore is in-memory HashMap |
| B-DNA size | 64 bits (Vec<bool> len == 64) | Fixed by ADR-001 |
| frequency_memory entries per soul | 256 | Hz keys are u32; 256 covers the meaningful audible range |
| SocialBond count per SocialGraph | 10 000 | O(n) bond lookup in `bond_between` |

### AssetRegistry limits

| Metric | Limit |
|---|---|
| Total manifests scanned | 10 000 |
| Manifests loaded into memory | 10 000 |
| Room chain length (RoomChain.room_count default) | 6 |
| Room chain length (hard max) | 64 |

### Memory targets (process-wide, not enforced yet)

| Component | Target RSS |
|---|---|
| Genesis (no assets loaded) | < 200 MB |
| Genesis (full room chain, 1 000 souls) | < 512 MB |
| Library | < 128 MB |
| core-supervisor | < 32 MB |

---

## Enforcement plan

Phase 5: Add `tracing::info!` timing spans to `tick_souls` and `wander_system`
using `std::time::Instant` so per-frame cost is visible in logs during dev.

Phase 6 (Studio Scribe): The Scribe's frame-end system writes timing metadata
alongside entity events, giving a continuous performance record in the binary
stream. The Quantum Quill can flag frames that exceed budget as low-coherence
events.

Phase 8+: If SoulStore exceeds 500 souls, `tick_souls` moves to `FixedUpdate`
at 10Hz (100ms interval) rather than every frame. Soul tick results are cached
in `ConsciousnessState` for the renderer to read at full framerate.

---

## Consequences

### Positive
- Clear go/no-go thresholds during development
- Scribe timing data gives an objective perf record over time
- `validate_room_adjacency` O(n²) is acceptable at ≤64 rooms (4096 comparisons)
  and only runs once at PostStartup — not a per-frame concern

### Negative
- 1 000-soul limit per Genesis instance may feel low for a "living world."
  Mitigation: multiple Genesis instances on separate ports can host different
  world zones, each under the limit. Cross-instance soul migration uses the
  `SoulMigration` protocol (ADR-002, Phase 9 transport layer).

### Open questions
- The 1ms master clock target assumes a dedicated machine. On shared hardware
  the budget should be relaxed to 4ms. A `GENESIS_TICK_BUDGET_MS` environment
  variable will be added in Phase 5 to allow runtime tuning.
