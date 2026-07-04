# ADR-002: Actor Identity Model

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** BioSpark Studios / Quantum Genesis Engine  

---

## Context

Every Quill Actor currently carries **two distinct UUIDs**:

1. `QuillActor.myth_id: String` — generated at ECS spawn time by
   `entity_instantiator.rs`, stored as a component in the Bevy world.

2. `ActorSoul.id: MythId` — generated at soul initialisation time by
   `init_souls` in `instinct.rs`, stored in `SoulStore` (a Bevy Resource).

`SoulRef { soul_id: String }` bridges the ECS entity to its soul by holding
`soul.id.as_str()`. The `QuillActor.myth_id` is currently orphaned — it is
set at spawn but never used to look up anything.

This was identified during a Phase 4 review as design debt that must be
resolved before the Studio Scribe (Vault Atom 16) is built, because the
Scribe needs a single canonical ID to log against.

---

## Decision

### Canonical identity: `ActorSoul.id` (the soul ID)

The soul ID is the **eternal, portable identity** of an actor across:
- Vault migrations (`SoulMigration` records this ID)
- BLAKE3 departure fingerprints
- Social bonds (`SocialBond.source`, `.target` are soul IDs)
- Canon event archive entries
- Cross-world resonance navigation lookups

Rationale: the soul survives ECS entity death (e.g., actor is unloaded from a
Genesis instance but the soul persists in the SoulVault). The ECS entity is
ephemeral; the soul is not.

### ECS spawn alias: `QuillActor.myth_id` (the entity ID)

`myth_id` retains its current role as the ECS spawn-time identifier. It is
used for:
- Bevy `Name` component association (debug, inspector)
- Log lines in `entity_instantiator.rs` (`info!(id = %id, ...)`)
- Any future ECS system that needs to identify an entity before `init_souls`
  has run (i.e., in the first frame after spawn)

### Bridge: `SoulRef.soul_id`

`SoulRef` is the authoritative bridge between the ECS world and the SoulStore.
It is inserted by `init_souls` after soul creation and holds `soul.id.as_str()`.

```
ECS entity
  └─ QuillActor.myth_id  →  spawn-time alias (ephemeral)
  └─ SoulRef.soul_id     →  soul.id (eternal) → SoulStore lookup
```

### Studio Scribe logging rule

**The Scribe MUST log against `SoulRef.soul_id`, not `QuillActor.myth_id`.**

When logging an entity event, the Scribe system queries:
```rust
Query<(&SoulRef, &Transform, &ConsciousnessState), With<QuillActor>>
```
and uses `soul_ref.soul_id` as the entity identifier in all binary streams.

### Future: deprecate `QuillActor.myth_id`

Once the Scribe is implemented and the soul ID is established as the single
canonical key, `QuillActor.myth_id` should be removed from the component and
replaced with a `SoulRef` query wherever an entity ID is needed. This removes
the dual-identity ambiguity entirely.

Until that migration is complete, any system that needs "the ID of this actor"
MUST prefer `SoulRef.soul_id` over `QuillActor.myth_id`.

---

## Consequences

### Positive
- Single canonical identity for all cross-system actor references
- Scribe, Quill, migration logs, and social graph all use the same key
- No ambiguity when debugging "which ID should I look up?"

### Negative
- `QuillActor.myth_id` is currently unused beyond spawn logging — this is
  acknowledged debt. It will generate a dead_code warning once Rust's linter
  is run in strict mode. Suppress with `#[allow(dead_code)]` until migration.

### Not decided
- Whether `QuillActor.myth_id` and `ActorSoul.id` should be unified into a
  single UUID at spawn time (i.e., entity_instantiator creates the MythId and
  passes it into soul genesis). This would require `ActorSoul::genesis_with_id()`
  factory method. Deferred to Phase 6 when the Scribe is built and the cost
  of the dual-ID pattern becomes measurable.
