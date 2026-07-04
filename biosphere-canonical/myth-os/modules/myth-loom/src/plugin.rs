use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;
use crate::types::*;

pub struct LoomInstrument {
    vault: Option<Arc<VaultRegistry>>,
    config: LoomConfig,
}

impl LoomInstrument {
    pub fn new() -> Self {
        Self { vault: None, config: LoomConfig::default() }
    }
}

impl Default for LoomInstrument {
    fn default() -> Self { Self::new() }
}

impl MythPlugin for LoomInstrument {
    fn id(&self) -> &str { "loom-instrument" }
    fn name(&self) -> &str { "Loom Instrument" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn heraldry_symbol(&self) -> &str { "Crest:Loom" }
    fn wire_in(&self) -> &[WireType] {
        &[WireType::Control, WireType::Behavioral, WireType::Spatial, WireType::Temporal]
    }
    fn wire_out(&self) -> &[WireType] {
        &[WireType::Visual, WireType::Spatial, WireType::Data, WireType::Event]
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heraldry_is_crest() {
        assert!(LoomInstrument::new().heraldry_symbol().starts_with("Crest:"));
    }

    #[test]
    fn wire_contracts_non_empty() {
        let i = LoomInstrument::new();
        assert!(!i.wire_in().is_empty());
        assert!(!i.wire_out().is_empty());
    }
}
