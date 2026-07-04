use std::collections::HashMap;

use egui_snarl::Snarl;
use serde::{Deserialize, Serialize};

use crate::layers::{LayerStack, RenderMode};
use crate::nodes::PrismNode;
use crate::plugin::{NodeRegistration, Plugin, PluginId, PluginManifest, PluginStatus};
use crate::portal::PortalPacket;

pub const MAX_PLUGINS_PER_VAULT: usize = 16;
pub const MAX_SUB_VAULTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VaultId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultStatus {
    Booting,
    Active,
    Suspended,
    Sealed,
}

pub struct Vault {
    pub id: VaultId,
    pub name: String,
    /// User-defined type — whatever you want to call it
    pub vault_type: String,
    /// Optional image path for the vault card art
    pub card_image: Option<String>,
    pub render_mode: RenderMode,
    pub status: VaultStatus,
    pub layers: LayerStack,
    pub graph: Snarl<PrismNode>,
    plugins: HashMap<PluginId, Box<dyn Plugin>>,
    sub_vaults: HashMap<VaultId, SubVault>,
    portal_inbox: Vec<PortalPacket>,
    next_plugin_id: u32,
    next_sub_vault_id: u32,
    boot_log: Vec<VaultBootEntry>,
}

pub struct VaultBootEntry {
    pub atom_index: u8,
    pub layer: VaultBootLayer,
    pub name: String,
    pub status: VaultBootStatus,
}

#[derive(Debug, Clone, Copy)]
pub enum VaultBootLayer {
    Ingestion,
    Organization,
    Access,
    Archival,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultBootStatus {
    Active,
    Pending,
    Failed,
}

impl Vault {
    pub fn new(id: VaultId, name: &str, vault_type: &str) -> Self {
        let mut vault = Self {
            id,
            name: name.to_string(),
            vault_type: vault_type.to_string(),
            card_image: None,
            render_mode: RenderMode::TwoD,
            status: VaultStatus::Booting,
            layers: LayerStack::new_vault_default(),
            graph: Snarl::new(),
            plugins: HashMap::new(),
            sub_vaults: HashMap::new(),
            portal_inbox: Vec::new(),
            next_plugin_id: 0,
            next_sub_vault_id: 0,
            boot_log: Vec::with_capacity(16),
        };
        vault.run_init_sequence();
        vault.status = VaultStatus::Active;
        vault
    }

    fn run_init_sequence(&mut self) {
        let atoms = [
            (VaultBootLayer::Ingestion, "Asset Intake Processor"),
            (VaultBootLayer::Ingestion, "Schema Fingerprinter"),
            (VaultBootLayer::Ingestion, "Metadata Tagger"),
            (VaultBootLayer::Ingestion, "Deduplication Scanner"),
            (VaultBootLayer::Organization, "Tome Compiler"),
            (VaultBootLayer::Organization, "Collection Curator"),
            (VaultBootLayer::Organization, "Relationship Mapper"),
            (VaultBootLayer::Organization, "Version Controller"),
            (VaultBootLayer::Access, "Permission Gateway"),
            (VaultBootLayer::Access, "Portal Connector"),
            (VaultBootLayer::Access, "Search Index Engine"),
            (VaultBootLayer::Access, "Retrieval Optimizer"),
            (VaultBootLayer::Archival, "Long-term Crystallizer"),
            (VaultBootLayer::Archival, "Integrity Auditor"),
            (VaultBootLayer::Archival, "Backup Sequencer"),
            (VaultBootLayer::Archival, "Purge Protocol"),
        ];

        for (i, (layer, name)) in atoms.iter().enumerate() {
            self.boot_log.push(VaultBootEntry {
                atom_index: (i + 1) as u8,
                layer: *layer,
                name: name.to_string(),
                status: VaultBootStatus::Active,
            });
        }
    }

    pub fn install_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<PluginId, String> {
        if self.plugins.len() >= MAX_PLUGINS_PER_VAULT {
            return Err(format!(
                "Vault '{}' at capacity ({MAX_PLUGINS_PER_VAULT} plugins max)",
                self.name
            ));
        }
        let id = PluginId(self.next_plugin_id);
        self.next_plugin_id += 1;
        self.plugins.insert(id, plugin);
        Ok(id)
    }

