// ─────────────────────────────────────────────────────────────────────────────
// Genesis Boot Sequence
//
// The cinematic world-initialization protocol. Plays after INITIATE GENESIS,
// before the vault opens. 7 phases driven entirely by egui Painter — no 3D,
// just light, time, and the data the vault already contains.
//
// Phase timeline (seconds from boot start):
//   0.0 – 3.0  CALIBRATION   4 bus channels power up one by one
//   3.0 – 5.0  CHARGE        Energy streams converge at the center
//   5.0 – 6.0  IGNITION      One explosive moment of white light
//   6.0 – 8.0  STAR          A proto-star condenses from the chaos
//   8.0 – 11.0 ACCRETION     The star collapses; a planet forms
//  11.0 – 13.0 EMERGENCE     City lights blink on across the surface
//  13.0 – 17.0 SETTLING      The world becomes a roulette ball — and picks its number
//  17.0 +      COMPLETE      Transition to VaultView
// ─────────────────────────────────────────────────────────────────────────────

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Align2, Color32, FontId, Pos2, Rect, Rounding, Sense, Stroke, pos2, vec2};
use std::f32::consts::TAU;
use uuid::Uuid;

use crate::state::AppScreen;
use crate::vault_store::{SelectedVault, VaultStore, bdna_to_bits};
use super::UiSet;
use super::theme::{a, VOID, GOLD, GOLD_LT};

// ── Phase boundary constants ──────────────────────────────────────────────────

const T_CALIBRATION_END: f32 = 3.0;
const T_CHARGE_END:      f32 = 5.0;
const T_IGNITION_END:    f32 = 6.0;
const T_STAR_END:        f32 = 8.0;
const T_ACCRETION_END:   f32 = 11.0;
const T_EMERGENCE_END:   f32 = 13.0;
const T_SETTLING_END:    f32 = 17.0;

// ── Bus definitions ───────────────────────────────────────────────────────────

struct BusDef {
    name:  &'static str,
    glyph: &'static str,
    col:   Color32,
}

const BUSES: [BusDef; 4] = [
    BusDef { name: "STRUCTURE",  glyph: "◈", col: Color32::from_rgb(212, 160,  48) },
    BusDef { name: "ENTITIES",   glyph: "✦", col: Color32::from_rgb(140,  80, 255) },
    BusDef { name: "ATMOSPHERE", glyph: "⊗", col: Color32::from_rgb(  0, 200, 180) },
    BusDef { name: "DYNAMICS",   glyph: "⚒", col: Color32::from_rgb(255, 100,   0) },
];

// ── State resource ────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GenesisBootState {
    pub boot_elapsed:      f32,
    #[allow(dead_code)]   // retained for future vault-introspection hooks
    pub vault_id:          Option<Uuid>,
    pub vault_name:       String,
    pub vault_col:        Color32,
    pub bdna_sig:         String,
    pub bdna_bits:        [bool; 64],
    // Roulette settling
    pub roulette_pos:     f32,   // current angle around the orbit (radians)
    pub roulette_vel:     f32,   // angular velocity (rad/s)
    pub roulette_target:  f32,   // final resting angle
    pub roulette_settled: bool,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct GenesisBootPlugin;

impl Plugin for GenesisBootPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppScreen::GenesisBootSequence), init_boot_state)
            .add_systems(OnExit(AppScreen::GenesisBootSequence),  cleanup_boot_state)
            .add_systems(
                Update,
                draw_genesis_boot
                    .run_if(in_state(AppScreen::GenesisBootSequence))
                    .in_set(UiSet::Page),
            );
    }
}

// ── Lifecycle systems ─────────────────────────────────────────────────────────

fn init_boot_state(
    selected: Res<SelectedVault>,
    store:    Res<VaultStore>,
    mut cmd:  Commands,
) {
    let vault = selected.0.and_then(|id| store.by_id(id));

    let (vault_name, vault_col, bdna_sig) = vault
        .map(|v| (v.name.clone(), v.color, v.bdna_signature.clone()))
        .unwrap_or_else(|| ("Unnamed World".into(), GOLD, String::new()));

    let bdna_bits = bdna_to_bits(&bdna_sig);

    // Pick the roulette landing slot from the vault's B-DNA.
    let ones = bdna_bits[..8].iter().filter(|&&b| b).count();
    let target_slot = ones % 8;
    let roulette_target = target_slot as f32 * TAU / 8.0 - TAU / 4.0; // top = -π/2

    cmd.insert_resource(GenesisBootState {
        boot_elapsed:     0.0,
        vault_id:         selected.0,
        vault_name,
        vault_col,
        bdna_sig,
        bdna_bits,
        roulette_pos:     -TAU / 4.0, // start at top
        roulette_vel:     TAU * 2.8,  // 2.8 full rotations per second
        roulette_target,
        roulette_settled: false,
    });
}

fn cleanup_boot_state(mut cmd: Commands) {
    cmd.remove_resource::<GenesisBootState>();
}

// ── Main draw system ──────────────────────────────────────────────────────────

