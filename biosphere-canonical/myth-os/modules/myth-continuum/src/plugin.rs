use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;
use crate::types::*;

pub struct ContinuumInstrument {
    vault: Option<Arc<VaultRegistry>>,
    config: ContinuumConfig,
}

impl ContinuumInstrument {
    pub fn new() -> Self {
        Self { vault: None, config: ContinuumConfig::default() }
    }
}

impl Default for ContinuumInstrument {
    fn default() -> Self { Self::new() }
}

impl MythPlugin for ContinuumInstrument {
    fn id(&self) -> &str { "continuum-instrument" }
    fn name(&self) -> &str { "Continuum Instrument" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn heraldry_symbol(&self) -> &str { "Crest:Continuum" }
    fn wire_in(&self) -> &[WireType] {
        &[WireType::Control, WireType::Temporal, WireType::Spatial, WireType::Energy, WireType::Logic]
    }
    fn wire_out(&self) -> &[WireType] {
        &[WireType::Spatial, WireType::Energy, WireType::Event, WireType::Data, WireType::Temporal]
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
        assert!(ContinuumInstrument::new().heraldry_symbol().starts_with("Crest:"));
    }

    #[test]
    fn wire_contracts_non_empty() {
        let i = ContinuumInstrument::new();
        assert!(!i.wire_in().is_empty());
        assert!(!i.wire_out().is_empty());
    }
}
