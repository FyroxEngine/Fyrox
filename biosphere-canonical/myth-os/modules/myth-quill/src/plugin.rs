use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;
use crate::types::*;

pub struct QuillInstrument {
    vault: Option<Arc<VaultRegistry>>,
    config: QuillConfig,
}

impl QuillInstrument {
    pub fn new() -> Self {
        Self {
            vault: None,
            config: QuillConfig::default(),
        }
    }
}

impl Default for QuillInstrument {
    fn default() -> Self {
        Self::new()
    }
}

impl MythPlugin for QuillInstrument {
    fn id(&self) -> &str { "quill-instrument" }
    fn name(&self) -> &str { "Quill Instrument" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn heraldry_symbol(&self) -> &str { "Crest:Quill" }

    fn wire_in(&self) -> &[WireType] {
        &[WireType::Control, WireType::Narrative, WireType::Event, WireType::Social, WireType::Identity]
    }

    fn wire_out(&self) -> &[WireType] {
        &[WireType::Narrative, WireType::Data, WireType::Asset, WireType::Event]
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
        let inst = QuillInstrument::new();
        assert!(inst.heraldry_symbol().starts_with("Crest:"));
    }

    #[test]
    fn wire_contracts_non_empty() {
        let inst = QuillInstrument::new();
        assert!(!inst.wire_in().is_empty());
        assert!(!inst.wire_out().is_empty());
    }
}