fn draw_genesis_boot(
    mut contexts: EguiContexts,
    mut state:    ResMut<GenesisBootState>,
    time:         Res<Time>,
    mut next:     ResMut<NextState<AppScreen>>,
) {
    let dt = time.delta_seconds();
    state.boot_elapsed += dt;
    let t = state.boot_elapsed;

    // Update roulette physics during settling
    if t >= T_EMERGENCE_END && !state.roulette_settled {
        state.roulette_vel *= 0.93_f32.powf(dt * 60.0);
        state.roulette_pos += state.roulette_vel * dt;
        if state.roulette_vel < 0.12 {
            state.roulette_pos    = state.roulette_target;
            state.roulette_vel    = 0.0;
            state.roulette_settled = true;
        }
    }

    // Transition when done
    if t >= T_SETTLING_END {
        next.set(AppScreen::VaultView);
        return;
    }

    let ctx = contexts.ctx_mut();
    ctx.request_repaint(); // drive smooth animation every frame

    let mut skip = false;

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(VOID))
        .show(ctx, |ui| {
            let rect   = ui.max_rect();
            let center = rect.center();
            let p      = ui.painter().clone();

            // Dispatch to current phase painter
            if t < T_CALIBRATION_END {
                paint_calibration(&p, rect, center, t);
            } else if t < T_CHARGE_END {
                paint_charge(&p, rect, center, t - T_CALIBRATION_END);
            } else if t < T_IGNITION_END {
                paint_ignition(&p, rect, center, t - T_CHARGE_END);
            } else if t < T_STAR_END {
                paint_star(&p, rect, center, t - T_IGNITION_END, &state);
            } else if t < T_ACCRETION_END {
                paint_accretion(&p, rect, center, t - T_STAR_END, &state);
            } else if t < T_EMERGENCE_END {
                paint_emergence(&p, rect, center, t - T_ACCRETION_END, &state);
            } else {
                paint_settling(&p, rect, center, t - T_EMERGENCE_END, &state);
            }

            // Skip button — appears after 2 seconds, always in bottom-right
            if t > 2.0 {
                let btn_fade = ((t - 2.0) / 0.8).min(1.0);
                let skip_r = Rect::from_min_size(
                    pos2(rect.right() - 90.0, rect.bottom() - 34.0),
                    vec2(78.0, 22.0),
                );
                let resp    = ui.interact(skip_r, ui.id().with("skip"), Sense::click());
                let hov     = resp.hovered();
                let ba      = (btn_fade * if hov { 120.0 } else { 55.0 }) as u8;
                let pa      = ui.painter_at(skip_r);
                pa.rect_stroke(skip_r, Rounding::same(3.0), Stroke::new(1.0, a(GOLD, ba)));
                pa.text(skip_r.center(), Align2::CENTER_CENTER,
                    "SKIP  →", FontId::monospace(9.0), a(GOLD, ba));
                if resp.clicked() { skip = true; }
            }
        });

    if skip {
        next.set(AppScreen::VaultView);
    }
}

// ── Phase 0 — CALIBRATION ─────────────────────────────────────────────────────

