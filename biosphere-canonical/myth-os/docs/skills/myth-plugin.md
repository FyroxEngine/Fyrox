---
name: myth-plugin
description: The myth-os Plugin and Addon system — the extensibility layer that sits between the Quantum Vault and all instruments. Load this skill before building any plugin, addon, or core instrument. Defines what plugins and addons ARE, the trait surface, heraldry routing symbols (Crest/Glyph/Sigil), the closed-system principle, crate structure, and templates. TRIGGER: any time someone mentions plugins, addons, instruments, the plugin registry, or asks how to extend the system without touching the core.
---

# myth-os Plugin and Addon System

The plugin system is the extensibility layer of myth-os. It sits between
the Quantum Vault and all instruments. Everything above the Vault is either
a plugin or an addon — including the core instruments that ship with the program.

There is no special "built-in" category. Core instruments are plugins
that use compile-time ATOMs instead of runtime ATOMs. The architecture
is identical. The difference is a setting, not a hardcoded distinction.

---

## Mental Model

```
Quantum Vault  (raw bytes, MythId, BLAKE3 — knows nothing above it)
      │
      ▼
Plugin Registry  (myth-plugin crate — routes WirePackets to instruments)
      │
      ├── Instrument A  [CREST: Atlas]      ← core terrain instrument
      │       ├── Plugin A1  [GLYPH: ↑Atlas]  ← user-made erosion plugin
      │       └── Plugin A2  [GLYPH: ↑Atlas]  ← user-made cave plugin
      │               └── Addon X  [SIGIL]    ← water-fill addon on A2
      │
      └── Instrument B  [CREST: Composer]   ← core audio instrument
              └── Addon X  [SIGIL]          ← same addon, different host
```

Key points:
- Vaults are technically plugins to each other.
- Addons are cross-instrument — the same addon can attach to any instrument.
- The registry routes by WireType, not by name.

---

## Three Concepts

### Instrument (Core Plugin)
A built-in plugin shipped with myth-os. Composed from **compile-time ATOMs**.
There are 16 canonical instruments, one per Genesis module.
Each carries a **Crest** — the highest heraldry symbol.

The 16 Crests: Atlas · Mythos · Architect · Prism · Animus · Loom ·
Instinct · Order · Chronicle · Quill · Codex · Composer ·
Axiom · Continuum · Forge · Nexus

### Plugin
An external or user-created instrument. Composed from **runtime ATOMs**.
Attaches to a specific Vault as an instrument.
Carries a **Glyph** that inherits the Crest of the instrument it extends.
A terrain plugin's Glyph points upward to the Atlas Crest.

### Addon
A cross-instrument modifier. Attaches to **any** instrument, not one specific plugin.
Carries a **Sigil** — independent heraldry, no parent Crest required.
Addons hook into a plugin's output stream after `process()` runs.
The same addon binary can be attached to multiple instruments simultaneously.

---

## Heraldry as Routing

Heraldry in this system is **symbolic routing** — not decoration.
A Glyph tells the registry which instrument family a plugin belongs to.
A Sigil tells the registry an addon is self-contained and cross-compatible.

Heraldry also serves as access control at the Vault Portal:
an entity's heraldry symbol determines which Vaults it can enter.
Agents can carry forged or transformed heraldry — this is a simulation
mechanic, not a security flaw. See: Order of the Quantum Quill skill
for the full Portal access system. Do not redesign this independently.

---

## The Closed System Principle

Plugins and addons interact with the world **only through WirePackets**.
No plugin can directly mutate simulation state.
No addon can reach past the packet stream into the instrument internals.

This is intentional and non-negotiable. It means:
- A destructive agent cannot "delete the house" by calling a method.
- It must emit a packet that the simulation's physics processes and may refuse.
- Simulation integrity is guaranteed at the wire layer, not by trust.

```
              ┌────────────────────────────┐
              │      Simulation State       │
              │   (no direct access ever)   │
              └─────────────┬──────────────┘
                            │  WirePackets only
              ┌─────────────▼──────────────┐
              │    Plugin / Addon layer     │
              │  process(packet) → packets  │
              └────────────────────────────┘
```

---

## Trait Surface

### MythPlugin

```rust
pub trait MythPlugin: Send + Sync + 'static {
    // Identity
    fn id(&self)      -> &str;
    fn name(&self)    -> &str;
    fn version(&self) -> (u32, u32, u32);   // semver
    fn wire_in(&self)  -> &[WireType];      // packet types this plugin consumes
    fn wire_out(&self) -> &[WireType];      // packet types this plugin emits
    fn heraldry_symbol(&self) -> &str { "" }  // "Glyph:Symbol↑Crest" or "Sigil:Symbol"

    // Lifecycle
    fn on_attach(&mut self, vault: Arc<VaultRegistry>) -> PluginResult<()>;
    fn on_detach(&mut self) -> PluginResult<()>;

    // Work
    fn process(&mut self, packet: &WirePacket) -> PluginResult<Vec<WirePacket>>;

    // Optional tick — for plugins that need per-frame updates
    fn tick(&mut self, delta_ms: u64) -> PluginResult<Vec<WirePacket>> {
        let _ = delta_ms;
        Ok(vec![])
    }

    // Layout — declare which UI slots this plugin wants
    fn layout_request(&self) -> LayoutRequest {
        LayoutRequest::default()   // default = no UI slots (headless/background)
    }
}
```

