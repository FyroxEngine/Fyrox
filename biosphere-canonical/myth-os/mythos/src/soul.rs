// The Actor Soul — the sealed, portable identity of every Quill Actor.
//
// Architecture (Phantori Phase 4):
//   ConsciousMind     — inspectable, active parameters and goal state
//   SubconsciousMind  — hidden trait-weight graph; drives behaviour without direct read access
//   B-DNA             — 64-position boolean genome; heritable, immutable after birth
//   HeraldricPosition — social/cosmic rank in the Quantum Ecosystem hierarchy
//   ActorSoul         — the sealed container carrying all of the above
//   PersonalEngine    — trait for self-contained minimal simulation (no Bevy dependency)
//   SoulMigration     — protocol data for transferring a soul between vaults

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::identity::MythId;

// ── Faction ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Faction {
    #[default]
    Venturan,  // explorer / trader — blue
    Hydralis,  // ocean / water — teal
    Luminar,   // light-bringers — gold
    Xyrona,    // void / alien — violet
    Nexari,    // networkers / connectors — white
}

impl Faction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Venturan => "VENTURAN",
            Self::Hydralis => "HYDRALIS",
            Self::Luminar  => "LUMINAR",
            Self::Xyrona   => "XYRONA",
            Self::Nexari   => "NEXARI",
        }
    }

    pub fn color_hex(&self) -> &'static str {
        match self {
            Self::Venturan => "#1e8cff",
            Self::Hydralis => "#00c8b4",
            Self::Luminar  => "#ffd060",
            Self::Xyrona   => "#8c50ff",
            Self::Nexari   => "#e8e8ff",
        }
    }

    pub fn from_seed(seed: u64) -> Self {
        match seed % 5 {
            0 => Self::Venturan,
            1 => Self::Hydralis,
            2 => Self::Luminar,
            3 => Self::Xyrona,
            _ => Self::Nexari,
        }
    }
}

// ── B-DNA ─────────────────────────────────────────────────────────────────────

/// 64-position boolean genome.  Stored as Vec<bool> for serde compatibility.
/// Invariant: always exactly 64 elements after construction.
pub type BDna = Vec<bool>;

/// First-generation B-DNA from an entropy seed (no parents).
pub fn bdna_from_seed(seed: u64) -> BDna {
    let mut dna = vec![false; 64];
    let mut s   = seed;
    for (i, bit) in dna.iter_mut().enumerate() {
        if i % 64 == 0 {
            s = s.wrapping_mul(6_364_136_223_846_793_005)
                 .wrapping_add(1_442_695_040_888_963_407);
        }
        *bit = (s >> (i % 64)) & 1 == 1;
    }
    dna
}

/// Inherit B-DNA from one or two parents with an environmental mutation seed.
/// Algorithm: child[i] = parent_a[i] XOR parent_b[i] XOR env_mask[i]
pub fn inherit_bdna(parent_a: &BDna, parent_b: Option<&BDna>, env_seed: u64) -> BDna {
    let mask = bdna_from_seed(env_seed);
    let mut child = vec![false; 64];
    for i in 0..64 {
        let b = parent_b.map(|p| p[i]).unwrap_or(false);
        child[i] = (parent_a[i] ^ b) ^ mask[i];
    }
    child
}

/// Compact 16-character hex fingerprint (8 bytes → 16 hex chars).
pub fn bdna_to_hex(dna: &BDna) -> String {
    let mut bytes = [0u8; 8];
    for (i, &bit) in dna.iter().enumerate() {
        if bit { bytes[i / 8] |= 1 << (i % 8); }
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compute a u64 "resonance value" from B-DNA (used for Hz assignment).
pub fn bdna_resonance(dna: &BDna) -> u64 {
    let mut v = 0u64;
    for (i, &b) in dna.iter().enumerate() {
        if b { v |= 1u64 << (i % 64); }
    }
    v
}

// ── Trait Axes ────────────────────────────────────────────────────────────────

/// Indices into SubconsciousMind::weights.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum TraitAxis {
    Aggression   = 0,
    Curiosity    = 1,
    SocialDrive  = 2,
    FearBase     = 3,
    EnergyDrive  = 4,
    Loyalty      = 5,
    Cunning      = 6,
    Empathy      = 7,
    Ambition     = 8,
    Wanderlust   = 9,
    Creativity   = 10,
    Skepticism   = 11,
    Resilience   = 12,
    Deception    = 13,
    Spirituality = 14,
    Dominance    = 15,
}

// ── HeraldricPosition ─────────────────────────────────────────────────────────

/// The actor's social and cosmic rank inside the Quantum Ecosystem hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeraldricPosition {
    /// Tier 0 = unranked; 1 = Initiate … 8 = Archon
    pub order:         u8,
    /// Rank within the order (1–16, per Octave Capacity Law)
    pub rank:          u8,
    /// Sigil glyph assigned at creation from B-DNA
    pub sigil:         String,
    /// Accumulated ascent points — determines advancement
    pub ascent_points: u32,
}