fn paint_calibration(p: &egui::Painter, rect: Rect, center: Pos2, t: f32) {
    p.rect_filled(rect, Rounding::ZERO, VOID);

    // Faint grid
    let ga = (t / T_CALIBRATION_END * 10.0) as u8;
    paint_grid(p, rect, 48.0, a(GOLD, ga));

    // Header
    let hdr_a = (t / 0.6 * 255.0).min(255.0) as u8;
    p.text(pos2(center.x, rect.top() + 44.0), Align2::CENTER_CENTER,
        "⬡  GENESIS PROTOCOL — INITIATING",
        FontId::monospace(11.0), a(GOLD, hdr_a));

    // Sub-header
    p.text(pos2(center.x, rect.top() + 64.0), Align2::CENTER_CENTER,
        "CALIBRATING QUANTUM BUS ARRAY",
        FontId::monospace(8.0), a(GOLD, (hdr_a as f32 * 0.45) as u8));

    // Four bus panels — vertical stack
    let panel_w = 440.0_f32;
    let panel_h = 62.0_f32;
    let gap     = 14.0_f32;
    let total_h = 4.0 * panel_h + 3.0 * gap;
    let start_y = center.y - total_h * 0.5 - 10.0;
    let start_x = center.x - panel_w * 0.5;

    for (i, bus) in BUSES.iter().enumerate() {
        let bus_start = i as f32 * 0.75;
        let bus_end   = bus_start + 0.75;
        let progress  = if t >= bus_start {
            ((t - bus_start) / 0.75).min(1.0)
        } else { 0.0 };
        let is_online  = t >= bus_end;
        let appear     = ((t - bus_start + 0.4) / 0.5).clamp(0.0, 1.0);

        if appear <= 0.01 { continue; }

        let py   = start_y + i as f32 * (panel_h + gap);
        let pr   = Rect::from_min_size(pos2(start_x, py), vec2(panel_w, panel_h));
        let col  = bus.col;

        // Panel bg + border
        let border_a = if is_online { 120_u8 } else { 40 };
        let glow_a   = if is_online { (0.5 + 0.5 * (t * 2.0).sin() * 0.3) * 24.0 } else { 0.0 };
        p.rect_filled(pr, Rounding::same(5.0),
            Color32::from_rgba_unmultiplied(10, 8, 6, (appear * 220.0) as u8));
        p.rect_stroke(pr.expand(2.0), Rounding::same(7.0),
            Stroke::new(5.0, a(col, (appear * glow_a) as u8)));
        p.rect_stroke(pr, Rounding::same(5.0),
            Stroke::new(1.0, a(col, (appear * border_a as f32) as u8)));

        // Glyph
        p.text(pos2(pr.left() + 28.0, pr.center().y), Align2::CENTER_CENTER,
            bus.glyph, FontId::proportional(20.0),
            a(col, (appear * 200.0) as u8));

        // Bus name
        p.text(pos2(pr.left() + 56.0, pr.top() + 18.0), Align2::LEFT_CENTER,
            bus.name, FontId::monospace(10.0),
            a(col, (appear * 180.0) as u8));

        // Status
        let status = if is_online { "ONLINE  ✓" }
                     else if progress > 0.0 { "CALIBRATING…" }
                     else { "STANDBY" };
        let status_a = if is_online { 200_u8 } else { 100 };
        p.text(pos2(pr.right() - 12.0, pr.top() + 18.0), Align2::RIGHT_CENTER,
            status, FontId::monospace(9.0),
            a(col, (appear * status_a as f32) as u8));

        // Progress bar track
        let bar_l = pr.left() + 56.0;
        let bar_r = pr.right() - 12.0;
        let bar_y = pr.bottom() - 18.0;
        let bar_w = bar_r - bar_l;
        p.line_segment(
            [pos2(bar_l, bar_y), pos2(bar_r, bar_y)],
            Stroke::new(2.0, a(col, (appear * 25.0) as u8)),
        );

        // Progress bar fill
        if progress > 0.0 {
            let fill_r = bar_l + bar_w * progress;
            // Shimmer: bright leading edge
            if !is_online {
                p.line_segment(
                    [pos2(bar_l, bar_y), pos2(fill_r, bar_y)],
                    Stroke::new(2.0, a(col, (appear * 80.0) as u8)),
                );
                // Bright head
                let head_a = 220_u8;
                p.circle_filled(pos2(fill_r, bar_y), 3.0, a(col, (appear * head_a as f32) as u8));
            } else {
                // Fully lit bar
                let blink = 0.85 + 0.15 * (t * 1.8 + i as f32).sin();
                p.line_segment(
                    [pos2(bar_l, bar_y), pos2(bar_r, bar_y)],
                    Stroke::new(3.0, a(col, (appear * blink * 160.0) as u8)),
                );
            }
        }

        // Channel number
        p.text(pos2(pr.left() + 8.0, pr.bottom() - 10.0), Align2::LEFT_CENTER,
            &format!("CH{}", i + 1), FontId::monospace(7.0),
            a(col, (appear * 60.0) as u8));
    }

    // Footer
    let f_a = ((t - 0.5) / 0.8 * 60.0).clamp(0.0, 60.0) as u8;
    p.text(pos2(center.x, rect.bottom() - 16.0), Align2::CENTER_CENTER,
        "BIOSPARK STUDIOS  ·  QUANTUM ECOSYSTEM  ·  v0.1.0",
        FontId::monospace(8.0), a(GOLD, f_a));

    corner_marks(p, rect, 18.0, 14.0, Stroke::new(1.0, a(GOLD, (t / T_CALIBRATION_END * 55.0) as u8)));
}

// ── Phase 1 — CHARGE ─────────────────────────────────────────────────────────

