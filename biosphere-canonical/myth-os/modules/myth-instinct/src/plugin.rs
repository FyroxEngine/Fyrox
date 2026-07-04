use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;
use crate::types::*;

pub struct InstinctInstrument {
    vault: Option<Arc<VaultRegistry>>,
    config: InstinctConfig,
}

impl InstinctInstrument {
    pub fn new() -> Self {
        Self { vault: None, config: InstinctConfig::default() }
    }
}

impl Default for InstinctInstrument {
    fn default() -> Self { Self::new() }
}

impl MythPlugin for InstinctInstrument {
    fn id(&self) -> &str { "instinct-instrument" }
    fn name(&self) -> &str { "Instinct Instrument" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn heraldry_symbol(&self) -> &str { "Crest:Instinct" }
    fn wire_in(&self) -> &[WireType] {
        &[WireType::Control, WireType::Agent, WireType::Social, WireType::Energy, WireType::Event]
    }
    fn wire_out(&self) -> &[WireType] {
        &[WireType::Behavioral, WireType::Agent, WireType::Event, WireType::Logic]
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
        assert!(InstinctInstrument::new().heraldry_symbol().starts_with("Crest:"));
    }

    #[test]
    fn wire_contracts_non_empty() {
        let i = InstinctInstrument::new();
        assert!(!i.wire_in().is_empty());
        assert!(!i.wire_out().is_empty());
    }
}
