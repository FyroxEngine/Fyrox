// THEATRE-RACK-UI: Design tokens and custom egui widgets.
//
// Visual language from BioSpark Procedural_Rack_Design_System.
// Standalone — no mythos / genesis crate dependencies.
//
// Widgets:
//   apply_rack_theme  — install the dark-rack egui style
//   knob_xs / knob_sm — arc-indicator knobs (20 / 26 px body)
//   fader_horiz       — horizontal metallic fader with segmented track
//   vu_horiz          — horizontal segmented VU bar
//   led               — 6 px dot with glow ring
//   rack_pad          — small lit M/S/A style pad button
//   transport_pad     — wider alloy transport button (▶ ■ ◀◀ ▶▶)
//   scene_pad         — scene slot button (filled/empty state)
//   board_decorations — gold corner L-brackets + left accent bar painted on a Rect

use egui::{Color32, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};

// ── VOID STACK ────────────────────────────────────────────────────────────────
pub const VOID:     Color32 = Color32::from_rgb(3,   5,  10);
pub const ABYSS:    Color32 = Color32::from_rgb(7,   9,  15);
pub const DEEP:     Color32 = Color32::from_rgb(13,  17,  23);
pub const SURFACE:  Color32 = Color32::from_rgb(17,  24,  39);
pub const RAISED:   Color32 = Color32::from_rgb(22,  29,  46);
pub const ELEVATED: Color32 = Color32::from_rgb(30,  41,  59);
pub const INLAY:    Color32 = Color32::from_rgb(36,  48,  68);

// ── FOREGROUND ────────────────────────────────────────────────────────────────
pub const FG_1:     Color32 = Color32::from_rgb(226, 232, 240);
pub const FG_2:     Color32 = Color32::from_rgb(148, 163, 184);
pub const FG_3:     Color32 = Color32::from_rgb(100, 116, 139);
pub const FG_MUTED: Color32 = Color32::from_rgb(71,   85, 105);

// ── BIOLUMINESCENT ACCENTS ────────────────────────────────────────────────────
pub const QUANTUM: Color32 = Color32::from_rgb(0,   229, 255);
pub const BIO:     Color32 = Color32::from_rgb(57,  255,  20);
pub const MYTHOS:  Color32 = Color32::from_rgb(192, 132, 252);
pub const GOLD:    Color32 = Color32::from_rgb(251, 191,  36);
pub const EMBER:   Color32 = Color32::from_rgb(249, 115,  22);
pub const SUCCESS: Color32 = Color32::from_rgb(0,   200,  90);
pub const WARN:    Color32 = Color32::from_rgb(255, 180,  40);
pub const DANGER:  Color32 = Color32::from_rgb(255,  60,  80);

// ── CHASSIS / PANEL FINISHES ──────────────────────────────────────────────────
pub const MOD_BASE:   Color32 = Color32::from_rgb(20,  23,  28);
pub const SCREW_MID:  Color32 = Color32::from_rgb(160, 168, 186);
pub const SCREW_EDGE: Color32 = Color32::from_rgb(42,  46,  56);
pub const GOLD_MARK:  Color32 = Color32::from_rgba_premultiplied(126, 96, 18, 128);
pub const LED_OFF:    Color32 = Color32::from_rgb(14,  22,  14);

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Recolour with a different alpha.
pub fn a(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// Per-layer-type accent colour for channel strip badges.
pub fn layer_accent(tag: &str) -> Color32 {
    match tag {
        "BV" => MYTHOS,
        "P5" => Color32::from_rgb(100, 180, 255),
        "GL" => EMBER,
        "HT" => Color32::from_rgb(20,  200, 180),
        "AU" => SUCCESS,
        _    => FG_2,
    }
}

// ── apply_rack_theme ──────────────────────────────────────────────────────────

pub fn apply_rack_theme(ctx: &egui::Context) {
    let mut style = egui::Style::default();
    style.visuals = egui::Visuals::dark();

    style.visuals.panel_fill          = DEEP;
    style.visuals.window_fill         = SURFACE;
    style.visuals.faint_bg_color      = RAISED;
    style.visuals.extreme_bg_color    = VOID;
    style.visuals.window_rounding     = egui::Rounding::same(4.0);

    style.visuals.widgets.noninteractive.bg_fill   = SURFACE;
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, FG_3);
    style.visuals.widgets.noninteractive.rounding  = egui::Rounding::same(3.0);

    style.visuals.widgets.inactive.bg_fill   = RAISED;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, FG_2);
    style.visuals.widgets.inactive.rounding  = egui::Rounding::same(3.0);

    style.visuals.widgets.hovered.bg_fill   = ELEVATED;
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, FG_1);
    style.visuals.widgets.hovered.rounding  = egui::Rounding::same(3.0);

    style.visuals.widgets.active.bg_fill   = INLAY;
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, QUANTUM);
    style.visuals.widgets.active.rounding  = egui::Rounding::same(3.0);

    style.visuals.selection.bg_fill = a(QUANTUM, 30);
    style.visuals.selection.stroke  = Stroke::new(1.0, QUANTUM);

    style.spacing.item_spacing   = egui::vec2(5.0, 3.0);
    style.spacing.button_padding = egui::vec2(6.0, 3.0);

    ctx.set_style(style);
}

