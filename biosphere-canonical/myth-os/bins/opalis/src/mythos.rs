use std::collections::HashMap;

use crate::portal::PortalManager;
use crate::vault::{Vault, VaultId};

pub const MAX_VAULTS: usize = 16;

pub struct MythOS {
    pub master_vault: MasterVault,
    pub portals: PortalManager,
    pub clock_tick: u64,
    pub boot_log: Vec<BootEntry>,
}

pub struct BootEntry {
    pub atom_index: u8,
    pub layer: BootLayer,
    pub name: String,
    pub status: BootStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootLayer {
    Foundation,
    Kernel,
    Ecosystem,
    Resonance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStatus {
    Success,
    Pending,
    Failed,
}

impl MythOS {
    pub fn boot() -> Self {
        let mut os = Self {
            master_vault: MasterVault::new(),
            portals: PortalManager::new(),
            clock_tick: 0,
            boot_log: Vec::with_capacity(16),
        };
        os.run_init_sequence();
        os
    }

    fn run_init_sequence(&mut self) {
        let atoms = [
            (BootLayer::Foundation, "Aethyr Bus Handshake"),
            (BootLayer::Foundation, "Quantum Clock Sync"),
            (BootLayer::Foundation, "Physics Constant Sealing"),
            (BootLayer::Foundation, "Hardware Mapping Engine"),
            (BootLayer::Kernel, "Axiom Logic Bootstrapper"),
            (BootLayer::Kernel, "Cipher Identity Shield"),
            (BootLayer::Kernel, "B-DNA Verification Loop"),
            (BootLayer::Kernel, "Agora Registry Mounting"),
            (BootLayer::Ecosystem, "Steward Awakening"),
            (BootLayer::Ecosystem, "Quantum Quill Deployment"),
            (BootLayer::Ecosystem, "SocioMind Connectivity"),
            (BootLayer::Ecosystem, "Portal Gate Calibration"),
            (BootLayer::Resonance, "Spectral Emotion Mapping"),
            (BootLayer::Resonance, "MIDI DNA Transcription"),
            (BootLayer::Resonance, "Studio Scribe Ingestion"),
            (BootLayer::Resonance, "Quantum Quill Interpretation"),
        ];

        for (i, (layer, name)) in atoms.iter().enumerate() {
            self.boot_log.push(BootEntry {
                atom_index: (i + 1) as u8,
                layer: *layer,
                name: name.to_string(),
                status: BootStatus::Success,
            });
        }
    }

    pub fn tick(&mut self) {
        self.clock_tick += 1;

        // Flush portal deliveries
        let deliveries = self.portals.flush_all();
        for (_portal_id, dest_vault, packet) in deliveries {
            if let Some(vault) = self.master_vault.vault_mut(dest_vault) {
                vault.receive_portal_packet(packet);
            }
        }

        self.master_vault.tick(self.clock_tick);
    }

    pub fn is_booted(&self) -> bool {
        self.boot_log.len() == 16
            && self.boot_log.iter().all(|e| e.status == BootStatus::Success)
    }
}

pub struct MasterVault {
    pub vaults: HashMap<VaultId, Vault>,
    next_id: u32,
}

impl MasterVault {
    pub fn new() -> Self {
        Self {
            vaults: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn create_vault(&mut self, name: &str, vault_type: &str) -> Result<VaultId, String> {
        if self.vaults.len() >= MAX_VAULTS {
            return Err(format!(
                "Master Vault capacity reached ({MAX_VAULTS} vaults max)"
            ));
        }
        let id = VaultId(self.next_id);
        self.next_id += 1;
        self.vaults.insert(id, Vault::new(id, name, vault_type));
        Ok(id)
    }

    pub fn remove_vault(&mut self, id: VaultId) -> Option<Vault> {
        self.vaults.remove(&id)
    }

    pub fn vault(&self, id: VaultId) -> Option<&Vault> {
        self.vaults.get(&id)
    }

    pub fn vault_mut(&mut self, id: VaultId) -> Option<&mut Vault> {
        self.vaults.get_mut(&id)
    }

    pub fn tick(&mut self, clock: u64) {
        let ids: Vec<VaultId> = self.vaults.keys().copied().collect();
        for id in ids {
            if let Some(vault) = self.vaults.get_mut(&id) {
                vault.tick(clock);
            }
        }
    }
}
