use egui::{Color32, FontId, FontFamily, Rounding, Stroke, Style, Visuals};

pub const VOID:      Color32 = Color32::from_rgb( 4,  4, 12);
pub const ABYSS:     Color32 = Color32::from_rgb( 6,  6, 16);
pub const DEEP:      Color32 = Color32::from_rgb( 8,  8, 22);
pub const SURFACE:   Color32 = Color32::from_rgb(14, 14, 32);
pub const RAISED:    Color32 = Color32::from_rgb(20, 20, 44);
pub const ELEVATED:  Color32 = Color32::from_rgb(28, 28, 58);
pub const INLAY:     Color32 = Color32::from_rgb(35, 35, 70);
pub const BORDER:    Color32 = Color32::from_rgb(40, 55, 88); // approx — rgba const not stable

pub const FG1:       Color32 = Color32::from_rgb(230, 235, 255);
pub const FG2:       Color32 = Color32::from_rgb(160, 170, 200);
pub const FG3:       Color32 = Color32::from_rgb(100, 110, 145);
pub const FG_MUTED:  Color32 = Color32::from_rgb( 60,  68,  95);

pub const QUANTUM:   Color32 = Color32::from_rgb(  0, 200, 255);
pub const BIO:       Color32 = Color32::from_rgb( 57, 255,  20);
pub const MYTHOS:    Color32 = Color32::from_rgb(168,  85, 247);
pub const GOLD:      Color32 = Color32::from_rgb(251, 191,  36);
pub const EMBER:     Color32 = Color32::from_rgb(251, 113,  36);
pub const ROSE:      Color32 = Color32::from_rgb(251, 113, 133);

pub fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();
    style.visuals = Visuals::dark();
    style.visuals.window_fill        = VOID;
    style.visuals.panel_fill         = VOID;
    style.visuals.faint_bg_color     = DEEP;
    style.visuals.extreme_bg_color   = ABYSS;
    style.visuals.widgets.noninteractive.bg_fill   = SURFACE;
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, FG3);
    style.visuals.widgets.inactive.bg_fill         = RAISED;
    style.visuals.widgets.inactive.fg_stroke       = Stroke::new(1.0, FG2);
    style.visuals.widgets.hovered.bg_fill          = ELEVATED;
    style.visuals.widgets.hovered.fg_stroke        = Stroke::new(1.0, FG1);
    style.visuals.widgets.active.bg_fill           = INLAY;
    style.visuals.widgets.active.fg_stroke         = Stroke::new(1.0, QUANTUM);
    style.visuals.widgets.noninteractive.rounding  = Rounding::same(2.0);
    style.visuals.widgets.inactive.rounding        = Rounding::same(2.0);
    style.visuals.widgets.hovered.rounding         = Rounding::same(2.0);
    style.visuals.widgets.active.rounding          = Rounding::same(2.0);
    style.spacing.item_spacing  = egui::vec2(6.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(8.0);
    ctx.set_style(style);
}