// ── LED ───────────────────────────────────────────────────────────────────────

pub fn led(ui: &mut Ui, on: bool, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    if !ui.is_rect_visible(rect) { return; }
    let p = ui.painter();
    let c = rect.center();
    p.circle_filled(c, 3.0, if on { color } else { LED_OFF });
    if on {
        p.circle_stroke(c, 5.0, Stroke::new(1.0, a(color, 80)));
    }
}

// ── KNOB ─────────────────────────────────────────────────────────────────────
//
// Arc-indicator knob. 13 tick marks spanning −135° to +135° (270° sweep).
// Drag up/down to change value. Shift = fine (0.2×).
// Double-click = reset to 1.0.

pub fn knob_xs(ui: &mut Ui, value: &mut f32, label: &str, color: Color32) -> Response {
    knob_sized(ui, value, label, color, 20.0)
}

pub fn knob_sm(ui: &mut Ui, value: &mut f32, label: &str, color: Color32) -> Response {
    knob_sized(ui, value, label, color, 26.0)
}

pub fn knob_sized(ui: &mut Ui, value: &mut f32, label: &str, color: Color32, body_px: f32) -> Response {
    let tick_ext = (body_px * 0.20).max(4.0);
    let label_h  = if label.is_empty() { 0.0 } else { 20.0 };
    let total    = body_px + tick_ext * 2.0;

    let (rect, resp) = ui.allocate_exact_size(Vec2::new(total, total + label_h), Sense::drag());

    if resp.dragged() {
        let scale = if ui.input(|i| i.modifiers.shift) { 0.002 } else { 0.007 };
        *value = (*value - resp.drag_delta().y * scale).clamp(0.0, 1.0);
    }
    if resp.double_clicked() { *value = 1.0; }

    if ui.is_rect_visible(rect) {
        let cx = rect.min.x + tick_ext + body_px * 0.5;
        let cy = rect.min.y + tick_ext + body_px * 0.5;
        let c  = Pos2::new(cx, cy);
        let r  = body_px * 0.5 - 0.5;
        paint_knob(ui.painter_at(rect), c, r, color, *value, resp.hovered(), resp.dragged());

        if !label.is_empty() {
            let base_y = cy + r + tick_ext + 2.0;
            let pct    = format!("{:03}", (*value * 100.0) as u32);
            ui.painter().text(Pos2::new(cx, base_y),       egui::Align2::CENTER_TOP, label, egui::FontId::monospace(7.5), FG_3);
            ui.painter().text(Pos2::new(cx, base_y + 9.0), egui::Align2::CENTER_TOP, pct,   egui::FontId::monospace(7.5), color);
        }
    }
    resp
}

fn paint_knob(
    painter: egui::Painter, c: Pos2, r: f32, color: Color32,
    value: f32, hovered: bool, dragging: bool,
) {
    // Body — dark sphere
    painter.circle_filled(c, r, MOD_BASE);
    // Subtle top-left highlight
    painter.circle_filled(
        Pos2::new(c.x - r * 0.15, c.y - r * 0.18),
        r * 0.52,
        a(Color32::WHITE, 9),
    );
    // Bezel strokes
    painter.circle_stroke(c, r,       Stroke::new(1.0, a(Color32::WHITE, 22)));
    painter.circle_stroke(c, r - 1.0, Stroke::new(1.0, a(Color32::BLACK, 150)));

    // Tick ring (13 ticks, −135° to +135°)
    let tr_outer = r + r * 0.38;
    let tr_inner = r + r * 0.16;
    for i in 0u32..=12 {
        let frac  = i as f32 / 12.0;
        let rad   = ((-135.0_f32 + frac * 270.0) - 90.0).to_radians();
        let lit   = frac <= value;
        let tcol  = if lit { color } else { a(Color32::from_rgb(120, 180, 255), 40) };
        let tw    = if lit { 1.5_f32 } else { 1.0_f32 };
        let po    = Pos2::new(c.x + rad.cos() * tr_outer, c.y + rad.sin() * tr_outer);
        let pi    = Pos2::new(c.x + rad.cos() * tr_inner, c.y + rad.sin() * tr_inner);
        painter.line_segment([pi, po], Stroke::new(tw, tcol));
    }

    // Glow ring when hovered / dragging
    if hovered || dragging {
        let ga = if dragging { 50u8 } else { 22u8 };
        painter.circle_stroke(c, r + r * 0.5, Stroke::new(r * 0.55, a(color, ga)));
    }

    // Indicator line
    let ind_rad = ((-135.0_f32 + value * 270.0) - 90.0).to_radians();
    let p0 = Pos2::new(c.x + ind_rad.cos() * r * 0.18, c.y + ind_rad.sin() * r * 0.18);
    let p1 = Pos2::new(c.x + ind_rad.cos() * r * 0.80, c.y + ind_rad.sin() * r * 0.80);
    painter.line_segment([p0, p1], Stroke::new(3.5, a(color, 55)));
    painter.line_segment([p0, p1], Stroke::new(1.8, color));
}