impl HeraldricPosition {
    pub fn unranked() -> Self {
        Self { order: 0, rank: 1, sigil: "◦".into(), ascent_points: 0 }
    }

    pub fn from_bdna(dna: &BDna) -> Self {
        // Derive sigil from the first 5 B-DNA bits
        let glyphs = ["◈", "◉", "◇", "△", "▽", "⬡", "⬢", "◌", "○", "●",
                       "◐", "◑", "◒", "◓", "⊕", "⊗", "⊙", "⊚", "⊛", "⊜",
                       "⊞", "⊟", "⊠", "⊡", "⋆", "✦", "✧", "✩", "✪", "✫",
                       "✬", "✭"];
        let idx = dna.iter().take(5).fold(0usize, |acc, &b| acc * 2 + b as usize);
        Self {
            order: 0,
            rank:  1,
            sigil: glyphs[idx % glyphs.len()].into(),
            ascent_points: 0,
        }
    }

    pub fn title(&self) -> &'static str {
        match self.order {
            0 => "Unranked",
            1 => "Initiate",
            2 => "Adept",
            3 => "Practitioner",
            4 => "Scholar",
            5 => "Keeper",
            6 => "Warden",
            7 => "Sovereign",
            _ => "Archon",
        }
    }
}

// ── Conscious Mind ────────────────────────────────────────────────────────────

/// What the actor consciously "knows about itself" — inspectable by external systems.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsciousMind {
    /// Live float parameters (energy, curiosity_expression, tension, …)
    pub params:        HashMap<String, f32>,
    /// Current short-term goal narrative
    pub current_goal:  Option<String>,
    /// Emotional state label for display (calm / curious / afraid / excited / hostile)
    pub emotional_tag: String,
    /// Tick counter — increments every PersonalEngine::tick call
    pub tick_count:    u64,
}

impl ConsciousMind {
    pub fn new(curiosity: f32, energy: f32) -> Self {
        let mut params = HashMap::new();
        params.insert("energy".into(),               energy.clamp(0.0, 1.0));
        params.insert("curiosity_expression".into(), curiosity.clamp(0.0, 1.0));
        params.insert("social_warmth".into(),        0.5);
        params.insert("tension".into(),              0.0);
        params.insert("hunger".into(),               0.2);
        Self { params, current_goal: None, emotional_tag: "calm".into(), tick_count: 0 }
    }

    pub fn get(&self, key: &str) -> f32 {
        self.params.get(key).copied().unwrap_or(0.0)
    }

    pub fn set(&mut self, key: impl Into<String>, val: f32) {
        self.params.insert(key.into(), val.clamp(0.0, 1.0));
    }

    /// Decay needs over time. Returns true if energy reached zero (needs rest).
    pub fn tick_needs(&mut self, dt: f32) -> bool {
        // Use separate scopes so each mutable borrow is dropped before the next.
        {
            let e = self.params.entry("energy".into()).or_insert(1.0);
            *e = (*e - dt * 0.004).clamp(0.0, 1.0);
        }
        {
            let h = self.params.entry("hunger".into()).or_insert(0.0);
            *h = (*h + dt * 0.002).clamp(0.0, 1.0);
        }
        self.tick_count += 1;
        self.params.get("energy").copied().unwrap_or(0.0) < 0.05
    }
}

// ── Subconscious Mind ─────────────────────────────────────────────────────────

/// Hidden trait-weight graph — external systems read only the emergent intent,
/// never the raw weights.  Drives the subconscious simulation path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubconsciousMind {
    /// 16 trait weights derived from B-DNA at birth (see TraitAxis)
    pub(crate) weights:          [f32; 16],
    /// Rolling emotional buffer (0.0 = all-negative, 1.0 = all-positive)
    pub(crate) emotional_buffer: f32,
    /// Accumulated pressure — triggers conscious escalation when > 0.7
    pub(crate) drive_pressure:   f32,
    /// Resonance frequency → accumulated emotional weight [-1.0, 1.0].
    /// Positive = attraction, negative = trauma.
    /// e.g. fire voxel at 800 Hz → repeated pain stimuli → key 800 → strong negative.
    /// Influences future affordance scoring (full wiring in resonance navigation pass).
    pub(crate) frequency_memory: HashMap<u32, f32>,
}