### MythAddon

```rust
pub trait MythAddon: Send + Sync + 'static {
    fn id(&self)            -> &str;
    fn target_plugin(&self) -> &str;  // plugin id, or "*" for ALL plugins
    fn heraldry_symbol(&self) -> &str { "" }

    // Called AFTER the plugin processes a packet, before output is forwarded.
    fn on_output(
        &self,
        source_packet: &WirePacket,
        plugin_output: Vec<WirePacket>,
    ) -> PluginResult<Vec<WirePacket>>;

    // Called AFTER the plugin's tick(), if it emits anything.
    fn on_tick_output(
        &self,
        delta_ms: u64,
        tick_output: Vec<WirePacket>,
    ) -> PluginResult<Vec<WirePacket>> { Ok(tick_output) }
}
```

**Wildcard addon target:** Setting `target_plugin()` to `"*"` attaches the
addon to **every** registered plugin. `on_output` and `on_tick_output` will
run for every plugin's every packet. Use with care.

---

## Crate Structure

```
crates/myth-plugin/
    Cargo.toml          deps: myth-wire, myth-vault, thiserror, tracing, uuid
    src/
        lib.rs
        plugin.rs       MythPlugin trait
        addon.rs        MythAddon trait
        layout.rs       SlotType, Visibility, LayoutRequest, LayoutGrant, etc.
        registry.rs     PluginRegistry — routes packets, negotiates layout
        error.rs        PluginError, PluginResult<T>
```

### Dependency Rule
`myth-plugin` depends on `myth-wire` and `myth-vault` only.
No Bevy. No egui. No audio. No tokio.
Plugins that need rendering or audio take those deps themselves.

---

## Plugin Registry

```rust
pub struct PluginRegistry {
    plugins: Vec<(String, Box<dyn MythPlugin>)>,  // ordered, not HashMap
    addons:  HashMap<String, Vec<Box<dyn MythAddon>>>,
}
```

Registration order matters for routing priority. Core instruments register
first (at boot); external plugins register after.

### Two-Phase Routing (required by Rust borrow checker)

`route()` and `tick()` use a two-phase pattern to avoid borrow conflicts:

```
Phase 1 — mut borrow plugins:
  Collect Vec<(plugin_id, Vec<WirePacket>)> raw outputs.
  Release mut borrow.

Phase 2 — immut borrow addons:
  Run addons over collected outputs.
  Return all output packets.
```

Never collapse these into one loop — Rust will reject it because running addons
requires `&self.addons` while phase 1 holds `&mut self.plugins`.

### Layout Negotiation

After registering a plugin, call `negotiate_layout()` with the available slot
map from a loaded `UIGenesis`:

```rust
// Build slot map from UIGenesis
let slots: HashMap<String, (SlotType, Option<String>)> = ui
    .all_slots()
    .into_iter()
    .map(|s| (s.slot_id.clone(), (
        SlotType::from_str(&format!("{:?}", s.region)).unwrap(),
        s.occupant_heraldry,
    )))
    .collect();

let grant = registry.negotiate_layout("plugin-foundry", &slots)?;
// grant.granted — Vec<GrantedSlot> (slot_id, slot_type, label)
// grant.denied  — Vec<DeniedSlot>  (with occupant_heraldry and alternatives_offered)
```

The denial tells you exactly what's in your requested slot:
```
Plugin:   "I want CanvasLeft."
Registry: "Denied — Venturan is in slot_canvas_left.
           Alternatives available: FooterLeft, OverlayDrawer."
```

---

## Layout Types (myth-plugin/src/layout.rs)

```rust
pub enum SlotType {
    HeaderLeft, HeaderCenter, HeaderRight,
    CanvasToolbar, CanvasLeft, CanvasMain, CanvasRight, CanvasStatusBar,
    FooterLeft, FooterCenter, FooterRight,
    OverlayModal, OverlayDrawer, OverlayTooltip,
    Notification, ContextMenu,
}

pub enum Visibility { Always, OnDemand, Hidden }

pub struct SlotRequest {
    pub slot_type:         SlotType,
    pub label:             String,
    pub visibility:        Visibility,
    pub preferred_slot_id: Option<String>,
    pub preferred_width:   Option<f32>,
    pub preferred_height:  Option<f32>,
}

pub struct LayoutRequest { pub requests: Vec<SlotRequest> }
pub struct GrantedSlot   { pub slot_id: String, pub slot_type: SlotType, pub label: String }
pub struct DeniedSlot    { pub requested_type: SlotType, pub occupant_heraldry: Option<String>,
                           pub alternatives_offered: Vec<SlotType>, pub reason: String }
pub struct LayoutGrant   { pub granted: Vec<GrantedSlot>, pub denied: Vec<DeniedSlot> }
```

