// Engine-agnostic Eidolon Synthesis Rack logic, adapted from the user's Rust spec.

pub const COSMIC_LAWS: &[&str] = &[
    "Gravity",
    "Temporal Flow",
    "Logical Consistency",
    "Causality",
    "Entropy",
    "Emergence",
    "Resonance",
    "Probability",
];

pub const CORE_MODULE_ROLES: &[&str] = &[
    "World Weaver",
    "Flux Orchestrator",
    "Hydromancer Overseer",
    "Operations Architect",
    "Resonance Conduit",
    "Entanglement Architect",
    "Probability Seer",
    "Hyper-Geometry Mason",
    "Logic Scribe",
    "Nebula Systems Architect",
    "Quantum Simulation Director",
    "Void Lattice Engineer",
    "Sigil Actuary",
    "Ontological Herald",
    "Glyph Composer",
    "Crest Attestor",
    "Resonance Sealer",
    "Axiom Carver",
    "Mythos Cartographer",
    "Persona Forger",
    "Codex Legate",
    "Vestige Harmonizer",
    "Tidal Choreographer",
    "Chimera Censor",
    "Axiom Sculptor",
    "Emergence Botanist",
    "Tessellation Architect",
    "Syntactic Arborist",
    "Algorithmic Cartographer",
    "Recursion Delver",
];

/// 8 cosmic laws × 16 beat steps.
pub struct AxiomStepSequencer {
    pub laws: [&'static str; 8],
    pub steps: [[bool; 16]; 8],
    pub current_step: usize,
    pub bpm: f32,
}

impl Default for AxiomStepSequencer {
    fn default() -> Self {
        let mut steps = [[false; 16]; 8];
        // Default pattern: every 4th step for gravity, every 8th for temporal
        steps[0][0] = true; steps[0][4] = true; steps[0][8] = true; steps[0][12] = true;
        steps[1][0] = true; steps[1][8] = true;
        steps[2][2] = true; steps[2][6] = true; steps[2][10] = true; steps[2][14] = true;
        Self {
            laws: [
                "Gravity", "Temporal Flow", "Logic", "Causality",
                "Entropy", "Emergence", "Resonance", "Probability",
            ],
            steps,
            current_step: 0,
            bpm: 120.0,
        }
    }
}

impl AxiomStepSequencer {
    pub fn tick(&mut self) -> Vec<&str> {
        self.current_step = (self.current_step + 1) % 16;
        self.laws
            .iter()
            .enumerate()
            .filter(|(i, _)| self.steps[*i][self.current_step])
            .map(|(_, law)| *law)
            .collect()
    }

    pub fn toggle_step(&mut self, law: usize, step: usize) {
        if law < 8 && step < 16 {
            self.steps[law][step] = !self.steps[law][step];
        }
    }
}

pub struct SomaticPatch {
    pub source_module: String,
    pub target_module: String,
    pub signal_type: String,
}

pub struct SynthesisModule {
    pub title: String,
    pub role: String,
    pub connections: Vec<SomaticPatch>,
    pub active: bool,
}

/// Harmonic drawbar values for the Resonance Engine (0.0–1.0 per harmonic).
pub struct ResonanceHarmonics {
    pub values: [f32; 8],
}

impl Default for ResonanceHarmonics {
    fn default() -> Self {
        Self {
            values: [0.6, 0.8, 0.4, 0.75, 0.55, 0.3, 0.9, 0.5],
        }
    }
}
