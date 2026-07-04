// tick.rs — Pure simulation functions for the Xyrona Prime council.
//
// All functions take AgentCouncil by mutable (or shared) reference.
// None of these are methods on ForgeModule — they are stateless logic
// called in sequence by ForgeModule::run_tick().

use std::collections::HashMap;
use myth_agents::{AgentCouncil, LifecyclePhase, Race, SoulWeight};

// ── Lifecycle ─────────────────────────────────────────────────────────────────

/// Advance the council lifecycle phase based on tick counts and soul weight.
///
/// Phase transitions (deterministic, tick-counted):
///   Meditation  → Ideation     after 10 ticks
///   Ideation    → Fulfillment  after 20 ticks
///   Fulfillment → Harvest      when total_soul_weight.is_breakthrough()
///   Harvest     → Meditation   after 5 ticks
pub fn advance_lifecycle(council: &mut AgentCouncil) {
    let next = match council.phase {
        LifecyclePhase::Meditation if council.phase_tick >= 10 => {
            Some(LifecyclePhase::Ideation)
        }
        LifecyclePhase::Ideation if council.phase_tick >= 20 => {
            Some(LifecyclePhase::Fulfillment)
        }
        LifecyclePhase::Fulfillment if council.total_soul_weight.is_breakthrough() => {
            Some(LifecyclePhase::Harvest)
        }
        LifecyclePhase::Harvest if council.phase_tick >= 5 => {
            // Harvest → Meditation: seal bonus + reset cycle
            // RULE-EQUIL-02: chronicle seal bonus
            council.total_world_resonance += 0.008;
            council.total_soul_weight = SoulWeight(0.0);
            Some(LifecyclePhase::Meditation)
        }
        _ => None,
    };

    if let Some(phase) = next {
        council.phase      = phase;
        council.phase_tick = 0;
    }
}

// ── Soul Weight ───────────────────────────────────────────────────────────────

/// Accumulate soul weight from department synergies during Fulfillment.
///
/// For each agent pair sharing a Department:
///   synergy = ((crtv_a + crtv_b) / 2) * (1 + trust) * stability_a * stability_b
///   total_soul_weight += synergy * 0.1
pub fn update_soul_weight(council: &mut AgentCouncil) {
    if council.phase != LifecyclePhase::Fulfillment {
        return;
    }

    let n = council.agents.len();
    let mut delta = 0.0f32;

    for i in 0..n {
        for j in (i + 1)..n {
            if council.agents[i].department != council.agents[j].department {
                continue;
            }
            let crtv_a = council.agents[i].neural.crtv;
            let crtv_b = council.agents[j].neural.crtv;
            let stab_a = council.agents[i].race.soul_weight_stability();
            let stab_b = council.agents[j].race.soul_weight_stability();

            let id_b = council.agents[j].id.clone();
            let trust = council.agents[i]
                .trust_scores
                .get(&id_b)
                .copied()
                .unwrap_or(0.0);

            let synergy = ((crtv_a + crtv_b) / 2.0) * (1.0 + trust) * stab_a * stab_b;
            delta += synergy * 0.1;
        }
    }

    council.total_soul_weight.0 += delta;
}

// ── Trust ─────────────────────────────────────────────────────────────────────

/// Build or decay trust between agents who share a Department.
///
///   Fulfillment: trust += 0.01  (clamped to 1.0)
///   Meditation:  trust -= 0.001 (floored at 0.0)
pub fn update_trust(council: &mut AgentCouncil) {
    let delta: f32 = match council.phase {
        LifecyclePhase::Fulfillment =>  0.01,
        LifecyclePhase::Meditation  => -0.001,
        _                           =>  0.0,
    };

    if delta == 0.0 {
        return;
    }

    let n = council.agents.len();

    // Collect (i, j, dept_match) first to avoid double-borrow
    let pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .filter(|&(i, j)| council.agents[i].department == council.agents[j].department)
        .collect();

    for (i, j) in pairs {
        let id_j = council.agents[j].id.clone();
        let id_i = council.agents[i].id.clone();

        let score_ij = council.agents[i]
            .trust_scores
            .get(&id_j)
            .copied()
            .unwrap_or(0.0);
        let score_ji = council.agents[j]
            .trust_scores
            .get(&id_i)
            .copied()
            .unwrap_or(0.0);

        council.agents[i]
            .trust_scores
            .insert(id_j, (score_ij + delta).clamp(0.0, 1.0));
        council.agents[j]
            .trust_scores
            .insert(id_i, (score_ji + delta).clamp(0.0, 1.0));
    }
}

