# ADR-004 — Architecture Decisions 2026-06-14

*Captured from design session. Skills to be updated in a batch later.*

---

## 1. Plugin / Addon / Instrument Model

- **Core Instruments** — built-in plugins. Same plugin architecture as external plugins, just baked in at compile time. Not a separate concept.
- **Plugins** — attach to a Vault as instruments. Composed from runtime ATOMs.
- **Addons** — attach to any instrument (not one specific plugin). Composed from runtime ATOMs. Cross-instrument by design.
- **Vaults** — are technically plugins to each other.
- Plugin system must exist BEFORE building any instrument. Everything downstream depends on it.
- New crate needed: `crates/myth-plugin` — trait definitions, registry, error types. Deps: myth-wire + myth-vault only.
- Templates needed: `templates/plugin-template/` and `templates/addon-template/` — must compile as workspace members.

## 2. ATOMs

ATOMs are the visual scripting building blocks of every module.

- **Every module has exactly 256 ATOMs.** This is a fixed law — same principle as the Law of 16 for Genesis Containers. The 256 are the complete vocabulary of what that instrument can do.
- Each ATOM is a small, discrete operation node with typed wire inputs and outputs. They look like nodes in a graph editor (and are rendered as such in Layer 4), but the canonical name is ATOM.
- **Two modes** (controlled by a setting, NOT hardcoded):
  - **Runtime ATOMs** — what plugins are composed from. Hot-loadable.
  - **Compile-time ATOMs** — what core modules are built from. Baked in.
- ATOMs are a **closed, sandboxed system**. No agent or human can reach directly into the simulation directly — wires are the only interface. This prevents "delete the house" direct mutation.
- **LLM ATOMs are first-class.** There are dedicated ATOMs for each inference backend: `OllamaInference` (local-first, always preferred), `ClaudeInference`, `GeminiInference`, and others. The Capsule going in carries the prompt/context. The Capsule coming out is typed to whatever the downstream needs — NarrativeArc, ShaderFragment, DialogueLine, etc. By routing LLM calls through ATOMs, the user controls exactly *when* and *where* inference happens in the graph.
- Humans build ATOM graphs visually (Layer 4 double-click into an instrument node). Agents can also compose ATOM graphs dynamically.
- A Plugin is a named, heraldry-stamped package of ATOMs. Core modules = same, compile-time ATOMs.

## 2b. MOLECULEs

A MOLECULE is a named, saved ATOM sub-graph — a reusable preset combination.

- MOLECULEs are the "remix presets". A Tornado Molecule might wire 4 ATOMs from myth-continuum to produce a specific fluid behavior. A FactionalWar Molecule might chain ATOMs across myth-order + myth-chronicle.
- OPAL can generate MOLECULEs by researching a domain and producing valid ATOM wiring specs, since it knows the 256-ATOM vocabulary of each module.
- Users can save, share, sell, and load MOLECULEs — this is the primary creative economy unit.
- Agents can *grow* new MOLECULEs by discovering valid ATOM combinations — analogous to chemical reactions in the chemistry system.
- The "DJ remix" model: modules = tracks, ATOMs = effects/filters, MOLECULEs = saved presets, CAPSULEs = the signal. Users and agents are both at the board simultaneously.

## 3. CAPSULEs

- CAPSULEs are always runtime, always data.
- They are what instruments read and operate on.
- Distinct from ATOMs: ATOMs = code units, CAPSULEs = data payloads.
- All CAPSULEs live in the QGCP hierarchy (Genesis → Mythos → Container → Capsule).

## 4. Heraldry

Heraldry is a **universal symbol routing system**. It is NOT only a rank hierarchy. It is multi-purpose:

- **Rank** — can express hierarchy/power (like nobility)
- **Color vector** — faction identity, like a kingdom's flag colors
- **Access control** — Vault Portal conditions check heraldry. An agent must present valid heraldry to enter.
- **Deception mechanic** — agents can carry a "Glyph of Transformation" (or equivalent) to impersonate heraldry and bypass Portal conditions. This is a simulation gameplay mechanic, not a security flaw.
- Natural human understanding: people recognize "that is a powerful group" or "that person belongs there" from symbols. That is the design intent.
- Full system defined in `/order-of-the-quantum-quill` skill — **do not redesign this independently**.
- Heraldry types referenced so far: Crest (core instruments), Glyph (plugins, inherits parent Crest), Sigil (addons, cross-instrument). More types exist in the Order skill.

## 5. World File Formats

Two formats. Likely a **directory structure** rather than two competing file types:

```
worlds/
    kasmir-delta/
        kasmir-delta.qgenesis    ← headless world state
        kasmir-delta.qgcp        ← media/asset bundle
```

- **`.qgenesis`** — pure world state output from world creation. No media. No samples. No images.
  - Used by: headless server, client-side agents.
  - Both server AND client agents need headless world state to reason about.
