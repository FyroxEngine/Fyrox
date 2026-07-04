// ── myth-os Plugin Template ───────────────────────────────────────────────────
//
// SETUP:
//   1. Copy this directory to a new location (e.g. crates/my-terrain-plugin/)
//   2. Rename the package in Cargo.toml
//   3. Rename TemplatePlugin to your plugin name
//   4. Set id(), name(), wire_in(), wire_out() to match your instrument
//   5. Implement process() — this is where your logic lives
//   6. Add to workspace Cargo.toml members list
//
// HERALDRY:
//   Set heraldry_symbol() to "Glyph:<YourSymbol>↑<ParentCrest>"
//   e.g. "Glyph:Erosion↑Atlas" for a terrain plugin under the Atlas crest.

use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;

pub struct TemplatePlugin {
    vault: Option<Arc<VaultRegistry>>,
    // Add your state fields here
}

impl TemplatePlugin {
    pub fn new() -> Self {
        Self { vault: None }
    }
}

impl Default for TemplatePlugin {
    fn default() -> Self { Self::new() }
}

impl MythPlugin for TemplatePlugin {
    fn id(&self)      -> &str { "template-plugin" }
    fn name(&self)    -> &str { "Template Plugin" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }

    // Change these to the wire types your plugin actually handles
    fn wire_in(&self)  -> &[WireType] { &[WireType::Data] }
    fn wire_out(&self) -> &[WireType] { &[WireType::Data] }

    // Set this to your heraldry glyph once assigned
    fn heraldry_symbol(&self) -> &str { "" }

    fn on_attach(&mut self, vault: Arc<VaultRegistry>) -> PluginResult<()> {
        self.vault = Some(vault);
        // Load any initial state from the vault here
        Ok(())
    }

    fn on_detach(&mut self) -> PluginResult<()> {
        self.vault = None;
        Ok(())
    }

    fn process(&mut self, packet: &WirePacket) -> PluginResult<Vec<WirePacket>> {
        // Your logic here.
        // Read from self.vault if you need capsule data.
        // Return output packets — or an empty vec if this packet produces no output.
        let _ = packet;
        Ok(vec![])
    }

    // Override tick() if you need time-driven output (weather cycles, etc.)
    // fn tick(&mut self, delta_ms: u64) -> PluginResult<Vec<WirePacket>> { ... }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myth_vault::VaultRegistry;
    use myth_wire::{MythId, WirePacket, WireType};
    use std::sync::Arc;

    fn make_vault() -> Arc<VaultRegistry> {
        let tmp = std::env::temp_dir().join("myth_plugin_template_test");
        Arc::new(VaultRegistry::open(&tmp).expect("vault"))
    }

    #[test]
    fn attaches_and_processes() {
        let mut p = TemplatePlugin::new();
        p.on_attach(make_vault()).unwrap();
        let packet = WirePacket::new(WireType::Data, MythId::new(), 0, vec![]);
        let out = p.process(&packet).unwrap();
        // Template returns empty — update this test when you add real logic
        assert!(out.is_empty());
    }
}
