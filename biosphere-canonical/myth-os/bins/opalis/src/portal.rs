use serde::{Deserialize, Serialize};

use crate::vault::VaultId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortalDirection {
    /// Both vaults can push/pull assets
    Bidirectional,
    /// Only source can push to target
    Unidirectional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictStrategy {
    PreferLocal,
    PreferRemote,
    /// Future: AI-assisted merge
    AiMerge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalFilter {
    /// Only sync assets with these tags (empty = all)
    pub include_tags: Vec<String>,
    /// Never sync assets with these tags
    pub exclude_tags: Vec<String>,
    /// Only sync these wire types (empty = all)
    pub wire_types: Vec<String>,
}

impl Default for PortalFilter {
    fn default() -> Self {
        Self {
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
            wire_types: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portal {
    pub id: PortalId,
    pub name: String,
    pub source: VaultId,
    pub target: VaultId,
    pub direction: PortalDirection,
    pub conflict_strategy: ConflictStrategy,
    pub filter: PortalFilter,
    pub active: bool,
    /// Messages waiting to be delivered
    pub pending_messages: Vec<PortalPacket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalPacket {
    pub from_vault: VaultId,
    pub channel: String,
    pub payload: String,
    pub tags: Vec<String>,
}

impl Portal {
    pub fn new(
        id: PortalId,
        name: &str,
        source: VaultId,
        target: VaultId,
        direction: PortalDirection,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            source,
            target,
            direction,
            conflict_strategy: ConflictStrategy::PreferLocal,
            filter: PortalFilter::default(),
            active: true,
            pending_messages: Vec::new(),
        }
    }

    /// Check if a packet passes this portal's filter
    pub fn allows(&self, packet: &PortalPacket) -> bool {
        if !self.active {
            return false;
        }

        // Check exclude tags first (blocklist takes priority)
        for tag in &packet.tags {
            if self.filter.exclude_tags.contains(tag) {
                return false;
            }
        }

        // Check include tags (if empty, allow all)
        if !self.filter.include_tags.is_empty() {
            let has_match = packet.tags.iter().any(|t| self.filter.include_tags.contains(t));
            if !has_match {
                return false;
            }
        }

        true
    }

    /// Queue a packet for delivery (after filter check)
    pub fn send(&mut self, packet: PortalPacket) -> Result<(), String> {
        if !self.active {
            return Err("Portal is inactive".into());
        }

        // Verify direction
        match self.direction {
            PortalDirection::Unidirectional => {
                if packet.from_vault != self.source {
                    return Err("Unidirectional portal: only source can send".into());
                }
            }
            PortalDirection::Bidirectional => {
                if packet.from_vault != self.source && packet.from_vault != self.target {
                    return Err("Packet sender is not part of this portal".into());
                }
            }
        }

        if !self.allows(&packet) {
            return Err("Packet filtered by portal rules".into());
        }

        self.pending_messages.push(packet);
        Ok(())
    }

    /// Drain pending messages for delivery
    pub fn drain(&mut self) -> Vec<PortalPacket> {
        std::mem::take(&mut self.pending_messages)
    }

    /// Which vault should receive a packet from a given sender?
    pub fn destination_for(&self, sender: VaultId) -> Option<VaultId> {
        if sender == self.source {
            Some(self.target)
        } else if sender == self.target && self.direction == PortalDirection::Bidirectional {
            Some(self.source)
        } else {
            None
        }
    }
}

/// Manages all portals in the system
pub struct PortalManager {
    portals: Vec<Portal>,
    next_id: u32,
}

impl PortalManager {
    pub fn new() -> Self {
        Self {
            portals: Vec::new(),
            next_id: 0,
        }
    }

    pub fn create_portal(
        &mut self,
        name: &str,
        source: VaultId,
        target: VaultId,
        direction: PortalDirection,
    ) -> PortalId {
        let id = PortalId(self.next_id);
        self.next_id += 1;
        self.portals
            .push(Portal::new(id, name, source, target, direction));
        id
    }

    pub fn remove_portal(&mut self, id: PortalId) {
        self.portals.retain(|p| p.id != id);
    }

    pub fn portal(&self, id: PortalId) -> Option<&Portal> {
        self.portals.iter().find(|p| p.id == id)
    }

    pub fn portal_mut(&mut self, id: PortalId) -> Option<&mut Portal> {
        self.portals.iter_mut().find(|p| p.id == id)
    }

    /// Get all portals connected to a specific vault
    pub fn portals_for_vault(&self, vault_id: VaultId) -> Vec<&Portal> {
        self.portals
            .iter()
            .filter(|p| p.source == vault_id || p.target == vault_id)
            .collect()
    }

    /// Get all portals (for UI display)
    pub fn all_portals(&self) -> &[Portal] {
        &self.portals
    }

    /// Process all pending portal deliveries
    /// Returns (portal_id, destination_vault, packet) tuples ready for delivery
    pub fn flush_all(&mut self) -> Vec<(PortalId, VaultId, PortalPacket)> {
        let mut deliveries = Vec::new();

        for portal in &mut self.portals {
            let packets = portal.drain();
            for packet in packets {
                if let Some(dest) = portal.destination_for(packet.from_vault) {
                    deliveries.push((portal.id, dest, packet));
                }
            }
        }

        deliveries
    }
}