fn paint_charge(p: &egui::Painter, rect: Rect, center: Pos2, t: f32) {
    // t: 0→2
    p.rect_filled(rect, Rounding::ZERO, VOID);
    paint_grid(p, rect, 48.0, a(GOLD, 8));

    let charge_pct = t / 2.0; // 0→1

    // Bus indicators — four small icons at cardinal positions
    let orbit = 210.0_f32;
    let bus_angles = [-TAU / 4.0, 0.0, TAU / 4.0, TAU / 2.0]; // N E S W
    for (i, (&angle, bus)) in bus_angles.iter().zip(BUSES.iter()).enumerate() {
        let bx = center.x + orbit * angle.cos();
        let by = center.y + orbit * angle.sin();
        let blink = 0.7 + 0.3 * ((t * 1.4 + i as f32 * 0.8) * TAU).sin();
        let col = bus.col;

        // Glow blob
        for (r, ba) in [(35.0_f32, 18_u8), (22.0, 30), (13.0, 50)] {
            p.circle_filled(pos2(bx, by), r, a(col, (ba as f32 * blink) as u8));
        }
        p.text(pos2(bx, by), Align2::CENTER_CENTER,
            bus.glyph, FontId::proportional(16.0),
            a(col, (200.0 * blink) as u8));

        // Energy stream — quadratic bezier toward center
        // Control point halfway between bus and center, offset perpendicular
        let perp_x = -(by - center.y).signum() * orbit * 0.3;
        let perp_y =  (bx - center.x).signum() * orbit * 0.3;
        let ctrl   = pos2(
            (bx + center.x) * 0.5 + perp_x,
            (by + center.y) * 0.5 + perp_y,
        );
        let dash_offset = (t * 0.7 + i as f32 * 0.25) % 1.0;
        let n_segs = 28_usize;
        for seg in 0..n_segs {
            let seg_t0 = seg as f32 / n_segs as f32;
            let seg_t1 = (seg + 1) as f32 / n_segs as f32;
            let pt0 = bezier(pos2(bx, by), ctrl, center, seg_t0);
            let pt1 = bezier(pos2(bx, by), ctrl, center, seg_t1);
            // Particle density: bright near the front of the stream
            let particle_pos = (seg_t0 + dash_offset) % 1.0;
            let brightness   = if particle_pos < 0.18 { 1.0 - particle_pos / 0.18 } else { 0.0 };
            let stream_a     = (charge_pct * (30.0 + brightness * 140.0)) as u8;
            p.line_segment([pt0, pt1], Stroke::new(1.5, a(col, stream_a)));
        }
    }

    // Central energy orb
    let pulse = 0.5 + 0.5 * (t * 3.2 * TAU).sin();
    let orb_r = 8.0 + charge_pct * 44.0 + pulse * 6.0;
    for (r, ba) in [
        (orb_r * 3.5, 6_u8),
        (orb_r * 2.2, 14),
        (orb_r * 1.5, 26),
        (orb_r,        60),
        (orb_r * 0.5, 160),
    ] {
        p.circle_filled(center, r, a(GOLD, ba));
    }

    // Charge arc (approximate with line segments)
    let arc_r    = 88.0_f32;
    let arc_segs = 64_usize;
    let arc_end  = -TAU / 4.0 + charge_pct * TAU;
    for seg in 0..arc_segs {
        let a0 = -TAU / 4.0 + seg as f32 / arc_segs as f32 * TAU;
        let a1 = -TAU / 4.0 + (seg + 1) as f32 / arc_segs as f32 * TAU;
        if a0 > arc_end { break; }
        let p0 = pos2(center.x + arc_r * a0.cos(), center.y + arc_r * a0.sin());
        let p1 = pos2(center.x + arc_r * a1.cos(), center.y + arc_r * a1.sin());
        let pct = seg as f32 / arc_segs as f32;
        p.line_segment([p0, p1], Stroke::new(2.5, a(GOLD, (140.0 * (0.4 + 0.6 * pct)) as u8)));
    }

    // Charge percentage
    p.text(pos2(center.x, center.y - arc_r - 18.0), Align2::CENTER_CENTER,
        &format!("CHARGE  {:>3.0}%", charge_pct * 100.0),
        FontId::monospace(10.0), a(GOLD, 180));

    // Data readout
    let data_lines = [
        "RESONANCE:  174.6 Hz",
        "COHERENCE:  NOMINAL",
        "SYNC:       PHASE-LOCKED",
        "CAPACITOR:  CHARGING",
    ];
    for (i, line) in data_lines.iter().enumerate() {
        let scroll_a = if (t * 8.0 + i as f32 * 2.3) as usize % 7 == 0 { 40_u8 } else { 80 };
        p.text(
            pos2(rect.left() + 24.0, rect.bottom() - 80.0 + i as f32 * 16.0),
            Align2::LEFT_CENTER,
            line, FontId::monospace(9.0), a(GOLD, scroll_a),
        );
    }

    corner_marks(p, rect, 18.0, 14.0, Stroke::new(1.0, a(GOLD, 50)));
}

// ── Phase 2 — IGNITION ────────────────────────────────────────────────────────

fn paint_ignition(p: &egui::Painter, rect: Rect, center: Pos2, t: f32) {
    // t: 0→1
    p.rect_filled(rect, Rounding::ZERO, VOID);

    // Phase A (0→0.15): rapid build-up — concentric rings converging
    // Phase B (0.15→0.4): peak flash
    // Phase C (0.4→1.0): fade out with rays

    let peak = if t < 0.15 { t / 0.15 } else if t < 0.4 { 1.0 } else { 1.0 - (t - 0.4) / 0.6 };

    // Full-screen flash
    let flash_a = (peak * peak * 240.0) as u8;
    p.rect_filled(rect, Rounding::ZERO,
        Color32::from_rgba_unmultiplied(255, 215, 80, (flash_a as f32 * 0.55) as u8));

    // Bright core
    for (r, ba) in [
        (rect.width() * peak * 0.7, 10_u8),
        (300.0 * peak, 20),
        (180.0 * peak, 50),
        (80.0 * peak,  120),
        (30.0 * peak,  220),
        (8.0,          255),
    ] {
        p.circle_filled(center, r, Color32::from_rgba_unmultiplied(255, 220, 100, ba));
    }

    // Radial rays (phase C)
    if t > 0.3 {
        let ray_pct = ((t - 0.3) / 0.7).min(1.0);
        let n_rays  = 24_usize;
        for i in 0..n_rays {
            let angle = i as f32 * TAU / n_rays as f32;
            let len   = ray_pct * rect.width() * 0.65;
            let ray_a = ((1.0 - ray_pct) * 180.0) as u8;
            p.line_segment(
                [center, pos2(center.x + len * angle.cos(), center.y + len * angle.sin())],
                Stroke::new(1.0, a(GOLD, ray_a)),
            );
        }
    }
}

