// plugin-foundry — Example myth-os plugin + addon, and a tool for creating more.
//
// This crate serves two purposes:
//   1. It IS an example of a complete MythPlugin + MythAddon implementation.
//   2. It IS a tool — fill in the panels and forge a new plugin scaffold.
//
// STRUCTURE:
//   spec.rs          — PluginSpec data model (serialized to DAT WirePacket)
//   foundry.rs       — FoundryPlugin (MythPlugin implementation)
//   heraldry_addon.rs — HeraldryAddon (MythAddon, wildcard "*" target)
//   ui.rs            — egui panel structs (pure data, no renderer coupling)
//   app.rs           — FoundryApp (eframe::App host, owns registry + UI state)
//
// HERALDRY:
//   Plugin:  Glyph:Foundry↑Loom
//   Addon:   Sigil:Scribe
//
// LAYOUT SLOTS REQUESTED:
//   CanvasMain  — primary 6-panel workspace
//   HeaderRight — 32×32 forge icon (OnDemand)

pub mod app;
pub mod foundry;
pub mod heraldry_addon;
pub mod spec;
pub mod ui;

pub use foundry::FoundryPlugin;
pub use heraldry_addon::HeraldryAddon;
pub use spec::PluginSpec;
