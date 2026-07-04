use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;
use crate::types::*;

pub struct ComposerInstrument {
    vault: Option<Arc<VaultRegistry>>,
    config: ComposerConfig,
}

impl ComposerInstrument {
    pub fn new() -> Self {
        Self {
            vault: None,
            config: ComposerConfig::default(),
        }
    }
}

impl Default for ComposerInstrument {
    fn default() -> Self {
        Self::new()
    }
}

impl MythPlugin for ComposerInstrument {
    fn id(&self) -> &str { "composer-instrument" }
    fn name(&self) -> &str { "Composer Instrument" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn heraldry_symbol(&self) -> &str { "Crest:Composer" }

    fn wire_in(&self) -> &[WireType] {
        &[WireType::Control, WireType::Event, WireType::Behavioral, WireType::Spatial, WireType::Energy]
    }

    fn wire_out(&self) -> &[WireType] {
        &[WireType::Audio, WireType::Data, WireType::Event]
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
        let inst = ComposerInstrument::new();
        assert!(inst.heraldry_symbol().starts_with("Crest:"));
    }

    #[test]
    fn wire_contracts_non_empty() {
        let inst = ComposerInstrument::new();
        assert!(!inst.wire_in().is_empty());
        assert!(!inst.wire_out().is_empty());
    }
}
