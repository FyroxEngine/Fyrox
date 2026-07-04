use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ── AgentId ───────────────────────────────────────────────────────────────────

/// Unique agent identifier. Hardcoded canonical names for the 9 Xyrona Prime
/// guild agents. String-backed so EmergenceReport round-trips cleanly over
/// WirePacket bincode without lifetime gymnastics.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Race ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Race {
    Luminarite, // 700–800 THz — highly unstable, crystallizing
    Venturan,   // 450–510 THz — moderate instability
    Syntaran,   // 510–530 THz — stable, closest to 432 Hz forge mean
    Sylvanid,   // 540–580 THz — moderate instability
    Hydralis,   // 630–670 THz — frequency floor anchors
    Nyxari,     // <400 THz sub-visible infrared — most stable, Order-only
}

impl Race {
    /// Canonical midpoint of the race's THz range.
    pub fn base_thz(self) -> f32 {
        match self {
            Self::Luminarite => 720.0,
            Self::Venturan   => 490.0,
            Self::Syntaran   => 520.0,
            Self::Sylvanid   => 560.0,
            Self::Hydralis   => 650.0,
            Self::Nyxari     => 310.0,
        }
    }

    /// 0.0–1.0 stability coefficient derived from distance to the 432 Hz forge mean.
    /// Formula: 1.0 / (1.0 + (base_thz - 432.0).abs() / 100.0)
    /// Syntaran ≈ 0.91 (most stable among factions near 432 Hz)
    /// Nyxari   ≈ 0.81 (most stable overall in raw soul-weight terms)
    /// Luminarite ≈ 0.34 (least stable — crystallizing at 720 THz)
    pub fn soul_weight_stability(self) -> f32 {
        1.0 / (1.0 + (self.base_thz() - 432.0).abs() / 100.0)
    }
}

impl std::fmt::Display for Race {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Luminarite => "Luminarite",
            Self::Venturan   => "Venturan",
            Self::Syntaran   => "Syntaran",
            Self::Sylvanid   => "Sylvanid",
            Self::Hydralis   => "Hydralis",
            Self::Nyxari     => "Nyxari",
        };
        write!(f, "{s}")
    }
}

// ── EmotionArray ──────────────────────────────────────────────────────────────

/// Six-channel emotion state, each clamped 0.0–1.0.
/// Maps to rack instrument outputs: Waveform Scope, 6-Knob Array, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionArray {
    pub grief:   f32,
    pub wonder:  f32,
    pub memory:  f32,
    pub tension: f32,
    pub passion: f32,
    pub joy:     f32,
}

impl EmotionArray {
    pub fn new(
        grief: f32, wonder: f32, memory: f32,
        tension: f32, passion: f32, joy: f32,
    ) -> Self {
        Self {
            grief:   grief.clamp(0.0, 1.0),
            wonder:  wonder.clamp(0.0, 1.0),
            memory:  memory.clamp(0.0, 1.0),
            tension: tension.clamp(0.0, 1.0),
            passion: passion.clamp(0.0, 1.0),
            joy:     joy.clamp(0.0, 1.0),
        }
    }

    /// Returns the name and value of the highest-active emotion.
    pub fn dominant(&self) -> (&'static str, f32) {
        let candidates: [(&'static str, f32); 6] = [
            ("grief",   self.grief),
            ("wonder",  self.wonder),
            ("memory",  self.memory),
            ("tension", self.tension),
            ("passion", self.passion),
            ("joy",     self.joy),
        ];
        candidates
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(("joy", 0.0))
    }

    pub fn average_tension(&self) -> f32 {
        self.tension
    }
}

// ── NeuralRack ────────────────────────────────────────────────────────────────

/// Five-channel neural parameter rack, each clamped 0.0–1.0.
/// Maps directly to rack module knob positions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralRack {
    pub sens: f32, // Sensory      → Waveform Scope resolution
    pub emot: f32, // Emotional    → 6-Knob Emotion Array responsiveness
    pub mem:  f32, // Memory       → Associative Memory retrieval depth
    pub crtv: f32, // Creative     → Surprise & Novelty Generator risk multiplier
    pub expr: f32, // Expression   → Output Gain and post-processing intensity
}

