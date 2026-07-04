use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;
use crate::types::*;

pub struct NexusInstrument {
    vault: Option<Arc<VaultRegistry>>,
    config: NexusConfig,
}

impl NexusInstrument {
    pub fn new() -> Self {
        Self { vault: None, config: NexusConfig::default() }
    }
}

impl Default for NexusInstrument {
    fn default() -> Self { Self::new() }
}

impl MythPlugin for NexusInstrument {
    fn id(&self) -> &str { "nexus-instrument" }
    fn name(&self) -> &str { "Nexus Instrument" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn heraldry_symbol(&self) -> &str { "Crest:Nexus" }
    fn wire_in(&self) -> &[WireType] {
        &[WireType::Control, WireType::Data, WireType::Event, WireType::Identity, WireType::Meta]
    }
    fn wire_out(&self) -> &[WireType] {
        &[WireType::Data, WireType::Event, WireType::Identity, WireType::Control, WireType::Agent]
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
        assert!(NexusInstrument::new().heraldry_symbol().starts_with("Crest:"));
    }

    #[test]
    fn wire_contracts_non_empty() {
        let i = NexusInstrument::new();
        assert!(!i.wire_in().is_empty());
        assert!(!i.wire_out().is_empty());
    }
}