// ── Phase 3 — STAR ───────────────────────────────────────────────────────────

fn paint_star(p: &egui::Painter, rect: Rect, center: Pos2, t: f32, state: &GenesisBootState) {
    // t: 0→2
    p.rect_filled(rect, Rounding::ZERO, VOID);

    let appear  = spring_out((t / 1.4).min(1.0));
    let star_y  = center.y - appear * 20.0;
    let star    = pos2(center.x, star_y);
    let col     = state.vault_col;

    // Outer corona — 12 rays rotating
    let n_rays = 12_usize;
    for i in 0..n_rays {
        let angle  = i as f32 * TAU / n_rays as f32 + t * 0.4;
        let base_l = 65.0_f32;
        let len    = base_l + 18.0 * ((t * 1.1 + i as f32 * 0.7) * TAU).sin();
        let ray_a  = (appear * 60.0) as u8;
        p.line_segment(
            [star, pos2(star.x + len * angle.cos(), star.y + len * angle.sin())],
            Stroke::new(1.2, a(col, ray_a)),
        );
    }

    // Outer ring — slow CW spin
    let ring_a = (appear * 160.0) as u8;
    p.circle_stroke(star, 52.0, Stroke::new(1.5, a(col, ring_a)));
    p.circle_stroke(star, 54.0, Stroke::new(5.0, a(col, (ring_a as f32 * 0.2) as u8)));

    // Inner ring — CCW
    let inner_a = (appear * 130.0) as u8;
    p.circle_stroke(star, 34.0, Stroke::new(1.0, a(GOLD, inner_a)));

    // Core glow
    let pulse = 1.0 + 0.18 * (t * 2.4 * TAU).sin();
    for (r, ba) in [(60.0_f32, 18_u8), (40.0, 30), (24.0, 60), (14.0 * pulse, 180), (6.0, 255)] {
        p.circle_filled(star, r, a(col, (appear * ba as f32) as u8));
    }

    // B-DNA hex signature orbiting
    if !state.bdna_sig.is_empty() {
        let sig_r   = 88.0_f32;
        let chars: Vec<char> = state.bdna_sig.chars().take(16).collect();
        let n       = chars.len();
        for (i, ch) in chars.iter().enumerate() {
            let angle = t * 0.22 + i as f32 * TAU / n as f32;
            let cx    = star.x + sig_r * angle.cos();
            let cy    = star.y + sig_r * angle.sin();
            p.text(pos2(cx, cy), Align2::CENTER_CENTER,
                &ch.to_string(), FontId::monospace(8.0),
                a(col, (appear * 80.0) as u8));
        }
    }

    // Label
    let label_a = (appear * 160.0) as u8;
    p.text(pos2(center.x, star_y + 90.0), Align2::CENTER_CENTER,
        "PROTO-STAR FORMING",
        FontId::monospace(9.0), a(GOLD, label_a));
    p.text(pos2(center.x, star_y + 108.0), Align2::CENTER_CENTER,
        &format!("RESONANCE: {:.1} Hz", 174.6),
        FontId::monospace(8.0), a(col, (label_a as f32 * 0.55) as u8));

    corner_marks(p, rect, 18.0, 14.0, Stroke::new(1.0, a(col, 40)));
}

// ── Phase 4 — ACCRETION ───────────────────────────────────────────────────────

