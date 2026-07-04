use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;
use crate::types::*;

pub struct AnimusInstrument {
    vault: Option<Arc<VaultRegistry>>,
    config: AnimusConfig,
}

impl AnimusInstrument {
    pub fn new() -> Self {
        Self { vault: None, config: AnimusConfig::default() }
    }
}

impl Default for AnimusInstrument {
    fn default() -> Self { Self::new() }
}

impl MythPlugin for AnimusInstrument {
    fn id(&self) -> &str { "animus-instrument" }
    fn name(&self) -> &str { "Animus Instrument" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn heraldry_symbol(&self) -> &str { "Crest:Animus" }
    fn wire_in(&self) -> &[WireType] {
        &[WireType::Control, WireType::Asset, WireType::Identity, WireType::Spatial]
    }
    fn wire_out(&self) -> &[WireType] {
        &[WireType::Asset, WireType::Visual, WireType::Meta, WireType::Data]
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
        assert!(AnimusInstrument::new().heraldry_symbol().starts_with("Crest:"));
    }

    #[test]
    fn wire_contracts_non_empty() {
        let i = AnimusInstrument::new();
        assert!(!i.wire_in().is_empty());
        assert!(!i.wire_out().is_empty());
    }
}
