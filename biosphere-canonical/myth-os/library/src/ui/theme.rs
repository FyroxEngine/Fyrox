// The Great Library — design tokens.
// Warm amber/gold primary, deep violet secondary, void-black ground.
// All tokens are part of the canonical design vocabulary — unused today,
// referenced soon. Suppress dead_code warnings for the whole module.
#![allow(dead_code)]

use egui::Color32;

// ── VOID STACK ─────────────────────────────────────────────────────────────
pub const VOID:     Color32 = Color32::from_rgb(6,   4,  3);
pub const ABYSS:    Color32 = Color32::from_rgb(10,  7,  5);
pub const DEEP:     Color32 = Color32::from_rgb(15, 11,  8);
pub const SURFACE:  Color32 = Color32::from_rgb(20, 16, 12);
pub const RAISED:   Color32 = Color32::from_rgb(28, 22, 16);
pub const ELEVATED: Color32 = Color32::from_rgb(36, 29, 20);
pub const INLAY:    Color32 = Color32::from_rgb(48, 38, 26);

// ── FOREGROUND / INK ───────────────────────────────────────────────────────
pub const FG_1:     Color32 = Color32::from_rgb(226, 232, 240);
pub const FG_2:     Color32 = Color32::from_rgb(180, 160, 130);
pub const FG_3:     Color32 = Color32::from_rgb(120, 100,  80);
pub const FG_MUTED: Color32 = Color32::from_rgb( 80,  65,  50);

// ── LIBRARY ACCENTS ────────────────────────────────────────────────────────
pub const GOLD:    Color32 = Color32::from_rgb(212, 160,  48);
pub const GOLD_LT: Color32 = Color32::from_rgb(245, 208,  96);
pub const GOLD_DK: Color32 = Color32::from_rgb(100,  72,  18);

pub const VIOLET:    Color32 = Color32::from_rgb(140,  80, 255);
pub const VIOLET_LT: Color32 = Color32::from_rgb(180, 130, 255);

pub const TEAL:  Color32 = Color32::from_rgb(0, 200, 180);
pub const GREEN: Color32 = Color32::from_rgb(0, 192,  96);

// ── CARD / RAIL SURFACES ───────────────────────────────────────────────────
pub const CARD_BG:  Color32 = Color32::from_rgb(12,  9,  6);
pub const RAIL_BG:  Color32 = Color32::from_rgb( 8,  6,  4);

// ── COLOR HELPERS ──────────────────────────────────────────────────────────

/// Return `color` with a custom alpha (0–255).
pub fn a(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

/// Return `color` at `pct` percent opacity (0.0–1.0).
pub fn fade(color: Color32, pct: f32) -> Color32 {
    a(color, (pct.clamp(0.0, 1.0) * 255.0) as u8)
}

// ── GLOBAL THEME ──────────────────────────────────────────────────────────

pub fn apply(ctx: &egui::Context) {
    let mut style = egui::Style::default();
    style.visuals = egui::Visuals::dark();

    style.visuals.window_fill      = SURFACE;
    style.visuals.panel_fill       = DEEP;
    style.visuals.faint_bg_color   = RAISED;
    style.visuals.extreme_bg_color = VOID;
    style.visuals.window_rounding  = egui::Rounding::same(6.0);

    style.visuals.widgets.noninteractive.bg_fill    = SURFACE;
    style.visuals.widgets.noninteractive.fg_stroke  = egui::Stroke::new(1.0, FG_3);
    style.visuals.widgets.inactive.bg_fill          = RAISED;
    style.visuals.widgets.inactive.fg_stroke        = egui::Stroke::new(1.0, FG_2);
    style.visuals.widgets.hovered.bg_fill           = ELEVATED;
    style.visuals.widgets.hovered.fg_stroke         = egui::Stroke::new(1.5, GOLD_LT);
    style.visuals.widgets.active.bg_fill            = INLAY;
    style.visuals.widgets.active.fg_stroke          = egui::Stroke::new(1.5, GOLD);

    style.visuals.selection.bg_fill = a(GOLD, 30);
    style.visuals.selection.stroke  = egui::Stroke::new(1.0, GOLD);

    style.spacing.item_spacing   = egui::vec2(8.0,  6.0);
    style.spacing.window_margin  = egui::Margin::same(16.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);

    ctx.set_style(style);
}