// ── HORIZONTAL FADER ─────────────────────────────────────────────────────────
//
// Metallic thumb with split-gradient (light top / dark bottom), segmented track,
// lit fill from left edge to thumb position.
// Total allocated height: THUMB_H = 16 px.

pub fn fader_horiz(ui: &mut Ui, value: &mut f32, color: Color32, width: f32) -> Response {
    const TRACK_H: f32 = 8.0;
    const THUMB_W: f32 = 10.0;
    const THUMB_H: f32 = 16.0;
    const VAL_W:   f32 = 24.0;   // monospace "100%" label

    let track_pw = (width - VAL_W - 6.0).max(20.0);

    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, THUMB_H), Sense::drag());

    if resp.dragged() {
        let dx = resp.drag_delta().x / track_pw;
        *value = (*value + dx).clamp(0.0, 1.0);
    }

    if ui.is_rect_visible(rect) {
        let p = ui.painter();

        let track_y = rect.min.y + (THUMB_H - TRACK_H) * 0.5;
        let track_r = Rect::from_min_size(
            Pos2::new(rect.min.x, track_y),
            Vec2::new(track_pw, TRACK_H),
        );

        // Track background
        p.rect_filled(track_r, Rounding::same(2.0), ABYSS);

        // Segmentation lines every 6 px
        let seg  = 6.0_f32;
        let nseg = (track_pw / seg) as u32;
        for i in 0..nseg {
            let x = track_r.min.x + i as f32 * seg;
            p.line_segment(
                [Pos2::new(x, track_r.min.y), Pos2::new(x, track_r.max.y)],
                Stroke::new(0.5, a(Color32::BLACK, 80)),
            );
        }

        // Lit fill
        let lit_right = track_r.min.x + *value * track_pw;
        if *value > 0.01 {
            p.rect_filled(
                Rect::from_min_max(
                    Pos2::new(track_r.min.x + 1.0, track_r.min.y + 1.0),
                    Pos2::new(lit_right,            track_r.max.y   - 1.0),
                ),
                Rounding::same(1.0),
                a(color, 80),
            );
        }

        // Thumb — center X clamped so thumb stays inside track
        let thumb_cx = lit_right.clamp(
            track_r.min.x + THUMB_W * 0.5,
            track_r.max.x - THUMB_W * 0.5,
        );
        let thumb = Rect::from_center_size(
            Pos2::new(thumb_cx, rect.center().y),
            Vec2::new(THUMB_W, THUMB_H),
        );
        let mid_y = thumb.center().y;
        // Top half: light metallic
        p.rect_filled(
            Rect::from_min_max(thumb.min, Pos2::new(thumb.max.x, mid_y)),
            Rounding { nw: 2.0, ne: 2.0, sw: 0.0, se: 0.0 },
            Color32::from_rgb(188, 194, 208),
        );
        // Bottom half: dark metallic
        p.rect_filled(
            Rect::from_min_max(Pos2::new(thumb.min.x, mid_y), thumb.max),
            Rounding { nw: 0.0, ne: 0.0, sw: 2.0, se: 2.0 },
            Color32::from_rgb(36, 40, 50),
        );
        // Centre groove
        p.line_segment(
            [Pos2::new(thumb.min.x + 2.0, mid_y), Pos2::new(thumb.max.x - 2.0, mid_y)],
            Stroke::new(0.5, a(Color32::BLACK, 100)),
        );
        // Thumb border
        p.rect_stroke(thumb, Rounding::same(2.0), Stroke::new(0.5, a(Color32::WHITE, 28)));
        // LED pip — tiny coloured dot on thumb
        p.circle_filled(Pos2::new(thumb_cx, mid_y), 1.5, color);

        // Percentage label right of track
        let pct = format!("{:.0}%", *value * 100.0);
        p.text(
            Pos2::new(track_r.max.x + VAL_W * 0.5 + 3.0, rect.center().y),
            egui::Align2::CENTER_CENTER,
            pct,
            egui::FontId::monospace(7.0),
            a(color, 180),
        );
    }

    resp
}

// ── VU METER — horizontal segmented ──────────────────────────────────────────

