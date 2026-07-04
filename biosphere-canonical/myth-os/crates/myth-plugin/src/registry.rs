use crate::{
    addon::MythAddon,
    error::{PluginError, PluginResult},
    layout::{DeniedSlot, GrantedSlot, LayoutGrant, SlotType},
    plugin::MythPlugin,
};
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, info};

/// The plugin registry — owns all registered plugins and their addons,
/// routes WirePackets, and manages lifecycle.
///
/// Registration order matters: packets are routed to plugins in the order
/// they were registered. Core instruments always register first at boot.
pub struct PluginRegistry {
    /// Plugins in registration order. IndexMap preserves insertion order
    /// without the overhead of sorting on every route call.
    plugins: Vec<(String, Box<dyn MythPlugin>)>,

    /// Addons keyed by the plugin id they are attached to.
    addons: HashMap<String, Vec<Box<dyn MythAddon>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            addons: HashMap::new(),
        }
    }

    // ── Registration ──────────────────────────────────────────────────────────

    /// Register a plugin and attach it to the given Vault.
    ///
    /// `on_attach()` is called immediately. If it returns an error the plugin
    /// is not added to the registry.
    pub fn register(
        &mut self,
        mut plugin: Box<dyn MythPlugin>,
        vault: Arc<VaultRegistry>,
    ) -> PluginResult<()> {
        let id = plugin.id().to_string();

        if self.plugins.iter().any(|(pid, _)| pid == &id) {
            return Err(PluginError::AlreadyRegistered(id));
        }

        plugin.on_attach(vault).map_err(|e| PluginError::AttachFailed(e.to_string()))?;

        info!(plugin = %id, name = %plugin.name(), "Plugin registered");
        self.plugins.push((id, plugin));
        Ok(())
    }

    /// Attach an addon to a registered plugin.
    ///
    /// The target plugin must already be registered. The addon's `target_plugin()`
    /// must match an existing plugin id, unless it is `"*"`.
    pub fn register_addon(&mut self, addon: Box<dyn MythAddon>) -> PluginResult<()> {
        let target = addon.target_plugin();

        if target != "*" && !self.plugins.iter().any(|(id, _)| id == target) {
            return Err(PluginError::TargetNotFound(target.to_string()));
        }

        info!(addon = %addon.id(), target = %target, "Addon registered");
        self.addons
            .entry(target.to_string())
            .or_default()
            .push(addon);
        Ok(())
    }

    /// Unregister a plugin by id. Calls `on_detach()` and removes all its addons.
    pub fn unregister(&mut self, id: &str) -> PluginResult<()> {
        let pos = self.plugins.iter().position(|(pid, _)| pid == id)
            .ok_or_else(|| PluginError::NotFound(id.to_string()))?;

        let (_, mut plugin) = self.plugins.remove(pos);
        plugin.on_detach().map_err(|e| PluginError::DetachFailed(e.to_string()))?;
        self.addons.remove(id);

        info!(plugin = %id, "Plugin unregistered");
        Ok(())
    }

    // ── Routing ───────────────────────────────────────────────────────────────

    /// Route a WirePacket to all plugins that declare its wire type in `wire_in()`.
    ///
    /// After each plugin processes the packet, its addons run over the output
    /// in registration order. Returns all output packets from all plugins.
    pub fn route(&mut self, packet: &WirePacket) -> PluginResult<Vec<WirePacket>> {
        // Phase 1: collect raw outputs (requires &mut plugins).
        let mut raw: Vec<(String, Vec<WirePacket>)> = Vec::new();
        for (id, plugin) in &mut self.plugins {
            if !plugin.wire_in().contains(&packet.wire_type) {
                continue;
            }
            debug!(plugin = %id, wire = %packet.wire_type, "Routing packet");
            let output = plugin.process(packet).map_err(|e| PluginError::ProcessError {
                plugin: id.clone(),
                reason: e.to_string(),
            })?;
            raw.push((id.clone(), output));
        }

        // Phase 2: run addons (requires only &self.addons).
        let mut all_output = Vec::new();
        for (id, output) in raw {
            let output = self.run_addons(&id, packet, output)?;
            all_output.extend(output);
        }
        Ok(all_output)
    }

    /// Advance all plugins by `delta_ms` milliseconds.
    ///
    /// Tick output also passes through addons.
    pub fn tick(&mut self, delta_ms: u64) -> PluginResult<Vec<WirePacket>> {
        // Phase 1: collect tick outputs (requires &mut plugins).
        let mut raw: Vec<(String, Vec<WirePacket>)> = Vec::new();
        for (id, plugin) in &mut self.plugins {
            let output = plugin.tick(delta_ms).map_err(|e| PluginError::ProcessError {
                plugin: id.clone(),
                reason: e.to_string(),
            })?;
            if !output.is_empty() {
                raw.push((id.clone(), output));
            }
        }

        // Phase 2: run addons (requires only &self.addons).
        let mut all_output = Vec::new();
        for (id, output) in raw {
            let output = self.run_addon_ticks(&id, delta_ms, output)?;
            all_output.extend(output);
        }
        Ok(all_output)
    }

    // ── Layout Negotiation ────────────────────────────────────────────────────

    /// Negotiate layout slots for a plugin against the available slot map.
    ///
    /// `available_slots` is a map of `slot_id → (SlotType, occupant_heraldry)`.
    /// This is normally constructed by reading a UIGenesis and listing its
    /// current `all_slots()`. Pass `None` as occupant to mark a slot as vacant.
    ///
    /// The conversation goes:
    /// ```text
    /// Plugin:   "I want CanvasLeft and a HeaderRight icon slot."
    /// Registry: "CanvasLeft granted as slot_canvas_left.
    ///            HeaderRight denied — Venturan is in slot_header_right.
    ///            Alternatives: HeaderLeft, FooterRight."
    /// ```
    pub fn negotiate_layout(
        &self,
        plugin_id: &str,
        available_slots: &HashMap<String, (SlotType, Option<String>)>,
    ) -> PluginResult<LayoutGrant> {
        let plugin = self.plugins.iter()
            .find(|(id, _)| id == plugin_id)
            .map(|(_, p)| p.as_ref())
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        let request = plugin.layout_request();
        let mut grant = LayoutGrant::default();

        if request.is_empty() {
            return Ok(grant);
        }

        for req in &request.requests {
            // If the plugin has a preferred slot_id, try it first.
            if let Some(ref preferred) = req.preferred_slot_id {
                if let Some((slot_type, occupant)) = available_slots.get(preferred) {
                    if slot_type == &req.slot_type {
                        if occupant.is_none() {
                            grant.granted.push(GrantedSlot {
                                slot_id: preferred.clone(),
                                slot_type: slot_type.clone(),
                                label: req.label.clone(),
                            });
                            continue;
                        }
                    }
                }
            }

            // Find any vacant slot of the requested type.
            let vacant = available_slots.iter().find(|(_, (st, occ))| {
                st == &req.slot_type && occ.is_none()
            });

            if let Some((slot_id, (slot_type, _))) = vacant {
                grant.granted.push(GrantedSlot {
                    slot_id: slot_id.clone(),
                    slot_type: slot_type.clone(),
                    label: req.label.clone(),
                });
            } else {
                // No vacant slot — find occupant heraldry and suggest alternatives.
                let occupant_heraldry = available_slots.iter()
                    .find(|(_, (st, _))| st == &req.slot_type)
                    .and_then(|(_, (_, occ))| occ.clone());

                let alternatives: Vec<SlotType> = available_slots.iter()
                    .filter(|(_, (_, occ))| occ.is_none())
                    .map(|(_, (st, _))| st.clone())
                    .take(3)
                    .collect();

                let reason = match &occupant_heraldry {
                    Some(h) => format!("{} is already in {:?}", h, req.slot_type),
                    None    => format!("No {:?} slot available in this layout", req.slot_type),
                };

                grant.denied.push(DeniedSlot {
                    requested_type: req.slot_type.clone(),
                    label: req.label.clone(),
                    occupant_heraldry,
                    alternatives_offered: alternatives,
                    reason,
                });
            }
        }

        info!(
            plugin = %plugin_id,
            granted = grant.granted.len(),
            denied = grant.denied.len(),
            "Layout negotiation complete"
        );

        Ok(grant)
    }

    // ── Introspection ─────────────────────────────────────────────────────────

    /// Number of registered plugins.
    pub fn plugin_count(&self) -> usize { self.plugins.len() }

    /// Number of addons attached to a specific plugin.
    pub fn addon_count(&self, plugin_id: &str) -> usize {
        self.addons.get(plugin_id).map(|v| v.len()).unwrap_or(0)
    }

    /// All plugin ids in registration order.
    pub fn plugin_ids(&self) -> Vec<&str> {
        self.plugins.iter().map(|(id, _)| id.as_str()).collect()
    }

    /// All wire types consumed by at least one registered plugin.
    pub fn active_wire_types(&self) -> Vec<WireType> {
        let mut types: Vec<WireType> = self.plugins.iter()
            .flat_map(|(_, p)| p.wire_in().iter().copied())
            .collect();
        types.sort_unstable_by_key(|w| *w as u8);
        types.dedup();
        types
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn run_addons(
        &self,
        plugin_id: &str,
        source: &WirePacket,
        mut output: Vec<WirePacket>,
    ) -> PluginResult<Vec<WirePacket>> {
        // Run plugin-specific addons, then wildcard addons.
        for addon_set in [self.addons.get(plugin_id), self.addons.get("*")] {
            if let Some(addons) = addon_set {
                for addon in addons {
                    output = addon.on_output(source, output)?;
                }
            }
        }
        Ok(output)
    }

    fn run_addon_ticks(
        &self,
        plugin_id: &str,
        delta_ms: u64,
        mut output: Vec<WirePacket>,
    ) -> PluginResult<Vec<WirePacket>> {
        for addon_set in [self.addons.get(plugin_id), self.addons.get("*")] {
            if let Some(addons) = addon_set {
                for addon in addons {
                    output = addon.on_tick_output(delta_ms, output)?;
                }
            }
        }
        Ok(output)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myth_wire::{MythId, WirePacket, WireType};

    // ── Minimal plugin for testing ────────────────────────────────────────────

    struct EchoPlugin { attached: bool }

    impl EchoPlugin {
        fn new() -> Box<Self> { Box::new(Self { attached: false }) }
    }

    impl MythPlugin for EchoPlugin {
        fn id(&self)      -> &str { "test-echo" }
        fn name(&self)    -> &str { "Echo Plugin" }
        fn version(&self) -> (u32, u32, u32) { (0, 1, 0) }
        fn wire_in(&self)  -> &[WireType] { &[WireType::Data] }
        fn wire_out(&self) -> &[WireType] { &[WireType::Data] }

        fn on_attach(&mut self, _vault: Arc<VaultRegistry>) -> PluginResult<()> {
            self.attached = true;
            Ok(())
        }
        fn on_detach(&mut self) -> PluginResult<()> {
            self.attached = false;
            Ok(())
        }
        fn process(&mut self, packet: &WirePacket) -> PluginResult<Vec<WirePacket>> {
            Ok(vec![packet.clone()])
        }
    }

    // ── Minimal addon for testing ─────────────────────────────────────────────

    struct TagAddon;

    impl MythAddon for TagAddon {
        fn id(&self)            -> &str { "test-tag" }
        fn target_plugin(&self) -> &str { "test-echo" }

        fn on_output(
            &self,
            _source: &WirePacket,
            output: Vec<WirePacket>,
        ) -> PluginResult<Vec<WirePacket>> {
            // Pass through — just confirms addon chain runs.
            Ok(output)
        }
    }

    fn make_vault() -> Arc<VaultRegistry> {
        let tmp = std::env::temp_dir().join(format!("myth_plugin_test_{}", uuid::Uuid::new_v4()));
        Arc::new(VaultRegistry::open(&tmp).expect("vault open"))
    }

    #[test]
    fn register_and_route() {
        let mut reg = PluginRegistry::new();
        reg.register(EchoPlugin::new(), make_vault()).unwrap();

        assert_eq!(reg.plugin_count(), 1);

        let packet = WirePacket::new(WireType::Data, MythId::new(), 0, vec![1, 2, 3]);
        let out = reg.route(&packet).unwrap();
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn wrong_wire_type_not_routed() {
        let mut reg = PluginRegistry::new();
        reg.register(EchoPlugin::new(), make_vault()).unwrap();

        let packet = WirePacket::new(WireType::Audio, MythId::new(), 0, vec![]);
        let out = reg.route(&packet).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn duplicate_registration_fails() {
        let mut reg = PluginRegistry::new();
        reg.register(EchoPlugin::new(), make_vault()).unwrap();
        let err = reg.register(EchoPlugin::new(), make_vault());
        assert!(matches!(err, Err(PluginError::AlreadyRegistered(_))));
    }

    #[test]
    fn addon_requires_registered_target() {
        let mut reg = PluginRegistry::new();
        let err = reg.register_addon(Box::new(TagAddon));
        assert!(matches!(err, Err(PluginError::TargetNotFound(_))));
    }

    #[test]
    fn addon_attaches_after_plugin() {
        let mut reg = PluginRegistry::new();
        reg.register(EchoPlugin::new(), make_vault()).unwrap();
        reg.register_addon(Box::new(TagAddon)).unwrap();
        assert_eq!(reg.addon_count("test-echo"), 1);
    }

    #[test]
    fn unregister_removes_addons() {
        let mut reg = PluginRegistry::new();
        reg.register(EchoPlugin::new(), make_vault()).unwrap();
        reg.register_addon(Box::new(TagAddon)).unwrap();
        reg.unregister("test-echo").unwrap();
        assert_eq!(reg.plugin_count(), 0);
        assert_eq!(reg.addon_count("test-echo"), 0);
    }

    #[test]
    fn active_wire_types_deduped() {
        let mut reg = PluginRegistry::new();
        reg.register(EchoPlugin::new(), make_vault()).unwrap();
        let types = reg.active_wire_types();
        assert_eq!(types, vec![WireType::Data]);
    }
}