// ── Emotions ──────────────────────────────────────────────────────────────────

/// Exchange emotion values between canonical partner pairs during Ideation.
///
/// Canonical exchange pairs (fixed by role):
///   Vaelindra  + Thalindre  → average wonder
///   Ashoren    + Noxaren    → average memory
///   Thravex    + Kolthren   → average tension + passion
///   Sorvaine   + Sylvaeth   → average joy
///   Hyvrael    (no pair)    → tension decays 0.02/tick (tidal meditation)
pub fn update_emotions(council: &mut AgentCouncil) {
    if council.phase != LifecyclePhase::Ideation {
        return;
    }

    exchange_wonder(council,  "vaelindra", "thalindre");
    exchange_memory(council,  "ashoren",   "noxaren");
    exchange_tension_passion(council, "thravex", "kolthren");
    exchange_joy(council,     "sorvaine",  "sylvaeth");
    decay_hyvrael_tension(council);
}

fn exchange_wonder(council: &mut AgentCouncil, id_a: &str, id_b: &str) {
    let (ia, ib) = match (council.index_of(id_a), council.index_of(id_b)) {
        (Some(a), Some(b)) => (a, b),
        _ => return,
    };
    let avg = (council.agents[ia].emotions.wonder + council.agents[ib].emotions.wonder) / 2.0;
    council.agents[ia].emotions.wonder = avg;
    council.agents[ib].emotions.wonder = avg;
}

fn exchange_memory(council: &mut AgentCouncil, id_a: &str, id_b: &str) {
    let (ia, ib) = match (council.index_of(id_a), council.index_of(id_b)) {
        (Some(a), Some(b)) => (a, b),
        _ => return,
    };
    let avg = (council.agents[ia].emotions.memory + council.agents[ib].emotions.memory) / 2.0;
    council.agents[ia].emotions.memory = avg;
    council.agents[ib].emotions.memory = avg;
}

fn exchange_tension_passion(council: &mut AgentCouncil, id_a: &str, id_b: &str) {
    let (ia, ib) = match (council.index_of(id_a), council.index_of(id_b)) {
        (Some(a), Some(b)) => (a, b),
        _ => return,
    };
    let avg_t = (council.agents[ia].emotions.tension + council.agents[ib].emotions.tension) / 2.0;
    let avg_p = (council.agents[ia].emotions.passion + council.agents[ib].emotions.passion) / 2.0;
    council.agents[ia].emotions.tension = avg_t;
    council.agents[ib].emotions.tension = avg_t;
    council.agents[ia].emotions.passion = avg_p;
    council.agents[ib].emotions.passion = avg_p;
}

fn exchange_joy(council: &mut AgentCouncil, id_a: &str, id_b: &str) {
    let (ia, ib) = match (council.index_of(id_a), council.index_of(id_b)) {
        (Some(a), Some(b)) => (a, b),
        _ => return,
    };
    let avg = (council.agents[ia].emotions.joy + council.agents[ib].emotions.joy) / 2.0;
    council.agents[ia].emotions.joy = avg;
    council.agents[ib].emotions.joy = avg;
}

fn decay_hyvrael_tension(council: &mut AgentCouncil) {
    if let Some(i) = council.index_of("hyvrael") {
        council.agents[i].emotions.tension =
            (council.agents[i].emotions.tension - 0.02).max(0.0);
    }
}

// ── Cultural Dominance ────────────────────────────────────────────────────────