    pub fn uninstall_plugin(&mut self, id: PluginId) -> Option<Box<dyn Plugin>> {
        self.plugins.remove(&id)
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    pub fn plugin_list(&self) -> Vec<(PluginId, &PluginManifest, PluginStatus)> {
        self.plugins
            .iter()
            .map(|(&id, p)| (id, p.manifest(), p.status()))
            .collect()
    }

    /// Collect ALL node registrations from ALL installed plugins.
    pub fn available_nodes(&self) -> Vec<(PluginId, NodeRegistration)> {
        let mut result = Vec::new();
        for (&pid, plugin) in &self.plugins {
            if plugin.status() == PluginStatus::Active {
                for reg in plugin.registered_nodes() {
                    result.push((pid, reg.clone()));
                }
            }
        }
        result
    }

    /// Get available nodes grouped by category
    pub fn available_nodes_by_category(&self) -> Vec<(String, Vec<NodeRegistration>)> {
        let nodes = self.available_nodes();
        let mut categories: HashMap<String, Vec<NodeRegistration>> = HashMap::new();
        for (_pid, reg) in nodes {
            categories
                .entry(reg.category.clone())
                .or_default()
                .push(reg);
        }
        let mut sorted: Vec<(String, Vec<NodeRegistration>)> = categories.into_iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted
    }

    pub fn create_sub_vault(&mut self, name: &str, vault_type: &str) -> Result<VaultId, String> {
        if self.sub_vaults.len() >= MAX_SUB_VAULTS {
            return Err(format!(
                "Vault '{}' sub-vault capacity reached ({MAX_SUB_VAULTS} max)",
                self.name
            ));
        }
        let id = VaultId(self.next_sub_vault_id + 10000);
        self.next_sub_vault_id += 1;
        self.sub_vaults
            .insert(id, SubVault::new(id, name, vault_type, self.id));
        Ok(id)
    }

    pub fn receive_portal_packet(&mut self, packet: PortalPacket) {
        self.portal_inbox.push(packet);
    }

    pub fn drain_inbox(&mut self) -> Vec<PortalPacket> {
        std::mem::take(&mut self.portal_inbox)
    }

    pub fn inbox_count(&self) -> usize {
        self.portal_inbox.len()
    }

    pub fn tick(&mut self, clock: u64) {
        let plugin_ids: Vec<PluginId> = self.plugins.keys().copied().collect();
        for id in plugin_ids {
            if let Some(p) = self.plugins.get_mut(&id) {
                p.tick(clock);
            }
        }

        let sub_ids: Vec<VaultId> = self.sub_vaults.keys().copied().collect();
        for id in sub_ids {
            if let Some(sv) = self.sub_vaults.get_mut(&id) {
                sv.tick(clock);
            }
        }
    }

    pub fn boot_log(&self) -> &[VaultBootEntry] {
        &self.boot_log
    }
}

/// A sub-vault lives inside a parent vault.
/// Communicates UP to parent via Gates only.
pub struct SubVault {
    pub id: VaultId,
    pub name: String,
    pub vault_type: String,
    pub parent_id: VaultId,
    pub status: VaultStatus,
    pub graph: Snarl<PrismNode>,
    plugins: HashMap<PluginId, Box<dyn Plugin>>,
    gate_outbox: Vec<GateMessage>,
    next_plugin_id: u32,
}

#[derive(Debug, Clone)]
pub struct GateMessage {
    pub channel: String,
    pub payload: String,
}

impl SubVault {
    pub fn new(id: VaultId, name: &str, vault_type: &str, parent_id: VaultId) -> Self {
        Self {
            id,
            name: name.to_string(),
            vault_type: vault_type.to_string(),
            parent_id,
            status: VaultStatus::Active,
            graph: Snarl::new(),
            plugins: HashMap::new(),
            gate_outbox: Vec::new(),
            next_plugin_id: 0,
        }
    }

    pub fn install_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<PluginId, String> {
        if self.plugins.len() >= MAX_PLUGINS_PER_VAULT {
            return Err(format!(
                "Sub-vault '{}' at capacity ({MAX_PLUGINS_PER_VAULT} plugins max)",
                self.name
            ));
        }
        let id = PluginId(self.next_plugin_id);
        self.next_plugin_id += 1;
        self.plugins.insert(id, plugin);
        Ok(id)
    }

    pub fn gate_send(&mut self, message: GateMessage) {
        self.gate_outbox.push(message);
    }

    pub fn drain_gate_outbox(&mut self) -> Vec<GateMessage> {
        std::mem::take(&mut self.gate_outbox)
    }

    pub fn tick(&mut self, clock: u64) {
        let plugin_ids: Vec<PluginId> = self.plugins.keys().copied().collect();
        for id in plugin_ids {
            if let Some(p) = self.plugins.get_mut(&id) {
                p.tick(clock);
            }
        }
    }
}
