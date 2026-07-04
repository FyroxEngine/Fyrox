// GEN-ATOM-09: Instinct Tree Resolver — autonomous agent wandering + soul wiring.
//
// Phase 3: WanderBrain component, inline LCG, random target every 3–5 s.
// Phase 4 (Phantori): SoulRef + SoulStore — each actor is backed by an ActorSoul.
//   The conscious/subconscious split ratio (soul.conscious_fraction()) is written
//   to a ConsciousnessState component each tick for the inspector UI to read.

use bevy::prelude::*;
use std::collections::HashMap;

use mythos::soul::{ActorSoul, ActorIntent, PersonalEngine};

use super::entity_instantiator::QuillActor;

// ── Soul storage ──────────────────────────────────────────────────────────────

/// Maps actor entity (by its myth_id string) to its full ActorSoul.
#[derive(Resource, Default)]
pub struct SoulStore {
    pub souls: HashMap<String, ActorSoul>,
}

impl SoulStore {
    pub fn insert(&mut self, soul: ActorSoul) {
        self.souls.insert(soul.id.as_str(), soul);
    }
    pub fn get(&self, myth_id: &str) -> Option<&ActorSoul> {
        self.souls.get(myth_id)
    }
    pub fn get_mut(&mut self, myth_id: &str) -> Option<&mut ActorSoul> {
        self.souls.get_mut(myth_id)
    }
}

// ── SocialGraph ───────────────────────────────────────────────────────────────

/// SOC-wire bond graph between actor souls in the current world.
#[derive(Resource, Default)]
pub struct SocialGraph {
    pub bonds: Vec<mythos::soul::SocialBond>,
}

impl SocialGraph {
    pub fn bond_between(&self, a: &str, b: &str) -> Option<&mythos::soul::SocialBond> {
        self.bonds.iter().find(|bnd| {
            (bnd.source.as_str() == a && bnd.target.as_str() == b)
            || (bnd.source.as_str() == b && bnd.target.as_str() == a)
        })
    }

    pub fn update_or_create(
        &mut self,
        source: mythos::identity::MythId,
        target: mythos::identity::MythId,
        delta:  f32,
    ) {
        let source_str = source.as_str();
        let target_str = target.as_str();
        if let Some(bnd) = self.bonds.iter_mut().find(|b| {
            (b.source.as_str() == source_str && b.target.as_str() == target_str)
            || (b.source.as_str() == target_str && b.target.as_str() == source_str)
        }) {
            bnd.reinforce(delta);
        } else {
            use mythos::soul::{BondType, SocialBond};
            let mut bnd = SocialBond::new(source, target, BondType::Neutral);
            bnd.reinforce(delta);
            self.bonds.push(bnd);
        }
    }
}

// ── SoulRef component ─────────────────────────────────────────────────────────

/// Links a QuillActor ECS entity to its soul in `SoulStore`.
#[derive(Component, Debug, Clone)]
pub struct SoulRef {
    pub soul_id: String,
}

/// Tracks the split between subconscious-driven and conscious-driven behaviour.
/// Updated each tick; read by the actor inspector UI.
#[derive(Component, Default)]
pub struct ConsciousnessState {
    /// 0.0 = fully subconscious, 1.0 = fully conscious
    pub conscious_fraction: f32,
    /// True when the soul requested conscious escalation this tick
    pub escalated:          bool,
    /// Most recent intent label for the inspector
    pub last_intent:        String,
}

// ── WanderBrain component ─────────────────────────────────────────────────────

#[derive(Component)]
pub struct WanderBrain {
    pub target:   Vec3,
    pub timer:    f32,
    pub interval: f32,
    pub speed:    f32,
    rng:          u64,
}