fn paint_accretion(p: &egui::Painter, rect: Rect, center: Pos2, t: f32, state: &GenesisBootState) {
    // t: 0→3
    p.rect_filled(rect, Rounding::ZERO, VOID);

    let form    = spring_out((t / 3.0).min(1.0));
    let col     = state.vault_col;
    let planet_r = form * 165.0_f32;

    // Accreting matter streams (early phase)
    if t < 1.5 {
        let matter_pct = t / 1.5;
        let n_streams  = 16_usize;
        for i in 0..n_streams {
            let angle  = i as f32 * TAU / n_streams as f32 + t * 0.3;
            let start_r = 240.0 * (1.0 - matter_pct * 0.7);
            let end_r   = planet_r + 8.0;
            let sx = center.x + start_r * angle.cos();
            let sy = center.y + start_r * angle.sin();
            let ex = center.x + end_r   * angle.cos();
            let ey = center.y + end_r   * angle.sin();
            let stream_a = (matter_pct * 60.0) as u8;
            p.line_segment([pos2(sx, sy), pos2(ex, ey)],
                Stroke::new(1.0, a(col, stream_a)));
        }
    }

    // Atmosphere rings (3 expanding rings around planet)
    for (i, (dr, base_a)) in [(14.0_f32, 30_u8), (22.0, 18), (34.0, 10)].iter().enumerate() {
        let pulse = 0.5 + 0.5 * ((t * 0.9 + i as f32 * 1.3) * TAU).sin();
        let r = planet_r + dr;
        let aa = (form * *base_a as f32 * pulse) as u8;
        p.circle_stroke(center, r, Stroke::new(4.0, a(col, aa)));
    }

    // Planet surface
    p.circle_filled(center, planet_r, a(col, (form * 14.0) as u8));
    p.circle_filled(center, planet_r, Color32::from_rgba_unmultiplied(6, 4, 3, (form * 180.0) as u8));

    // Continents — 8 positions mapped to bdna_bits
    if planet_r > 20.0 {
        let continent_positions: [(f32, f32); 8] = [
            (0.0,  -0.55),  // N
            (0.5,  -0.35),  // NE
            (0.55,  0.0),   // E
            (0.4,   0.45),  // SE
            (0.0,   0.55),  // S
            (-0.45, 0.35),  // SW
            (-0.55, 0.0),   // W
            (-0.35,-0.45),  // NW
        ];

        for (i, &(cx_f, cy_f)) in continent_positions.iter().enumerate() {
            if !state.bdna_bits[i] { continue; }
            let cx = center.x + cx_f * planet_r;
            let cy = center.y + cy_f * planet_r;
            let c_r = planet_r * (0.18 + state.bdna_bits[i + 8] as i32 as f32 * 0.06);
            let land_appear = ((t - 0.8) / 2.2).clamp(0.0, 1.0);
            for (r, ba) in [(c_r * 1.4, 18_u8), (c_r, 45), (c_r * 0.6, 70)] {
                p.circle_filled(pos2(cx, cy), r, a(col, (land_appear * ba as f32) as u8));
            }
        }
    }

    // Latitude / longitude lines (appear after form > 0.55)
    if form > 0.55 {
        let grid_appear = ((form - 0.55) / 0.45).min(1.0);
        let grid_a      = (grid_appear * 18.0) as u8;
        // 4 latitude rings
        for &lat_f in &[-0.55_f32, -0.25, 0.25, 0.55] {
            let lat_y  = center.y + lat_f * planet_r;
            let lat_r  = (planet_r * planet_r - (lat_f * planet_r).powi(2)).sqrt();
            if lat_r > 2.0 {
                p.circle_stroke(pos2(center.x, lat_y), lat_r,
                    Stroke::new(0.6, a(col, grid_a)));
            }
        }
        // 3 longitude arcs (approximate as vertical ellipses)
        for &lon_scale in &[0.35_f32, 0.7, 1.0] {
            for seg in 0..32_usize {
                let a0 = seg as f32 / 32.0 * TAU;
                let a1 = (seg + 1) as f32 / 32.0 * TAU;
                let p0 = pos2(center.x + planet_r * lon_scale * a0.cos(),
                              center.y + planet_r * a0.sin());
                let p1 = pos2(center.x + planet_r * lon_scale * a1.cos(),
                              center.y + planet_r * a1.sin());
                p.line_segment([p0, p1], Stroke::new(0.5, a(col, grid_a)));
            }
        }
    }

    // Planet border stroke
    p.circle_stroke(center, planet_r, Stroke::new(1.0, a(col, (form * 80.0) as u8)));

    // Vault name fading in
    let name_a = ((form - 0.7) / 0.3).clamp(0.0, 1.0);
    if name_a > 0.01 {
        p.text(pos2(center.x, center.y - planet_r - 28.0), Align2::CENTER_CENTER,
            &state.vault_name, FontId::proportional(22.0),
            a(col, (name_a * 200.0) as u8));
    }

    corner_marks(p, rect, 18.0, 14.0, Stroke::new(1.0, a(col, (form * 50.0) as u8)));
}

// ── Phase 5 — EMERGENCE ───────────────────────────────────────────────────────