`SlotType` mirrors `UIGenesis::LayoutRegion` but as a typed Rust enum.
myth-qgcp stores regions as strings in capsule payloads (no circular dep).
myth-plugin parses strings → `SlotType` during negotiation.

---

## Plugin Foundry — Example Implementation

`examples/plugin-foundry/` is both an example of a complete plugin+addon and
a tool for generating more plugins. Study it before building your first plugin.

```
examples/plugin-foundry/
    src/
        spec.rs          PluginSpec data model (serialized as DAT WirePacket when forged)
        foundry.rs       FoundryPlugin — MythPlugin implementation
        heraldry_addon.rs HeraldryAddon — MythAddon wildcard "*" target
        ui.rs            egui panel structs (6 panels, pure data)
        app.rs           FoundryApp — eframe::App host
        main.rs          entry point
```

**FoundryPlugin heraldry:** `Glyph:Foundry↑Loom`
**HeraldryAddon heraldry:** `Sigil:Scribe`
**HeraldryAddon behavior:** Stamps `_heraldry` metadata into every JSON packet
payload that passes through any plugin. Non-JSON payloads pass through unchanged.

**Layout request from FoundryPlugin:**
```rust
fn layout_request(&self) -> LayoutRequest {
    LayoutRequest::new()
        .add(SlotRequest::new(SlotType::CanvasMain, "Plugin Foundry — Workspace"))
        .add(SlotRequest::new(SlotType::HeaderRight, "Plugin Foundry — Forge Icon")
            .with_size(32.0, 32.0)
            .on_demand())
}
```

Run it: `cargo run -p plugin-foundry`

---

## Templates

Two templates ship as workspace members so they compile and prove the traits work.
A template that does not compile is not a template.

```
templates/plugin-template/
    Cargo.toml          dep: myth-plugin
    src/lib.rs          minimal MythPlugin impl — copy, rename, build on it

templates/addon-template/
    Cargo.toml          dep: myth-plugin
    src/lib.rs          minimal MythAddon impl — copy, rename, build on it
```

### plugin-template/src/lib.rs

```rust
use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;

pub struct TemplatePlugin {
    vault: Option<Arc<VaultRegistry>>,
}

impl TemplatePlugin {
    pub fn new() -> Self { Self { vault: None } }
}

impl MythPlugin for TemplatePlugin {
    fn id(&self)      -> &str { "template-plugin" }
    fn name(&self)    -> &str { "Template Plugin" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn wire_in(&self)  -> &[WireType] { &[WireType::Data] }
    fn wire_out(&self) -> &[WireType] { &[WireType::Data] }

    fn on_attach(&mut self, vault: Arc<VaultRegistry>) -> PluginResult<()> {
        self.vault = Some(vault);
        Ok(())
    }

    fn on_detach(&mut self) -> PluginResult<()> {
        self.vault = None;
        Ok(())
    }

    fn process(&mut self, packet: &WirePacket) -> PluginResult<Vec<WirePacket>> {
        let _ = packet;
        Ok(vec![])
    }
}
```

### addon-template/src/lib.rs

```rust
use myth_plugin::{MythAddon, PluginResult};
use myth_wire::WirePacket;

pub struct TemplateAddon;

impl MythAddon for TemplateAddon {
    fn id(&self)            -> &str { "template-addon" }
    fn target_plugin(&self) -> &str { "template-plugin" }

    fn on_output(
        &self,
        _source: &WirePacket,
        output: Vec<WirePacket>,
    ) -> PluginResult<Vec<WirePacket>> {
        Ok(output)
    }
}
```

---

## What This Skill Does NOT Cover

- **ATOM internals** — ATOMs are the node-graph programming system that implements
  plugin logic. They have their own skill. The plugin trait is the external interface;
  ATOMs are the internal implementation mechanism.

- **Portal access control** — heraldry-based Vault access, the Glyph of Transformation
  mechanic, and agent deception are defined in the Order of the Quantum Quill skill.
  Do not redesign the access system from this skill.

- **Genesis Container loading** — how instruments load WorldGenesis capsules on boot
  is defined in the myth-qgcp skill (pending). The plugin `on_attach()` receives a
  VaultRegistry handle — what it does with that handle is its own concern.
