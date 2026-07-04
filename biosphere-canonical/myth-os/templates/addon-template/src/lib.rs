// ── myth-os Addon Template ────────────────────────────────────────────────────
//
// SETUP:
//   1. Copy this directory to a new location (e.g. crates/my-water-addon/)
//   2. Rename the package in Cargo.toml
//   3. Rename TemplateAddon to your addon name
//   4. Set id() and target_plugin() — target must match a registered plugin id
//   5. Implement on_output() — inspect and modify the packet stream
//   6. Add to workspace Cargo.toml members list
//
// HERALDRY:
//   Set heraldry_symbol() to "Sigil:<YourSymbol>"
//   Addons carry Sigils — independent heraldry, no parent Crest required.
//   The same addon can attach to multiple plugins.
//
// WILDCARD TARGET:
//   Set target_plugin() to "*" to attach to ALL registered plugins.
//   Use with care — your on_output() will run for every plugin's every packet.

use myth_plugin::{MythAddon, PluginResult};
use myth_wire::WirePacket;

pub struct TemplateAddon;

impl TemplateAddon {
    pub fn new() -> Self { Self }
}

impl Default for TemplateAddon {
    fn default() -> Self { Self::new() }
}

impl MythAddon for TemplateAddon {
    fn id(&self)            -> &str { "template-addon" }
    fn target_plugin(&self) -> &str { "template-plugin" }  // ← change this

    fn heraldry_symbol(&self) -> &str { "" }  // ← set to "Sigil:YourSymbol"

    fn on_output(
        &self,
        _source: &WirePacket,
        output: Vec<WirePacket>,
    ) -> PluginResult<Vec<WirePacket>> {
        // Inspect, filter, modify, or augment output packets here.
        // Return the modified list — the next addon in the chain receives it.
        Ok(output)
    }

    // Override on_tick_output() if you also need to modify tick-driven output
    // fn on_tick_output(&self, delta_ms: u64, output: Vec<WirePacket>) -> PluginResult<Vec<WirePacket>> { ... }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myth_wire::{MythId, WirePacket, WireType};

    #[test]
    fn passes_through_by_default() {
        let addon = TemplateAddon::new();
        let source = WirePacket::new(WireType::Data, MythId::new(), 0, vec![]);
        let output = vec![WirePacket::new(WireType::Data, MythId::new(), 0, vec![1, 2, 3])];
        let result = addon.on_output(&source, output.clone()).unwrap();
        assert_eq!(result.len(), output.len());
    }
}