fn paint_emergence(p: &egui::Painter, rect: Rect, center: Pos2, t: f32, state: &GenesisBootState) {
    // t: 0→2
    p.rect_filled(rect, Rounding::ZERO, VOID);

    let col      = state.vault_col;
    let planet_r = 165.0_f32;

    // Static planet (fully formed)
    p.circle_filled(center, planet_r, Color32::from_rgba_unmultiplied(6, 4, 3, 200));
    for (i, &(cx_f, cy_f)) in [
        (0.0f32, -0.55), (0.5, -0.35), (0.55, 0.0), (0.4, 0.45),
        (0.0, 0.55), (-0.45, 0.35), (-0.55, 0.0), (-0.35,-0.45),
    ].iter().enumerate() {
        if !state.bdna_bits[i] { continue; }
        let cx  = center.x + cx_f * planet_r;
        let cy  = center.y + cy_f * planet_r;
        let c_r = planet_r * (0.18 + state.bdna_bits[i + 8] as i32 as f32 * 0.06);
        for (r, ba) in [(c_r * 1.4, 18_u8), (c_r, 45), (c_r * 0.6, 70)] {
            p.circle_filled(pos2(cx, cy), r, a(col, ba));
        }
    }
    p.circle_stroke(center, planet_r, Stroke::new(1.0, a(col, 80)));
    // Atmosphere
    for (dr, ba) in [(14.0_f32, 28_u8), (22.0, 16), (34.0, 8)] {
        p.circle_stroke(center, planet_r + dr, Stroke::new(4.0, a(col, ba)));
    }

    // City lights — 16 dots at positions on the dark hemisphere
    // Positions derived from bdna_bits[16..32]
    let city_positions: [(f32, f32); 16] = [
        (-0.30,  0.18), ( 0.40,  0.25), (-0.10, -0.30), ( 0.22,  0.50),
        (-0.48,  0.10), ( 0.15, -0.40), ( 0.38, -0.22), (-0.25,  0.42),
        ( 0.50,  0.00), (-0.35, -0.20), ( 0.05,  0.38), (-0.42,  0.32),
        ( 0.28,  0.15), (-0.18, -0.48), ( 0.45, -0.10), (-0.08,  0.52),
    ];

    // Count active cities for coherence display
    let mut active_cities = 0_usize;

    for (i, &(cx_f, cy_f)) in city_positions.iter().enumerate() {
        let appear_t = i as f32 * 0.10;
        if t < appear_t { continue; }
        active_cities += 1;

        let fade_in = ((t - appear_t) / 0.3).min(1.0);
        let cx = center.x + cx_f * planet_r;
        let cy = center.y + cy_f * planet_r;
        // Skip if outside planet
        if (cx - center.x).hypot(cy - center.y) > planet_r * 0.92 { continue; }

        let blink  = 0.6 + 0.4 * ((t * 1.2 + i as f32 * 0.9) * TAU).sin();
        let city_a = (fade_in * blink * 200.0) as u8;
        let glow_a = (fade_in * blink * 40.0) as u8;

        p.circle_filled(pos2(cx, cy), 5.0, a(col, glow_a));
        p.circle_filled(pos2(cx, cy), 2.5, a(GOLD_LT, city_a));

        // Connection lines to nearby cities
        if i > 0 && state.bdna_bits[i] {
            let prev = city_positions[i - 1];
            let px   = center.x + prev.0 * planet_r;
            let py   = center.y + prev.1 * planet_r;
            if (px - center.x).hypot(py - center.y) < planet_r * 0.92 {
                p.line_segment(
                    [pos2(cx, cy), pos2(px, py)],
                    Stroke::new(0.6, a(col, (fade_in * 25.0) as u8)),
                );
            }
        }
    }

    // Vault name
    p.text(pos2(center.x, center.y - planet_r - 28.0), Align2::CENTER_CENTER,
        &state.vault_name, FontId::proportional(22.0), a(col, 200));

    // Coherence readout
    let coherence = (active_cities as f32 / 16.0 * 88.0).round();
    p.text(pos2(center.x, center.y + planet_r + 24.0), Align2::CENTER_CENTER,
        &format!("COHERENCE: {:.0}%", coherence),
        FontId::monospace(10.0), a(GOLD, 160));
    p.text(pos2(center.x, center.y + planet_r + 42.0), Align2::CENTER_CENTER,
        "CIVILIZATIONS EMERGING…",
        FontId::monospace(8.0), a(col, 80));

    corner_marks(p, rect, 18.0, 14.0, Stroke::new(1.0, a(col, 50)));
}

// ── Phase 6 — SETTLING ────────────────────────────────────────────────────────

