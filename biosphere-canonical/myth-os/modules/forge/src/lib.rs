// forge — BioSpark Studio Simulation Module (Layer 2)
//
// Owns the tick loop that drives the 9-agent Xyrona Prime council.
// Emits WireType::Agent packets containing bincode-serialized EmergenceReports.
//
// No renderer deps. No tokio. std::sync::mpsc only.

pub mod tick;

use std::sync::mpsc::Sender;
use myth_wire::{MythId, WirePacket, WireType};
use myth_agents::AgentCouncil;

pub struct ForgeModule {
    council: AgentCouncil,
    tx:      Sender<WirePacket>,
    id:      MythId,
}

impl ForgeModule {
    pub fn new(tx: Sender<WirePacket>) -> Self {
        Self {
            council: AgentCouncil::new(),
            tx,
            id: MythId::new(),
        }
    }

    /// Advance one simulation tick and emit an EmergenceReport WirePacket.
    pub fn run_tick(&mut self) {
        // Drive all simulation logic via pure functions in tick.rs
        tick::advance_lifecycle(&mut self.council);
        tick::update_soul_weight(&mut self.council);
        tick::update_trust(&mut self.council);
        tick::update_emotions(&mut self.council);
        tick::compute_cultural_dominance(&mut self.council);

        // Generate report (logic lives in AgentCouncil::emergence_report)
        let report = self.council.emergence_report();

        // Emit as WireType::Agent packet
        if let Ok(packet) = WirePacket::encode(
            WireType::Agent,
            self.id.clone(),
            self.council.tick,
            &report,
        ) {
            let _ = self.tx.send(packet);
        }

        // Advance tick counters
        self.council.tick();
    }

    /// Read-only access to the council for inspection/testing.
    pub fn council(&self) -> &AgentCouncil {
        &self.council
    }
}