- **`.qgcp`** — media bundle. Audio samples, images, GLB refs, textures, anything binary.
  - Used by: client Theater/renderer adapters only.
  - Server never loads this.
  - Client agents also don't load this — only rendering adapters do.

Current state: `myth-qgcp` crate defines `GenesisContainer` which is effectively the headless `.qgenesis` format. A separate media bundle struct is needed. Crate may need restructuring or splitting.

`myth-quill` also has a parallel `GenesisContainer` — need to reconcile these two. `myth-qgcp` should be the canonical format spec; `myth-quill` should consume it.

## 6. Skills To Update (batch later)

- `biospark-theater.md` — update with plugin system context
- `myth-os-architecture.md` — update with ATOM/Capsule distinction, file format split
- `biospark-skill-registry.skill` — add plugin/addon entries when crate exists
- `/order-of-the-quantum-quill` — do not touch until the user provides it; heraldry is defined there
- Any skill referencing "ATOMs as Genesis Container building blocks" — this framing is wrong and needs correction

## 7. Genesis Container → Instrument Load Flow (draft)

Not yet finalized. Rough sequence:

1. World Architect (in-project, replaces OPAL) authors a world → outputs `.qgenesis` + `.qgcp`
2. Simulation boot: Myth Core loads `.qgenesis` from Quantum Vault by bDNA
3. For each of the 16 MythosModules in the container, the matching Instrument (by heraldry Crest) loads its Capsules
4. Instrument initializes state from Capsule data, begins emitting WirePackets
5. Plugins extend Instrument processing via ATOM graphs
6. Addons hook into output stream
7. Theater routes WirePackets to adapters
8. Client adapters load `.qgcp` for media references when rendering

## 8. Vault Profiles — Scoped Plugin Loading

Vaults are Studio spaces for a specific type of project. A 3D Genesis Vault is a
world-building sim. An Audio Studio Vault is a composition environment. They should
not share the same plugin set — each Vault declares a **VaultProfile** at creation
time that gates which plugins it will ever load.

**VaultProfile maps to Departments:**

| Profile            | Accepted Departments |
|--------------------|----------------------|
| `GenesisWorld`     | WorldConstruction, EntitySystems |
| `NarrativeStudio`  | NarrativeSystems, EntitySystems |
| `AudioStudio`      | NarrativeSystems (Composer), PipelineSystems |
| `FilmEdit`         | NarrativeSystems, PipelineSystems |
| `Custom(vec)`      | User-defined mix of departments |

**Rules:**
- Profile is stored in Vault metadata at creation. Immutable after creation (changing it
  would invalidate loaded plugin state).
- `myth-plugin-registry` filters the certified manifest by profile when a Vault queries
  for available plugins. The Vault never sees plugins outside its profile.
- Pipeline/utility plugins (`myth-axiom`, `myth-continuum`) can be tagged
  `Department::Universal` — accepted by all profiles.
- Plugin install attempt to wrong profile = clear rejection with reason, not a silent fail.
- Same plugin can appear in multiple profiles if its Department qualifies — no duplication,
  no separate installs.

**Why this is the right separation:**
- Plugin Foundry is attached to myth-os (not to a Vault)
- Registry is attached to myth-os (not to a Vault)  
- Vaults are consumers: they query the Registry filtered by their profile
- Infinite loops / bad plugins in one Vault cannot affect another Vault's session

## 9. Plugin Loading Architecture (Runtime)

Two plugin tiers:

| Tier | Format | When loaded | Sandboxed | Who certifies |
|------|--------|-------------|-----------|---------------|
| Core instruments | Compiled Rust | App startup | No (trusted) | N/A — ships with myth-os |
| User plugins | `.wasm` file | On demand / Vault boot | Yes — Wasmtime, no capabilities | myth-plugin-registry |

Plugin Foundry flow:
1. User builds plugin → Plugin Foundry compiles to `.wasm`
2. Foundry submits to `myth-plugin-registry` (NOT to a Vault)
3. Registry validates WASM magic, hashes bytes, stamps Heraldry Glyph, writes to manifest
4. Vault queries Registry at boot: "certified plugins matching my VaultProfile"
5. `myth-wasm-host` loads each `.wasm` into a sandboxed Wasmtime Store
6. Host wraps it as `Box<dyn MythPlugin>` — rest of myth-os treats it like a core instrument

Vault never touches raw WASM. Foundry never touches live world state.
Registry is the clean boundary between the two environments.

## 10. Open Questions

- Exact ATOM node graph runtime: how are ATOM graphs evaluated? Interpreted at runtime, compiled to Wasm, or transpiled to Rust?
- Chemistry crate integration: how do chemical properties gate which ATOMs an agent can access or combine?
- `myth-qgcp` vs `myth-quill` GenesisContainer reconciliation: which is canonical?
- World Architect tool: new crate (`myth-architect`) or part of `myth-stencil`?
- Vault Portal access control: defined in Order of the Quantum Quill — need to read that skill before designing plugin registry security.
