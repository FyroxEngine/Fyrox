# QuillOS Naming Conventions

Three rules. No exceptions.

---

## Rule 1 — Crate names use function words

Crate names describe what the crate **does**, not what it is called in the lore.

| Crate | Function |
|-------|----------|
| `quantum-core` | Headless game server / clock supervisor |
| `quantum-vault` | UI application (Quantum Vault product) |
| `vault` | VaultRegistry storage engine |
| `QuillOS` | Core Rust types shared across all crates |
| `biospark-theatre` | Theatre compositor, channel model, layout blueprints |

**Wrong:** `great-library`, `xyrona`, `order-of-the-quill`  
**Right:** `quantum-vault`, `QuillOS-core`, `vault`

---

## Rule 2 — Rust types and field names use generic nouns

`struct`, `enum`, `fn`, field names, and constant names must be plain English technical terms. No lore inside identifiers.

| Wrong | Right |
|-------|-------|
| `XyronaPlayer` | `AudioPlayer` |
| `GreatLibraryState` | `AppState` |
| `OrderOfQuillPlugin` | `NarrativePlugin` |
| `VaultType::Xyrona` | `VaultType::Audio` |
| `id: "composer.xyrona"` | `id: "composer.player"` |

This applies to: struct names, enum variants, function names, field names, module names, and plugin `id` strings.

---

## Rule 3 — Display strings are the only lore zone

`.name`, `.description`, `.title()`, `.tagline()`, window titles, UI labels — these are the **only** places lore belongs.

```rust
// CORRECT
PluginDef {
    id:          "composer.player",   // ← generic, stable, no lore
    name:        "Xyrona Player",     // ← lore lives here, display only
    description: "Music of the Void. Resonance from the first signal.",
    module:      QuantumModule::Composer,
}

// WRONG
PluginDef {
    id:   "xyrona.player",   // ← lore in id — breaks cross-vault references
    name: "Xyrona Player",
    ...
}
```

Display strings can change freely — they are rendered text, not code paths.  
`id` strings are **permanent keys** and may never contain lore.

---

## Vault addressing

Vaults follow a powers-of-2 hierarchy mirroring Genesis Containers:

```
Tier 0 — Master Vault  (1 slot)
Tier 1 — 2 vaults
Tier 2 — 4 vaults
Tier 3 — 8 vaults
Tier 4 — 16 instrument slots
```

Address format: `V:0.1.3` (tier.index.subindex)  
B-DNA fingerprint is the stable identity; the display name is just lore.

---

## Product naming (for context only — not code)

| Code name | Product name |
|-----------|--------------|
| `QuillOS-core` binary | **Quantum Core** — headless game server |
| `quantum-vault` binary | **Quantum Vault** — dimensional project manager |
| `biospark-theatre` crate | **Vault Renderer** — multi-layer composite canvas |
| All together | **QuilloS** — the Quantum OS |

The Great Library is internal order lore. It is used in marketing capaings only.  Not in the Code Base.

---

## Lore drift detection

If you see lore in a Rust identifier (type name, field name, function name, module name, or plugin `id`), that is drift. Fix it:

1. Rename the Rust identifier to a plain technical term.
2. Move the lore string into the nearest `.name` or `.description` field.
3. Run `/conventions` (the project skill) to audit before opening a PR.
