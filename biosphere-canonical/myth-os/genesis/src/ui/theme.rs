// Design tokens from BioSpark Quantum-Mythic system + Axiom Signal Chain Rack overlay.
// Exact hex values extracted from colors_and_type.css and rack.css.

use egui::Color32;

// ── VOID STACK ─────────────────────────────────────────────────────────────
pub const VOID:     Color32 = Color32::from_rgb(3, 5, 10);       // #03050a
pub const ABYSS:    Color32 = Color32::from_rgb(7, 9, 15);       // #07090f
pub const DEEP:     Color32 = Color32::from_rgb(13, 17, 23);     // #0d1117
pub const SURFACE:  Color32 = Color32::from_rgb(17, 24, 39);     // #111827
pub const RAISED:   Color32 = Color32::from_rgb(22, 29, 46);     // #161d2e
pub const ELEVATED: Color32 = Color32::from_rgb(30, 41, 59);     // #1e293b
pub const INLAY:    Color32 = Color32::from_rgb(36, 48, 68);     // #243044

// ── FOREGROUND / INK ───────────────────────────────────────────────────────
pub const FG_1:     Color32 = Color32::from_rgb(226, 232, 240);  // #e2e8f0
pub const FG_2:     Color32 = Color32::from_rgb(148, 163, 184);  // #94a3b8
pub const FG_3:     Color32 = Color32::from_rgb(100, 116, 139);  // #64748b
pub const FG_MUTED: Color32 = Color32::from_rgb(71, 85, 105);    // #475569

// ── BIOLUMINESCENT ACCENTS (BioSpark base) ─────────────────────────────────
pub const QUANTUM: Color32 = Color32::from_rgb(0, 229, 255);     // #00e5ff
pub const BIO:     Color32 = Color32::from_rgb(57, 255, 20);     // #39ff14
pub const MYTHOS:  Color32 = Color32::from_rgb(192, 132, 252);   // #c084fc
pub const GOLD:    Color32 = Color32::from_rgb(251, 191, 36);    // #fbbf24
pub const EMBER:   Color32 = Color32::from_rgb(249, 115, 22);    // #f97316
pub const ROSE:    Color32 = Color32::from_rgb(251, 113, 133);   // #fb7185

// ── ASTRAL GATEWAY ACCENTS (Axiom rack overlay) ────────────────────────────
pub const ASTRAL_CYAN:    Color32 = Color32::from_rgb(0, 191, 255);    // #00bfff
pub const ASTRAL_VIOLET:  Color32 = Color32::from_rgb(148, 0, 211);    // #9400d3
pub const ASTRAL_MAGENTA: Color32 = Color32::from_rgb(255, 20, 147);   // #ff1493

// ── CHASSIS / PANEL FINISHES ───────────────────────────────────────────────
pub const RAIL_TOP:   Color32 = Color32::from_rgb(26, 31, 44);   // #1a1f2c
pub const RAIL_SHIFT: Color32 = Color32::from_rgb(42, 36, 54);   // #2a2436 (iridescent purple)
pub const RAIL_DARK:  Color32 = Color32::from_rgb(14, 17, 25);   // #0e1119
pub const MOD_TOP:    Color32 = Color32::from_rgb(42, 47, 56);   // #2a2f38 alloy top
pub const MOD_MID:    Color32 = Color32::from_rgb(26, 29, 36);   // #1a1d24 alloy mid
pub const MOD_BASE:   Color32 = Color32::from_rgb(20, 23, 28);   // #14171c alloy base

// ── WIRE-TYPE TAXONOMY (15 Genesis signal types) ──────────────────────────
pub const WIRE_SPA: Color32 = Color32::from_rgb(0, 191, 255);    // Spatial     — cyan
pub const WIRE_BHV: Color32 = Color32::from_rgb(192, 132, 252);  // Behavior    — violet
pub const WIRE_IDN: Color32 = Color32::from_rgb(251, 191, 36);   // Identity    — gold
pub const WIRE_DAT: Color32 = Color32::from_rgb(57, 255, 20);    // Data        — bio
pub const WIRE_TMP: Color32 = Color32::from_rgb(255, 20, 147);   // Temporal    — magenta
pub const WIRE_NAR: Color32 = Color32::from_rgb(139, 92, 246);   // Narrative   — mythos
pub const WIRE_AST: Color32 = Color32::from_rgb(245, 158, 11);   // Asset       — amber
pub const WIRE_AUD: Color32 = Color32::from_rgb(249, 115, 22);   // Audio       — ember
pub const WIRE_VIS: Color32 = Color32::from_rgb(93, 202, 165);   // Visual      — aqua
pub const WIRE_LGC: Color32 = Color32::from_rgb(20, 184, 166);   // Logic       — teal
pub const WIRE_ENR: Color32 = Color32::from_rgb(251, 113, 133);  // Energy      — rose
pub const WIRE_SOC: Color32 = Color32::from_rgb(99, 102, 241);   // Social      — indigo
pub const WIRE_EVT: Color32 = Color32::from_rgb(239, 68, 68);    // Event       — red
pub const WIRE_AGT: Color32 = Color32::from_rgb(42, 212, 200);   // Agent       — hydralis
pub const WIRE_CTL: Color32 = Color32::from_rgb(148, 163, 184);  // Control     — silver