impl NeuralRack {
    pub fn new(sens: f32, emot: f32, mem: f32, crtv: f32, expr: f32) -> Self {
        Self {
            sens: sens.clamp(0.0, 1.0),
            emot: emot.clamp(0.0, 1.0),
            mem:  mem.clamp(0.0, 1.0),
            crtv: crtv.clamp(0.0, 1.0),
            expr: expr.clamp(0.0, 1.0),
        }
    }
}

// ── SoulWeight ────────────────────────────────────────────────────────────────

/// Accumulated creative mass. Tracks artistic momentum across a council cycle.
/// Breakthroughs seal the cycle. Vitrification risk occurs at extreme values.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SoulWeight(pub f32);

impl SoulWeight {
    pub const BREAKTHROUGH:           f32 = 3.0;
    pub const VITRIFICATION_RISK:     f32 = 720.0;
    pub const FORGE_MEAN_HZ:          f32 = 432.0;
    pub const DESPERATE_EQUILIBRIUM_HZ: f32 = 88.0;

    /// soul_weight = (token_count × impact) / (|current_thz − 432| + 1)
    pub fn compute(token_count: u32, impact: f32, current_thz: f32) -> Self {
        let numerator   = (token_count as f32) * impact;
        let denominator = (current_thz - Self::FORGE_MEAN_HZ).abs() + 1.0;
        Self(numerator / denominator)
    }

    pub fn is_breakthrough(&self) -> bool { self.0 >= Self::BREAKTHROUGH }
    pub fn is_vitrifying(&self)   -> bool { self.0 >= Self::VITRIFICATION_RISK }
}

impl std::fmt::Display for SoulWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} kΩ", self.0)
    }
}

// ── AgentRole ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    VisionaryDirector,      // Sets project DNA, Race selection, symbolic seed
    SoundDesigner,          // Spectral binding, sequencer programming
    ChaosArtist,            // Surprise injection, deliberate de-sync
    PerfectionistProducer,  // Quality lock, resonance clamping
    TechnicalWizard,        // Render pipeline, hardware management
    DataStrategist,         // Metrics, memory, retrieval
    SymbolicWeaver,         // Glyph routing, capsule construction
    AestheticGuardian,      // Consistency check, final seal
    ConflictMediator,       // Council tension resolution, amber-state intervention
}

// ── Department ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Department {
    Weavers,   // Content Creation
    Analyzers, // Temporal / Spectral
    Sculptors, // Render / Synthesis
    Guardians, // Quality / Release
}

// ── LifecyclePhase ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecyclePhase {
    Meditation,  // Idle — memory consolidation, graph pruning
    Ideation,    // Brainstorm — symbolic seed exchange, novelty injection
    Fulfillment, // Execution — active production, soul weight accumulation
    Harvest,     // Release — sealed, archived, cycle feeds back
}

impl std::fmt::Display for LifecyclePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Meditation  => "Meditation",
            Self::Ideation    => "Ideation",
            Self::Fulfillment => "Fulfillment",
            Self::Harvest     => "Harvest",
        };
        write!(f, "{s}")
    }
}

// ── Agent ─────────────────────────────────────────────────────────────────────

/// A single Xyrona Prime guild agent. Static name is a compile-time constant;
/// Agent only needs Serialize (never decoded back from a WirePacket payload).
#[derive(Debug, Clone, Serialize)]
pub struct Agent {
    pub id:                  AgentId,
    pub name:                &'static str,
    pub role:                AgentRole,
    pub department:          Department,
    pub race:                Race,
    pub current_thz:         f32,
    pub emotions:            EmotionArray,
    pub neural:              NeuralRack,
    pub soul_weight:         SoulWeight,
    pub trust_scores:        HashMap<AgentId, f32>,
    pub learned_preferences: Vec<String>,
    pub skill_levels:        HashMap<String, f32>,
}

impl Agent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id:         AgentId,
        name:       &'static str,
        role:       AgentRole,
        department: Department,
        race:       Race,
        current_thz: f32,
        emotions:   EmotionArray,
        neural:     NeuralRack,
    ) -> Self {
        Self {
            id,
            name,
            role,
            department,
            race,
            current_thz,
            emotions,
            neural,
            soul_weight:         SoulWeight(0.0),
            trust_scores:        HashMap::new(),
            learned_preferences: Vec::new(),
            skill_levels:        HashMap::new(),
        }
    }
}