impl SubconsciousMind {
    /// Derive trait weights deterministically from B-DNA.
    pub fn from_bdna(dna: &BDna) -> Self {
        let mut weights = [0.0_f32; 16];
        for (i, w) in weights.iter_mut().enumerate() {
            // Average 4 adjacent DNA bits → weight for this trait axis
            let base = (i * 4).min(60);
            let sum: u32 = dna[base..base + 4].iter().map(|&b| b as u32).sum();
            *w = sum as f32 / 4.0;
        }
        Self { weights, emotional_buffer: 0.5, drive_pressure: 0.0, frequency_memory: HashMap::new() }
    }

    /// Read the raw drive weight for a named axis (cloned so weights stay hidden).
    pub fn drive(&self, axis: TraitAxis) -> f32 {
        self.weights[axis as usize]
    }

    /// Absorb a stimulus (collision, social interaction, threat).
    /// `intensity` in [0, 1]; `valence` in [-1, 1] (positive = good, negative = bad).
    /// `source_hz` — optional resonance frequency of the emitting voxel/entity.
    /// When present, the frequency is linked to the emotional outcome in `frequency_memory`,
    /// building the Associative Resonance Memory used by the affordance scoring system.
    pub fn absorb_stimulus(&mut self, intensity: f32, valence: f32, source_hz: Option<f32>) {
        let norm = (intensity * valence * 0.5 + 0.5).clamp(0.0, 1.0);
        self.emotional_buffer = self.emotional_buffer * 0.9 + norm * 0.1;
        self.drive_pressure   = (self.drive_pressure + intensity * 0.25).clamp(0.0, 1.0);

        // Stamp the frequency memory — slow convergence (α = 0.1) so a single event
        // doesn't permanently traumatise; repeated exposure accumulates the weight.
        if let Some(hz) = source_hz {
            let key = hz.round() as u32;
            let entry = self.frequency_memory.entry(key).or_insert(0.0);
            *entry = (*entry + intensity * valence * 0.1).clamp(-1.0, 1.0);
        }
    }

    /// True when pressure or emotional deviation requests conscious escalation.
    pub fn wants_conscious_eval(&self) -> bool {
        self.drive_pressure > 0.70
            || (self.emotional_buffer - 0.5).abs() > 0.38
    }

    /// Decay drive pressure each tick (call with dt in seconds).
    pub fn tick_decay(&mut self, dt: f32) {
        self.drive_pressure = (self.drive_pressure - dt * 0.06).max(0.0);
    }

    /// How much of behavior is subconscious-driven (0 = conscious, 1 = subconscious).
    pub fn subconscious_weight(&self) -> f32 {
        1.0 - self.drive_pressure.min(0.7) / 0.7
    }
}

// ── Actor Intent ──────────────────────────────────────────────────────────────

/// The output signal of `PersonalEngine::tick` — what the actor intends to do.
/// Genesis maps these to ECS mutations; a standalone engine produces them directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorIntent {
    MoveTo    { x: f32, y: f32, z: f32 },
    Interact  { target_id: String },
    Speak     { text: String },
    Rest      { seconds: f32 },
    /// Flag a novel event for conscious evaluation (future: LLM call)
    Escalate  { reason: String },
    Idle,
}

// ── PersonalEngine trait ──────────────────────────────────────────────────────

/// Minimal autonomous simulation capability — can run without Bevy.
/// Implemented on anything that wraps an `ActorSoul`.
pub trait PersonalEngine: Send + Sync {
    /// Advance the engine by `dt` seconds; return a list of intents.
    fn tick(&mut self, dt: f32) -> Vec<ActorIntent>;
    /// Absorb an external stimulus. `source_hz` tags the resonance frequency of
    /// the emitting source so the subconscious can build frequency memory.
    fn stimulate(&mut self, intensity: f32, valence: f32, source_hz: Option<f32>);
    /// Serialise engine state to bytes (used for soul migration).
    fn to_bytes(&self) -> Vec<u8>;
}

// ── ActorSoul ─────────────────────────────────────────────────────────────────