/// Count agents per Race, normalize to 0.0–1.0 fractions.
pub fn compute_cultural_dominance(council: &mut AgentCouncil) {
    let mut counts: HashMap<Race, u32> = HashMap::new();
    for agent in &council.agents {
        *counts.entry(agent.race).or_insert(0) += 1;
    }
    let total = council.agents.len() as f32;
    council.cultural_dominance = counts
        .into_iter()
        .map(|(race, count)| (race, count as f32 / total))
        .collect();
}

// ── Checks (used by AgentCouncil::emergence_report internally) ────────────────

/// Returns true if any agent tension > 0.95 OR resonance rate > 720.0.
pub fn check_vitrification(council: &AgentCouncil) -> bool {
    council.agents.iter().any(|a| a.emotions.tension > 0.95)
        || (council.total_world_resonance / (council.tick as f32 + 1.0) > 720.0)
}

/// Returns true when Vaelindra tension > 0.8 AND Hyvrael is not in Fulfillment.
/// Hydralis must be active when the Luminarite director crystallizes.
pub fn check_council_summons(council: &AgentCouncil) -> bool {
    let vaelindra_tense = council
        .find("vaelindra")
        .map(|v| v.emotions.tension > 0.8)
        .unwrap_or(false);
    vaelindra_tense && council.phase != LifecyclePhase::Fulfillment
}

/// Hydralis frequency floor is active when Hyvrael is in Fulfillment and stable.
pub fn check_hydralis_floor(council: &AgentCouncil) -> bool {
    council
        .find("hyvrael")
        .map(|h| council.phase == LifecyclePhase::Fulfillment && h.emotions.tension < 0.8)
        .unwrap_or(false)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use myth_agents::AgentCouncil;

    #[test]
    fn meditation_transitions_to_ideation_at_tick_10() {
        let mut c = AgentCouncil::new();
        assert_eq!(c.phase, LifecyclePhase::Meditation);
        // advance_lifecycle runs before tick(), so it sees phase_tick 0..9 in iterations
        // 1-10. The transition fires on iteration 11 when phase_tick=10 is seen.
        for _ in 0..11 {
            advance_lifecycle(&mut c);
            c.tick();
        }
        assert_eq!(c.phase, LifecyclePhase::Ideation);
    }

    #[test]
    fn fulfillment_transitions_on_breakthrough() {
        let mut c = AgentCouncil::new();
        c.phase = LifecyclePhase::Fulfillment;
        c.phase_tick = 0;
        c.total_soul_weight = SoulWeight(2.9);
        advance_lifecycle(&mut c);
        assert_eq!(c.phase, LifecyclePhase::Fulfillment); // not yet
        c.total_soul_weight = SoulWeight(3.0);
        advance_lifecycle(&mut c);
        assert_eq!(c.phase, LifecyclePhase::Harvest);
    }

    #[test]
    fn harvest_seal_increments_world_resonance() {
        let mut c = AgentCouncil::new();
        c.phase = LifecyclePhase::Harvest;
        c.phase_tick = 5;
        let before = c.total_world_resonance;
        advance_lifecycle(&mut c);
        assert_eq!(c.phase, LifecyclePhase::Meditation);
        assert!((c.total_world_resonance - before - 0.008).abs() < 1e-6);
        assert_eq!(c.total_soul_weight.0, 0.0);
    }

    #[test]
    fn trust_builds_in_fulfillment() {
        let mut c = AgentCouncil::new();
        c.phase = LifecyclePhase::Fulfillment;
        update_trust(&mut c);
        // Sorvaine and Sylvaeth share Guardians — should have trust
        let t = c.trust_between("sorvaine", "sylvaeth");
        assert!(t > 0.0, "expected trust to build between Guardians");
    }

    #[test]
    fn hydralis_floor_active_in_fulfillment() {
        let mut c = AgentCouncil::new();
        c.phase = LifecyclePhase::Fulfillment;
        assert!(check_hydralis_floor(&c));
    }

    #[test]
    fn cultural_dominance_sums_to_one() {
        let mut c = AgentCouncil::new();
        compute_cultural_dominance(&mut c);
        let sum: f32 = c.cultural_dominance.values().sum();
        assert!((sum - 1.0).abs() < 1e-5, "dominance sum = {sum}");
    }
}
