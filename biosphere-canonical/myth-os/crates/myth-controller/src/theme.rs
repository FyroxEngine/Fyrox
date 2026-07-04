use egui::{Color32, FontFamily, FontId, Margin, Rounding, Stroke, Style, Visuals};

// ─── Void Stack — deep backgrounds ───────────────────────────────────────────
pub const VOID:     Color32 = Color32::from_rgb(3,   5,  10);
pub const ABYSS:    Color32 = Color32::from_rgb(7,   9,  15);
pub const DEEP:     Color32 = Color32::from_rgb(13,  17, 23);
pub const SURFACE:  Color32 = Color32::from_rgb(17,  24, 39);
pub const RAISED:   Color32 = Color32::from_rgb(22,  29, 46);
pub const ELEVATED: Color32 = Color32::from_rgb(30,  41, 59);
pub const INLAY:    Color32 = Color32::from_rgb(36,  48, 68);

// ─── Foreground ───────────────────────────────────────────────────────────────
pub const FG1:      Color32 = Color32::from_rgb(226, 232, 240);
pub const FG2:      Color32 = Color32::from_rgb(148, 163, 184);
pub const FG3:      Color32 = Color32::from_rgb(100, 116, 139);
pub const FG_MUTED: Color32 = Color32::from_rgb(71,  85,  105);

// ─── Bioluminescent Accents ───────────────────────────────────────────────────
pub const QUANTUM:  Color32 = Color32::from_rgb(0,   229, 255); // cyan
pub const BIO:      Color32 = Color32::from_rgb(57,  255, 20);  // lab-green
pub const MYTHOS:   Color32 = Color32::from_rgb(192, 132, 252); // amethyst
pub const GOLD:     Color32 = Color32::from_rgb(251, 191, 36);  // heraldic
pub const EMBER:    Color32 = Color32::from_rgb(249, 115, 22);  // plasma orange
pub const ROSE:     Color32 = Color32::from_rgb(251, 113, 133); // tension

// ─── Borders ──────────────────────────────────────────────────────────────────
pub const BORDER:     Color32 = Color32::from_rgba_premultiplied(18, 28, 40, 255);
pub const BORDER_LIT: Color32 = Color32::from_rgba_premultiplied(26, 51, 71, 255);

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Returns `color` with its alpha replaced by `alpha`, unmultiplied.
pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

/// Scales all RGB channels toward black by `factor` (0.0 = black, 1.0 = unchanged).
pub fn darken(color: Color32, factor: f32) -> Color32 {
    Color32::from_rgb(
        (color.r() as f32 * factor) as u8,
        (color.g() as f32 * factor) as u8,
        (color.b() as f32 * factor) as u8,
    )
}

/// Monospace label font at `size` pt.
pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// Apply the full BioSpark quantum-mythic theme to the egui context.
pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();
    let mut v = Visuals::dark();

    v.override_text_color      = Some(FG1);
    v.window_fill              = ABYSS;
    v.panel_fill               = DEEP;
    v.faint_bg_color           = SURFACE;
    v.extreme_bg_color         = VOID;
    v.code_bg_color            = SURFACE;
    v.window_stroke            = Stroke::new(1.0, BORDER);
    v.window_rounding          = Rounding::same(3.0);
    v.selection.bg_fill        = Color32::from_rgba_unmultiplied(0, 60, 80, 60);
    v.selection.stroke         = Stroke::new(1.0, QUANTUM);

    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.rounding = Rounding::same(2.0);
    }

    v.widgets.noninteractive.bg_fill     = SURFACE;
    v.widgets.noninteractive.bg_stroke   = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.fg_stroke   = Stroke::new(1.0, FG3);

    v.widgets.inactive.bg_fill           = SURFACE;
    v.widgets.inactive.bg_stroke         = Stroke::new(1.0, BORDER);
    v.widgets.inactive.fg_stroke         = Stroke::new(1.5, FG2);

    v.widgets.hovered.bg_fill            = ELEVATED;
    v.widgets.hovered.bg_stroke          = Stroke::new(1.0, BORDER_LIT);
    v.widgets.hovered.fg_stroke          = Stroke::new(1.5, FG1);

    v.widgets.active.bg_fill             = INLAY;
    v.widgets.active.bg_stroke           = Stroke::new(1.0, QUANTUM);
    v.widgets.active.fg_stroke           = Stroke::new(2.0, FG1);

    style.visuals = v;
    style.spacing.item_spacing           = egui::vec2(4.0, 4.0);
    style.spacing.window_margin          = Margin::same(10.0);
    style.spacing.button_padding         = egui::vec2(6.0, 3.0);

    ctx.set_style(style);
}