/// The sealed, portable identity of a Quill Actor.
/// Stored as `CapsuleKind::Actor` in the SoulVault; transferred via migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSoul {
    /// Stable across vault migrations — this is the actor's eternal identity
    pub id:           MythId,
    pub name:         String,
    /// B-DNA genome — immutable after creation
    pub bdna:         BDna,
    pub faction:      Faction,
    pub heraldric:    HeraldricPosition,
    pub conscious:    ConsciousMind,
    pub subconscious: SubconsciousMind,
    /// Ancestry chain (empty = first generation)
    pub lineage:      Vec<MythId>,
    /// Vault currently hosting this soul (None = in transit)
    pub home_vault:   Option<MythId>,
    /// Unix timestamp of last significant state change
    pub last_changed: u64,
}

impl ActorSoul {
    /// Spawn a first-generation soul from a name and entropy seed.
    pub fn genesis(name: impl Into<String>, seed: u64) -> Self {
        let bdna      = bdna_from_seed(seed);
        let sub       = SubconsciousMind::from_bdna(&bdna);
        let heraldric = HeraldricPosition::from_bdna(&bdna);
        let conscious = ConsciousMind::new(
            sub.drive(TraitAxis::Curiosity),
            sub.drive(TraitAxis::EnergyDrive),
        );
        Self {
            id:          MythId::new(),
            name:        name.into(),
            bdna,
            faction:     Faction::from_seed(seed),
            heraldric,
            conscious,
            subconscious: sub,
            lineage:     Vec::new(),
            home_vault:  None,
            last_changed: unix_ts(),
        }
    }

    /// Create a child soul inheriting from one or two parents.
    pub fn inherit(
        name:     impl Into<String>,
        parent_a: &ActorSoul,
        parent_b: Option<&ActorSoul>,
        env_seed: u64,
    ) -> Self {
        let bdna      = inherit_bdna(&parent_a.bdna, parent_b.map(|p| &p.bdna), env_seed);
        let sub       = SubconsciousMind::from_bdna(&bdna);
        let heraldric = HeraldricPosition::from_bdna(&bdna);
        let conscious = ConsciousMind::new(
            sub.drive(TraitAxis::Curiosity),
            sub.drive(TraitAxis::EnergyDrive),
        );
        let mut lineage = vec![parent_a.id.clone()];
        if let Some(pb) = parent_b { lineage.push(pb.id.clone()); }

        Self {
            id:          MythId::new(),
            name:        name.into(),
            bdna,
            faction:     parent_a.faction,
            heraldric,
            conscious,
            subconscious: sub,
            lineage,
            home_vault:  None,
            last_changed: unix_ts(),
        }
    }

    /// Serialise to JSON bytes for Capsule payload storage.
    pub fn to_capsule_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Deserialise from Capsule payload bytes.
    pub fn from_capsule_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }

    /// 16-char hex fingerprint derived from B-DNA.
    pub fn bdna_hex(&self) -> String {
        bdna_to_hex(&self.bdna)
    }

    /// True when the subconscious is requesting conscious evaluation this tick.
    pub fn wants_conscious_eval(&self) -> bool {
        self.subconscious.wants_conscious_eval()
    }

    /// Conscious/subconscious split ratio (0.0 = fully subconscious, 1.0 = fully conscious).
    pub fn conscious_fraction(&self) -> f32 {
        1.0 - self.subconscious.subconscious_weight()
    }
}

impl PersonalEngine for ActorSoul {
    fn tick(&mut self, dt: f32) -> Vec<ActorIntent> {
        self.subconscious.tick_decay(dt);
        let exhausted = self.conscious.tick_needs(dt);
        self.last_changed = unix_ts();

        if exhausted {
            self.conscious.emotional_tag = "tired".into();
            return vec![ActorIntent::Rest { seconds: 3.0 }];
        }

        if self.subconscious.wants_conscious_eval() {
            return vec![ActorIntent::Escalate {
                reason: format!(
                    "drive_pressure={:.2} buffer={:.2}",
                    self.subconscious.drive_pressure,
                    self.subconscious.emotional_buffer
                ),
            }];
        }

        // Default subconscious wander drive
        let wander = self.subconscious.drive(TraitAxis::Wanderlust);
        if wander > 0.5 {
            vec![ActorIntent::Idle] // actual movement decided by WanderBrain in genesis
        } else {
            vec![ActorIntent::Idle]
        }
    }

