# ADR-001: B-DNA Determinism Rules

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** BioSpark Studios / Quantum Genesis Engine  

---

## Context

B-DNA is the 64-position boolean genome that defines every Quill Actor's identity,
trait weights, heraldric sigil, and resonance frequency. It is the foundation of:

- Procedural personality generation (SubconsciousMind trait weights)
- Heraldric position assignment (sigil derived from bits 0–4)
- Soul fingerprinting (16-char hex via `bdna_to_hex()`)
- Inheritance (child souls XOR parents with an environmental mask)
- Resonance Hz assignment (`bdna_resonance()` → Hz calculation)

Because B-DNA feeds into BLAKE3 soul migration fingerprints and canonical event
stamps, ANY change to its derivation algorithm is a **breaking change** that
invalidates all existing souls, migration logs, and canon archives.

---

## Decision

### 1. Storage type

`BDna` is defined as `Vec<bool>` with an invariant of **exactly 64 elements**.
Array type `[bool; 64]` was rejected because serde only supports fixed arrays
up to size 32. The Vec<bool> representation is identical in behavior.

**Invariant:** Every function that produces a `BDna` must return a Vec of length 64.
This is enforced by schema validation tests (see `mythos/src/soul.rs` test suite).

### 2. First-generation derivation — `bdna_from_seed(seed: u64) -> BDna`

Algorithm: inline LCG (Knuth multiplicative) stepped once per 64-bit word,
with each of the 64 bits extracted via `(s >> (i % 64)) & 1`.

**LCG constants (immutable — changing these breaks all existing souls):**
```
multiplier = 6_364_136_223_846_793_005
increment  = 1_442_695_040_888_963_407
```

These are the standard Knuth MMIX constants. They must never be changed.

### 3. Inheritance — `inherit_bdna(parent_a, parent_b, env_seed) -> BDna`

```
child[i] = (parent_a[i] XOR parent_b[i]) XOR env_mask[i]
```

Where `env_mask = bdna_from_seed(env_seed)`.  
If `parent_b` is `None`, it is treated as all-false (single parent).

### 4. Hex fingerprint — `bdna_to_hex(dna) -> String`

Pack 64 bits into 8 bytes (8 bits per byte, LSB first), then format as 16 lowercase
hex characters. **Always exactly 16 characters.**

### 5. Resonance value — `bdna_resonance(dna) -> u64`

Fold the 64 bits into a u64 via `v |= 1u64 << (i % 64)` for each true bit.
Used as the raw seed for a soul's `resonance_hz` assignment.

### 6. Genotype → seed contribution (Phase 4 addition)

When spawning an actor with an explicit `Genotype`, the genotype values are folded
into the position-based seed before soul creation:

```
seed ^= (curiosity.to_bits() as u64).wrapping_mul(0x9e3779b97f4a7c15)
seed ^= (aggression.to_bits() as u64).wrapping_mul(0x517cc1b727220a95)
```

The Fibonacci-prime multipliers (`0x9e3779b97f4a7c15` is the 64-bit golden ratio
constant) ensure good avalanche — a small change in curiosity produces a radically
different B-DNA. These constants may be extended with additional genotype axes in
future phases but **existing constants must not change**.

---

## Consequences

### Positive
- Fully reproducible: given the same seed, the same actor is always regenerated
- No external randomness — deterministic for testing and archival replay
- The BLAKE3 migration fingerprint reliably catches any B-DNA tampering

### Negative
- LCG constants are frozen. Any bug discovered in the LCG algorithm requires a
  versioned migration (new `bdna_version: u8` field on `ActorSoul`) rather than
  a silent fix.
- `Vec<bool>` is less memory-efficient than a bitfield, but this is acceptable
  given soul counts remain below the 1000-soul performance budget (see ADR-003).

### Migration path for algorithm changes
If the LCG constants must change (e.g., proven statistical weakness):
1. Increment `bdna_version` on `ActorSoul` (add the field, default = 1)
2. Implement `bdna_from_seed_v2()` alongside the original
3. Existing souls retain v1 fingerprints; new souls use v2
4. A one-time migration tool can optionally re-derive v1 souls to v2 with a
   recorded `MigrationReason::AlgorithmUpgrade`
