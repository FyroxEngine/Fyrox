use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;
use crate::types::*;

pub struct CodexInstrument {
    vault: Option<Arc<VaultRegistry>>,
    config: CodexConfig,
}

impl CodexInstrument {
    pub fn new() -> Self {
        Self {
            vault: None,
            config: CodexConfig::default(),
        }
    }
}

impl Default for CodexInstrument {
    fn default() -> Self {
        Self::new()
    }
}

impl MythPlugin for CodexInstrument {
    fn id(&self) -> &str { "codex-instrument" }
    fn name(&self) -> &str { "Codex Instrument" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn heraldry_symbol(&self) -> &str { "Crest:Codex" }

    fn wire_in(&self) -> &[WireType] {
        &[WireType::Control, WireType::Event, WireType::Narrative, WireType::Identity, WireType::Data]
    }

    fn wire_out(&self) -> &[WireType] {
        &[WireType::Data, WireType::Narrative, WireType::Meta, WireType::Identity]
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
        let inst = CodexInstrument::new();
        assert!(inst.heraldry_symbol().starts_with("Crest:"));
    }

    #[test]
    fn wire_contracts_non_empty() {
        let inst = CodexInstrument::new();
        assert!(!inst.wire_in().is_empty());
        assert!(!inst.wire_out().is_empty());
    }
}