// ── AgentCouncil ──────────────────────────────────────────────────────────────

/// The 9-agent Xyrona Prime creative studio council.
/// Owns simulation state; behavior (tick logic) lives in modules/forge.
#[derive(Debug, Clone, Serialize)]
pub struct AgentCouncil {
    pub agents:                Vec<Agent>,
    pub phase:                 LifecyclePhase,
    pub phase_tick:            u64,
    pub total_soul_weight:     SoulWeight,
    pub cultural_dominance:    HashMap<Race, f32>,
    pub total_world_resonance: f32,
    pub tick:                  u64,
}

impl AgentCouncil {
    /// Initialize the council with the 9 canonical Xyrona Prime guild agents.
    /// Names, races, roles — do not alter.
    pub fn new() -> Self {
        let agents = vec![
            // 1. Vaelindra — VisionaryDirector | Weavers | Luminarite | 720.0 THz
            //    High THz = crystallizing instability. Dominant emotion: wonder.
            Agent::new(
                AgentId::new("vaelindra"), "Vaelindra",
                AgentRole::VisionaryDirector, Department::Weavers,
                Race::Luminarite, 720.0,
                EmotionArray::new(0.1, 0.9, 0.5, 0.2, 0.8, 0.6),
                NeuralRack::new(0.7, 0.8, 0.6, 0.9, 0.8),
            ),
            // 2. Ashoren — SoundDesigner | Analyzers | Venturan | 490.0 THz
            //    Mid-range surge energy. Spectral binding, sequencer. Volatile but focused.
            Agent::new(
                AgentId::new("ashoren"), "Ashoren",
                AgentRole::SoundDesigner, Department::Analyzers,
                Race::Venturan, 490.0,
                EmotionArray::new(0.3, 0.7, 0.8, 0.2, 0.6, 0.5),
                NeuralRack::new(0.9, 0.6, 0.7, 0.7, 0.6),
            ),
            // 3. Thravex — ChaosArtist | Sculptors | Nyxari | 310.0 THz
            //    Sub-visible infrared. Most soul-weight stable. Deliberately de-syncs layers.
            Agent::new(
                AgentId::new("thravex"), "Thravex",
                AgentRole::ChaosArtist, Department::Sculptors,
                Race::Nyxari, 310.0,
                EmotionArray::new(0.5, 0.6, 0.3, 0.8, 0.7, 0.4),
                NeuralRack::new(0.6, 0.7, 0.4, 0.95, 0.8),
            ),
            // 4. Sorvaine — PerfectionistProducer | Guardians | Syntaran | 520.0 THz
            //    Closest to 432 Hz forge mean. Most structurally stable Guardian.
            Agent::new(
                AgentId::new("sorvaine"), "Sorvaine",
                AgentRole::PerfectionistProducer, Department::Guardians,
                Race::Syntaran, 520.0,
                EmotionArray::new(0.2, 0.4, 0.8, 0.6, 0.7, 0.5),
                NeuralRack::new(0.8, 0.5, 0.8, 0.6, 0.7),
            ),
            // 5. Kolthren — TechnicalWizard | Sculptors | Syntaran | 515.0 THz
            //    Forge-depth Syntaran. Manages render pipeline and hardware.
            Agent::new(
                AgentId::new("kolthren"), "Kolthren",
                AgentRole::TechnicalWizard, Department::Sculptors,
                Race::Syntaran, 515.0,
                EmotionArray::new(0.1, 0.5, 0.7, 0.5, 0.5, 0.6),
                NeuralRack::new(0.8, 0.4, 0.9, 0.7, 0.6),
            ),
            // 6. Sylvaeth — AestheticGuardian | Guardians | Sylvanid | 560.0 THz
            //    Living-green median frequency. Signs the final seal.
            Agent::new(
                AgentId::new("sylvaeth"), "Sylvaeth",
                AgentRole::AestheticGuardian, Department::Guardians,
                Race::Sylvanid, 560.0,
                EmotionArray::new(0.2, 0.7, 0.6, 0.3, 0.6, 0.8),
                NeuralRack::new(0.7, 0.8, 0.7, 0.7, 0.9),
            ),
            // 7. Noxaren — DataStrategist | Analyzers | Syntaran | 518.0 THz
            //    Archive-memory Syntaran. 99% retrieval depth. Calculates everything.
            Agent::new(
                AgentId::new("noxaren"), "Noxaren",
                AgentRole::DataStrategist, Department::Analyzers,
                Race::Syntaran, 518.0,
                EmotionArray::new(0.1, 0.5, 0.9, 0.4, 0.4, 0.5),
                NeuralRack::new(0.9, 0.3, 0.95, 0.5, 0.5),
            ),
            // 8. Thalindre — SymbolicWeaver | Weavers | Luminarite | 710.0 THz
            //    Second Luminarite. Routes Remix Capsules. Still crystallizing.
            Agent::new(
                AgentId::new("thalindre"), "Thalindre",
                AgentRole::SymbolicWeaver, Department::Weavers,
                Race::Luminarite, 710.0,
                EmotionArray::new(0.3, 0.8, 0.7, 0.2, 0.6, 0.7),
                NeuralRack::new(0.6, 0.7, 0.8, 0.8, 0.7),
            ),
            // 9. Hyvrael — ConflictMediator | Guardians | Hydralis | 645.0 THz
            //    CRITICAL: Hydralis frequency floor anchor. System requires this agent
            //    active during Fulfillment to keep resonance floor above 200 Hz.
            Agent::new(
                AgentId::new("hyvrael"), "Hyvrael",
                AgentRole::ConflictMediator, Department::Guardians,
                Race::Hydralis, 645.0,
                EmotionArray::new(0.4, 0.5, 0.6, 0.3, 0.6, 0.7),
                NeuralRack::new(0.7, 0.9, 0.6, 0.5, 0.8),
            ),
        ];

        Self {
            agents,
            phase:                 LifecyclePhase::Meditation,
            phase_tick:            0,
            total_soul_weight:     SoulWeight(0.0),
            cultural_dominance:    HashMap::new(),
            total_world_resonance: 0.0,
            tick:                  0,
        }
    }