    fn stimulate(&mut self, intensity: f32, valence: f32, source_hz: Option<f32>) {
        self.subconscious.absorb_stimulus(intensity, valence, source_hz);
        self.last_changed = unix_ts();
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_capsule_bytes()
    }
}

// ── Soul Migration Protocol ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationReason {
    PlayerTriggered,
    Death,
    Reincarnation,
    DigitalUpload,
    PortalCrossing,
}

/// Data packet for transferring a soul between vault instances.
/// Full transport via gRPC arrives in Phase 9; this struct is the serialised unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulMigration {
    pub soul:        ActorSoul,
    pub source_vault: MythId,
    pub dest_vault:   MythId,
    pub reason:       MigrationReason,
    pub timestamp:    u64,
    /// BLAKE3 hash of the serialised soul bytes at departure
    pub departure_fingerprint: String,
}

impl SoulMigration {
    pub fn new(
        soul:         ActorSoul,
        source_vault: MythId,
        dest_vault:   MythId,
        reason:       MigrationReason,
    ) -> Self {
        let bytes = soul.to_capsule_bytes();
        let departure_fingerprint = blake3_hex(&bytes);
        Self { soul, source_vault, dest_vault, reason, timestamp: unix_ts(), departure_fingerprint }
    }

    /// Verify the soul bytes haven't changed since departure.
    pub fn verify_integrity(&self) -> bool {
        let current_bytes = self.soul.to_capsule_bytes();
        blake3_hex(&current_bytes) == self.departure_fingerprint
    }
}

// ── Social Bond ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BondType {
    Kinship,     // inherited through lineage
    Alliance,    // formed through repeated cooperation
    Rivalry,     // formed through conflict
    Mentorship,  // knowledge transfer relationship
    Neutral,     // minimal history
}

/// A directional social connection between two actor souls.
/// The `SOC` wire type carries updates to these bonds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialBond {
    pub source:    MythId,
    pub target:    MythId,
    pub bond_type: BondType,
    /// Strength in [-1.0, 1.0]: positive = affinity, negative = animosity
    pub strength:  f32,
    pub formed_at: u64,
}

impl SocialBond {
    pub fn new(source: MythId, target: MythId, bond_type: BondType) -> Self {
        Self { source, target, bond_type, strength: 0.0, formed_at: unix_ts() }
    }

    /// Nudge strength toward a value (α = 0.1 — slow convergence).
    pub fn reinforce(&mut self, delta: f32) {
        self.strength = (self.strength + delta * 0.1).clamp(-1.0, 1.0);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn unix_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn blake3_hex(bytes: &[u8]) -> String {
    let hash = blake3::hash(bytes);
    hash.to_hex().to_string()
}

// ── Schema validation tests ───────────────────────────────────────────────────
// Enforces the invariants documented in ADR-001 (B-DNA Determinism) and
// ADR-003 (Performance Budget). Run with `cargo test -p mythos`.

#[cfg(test)]
mod tests {
    use super::*;

    // ── ADR-001: B-DNA invariants ─────────────────────────────────────────

    #[test]
    fn bdna_always_64_bits() {
        let dna = bdna_from_seed(0xdeadbeef_cafe_1234);
        assert_eq!(dna.len(), 64, "B-DNA must always be exactly 64 bits");
    }

    #[test]
    fn bdna_from_seed_is_deterministic() {
        let seed = 0x1234_5678_90ab_cdef;
        assert_eq!(
            bdna_from_seed(seed),
            bdna_from_seed(seed),
            "Same seed must always produce identical B-DNA"
        );
    }

    #[test]
    fn different_seeds_produce_different_bdna() {
        assert_ne!(
            bdna_from_seed(0xdeadbeef),
            bdna_from_seed(0xcafebabe),
            "Different seeds must produce different B-DNA"
        );
    }

    #[test]
    fn inherit_bdna_always_64_bits() {
        let pa = bdna_from_seed(0xdeadbeef);
        let pb = bdna_from_seed(0xcafebabe);
        assert_eq!(inherit_bdna(&pa, Some(&pb), 0x12345678).len(), 64);
        assert_eq!(inherit_bdna(&pa, None, 0x12345678).len(), 64);
    }

    #[test]
    fn inherit_bdna_is_deterministic() {
        let pa = bdna_from_seed(0xaaaa);
        let pb = bdna_from_seed(0xbbbb);
        assert_eq!(
            inherit_bdna(&pa, Some(&pb), 0xcccc),
            inherit_bdna(&pa, Some(&pb), 0xcccc),
        );
    }

    #[test]
    fn bdna_hex_fingerprint_is_16_chars() {
        let hex = bdna_to_hex(&bdna_from_seed(0xdeadbeef));
        assert_eq!(hex.len(), 16, "B-DNA hex fingerprint must be exactly 16 chars");
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()), "Must be valid hex");
    }