pub fn wire_color(wire: &str) -> Color32 {
    match wire {
        "SPA" => WIRE_SPA, "BHV" => WIRE_BHV, "IDN" => WIRE_IDN,
        "DAT" => WIRE_DAT, "TMP" => WIRE_TMP, "NAR" => WIRE_NAR,
        "AST" => WIRE_AST, "AUD" => WIRE_AUD, "VIS" => WIRE_VIS,
        "LGC" => WIRE_LGC, "ENR" => WIRE_ENR, "SOC" => WIRE_SOC,
        "EVT" => WIRE_EVT, "AGT" => WIRE_AGT, "CTL" => WIRE_CTL,
        _ => FG_MUTED,
    }
}

// ── GENESIS DEPARTMENT COLORS (from rack-primitives.jsx GENESIS_REGISTRY) ──
pub const DEPT_I:   Color32 = Color32::from_rgb(30, 140, 255);   // World Construction — #1e8cff
pub const DEPT_II:  Color32 = Color32::from_rgb(244, 192, 37);   // Entity Systems     — #f4c025
pub const DEPT_III: Color32 = Color32::from_rgb(140, 80, 255);   // Narrative Systems  — #8c50ff
pub const DEPT_IV:  Color32 = Color32::from_rgb(48, 224, 96);    // Pipeline Systems   — #30e060

// Legacy aliases used by existing code
pub const DEPT_STRUCTURE:  Color32 = DEPT_I;
pub const DEPT_ENTITIES:   Color32 = DEPT_II;
pub const DEPT_ATMOSPHERE: Color32 = DEPT_III;
pub const DEPT_DYNAMICS:   Color32 = DEPT_IV;

// ── SEMANTIC ───────────────────────────────────────────────────────────────
// Premultiplied equivalents of rgba_unmultiplied values (from_rgba_premultiplied is const)
pub const BORDER:     Color32 = Color32::from_rgba_premultiplied(9,  14, 20, 20);   // rgba(120,180,255,20)
pub const BORDER_LIT: Color32 = Color32::from_rgba_premultiplied(28, 56, 71, 71);   // rgba(100,200,255,71)
pub const GOLD_MARK:  Color32 = Color32::from_rgba_premultiplied(126,96, 18, 128);  // rgba(251,191,36,128)
pub const LED_OFF:    Color32 = Color32::from_rgb(14, 22, 14);

// ── BEZEL SCREW GRADIENT (approximate with mid value) ─────────────────────
pub const SCREW_MID:  Color32 = Color32::from_rgb(160, 168, 186);  // #a0a8ba
pub const SCREW_EDGE: Color32 = Color32::from_rgb(42, 46, 56);     // #2a2e38

use mythos::quantum_module::Department;

pub fn dept_color(dept: &Department) -> Color32 {
    match dept {
        Department::Structure  => DEPT_STRUCTURE,
        Department::Entities   => DEPT_ENTITIES,
        Department::Atmosphere => DEPT_ATMOSPHERE,
        Department::Dynamics   => DEPT_DYNAMICS,
    }
}

/// Apply the Axiom rack egui theme.
pub fn apply(ctx: &egui::Context) {
    let mut style = egui::Style::default();
    style.visuals = egui::Visuals::dark();

    style.visuals.window_fill         = SURFACE;
    style.visuals.panel_fill          = DEEP;
    style.visuals.faint_bg_color      = RAISED;
    style.visuals.extreme_bg_color    = VOID;
    style.visuals.window_rounding     = egui::Rounding::same(4.0);

    style.visuals.widgets.noninteractive.bg_fill = SURFACE;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, FG_3);
    style.visuals.widgets.noninteractive.rounding  = egui::Rounding::same(3.0);
    style.visuals.widgets.inactive.bg_fill         = RAISED;
    style.visuals.widgets.inactive.fg_stroke       = egui::Stroke::new(1.0, FG_2);
    style.visuals.widgets.inactive.rounding        = egui::Rounding::same(3.0);
    style.visuals.widgets.hovered.bg_fill          = ELEVATED;
    style.visuals.widgets.hovered.fg_stroke        = egui::Stroke::new(1.0, FG_1);
    style.visuals.widgets.hovered.rounding         = egui::Rounding::same(3.0);
    style.visuals.widgets.active.bg_fill           = INLAY;
    style.visuals.widgets.active.fg_stroke         = egui::Stroke::new(1.0, ASTRAL_CYAN);
    style.visuals.widgets.active.rounding          = egui::Rounding::same(3.0);

    style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0, 191, 255, 30);
    style.visuals.selection.stroke  = egui::Stroke::new(1.0, ASTRAL_CYAN);

    style.spacing.item_spacing   = egui::vec2(6.0, 4.0);
    style.spacing.window_margin  = egui::Margin::same(12.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);

    ctx.set_style(style);
}