impl WanderBrain {
    fn new(seed: u64, wanderlust: f32) -> Self {
        let mut rng    = seed;
        let target     = random_ground_point(&mut rng);
        let interval   = 3.0 + rng_f32(&mut rng) * 2.0;
        // Speed scales with wanderlust trait [0.5 … 4.0]
        let speed      = 0.5 + wanderlust * 3.5;
        Self { target, timer: 0.0, interval, speed, rng }
    }
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct InstinctPlugin;

impl Plugin for InstinctPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoulStore>()
            .init_resource::<SocialGraph>()
            .add_systems(
                Update,
                // Ordering: init souls first, then brains, then tick souls, then wander
                (init_souls, init_wander_brains, tick_souls, wander_system)
                    .chain(),
            );
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Create an `ActorSoul` and `SoulRef` for every QuillActor that doesn't have one.
fn init_souls(
    mut commands: Commands,
    mut store:    ResMut<SoulStore>,
    actors: Query<(Entity, &QuillActor, &Transform), Without<SoulRef>>,
) {
    for (entity, actor, tf) in actors.iter() {
        // Fold genotype traits into the seed so B-DNA reflects the designed personality,
        // not just spawn position. Two actors at the same spot with different
        // curiosity/aggression values will diverge into genuinely different souls.
        let seed = tf.translation.x.to_bits() as u64
            ^ (tf.translation.z.to_bits() as u64).wrapping_mul(0xdeadbeef)
            ^ (actor.genotype.curiosity.to_bits() as u64).wrapping_mul(0x9e3779b97f4a7c15)
            ^ (actor.genotype.aggression.to_bits() as u64).wrapping_mul(0x517cc1b727220a95);

        let soul = ActorSoul::genesis(&actor.name, seed);
        let soul_id = soul.id.as_str();

        commands.entity(entity).insert((
            SoulRef { soul_id: soul_id.clone() },
            ConsciousnessState::default(),
        ));

        store.insert(soul);
    }
}

/// Insert WanderBrain on any QuillActor that has a SoulRef but no WanderBrain yet.
fn init_wander_brains(
    mut commands: Commands,
    store:  Res<SoulStore>,
    actors: Query<(Entity, &SoulRef, &Transform), (With<QuillActor>, Without<WanderBrain>)>,
) {
    for (entity, soul_ref, tf) in actors.iter() {
        let wanderlust = store.get(&soul_ref.soul_id)
            .map(|s| s.subconscious.drive(mythos::soul::TraitAxis::Wanderlust))
            .unwrap_or(0.5);

        let seed = tf.translation.x.to_bits() as u64
            ^ (tf.translation.z.to_bits() as u64).wrapping_mul(0xabcdef01);

        commands.entity(entity).insert(WanderBrain::new(seed, wanderlust));
    }
}

/// Tick every soul's PersonalEngine; write results to ConsciousnessState.
fn tick_souls(
    time:     Res<Time>,
    mut store: ResMut<SoulStore>,
    mut q:    Query<(&SoulRef, &mut ConsciousnessState)>,
) {
    let dt = time.delta_seconds();
    for (soul_ref, mut state) in q.iter_mut() {
        let Some(soul) = store.get_mut(&soul_ref.soul_id) else { continue };

        let intents = soul.tick(dt);

        state.conscious_fraction = soul.conscious_fraction();
        state.escalated          = false;
        state.last_intent        = "Idle".into();

        for intent in &intents {
            match intent {
                ActorIntent::Escalate { reason } => {
                    state.escalated   = true;
                    state.last_intent = format!("↑ {}", reason);
                }
                ActorIntent::Rest { .. } => {
                    state.last_intent = "Resting".into();
                }
                ActorIntent::MoveTo { .. } => {
                    state.last_intent = "Moving".into();
                }
                _ => {}
            }
        }
    }
}

/// Steer every actor with a WanderBrain toward its current target.
fn wander_system(
    time:  Res<Time>,
    store: Res<SoulStore>,
    mut q: Query<(&mut Transform, &mut WanderBrain, &SoulRef, &ConsciousnessState)>,
) {
    let dt = time.delta_seconds();

    for (mut tf, mut brain, soul_ref, cstate) in q.iter_mut() {
        // If the soul is escalated (conscious override), pause wandering
        if cstate.escalated { continue; }

        brain.timer += dt;
        if brain.timer >= brain.interval {
            brain.timer    = 0.0;
            brain.target   = random_ground_point(&mut brain.rng);
            brain.interval = 3.0 + rng_f32(&mut brain.rng) * 2.0;

            // Re-read wanderlust in case it changed
            if let Some(soul) = store.get(&soul_ref.soul_id) {
                let wl = soul.subconscious.drive(mythos::soul::TraitAxis::Wanderlust);
                brain.speed = 0.5 + wl * 3.5;
            }
        }

        let flat_pos    = Vec3::new(tf.translation.x, 0.0, tf.translation.z);
        let flat_target = Vec3::new(brain.target.x,   0.0, brain.target.z);
        let dir         = (flat_target - flat_pos).normalize_or_zero();

        if (flat_target - flat_pos).length() < 1.0 { continue; }

        tf.translation.x += dir.x * brain.speed * dt;
        tf.translation.z += dir.z * brain.speed * dt;
        tf.translation.y  = 1.0;

        if dir.length_squared() > 0.0001 {
            tf.rotation = Quat::from_rotation_y(dir.x.atan2(dir.z));
        }
    }
}

// ── Inline LCG ───────────────────────────────────────────────────────────────

#[inline]
fn lcg_step(s: u64) -> u64 {
    s.wrapping_mul(6_364_136_223_846_793_005)
     .wrapping_add(1_442_695_040_888_963_407)
}

#[inline]
fn rng_f32(s: &mut u64) -> f32 {
    *s = lcg_step(*s);
    ((*s >> 33) as f32) / (u32::MAX as f32)
}

fn random_ground_point(rng: &mut u64) -> Vec3 {
    let x = (rng_f32(rng) * 2.0 - 1.0) * 45.0;
    let z = (rng_f32(rng) * 2.0 - 1.0) * 45.0;
    Vec3::new(x, 1.0, z)
}