    /// Advance the tick and phase-tick counters.
    /// All simulation logic (lifecycle, soul weight, trust, emotions)
    /// is driven by modules/forge/src/tick.rs functions.
    pub fn tick(&mut self) {
        self.tick       += 1;
        self.phase_tick += 1;
    }

    pub fn phase(&self) -> &LifecyclePhase {
        &self.phase
    }

    /// Find an agent by string id (for tick logic lookups).
    pub fn find(&self, id: &str) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id.0 == id)
    }

    /// Find the index of an agent by string id.
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.agents.iter().position(|a| a.id.0 == id)
    }

    /// Trust score from agent `from` toward agent `to`. Returns 0.0 if unset.
    pub fn trust_between(&self, from: &str, to: &str) -> f32 {
        self.agents
            .iter()
            .find(|a| a.id.0 == from)
            .and_then(|a| a.trust_scores.get(&AgentId::new(to)))
            .copied()
            .unwrap_or(0.0)
    }

    /// EmergenceReport computed directly from current council state.
    /// Called by ForgeModule; also exposed here for testing.
    pub fn emergence_report(&self) -> EmergenceReport {
        let dominant_race = self.cultural_dominance
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(race, _)| *race)
            .unwrap_or(Race::Syntaran);

        let avg_tension: f32 = if self.agents.is_empty() {
            0.0
        } else {
            self.agents.iter().map(|a| a.emotions.tension).sum::<f32>()
                / self.agents.len() as f32
        };
        let system_stability = (1.0 - avg_tension).clamp(0.0, 1.0);

        // Top 3 trust pairs across the council
        let mut all_pairs: Vec<(AgentId, AgentId, f32)> = Vec::new();
        for a in &self.agents {
            for (target_id, &score) in &a.trust_scores {
                all_pairs.push((a.id.clone(), target_id.clone(), score));
            }
        }
        all_pairs.sort_by(|x, y| y.2.partial_cmp(&x.2).unwrap_or(std::cmp::Ordering::Equal));
        let agent_synergies: Vec<_> = all_pairs.into_iter().take(3).collect();

        let hydralis_floor_active = self
            .find("hyvrael")
            .map(|h| self.phase == LifecyclePhase::Fulfillment && h.emotions.tension < 0.8)
            .unwrap_or(false);

        let vitrification_warning = self.agents.iter().any(|a| a.emotions.tension > 0.95)
            || (self.total_world_resonance / (self.tick as f32 + 1.0) > 720.0);

        let council_summons_active = self
            .find("vaelindra")
            .map(|v| v.emotions.tension > 0.8)
            .unwrap_or(false)
            && self.phase != LifecyclePhase::Fulfillment;

        EmergenceReport {
            tick:                  self.tick,
            phase:                 self.phase,
            total_soul_weight:     self.total_soul_weight,
            total_world_resonance: self.total_world_resonance,
            dominant_race,
            agent_synergies,
            new_motifs:            vec![],
            system_stability,
            hydralis_floor_active,
            vitrification_warning,
            council_summons_active,
        }
    }
}

