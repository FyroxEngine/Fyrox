/// FoundryPlugin — the MythPlugin that drives the Plugin Foundry.
///
/// This plugin:
/// 1. Requests two UI slots on registration: CanvasMain + HeaderRight icon
/// 2. Processes inbound DAT packets as pre-filled PluginSpec JSON
///    (so other tools can feed the Foundry programmatically)
/// 3. Emits forged specs as DAT WirePackets when the user hits Forge
///
/// # Heraldry
/// Core Crest: Loom (the creation/weaving instrument)
/// Symbol: `Glyph:Foundry↑Loom`
///
/// # Layout request
/// - CanvasMain       — the main 6-panel workspace
/// - HeaderRight icon — the forge icon (32×32 px) in the header bar
use myth_plugin::{LayoutRequest, MythPlugin, PluginResult, SlotRequest, SlotType};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;

use crate::spec::PluginSpec;

pub struct FoundryPlugin {
    vault: Option<Arc<VaultRegistry>>,
    /// Most recently forged spec (if any). Cleared on next tick.
    pending_emit: Option<PluginSpec>,
}

impl FoundryPlugin {
    pub fn new() -> Box<Self> {
        Box::new(Self { vault: None, pending_emit: None })
    }

    /// Call this from the egui update loop when the user clicks Forge.
    /// The plugin will emit the spec as a DAT packet on the next route cycle.
    pub fn queue_forge(&mut self, spec: PluginSpec) {
        self.pending_emit = Some(spec);
    }
}

impl MythPlugin for FoundryPlugin {
    fn id(&self)      -> &str { "plugin-foundry" }
    fn name(&self)    -> &str { "Plugin Foundry" }
    fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }

    fn wire_in(&self)  -> &[WireType] { &[WireType::Data] }
    fn wire_out(&self) -> &[WireType] { &[WireType::Data] }

    fn heraldry_symbol(&self) -> &str { "Glyph:Foundry↑Loom" }

    fn layout_request(&self) -> LayoutRequest {
        LayoutRequest::new()
            .add(
                SlotRequest::new(SlotType::CanvasMain, "Plugin Foundry — Workspace"),
            )
            .add(
                SlotRequest::new(SlotType::HeaderRight, "Plugin Foundry — Forge Icon")
                    .with_size(32.0, 32.0)
                    .on_demand(),
            )
    }

    fn on_attach(&mut self, vault: Arc<VaultRegistry>) -> PluginResult<()> {
        self.vault = Some(vault);
        tracing::info!(plugin = "plugin-foundry", "Foundry attached to vault");
        Ok(())
    }

    fn on_detach(&mut self) -> PluginResult<()> {
        self.vault = None;
        Ok(())
    }

    fn process(&mut self, packet: &WirePacket) -> PluginResult<Vec<WirePacket>> {
        // Inbound DAT packets may be pre-filled PluginSpec JSON — accept them.
        if packet.wire_type == WireType::Data {
            if let Ok(spec) = serde_json::from_slice::<PluginSpec>(&packet.payload) {
                tracing::info!(
                    crate_name = %spec.crate_name,
                    "Foundry received pre-filled spec via wire"
                );
                self.pending_emit = Some(spec);
            }
        }
        Ok(vec![])
    }

    fn tick(&mut self, _delta_ms: u64) -> PluginResult<Vec<WirePacket>> {
        if let Some(spec) = self.pending_emit.take() {
            let json = spec.to_json().map_err(|e| myth_plugin::PluginError::ProcessError {
                plugin: self.id().into(),
                reason: e.to_string(),
            })?;
            let packet = WirePacket::new(
                WireType::Data,
                myth_wire::MythId::new(),
                0,
                json.into_bytes(),
            );
            tracing::info!(plugin = "plugin-foundry", "Emitting forged spec");
            return Ok(vec![packet]);
        }
        Ok(vec![])
    }
}

impl Default for FoundryPlugin {
    fn default() -> Self { Self { vault: None, pending_emit: None } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myth_vault::VaultRegistry;
    use myth_wire::WireType;
    use std::sync::Arc;

    fn make_vault() -> Arc<VaultRegistry> {
        let tmp = std::env::temp_dir().join(
            format!("myth_foundry_test_{}", uuid::Uuid::new_v4())
        );
        Arc::new(VaultRegistry::open(&tmp).expect("vault"))
    }

    #[test]
    fn foundry_requests_canvas_main() {
        let plugin = FoundryPlugin::default();
        let req = plugin.layout_request();
        assert!(req.requests.iter().any(|r| r.slot_type == SlotType::CanvasMain));
        assert!(req.requests.iter().any(|r| r.slot_type == SlotType::HeaderRight));
    }

    #[test]
    fn foundry_emits_forged_spec_on_tick() {
        let mut plugin = FoundryPlugin::default();
        plugin.on_attach(make_vault()).unwrap();

        let mut spec = PluginSpec::new_plugin("test-forge", "Test Forge");
        spec.symbol_name = "TestForge".into();
        spec.parent_crest = "Core".into();
        spec.build_heraldry();
        spec.wire_in.push(crate::spec::WireEntry::new("DAT", "test"));

        plugin.queue_forge(spec);

        // First tick emits the packet
        let out = plugin.tick(16).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].wire_type, WireType::Data);

        // Payload is valid JSON containing the crate name
        let body = String::from_utf8(out[0].payload.clone()).unwrap();
        assert!(body.contains("test-forge"));

        // Second tick: nothing pending
        let out2 = plugin.tick(16).unwrap();
        assert!(out2.is_empty());
    }

    #[test]
    fn foundry_accepts_spec_via_wire() {
        let mut plugin = FoundryPlugin::default();
        plugin.on_attach(make_vault()).unwrap();

        let mut spec = PluginSpec::new_plugin("wired-in", "Wired In");
        spec.symbol_name = "Wire".into();
        spec.parent_crest = "Core".into();
        spec.build_heraldry();
        spec.wire_in.push(crate::spec::WireEntry::new("DAT", "test"));

        let json = spec.to_json().unwrap();
        let packet = myth_wire::WirePacket::new(
            WireType::Data, myth_wire::MythId::new(), 0, json.into_bytes()
        );

        let immediate = plugin.process(&packet).unwrap();
        assert!(immediate.is_empty()); // ack is deferred to tick

        let out = plugin.tick(16).unwrap();
        assert_eq!(out.len(), 1);
        let body = String::from_utf8(out[0].payload.clone()).unwrap();
        assert!(body.contains("wired-in"));
    }
}
