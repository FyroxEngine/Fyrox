use myth_plugin::{MythPlugin, PluginResult};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;
use crate::types::*;

pub struct OrderInstrument {
    vault: Option<Arc<VaultRegistry>>,
    config: OrderConfig,
}

impl OrderInstrument {
    pub fn new() -> Self {
        Self { vault: None, config: OrderConfig::default() }
    }
}

impl Default for OrderInstrument {
    fn default() -> Self { Self::new() }
}

impl MythPlugin for OrderInstrument {
    fn id(&self) -> &str { "order-instrument" }
    fn name(&self) -> &str { "Order Instrument" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
    fn heraldry_symbol(&self) -> &str { "Crest:Order" }
    fn wire_in(&self) -> &[WireType] {
        &[WireType::Control, WireType::Behavioral, WireType::Event, WireType::Identity, WireType::Narrative]
    }
    fn wire_out(&self) -> &[WireType] {
        &[WireType::Social, WireType::Event, WireType::Data, WireType::Narrative]
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
        assert!(OrderInstrument::new().heraldry_symbol().starts_with("Crest:"));
    }

    #[test]
    fn wire_contracts_non_empty() {
        let i = OrderInstrument::new();
        assert!(!i.wire_in().is_empty());
        assert!(!i.wire_out().is_empty());
    }
}