    #[test]
    fn actor_soul_genesis_is_deterministic() {
        let seed = 0xfeedface_deadbeef_u64;
        let a = ActorSoul::genesis("TestActor", seed);
        let b = ActorSoul::genesis("TestActor", seed);
        assert_eq!(a.bdna_hex(), b.bdna_hex(), "Same seed → same B-DNA hex");
        assert_eq!(a.faction, b.faction,       "Same seed → same faction");
        assert_eq!(a.heraldric.sigil, b.heraldric.sigil, "Same seed → same sigil");
    }

    #[test]
    fn different_seeds_produce_different_souls() {
        let a = ActorSoul::genesis("Actor", 0xdeadbeef);
        let b = ActorSoul::genesis("Actor", 0xcafebabe);
        assert_ne!(a.bdna_hex(), b.bdna_hex());
    }

    // ── ADR-003: SubconsciousMind invariants ──────────────────────────────

    #[test]
    fn frequency_memory_starts_empty() {
        let sub = SubconsciousMind::from_bdna(&bdna_from_seed(0));
        assert!(sub.frequency_memory.is_empty(), "Fresh soul has no frequency memories");
    }

    #[test]
    fn negative_stimulus_builds_negative_memory() {
        let mut sub = SubconsciousMind::from_bdna(&bdna_from_seed(0));
        // Fire voxel at 800 Hz, max pain
        sub.absorb_stimulus(1.0, -1.0, Some(800.0));
        let weight = sub.frequency_memory.get(&800).copied().unwrap_or(0.0);
        assert!(weight < 0.0, "Negative valence must produce negative frequency memory");
    }

    #[test]
    fn positive_stimulus_builds_positive_memory() {
        let mut sub = SubconsciousMind::from_bdna(&bdna_from_seed(0));
        sub.absorb_stimulus(1.0, 1.0, Some(440.0));
        let weight = sub.frequency_memory.get(&440).copied().unwrap_or(0.0);
        assert!(weight > 0.0, "Positive valence must produce positive frequency memory");
    }

    #[test]
    fn frequency_memory_clamped_to_unit_range() {
        let mut sub = SubconsciousMind::from_bdna(&bdna_from_seed(0));
        // Repeat trauma 100× — should clamp at -1.0 not go below
        for _ in 0..100 {
            sub.absorb_stimulus(1.0, -1.0, Some(800.0));
        }
        let weight = sub.frequency_memory.get(&800).copied().unwrap_or(0.0);
        assert!(weight >= -1.0, "Frequency memory must never go below -1.0");
    }

    #[test]
    fn stimulus_without_hz_does_not_pollute_frequency_memory() {
        let mut sub = SubconsciousMind::from_bdna(&bdna_from_seed(0));
        sub.absorb_stimulus(1.0, -1.0, None);
        assert!(sub.frequency_memory.is_empty(), "No Hz = no frequency memory entry");
    }

    #[test]
    fn trait_weights_all_in_unit_range() {
        let dna = bdna_from_seed(0xdeadbeef_cafe_u64);
        let sub = SubconsciousMind::from_bdna(&dna);
        for (i, &w) in sub.weights.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&w),
                "Trait weight {i} = {w} is outside [0.0, 1.0]"
            );
        }
    }

    #[test]
    fn conscious_fraction_in_unit_range() {
        let soul = ActorSoul::genesis("RangeTest", 0xabcdef);
        let cf = soul.conscious_fraction();
        assert!((0.0..=1.0).contains(&cf), "conscious_fraction must be in [0.0, 1.0]");
    }

    // ── Serialisation round-trip ──────────────────────────────────────────

    #[test]
    fn soul_roundtrips_through_capsule_bytes() {
        let original = ActorSoul::genesis("RoundTrip", 0x1234);
        let bytes    = original.to_capsule_bytes();
        let restored = ActorSoul::from_capsule_bytes(&bytes)
            .expect("Should deserialise cleanly");
        assert_eq!(original.bdna_hex(), restored.bdna_hex());
        assert_eq!(original.name,       restored.name);
        assert_eq!(original.faction,    restored.faction);
    }
}
