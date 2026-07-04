# Module Spec + Plugin Contract: {{MODULE_NAME}}

> **This document serves two purposes simultaneously:**
> 1. **OPAL input spec** — fill every section with domain research before handing off to implementation.
> 2. **Plugin contract** — every section maps directly to Rust code. When this doc is complete, the crate writes itself.
>
> OPAL fills sections 1–9. Implementation reads sections 1–9 and writes the crate.
> Do not skip sections — write "none" or "N/A" if genuinely empty.
> Wire types must be chosen from the canonical 17: DAT CTL AUD NAR TMP AGT VIS SPA BHV SOC ENR IDN EVT AST MET LGC RES

---

## 1. Identity
*→ Maps to: `Cargo.toml` package name, `types.rs` constants, heraldry_symbol()*

| Field          | Value |
|----------------|-------|
| Crate name     | `myth-{{slug}}` |
| Crest symbol   | (3–5 char abbreviation, e.g. ATL, MYT, PRM) |
| Color hex      | (pick one that fits the domain) |
| Law / domain   | (e.g. Space, Time, Life, Order, Sound…) |
| Department     | WorldConstruction / EntitySystems / NarrativeSystems / PipelineSystems |
| Heraldry       | `Crest:{{Symbol}}` (core modules) or `Glyph:{{Symbol}}↑{{ParentCrest}}` (plugins) |

---

## 2. What This Module Does
*→ Maps to: crate-level doc comment in `lib.rs`*

One paragraph. Plain language. What does it simulate or process?
What is it responsible for that no other module handles?
What would break in the world if this module were missing?

---

## 3. Wire Inputs
*→ Maps to: `wire_in()` return value and `process()` match arms in `plugin.rs`*

| WireType | What it carries for this module | Required? |
|----------|---------------------------------|-----------|
| `SPA`    | (e.g. chunk coords to know which terrain to read) | Yes |
| …        | …                               | …         |

---

## 4. Wire Outputs
*→ Maps to: `wire_out()` return value and output structs in `types.rs`*

| WireType | Struct name      | Key fields |
|----------|------------------|------------|
| `SPA`    | `TerrainChunk`   | chunk_x, chunk_z, heightmap, biome_map, moisture |
| …        | …                | … |

---

## 5. Config Struct
*→ Maps to: `{{Name}}Config` in `types.rs` and Layer 2 Euro Rack knobs/faders*

| Field | Type | Default | Range / notes |
|-------|------|---------|---------------|
| …     | …    | …       | … |

---

## 6. ATOM Registry
*→ Maps to: `{{slug}}_atom_spec()` function in `types.rs`*

**Every module has exactly 256 ATOMs.** Fixed law. 16 containers × 16 ATOMs = 256.
ATOMs are named as verb-noun operations. "ThermalErosionSimulator" not "Thermal".
Include LLM ATOMs where inference adds value.

**LLM ATOM names (local-first order):**
`OllamaInference` → `ClaudeInference` → `GeminiInference` → `OpenAIInference`

For each container:

### Container N — {{Name}} (symbol: XXX)
Layer: (pick one appropriate to this domain)
Primary wire_out: `SPA` / `DAT` / etc.

| # | ATOM name | Symbol | wire_out | Notes |
|---|-----------|--------|----------|-------|
| 1 | …         | XXX    | `SPA`    | …     |

*(repeat for all 16 containers)*

---

## 7. Cross-Module Relationships
*→ Maps to: wire routing in myth-theater, integration tests*

| Direction  | Other module      | What crosses |
|------------|-------------------|--------------|
| Receives   | `myth-mythos`     | WeatherState → adjusts biome moisture |
| Sends to   | `myth-architect`  | SpawnPoint → place structures there |
| …          | …                 | … |

---

## 8. Molecule Suggestions
*→ Maps to: pre-built ATOM sub-graphs in `molecules/` directory*

List 3–5 MOLECULEs OPAL thinks are worth pre-building for this module.
A MOLECULE is a named, saved ATOM sub-graph — a reusable preset combination.

| Molecule name     | ATOMs involved | What it does |
|-------------------|----------------|--------------|
| `ErosionPreset`   | ThermalErosionSimulator + HydraulicWearModeler + ValleySmoothingAlgorithm | Standard erosion pass |
| `OllamaInferenceRouter` | OllamaInference → ClaudeInference → GeminiInference | Local-first LLM routing |
| …                 | …              | … |

---

## 9. External Data / Hardware
*→ Maps to: `myth-nexus` ExternalSource entries*

| Source type     | What it provides | ExternalSourceType |
|-----------------|------------------|--------------------|
| Traktor S4 NRPN | Height scale (NRPN 101/102), flora density (103) | `Midi` |
| …               | …                | … |

---

## 10. Shader / Asset Needs
*→ Maps to: `assets/shaders/` files and `adapters/myth-{{slug}}-bevy/` crate*

- Shader: `{{slug}}-main.vert.glsl` — uniforms: …
- Shader: `{{slug}}-main.frag.glsl` — biome/material channels: …
- Splatmap WGSL: channels R=… G=… B=… A=…
- GLB asset categories needed: …

If none: N/A

---

## 11. Crate Dependencies
*→ Maps directly to `[dependencies]` in `Cargo.toml`*

Always include: `myth-wire`, `myth-qgcp`, `myth-plugin`, `myth-vault`, `serde`, `serde_json`

Additional deps this module needs:
- `{{dep}}` — reason

---

## 12. Open Questions

Things OPAL could not resolve. A human decision is needed before implementation.

1. …
2. …