impl Default for AgentCouncil {
    fn default() -> Self {
        Self::new()
    }
}

// ── EmergenceReport ───────────────────────────────────────────────────────────

/// Snapshot of council state emitted every tick as a WireType::Agent packet.
/// This is the only type in this crate that requires Deserialize —
/// adapters decode it from WirePacket payloads for display/monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceReport {
    pub tick:                   u64,
    pub phase:                  LifecyclePhase,
    pub total_soul_weight:      SoulWeight,
    pub total_world_resonance:  f32,
    pub dominant_race:          Race,
    pub agent_synergies:        Vec<(AgentId, AgentId, f32)>,
    pub new_motifs:             Vec<String>,
    pub system_stability:       f32,
    pub hydralis_floor_active:  bool,
    pub vitrification_warning:  bool,
    pub council_summons_active: bool,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn council_has_nine_agents() {
        let c = AgentCouncil::new();
        assert_eq!(c.agents.len(), 9);
    }

    #[test]
    fn canonical_agent_names_present() {
        let c = AgentCouncil::new();
        let names: Vec<&str> = c.agents.iter().map(|a| a.name).collect();
        for expected in &[
            "Vaelindra","Ashoren","Thravex","Sorvaine","Kolthren",
            "Sylvaeth","Noxaren","Thalindre","Hyvrael",
        ] {
            assert!(names.contains(expected), "missing agent {expected}");
        }
    }

    #[test]
    fn hyvrael_is_hydralis() {
        let c = AgentCouncil::new();
        let h = c.find("hyvrael").unwrap();
        assert_eq!(h.race, Race::Hydralis);
    }

    #[test]
    fn luminarite_is_least_stable() {
        assert!(Race::Luminarite.soul_weight_stability() < Race::Syntaran.soul_weight_stability());
        assert!(Race::Luminarite.soul_weight_stability() < Race::Nyxari.soul_weight_stability());
    }

    #[test]
    fn soul_weight_display() {
        let sw = SoulWeight(1.5);
        assert_eq!(sw.to_string(), "1.50 kΩ");
    }

    #[test]
    fn soul_weight_breakthrough() {
        assert!(!SoulWeight(2.9).is_breakthrough());
        assert!(SoulWeight(3.0).is_breakthrough());
    }

    #[test]
    fn emotion_dominant() {
        let e = EmotionArray::new(0.1, 0.9, 0.2, 0.3, 0.4, 0.5);
        assert_eq!(e.dominant().0, "wonder");
    }

    #[test]
    fn emergence_report_serializes() {
        let c = AgentCouncil::new();
        let report = c.emergence_report();
        let bytes = bincode::serialize(&report).unwrap();
        let back: EmergenceReport = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back.tick, 0);
        assert_eq!(back.agent_synergies.len(), 0); // no trust built yet
    }

    #[test]
    fn tick_increments_counters() {
        let mut c = AgentCouncil::new();
        c.tick();
        assert_eq!(c.tick, 1);
        assert_eq!(c.phase_tick, 1);
    }
}