pub fn vu_horiz(ui: &mut Ui, value: f32, color: Color32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 5.0), Sense::hover());
    if !ui.is_rect_visible(rect) { return; }
    let p = ui.painter();
    p.rect_filled(rect, Rounding::same(1.0), VOID);
    let fill_w = rect.width() * value.clamp(0.0, 1.0);
    if fill_w > 0.5 {
        p.rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height())),
            Rounding::same(1.0),
            color,
        );
    }
    // Segmentation
    let seg   = 5.0_f32;
    let count = (rect.width() / seg) as u32;
    for i in 0..count {
        let x = rect.min.x + i as f32 * seg;
        p.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            Stroke::new(0.5, a(Color32::BLACK, 80)),
        );
    }
}

// ── RACK PAD (M / S / A style) ───────────────────────────────────────────────
//
// Small rectangular button that lights up when `active`.
// Returns true if clicked this frame.

pub fn rack_pad(ui: &mut Ui, label: &str, active: bool, color: Color32, w: f32, h: f32) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    if ui.is_rect_visible(r) {
        let fill   = if active { a(color, 55) } else { MOD_BASE };
        let border = if active { a(color, 180) } else { a(Color32::WHITE, 15) };
        let tcol   = if active { color } else { FG_MUTED };
        ui.painter().rect_filled(r, Rounding::same(2.0), fill);
        ui.painter().rect_stroke(r, Rounding::same(2.0), Stroke::new(0.5, border));
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER,
            label, egui::FontId::monospace(7.0), tcol);
    }
    resp.clicked()
}

// ── TRANSPORT PAD ─────────────────────────────────────────────────────────────
//
// Wider alloy-look pad for ◀◀ ▶ ■ ▶▶ buttons. Returns true if clicked.

pub fn transport_pad(ui: &mut Ui, label: &str, active: bool, color: Color32) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(28.0, 20.0), Sense::click());
    if ui.is_rect_visible(r) {
        let fill   = if active { a(color, 45) } else { RAISED };
        let border = if active { a(color, 160) } else { a(Color32::WHITE, 18) };
        let tcol   = if active { color } else { FG_2 };
        ui.painter().rect_filled(r, Rounding::same(3.0), fill);
        ui.painter().rect_stroke(r, Rounding::same(3.0), Stroke::new(0.5, border));
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER,
            label, egui::FontId::monospace(11.0), tcol);
    }
    resp.clicked()
}

// ── SCENE PAD ────────────────────────────────────────────────────────────────

pub fn scene_pad(ui: &mut Ui, label: &str, filled: bool, color: Color32, w: f32, h: f32) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    if ui.is_rect_visible(r) {
        let fill   = if filled { a(color, 40) } else { MOD_BASE };
        let border = if filled { a(color, 150) } else { a(Color32::WHITE, 12) };
        let tcol   = if filled { color } else { FG_MUTED };
        ui.painter().rect_filled(r, Rounding::same(2.0), fill);
        ui.painter().rect_stroke(r, Rounding::same(2.0), Stroke::new(0.5, border));
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER,
            label, egui::FontId::monospace(8.0), tcol);
    }
    resp.clicked()
}

// ── BOARD DECORATIONS ─────────────────────────────────────────────────────────
//
// Painted directly on a Rect (call after frame.show):
//   • Gold circuit L-brackets at top-left and bottom-right corners
//   • Left department accent bar (3 px)

pub fn board_decorations(ui: &mut Ui, rect: Rect, accent: Color32) {
    let p     = ui.painter();
    let mark  = 14.0_f32;
    let inset = 5.0_f32;

    // Top-left L
    p.line_segment(
        [Pos2::new(rect.min.x + inset,        rect.min.y + inset),
         Pos2::new(rect.min.x + inset + mark, rect.min.y + inset)],
        Stroke::new(1.0, GOLD_MARK),
    );
    p.line_segment(
        [Pos2::new(rect.min.x + inset, rect.min.y + inset),
         Pos2::new(rect.min.x + inset, rect.min.y + inset + mark)],
        Stroke::new(1.0, GOLD_MARK),
    );
    // Bottom-right L
    p.line_segment(
        [Pos2::new(rect.max.x - inset,        rect.max.y - inset),
         Pos2::new(rect.max.x - inset - mark, rect.max.y - inset)],
        Stroke::new(1.0, GOLD_MARK),
    );
    p.line_segment(
        [Pos2::new(rect.max.x - inset, rect.max.y - inset),
         Pos2::new(rect.max.x - inset, rect.max.y - inset - mark)],
        Stroke::new(1.0, GOLD_MARK),
    );
    // Left accent bar
    p.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
        Rounding { nw: 3.0, ne: 0.0, sw: 3.0, se: 0.0 },
        accent,
    );
}
