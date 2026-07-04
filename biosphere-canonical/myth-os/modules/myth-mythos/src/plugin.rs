use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;
use crate::types::*;

pub struct MythosInstrument {
    vault: Option<Arc<VaultRegistry>>,
    config: MythosConfig,
}

impl MythosInstrument {
    pub fn new() -> Self {
        Self {
            vault: None,
            config: MythosConfig::default(),
        }
    }
}

impl Default for MythosInstrument {
    fn default() -> Self {
        Self::new()
    }
}

impl MythPlugin for MythosInstrument {
    fn id(&self) -> &str { "mythos-instrument" }
    fn name(&self) -> &str { "Mythos Instrument" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn heraldry_symbol(&self) -> &str { "Crest:Mythos" }

    fn wire_in(&self) -> &[WireType] {
        &[WireType::Control, WireType::Temporal, WireType::Spatial]
    }

    fn wire_out(&self) -> &[WireType] {
        &[WireType::Energy, WireType::Visual, WireType::Data, WireType::Event]
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
        let inst = MythosInstrument::new();
        assert!(inst.heraldry_symbol().starts_with("Crest:"));
    }

    #[test]
    fn wire_contracts_non_empty() {
        let inst = MythosInstrument::new();
        assert!(!inst.wire_in().is_empty());
        assert!(!inst.wire_out().is_empty());
    }
}