fn paint_settling(p: &egui::Painter, rect: Rect, center: Pos2, t: f32, state: &GenesisBootState) {
    // t: 0→4
    p.rect_filled(rect, Rounding::ZERO, VOID);

    let col = state.vault_col;

    // How small the planet has become
    let shrink   = spring_out((t / 2.2).min(1.0));
    let planet_r = 165.0 * (1.0 - shrink * 0.82) + 30.0 * shrink;

    // Orbit ring (appears as planet shrinks)
    let orbit_r   = 195.0_f32;
    let ring_appear = shrink;
    let orbit_a     = (ring_appear * 30.0) as u8;
    // Dashed orbit circle
    let n_dash = 48_usize;
    for i in (0..n_dash).step_by(2) {
        let a0 = i as f32 / n_dash as f32 * TAU - TAU / 4.0;
        let a1 = (i + 1) as f32 / n_dash as f32 * TAU - TAU / 4.0;
        p.line_segment(
            [pos2(center.x + orbit_r * a0.cos(), center.y + orbit_r * a0.sin()),
             pos2(center.x + orbit_r * a1.cos(), center.y + orbit_r * a1.sin())],
            Stroke::new(1.0, a(col, orbit_a)),
        );
    }

    // 8 orbital slots
    for i in 0..8_usize {
        let slot_angle  = i as f32 * TAU / 8.0 - TAU / 4.0;
        let is_target   = (state.roulette_target - slot_angle).abs() < 0.01;
        let sx          = center.x + orbit_r * slot_angle.cos();
        let sy          = center.y + orbit_r * slot_angle.sin();
        let slot_a      = if is_target && state.roulette_settled {
            let pulse = 0.5 + 0.5 * (t * 2.5 * TAU).sin();
            (ring_appear * pulse * 220.0) as u8
        } else {
            (ring_appear * 40.0) as u8
        };
        p.circle_stroke(pos2(sx, sy), 12.0, Stroke::new(1.0, a(col, slot_a)));
        if is_target && state.roulette_settled {
            p.circle_filled(pos2(sx, sy), 10.0, a(col, (ring_appear * 30.0) as u8));
        }
    }

    // Planet position on orbit
    let px = center.x + orbit_r * state.roulette_pos.cos();
    let py = center.y + orbit_r * state.roulette_pos.sin();
    let orb_center = if shrink > 0.2 { pos2(px, py) } else { center };

    // Draw the planet (shrinking)
    p.circle_filled(orb_center, planet_r, Color32::from_rgba_unmultiplied(6, 4, 3, 200));
    if planet_r > 20.0 {
        for (i, &(cx_f, cy_f)) in [
            (0.0f32, -0.55), (0.5, -0.35), (0.55, 0.0), (0.4, 0.45),
        ].iter().enumerate() {
            if !state.bdna_bits[i] { continue; }
            let cx = orb_center.x + cx_f * planet_r;
            let cy = orb_center.y + cy_f * planet_r;
            let cr = planet_r * 0.22;
            p.circle_filled(pos2(cx, cy), cr, a(col, 55));
        }
    }
    // Atmosphere glow
    p.circle_stroke(orb_center, planet_r + 6.0, Stroke::new(4.0, a(col, 20)));
    p.circle_stroke(orb_center, planet_r,        Stroke::new(1.0, a(col, 80)));

    // "World initialized" text (appears when settled)
    if state.roulette_settled {
        let settle_t = (t - 2.0).max(0.0);
        let reveal   = spring_out((settle_t / 0.8).min(1.0));
        let settle_a = (reveal * 220.0) as u8;

        // Settled slot pulse flash
        let flash_a = ((1.0 - (settle_t / 0.5).min(1.0)) * 120.0) as u8;
        p.circle_filled(orb_center, planet_r + 20.0, a(col, flash_a));

        p.text(pos2(center.x, center.y - orbit_r - 40.0), Align2::CENTER_CENTER,
            "⬡  WORLD INITIALIZED",
            FontId::proportional(22.0), a(GOLD, settle_a));

        p.text(pos2(center.x, center.y - orbit_r - 16.0), Align2::CENTER_CENTER,
            &state.vault_name, FontId::proportional(14.0), a(col, settle_a));

        if !state.bdna_sig.is_empty() {
            let sig = &state.bdna_sig[..16.min(state.bdna_sig.len())];
            p.text(pos2(center.x, center.y + orbit_r + 28.0), Align2::CENTER_CENTER,
                &format!("B-DNA: {sig}…"),
                FontId::monospace(9.0), a(col, (settle_a as f32 * 0.55) as u8));
        }

        p.text(pos2(center.x, center.y + orbit_r + 48.0), Align2::CENTER_CENTER,
            "OPENING VAULT…",
            FontId::monospace(9.0), a(GOLD, (settle_a as f32 * 0.70) as u8));
    }

    corner_marks(p, rect, 18.0, 14.0, Stroke::new(1.0, a(col, 45)));
}

// ── Draw helpers ──────────────────────────────────────────────────────────────

fn paint_grid(p: &egui::Painter, rect: Rect, spacing: f32, col: Color32) {
    let mut x = rect.left() - rect.left() % spacing;
    while x <= rect.right() {
        p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(0.5, col));
        x += spacing;
    }
    let mut y = rect.top() - rect.top() % spacing;
    while y <= rect.bottom() {
        p.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(0.5, col));
        y += spacing;
    }
}

fn corner_marks(p: &egui::Painter, rect: Rect, margin: f32, size: f32, stroke: Stroke) {
    for (cx, cy, dx, dy) in [
        (rect.left()  + margin, rect.top()    + margin,  size,  size),
        (rect.right() - margin, rect.top()    + margin, -size,  size),
        (rect.left()  + margin, rect.bottom() - margin,  size, -size),
        (rect.right() - margin, rect.bottom() - margin, -size, -size),
    ] {
        p.line_segment([pos2(cx, cy), pos2(cx + dx, cy)],       stroke);
        p.line_segment([pos2(cx, cy), pos2(cx, cy + dy)],       stroke);
    }
}

fn bezier(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    pos2(u * u * p0.x + 2.0 * u * t * p1.x + t * t * p2.x,
         u * u * p0.y + 2.0 * u * t * p1.y + t * t * p2.y)
}

fn spring_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}
