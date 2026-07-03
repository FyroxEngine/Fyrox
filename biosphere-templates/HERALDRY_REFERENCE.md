Quantum Genesis — Heraldry Reference
=====================================

CONTAINER HIERARCHY
  Genesis  (Level 1) — Seal
    Mythos  (Level 2) — Crest
      Container  (Level 3) — Glyph | Device | Emblem
        Capsule  (Level 4) — Trait | Mark | Token | Sigil

CAPACITY LAW
  Each level holds at most 16 children.
  Overflow requires splitting into a sibling, never exceeding.
  Max atomic entities per Genesis: 16^3 = 4,096

SEAL (Genesis level)
  Greater Seal — primary Genesis Container
  Lesser Seal  — grouping of up to 16 sealed Genesis Containers

CRESTS (Mythos level) — 11 known, up to 5 custom
  Core     Atlas    Vault    Mythos   Codex
  Loom     Composer Forge    Order    Mind    Soul

CONTAINER HERALDRY (Level 3)
  Glyph   — composable capability unit
  Device  — standalone functional unit
  Emblem  — thematic grouping

CAPSULE HERALDRY (Level 4)
  Trait   — semi-permanent measurable attribute
  Mark    — variable tag or category
  Token   — temporary transactional element
  Sigil   — semi-permanent unique personal binding (ACTOR identity)

LIFECYCLE
  Seeding -> Active -> Sealed -> Archived -> Deprecated
  Sealed: hierarchy frozen, payload updates still allowed

THREE-WAY ALIGNMENT
  Every entity must align on three dimensions simultaneously:
    Structural — container level (Genesis/Mythos/Container/Capsule)
    Functional — ecosystem role (Engine/MajorSystem/Addon/Entity)
    Symbolic   — heraldic type (Seal/Crest/Glyph.../Trait...)
  Misalignment is rejected by validate_alignment().
