// Axiom Signal Chain Rack widget library for egui.
// Specs derived from rack.css and rack-primitives.jsx.

use egui::{Color32, Painter, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};
use crate::ui::theme;

// ── JACK KIND ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JackKind { Out, In, Cv }

// ── KNOB ─────────────────────────────────────────────────────────────────────
//
// CSS spec: --size: 40px default (xs=20, sm=28, md=40, lg=64)
// Body: radial-gradient dark sphere with top-left highlight
// Tick arc: 13 ticks at -135° to +135° (270° sweep), lit ticks use d-color
// Indicator line: top: 4px, height: size/3, transforms from center
// Drag: up = increase, shift = fine

pub fn knob(ui: &mut Ui, value: &mut f32, label: &str, color: Color32) -> Response {
    knob_sized(ui, value, label, color, 40.0)
}

pub fn knob_sm(ui: &mut Ui, value: &mut f32, label: &str, color: Color32) -> Response {
    knob_sized(ui, value, label, color, 28.0)
}

pub fn knob_xs(ui: &mut Ui, value: &mut f32, label: &str, color: Color32) -> Response {
    knob_sized(ui, value, label, color, 20.0)
}

pub fn knob_sized(ui: &mut Ui, value: &mut f32, label: &str, color: Color32, body_px: f32) -> Response {
    let tick_ext  = (body_px * 0.18).max(4.0); // ticks extend beyond knob body
    let label_h   = if label.is_empty() { 0.0 } else { 18.0 };
    let total_w   = body_px + tick_ext * 2.0;
    let total_h   = body_px + tick_ext * 2.0 + label_h;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(total_w, total_h), Sense::drag());

    if response.dragged() {
        let delta = response.drag_delta().y * (if ui.input(|i| i.modifiers.shift) { 0.002 } else { 0.006 });
        *value = (*value - delta).clamp(0.0, 1.0);
    }
    if response.double_clicked() {
        *value = 0.5;
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let c = Pos2::new(rect.min.x + tick_ext + body_px * 0.5, rect.min.y + tick_ext + body_px * 0.5);
        let r = body_px * 0.5 - 0.5;

        paint_knob_body(painter, c, r, color, *value, response.hovered(), response.dragged());

        // Label and value below the tick area
        if !label.is_empty() {
            let base_y = c.y + r + tick_ext + 1.0;
            let display = format!("{:03}", (*value * 100.0) as u32);
            painter.text(
                Pos2::new(c.x, base_y),
                egui::Align2::CENTER_TOP,
                label,
                egui::FontId::monospace(body_px * 0.175 + 4.5),
                theme::FG_3,
            );
            painter.text(
                Pos2::new(c.x, base_y + 9.0),
                egui::Align2::CENTER_TOP,
                display,
                egui::FontId::monospace(body_px * 0.175 + 4.5),
                color,
            );
        }
    }

    response
}

fn paint_knob_body(
    painter: &Painter, c: Pos2, r: f32, color: Color32,
    value: f32, hovered: bool, dragging: bool,
) {
    // Body — dark sphere: base fill, then subtle top-left highlight
    painter.circle_filled(c, r, theme::MOD_BASE);
    painter.circle_filled(
        Pos2::new(c.x - r * 0.17, c.y - r * 0.20),
        r * 0.55,
        Color32::from_rgba_unmultiplied(255, 255, 255, 8),
    );

    // Outer bezel strokes (bright top, dark bottom — milled aluminum effect)
    painter.circle_stroke(c, r, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 25)));
    painter.circle_stroke(c, r - 1.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 160)));

    // Tick marks (13 ticks at -135° to +135°, lit = filled with color)
    let tick_r_outer = r + r * 0.35;
    let tick_r_inner = r + r * 0.15;
    for i in 0u32..=12 {
        let frac = i as f32 / 12.0;
        let angle_deg = -135.0_f32 + frac * 270.0;
        let rad = (angle_deg - 90.0).to_radians();
        let lit = frac <= value;
        let tick_color = if lit { color } else { Color32::from_rgba_unmultiplied(120, 180, 255, 46) };
        let tick_w = if lit { 1.5_f32 } else { 1.0_f32 };
        let p_outer = Pos2::new(c.x + rad.cos() * tick_r_outer, c.y + rad.sin() * tick_r_outer);
        let p_inner = Pos2::new(c.x + rad.cos() * tick_r_inner, c.y + rad.sin() * tick_r_inner);
        painter.line_segment([p_inner, p_outer], Stroke::new(tick_w, tick_color));
    }

    // Glow ring when hovered/dragging
    if hovered || dragging {
        let glow_a = if dragging { 50 } else { 25 };
        let glow_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), glow_a);
        painter.circle_stroke(c, r + r * 0.5, Stroke::new(r * 0.6, glow_color));
    }

    // Indicator line (rotates from center to rim edge)
    let angle_deg = -135.0_f32 + value * 270.0;
    let rad = (angle_deg - 90.0).to_radians();
    let ind_inner = 4.0_f32.min(r * 0.2);
    let ind_outer = r * 0.82;
    let p_start = Pos2::new(c.x + rad.cos() * ind_inner, c.y + rad.sin() * ind_inner);
    let p_end   = Pos2::new(c.x + rad.cos() * ind_outer, c.y + rad.sin() * ind_outer);
    // Glow pass
    painter.line_segment([p_start, p_end], Stroke::new(4.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 50)));
    // Sharp indicator
    painter.line_segment([p_start, p_end], Stroke::new(2.0, color));
}

// ── FADER — vertical ──────────────────────────────────────────────────────────
//
// CSS spec: fader-track height=180px, width=14px
// Thumb: linear-gradient(180deg, #d0d4dc, #6b7383 50%, #2a2e38) with side LED pip
// Drag: position set from pointer Y on track

pub fn fader(ui: &mut Ui, value: &mut f32, color: Color32) -> Response {
    fader_h(ui, value, color, 180.0)
}

pub fn fader_h(ui: &mut Ui, value: &mut f32, color: Color32, height: f32) -> Response {
    let track_w  = 14.0_f32;
    let pip_pad  = 8.0_f32;   // space for LED pip on the right
    let value_h  = 16.0_f32;
    let total_w  = track_w + pip_pad + 2.0;
    let total_h  = height + value_h;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(total_w, total_h), Sense::drag());

    if response.dragged() {
        let dy = response.drag_delta().y / height;
        *value = (*value - dy).clamp(0.0, 1.0);
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let track_rect = Rect::from_min_size(
            Pos2::new(rect.min.x, rect.min.y),
            Vec2::new(track_w, height),
        );

        // Track background
        painter.rect_filled(track_rect, Rounding::same(2.0), theme::ABYSS);

        // Segmentation marks (CSS: repeating-linear-gradient every 5px)
        let seg_step = 5.0_f32;
        let seg_count = (height / seg_step) as u32;
        for i in 0..seg_count {
            let y = track_rect.min.y + i as f32 * seg_step;
            painter.line_segment(
                [Pos2::new(track_rect.min.x, y), Pos2::new(track_rect.max.x, y)],
                Stroke::new(0.5, Color32::from_rgba_unmultiplied(0, 0, 0, 100)),
            );
        }

        // Lit fill (bottom = 0, value = 1 at top)
        let lit_top = track_rect.min.y + (1.0 - *value) * height;
        if *value > 0.005 {
            let lit_rect = Rect::from_min_max(
                Pos2::new(track_rect.min.x + 1.0, lit_top),
                Pos2::new(track_rect.max.x - 1.0, track_rect.max.y - 1.0),
            );
            let lit_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 70);
            painter.rect_filled(lit_rect, Rounding::same(1.0), lit_color);
        }

        // Thumb — metallic gradient (top light → mid gray → bottom dark)
        let thumb_h = 18.0_f32;
        let thumb_rect = Rect::from_center_size(
            Pos2::new(track_rect.center().x, lit_top),
            Vec2::new(track_w + 4.0, thumb_h),
        );
        // Top half: #d0d4dc → #6b7383
        let top_half = Rect::from_min_max(thumb_rect.min, Pos2::new(thumb_rect.max.x, thumb_rect.center().y));
        let bot_half = Rect::from_min_max(Pos2::new(thumb_rect.min.x, thumb_rect.center().y), thumb_rect.max);
        painter.rect_filled(top_half, Rounding { nw: 3.0, ne: 3.0, sw: 0.0, se: 0.0 }, Color32::from_rgb(208, 212, 220));
        painter.rect_filled(bot_half, Rounding { nw: 0.0, ne: 0.0, sw: 3.0, se: 3.0 }, Color32::from_rgb(42, 46, 56));
        // Mid groove line
        painter.line_segment(
            [Pos2::new(thumb_rect.min.x + 3.0, thumb_rect.center().y),
             Pos2::new(thumb_rect.max.x - 3.0, thumb_rect.center().y)],
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(0, 0, 0, 120)),
        );
        // Thumb border
        painter.rect_stroke(thumb_rect, Rounding::same(3.0), Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 30)));

        // LED pip on right side
        let pip_x = track_rect.max.x + pip_pad * 0.5;
        let pip_c = Pos2::new(pip_x, thumb_rect.center().y);
        painter.circle_filled(pip_c, 2.5, color);
        painter.circle_stroke(pip_c, 4.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 60)));

        // Value label
        let display = format!("{:03}", (*value * 127.0) as u32);
        painter.text(
            Pos2::new(track_rect.center().x, track_rect.max.y + 3.0),
            egui::Align2::CENTER_TOP,
            display,
            egui::FontId::monospace(8.0),
            color,
        );
    }

    response
}

// ── JACK ──────────────────────────────────────────────────────────────────────
//
// CSS spec: 22px circle, radial-gradient:
//   #03050a 0–38%, #14171f 40%, #2a2e38 60%, #14171f 100%
// Out border = d-color, In border = muted d-color, Cv border = astral-violet

pub fn jack(ui: &mut Ui, kind: JackKind, label: &str, color: Color32) {
    let size = 22.0_f32;
    let label_h = if label.is_empty() { 0.0 } else { 10.0 };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size + label_h), Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let c = Pos2::new(rect.center().x, rect.min.y + size * 0.5);
        let r = size * 0.5 - 0.5;

        // Concentric rings (darkest core → lighter rim → outer ring)
        painter.circle_filled(c, r, theme::MOD_MID);                                              // outer body #14171f
        painter.circle_filled(c, r * 0.60, Color32::from_rgb(42, 46, 56));                        // ring #2a2e38
        painter.circle_filled(c, r * 0.40, Color32::from_rgb(20, 23, 28));                        // ring #14171f
        painter.circle_filled(c, r * 0.38, theme::VOID);                                          // core #03050a

        // Border by kind
        let border_color = match kind {
            JackKind::Out => color,
            JackKind::In  => Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 150),
            JackKind::Cv  => theme::ASTRAL_VIOLET,
        };
        painter.circle_stroke(c, r, Stroke::new(1.0, border_color));

        if !label.is_empty() {
            painter.text(
                Pos2::new(c.x, rect.min.y + size + 1.0),
                egui::Align2::CENTER_TOP,
                label,
                egui::FontId::monospace(6.0),
                theme::FG_MUTED,
            );
        }
    }
}

// ── LED ───────────────────────────────────────────────────────────────────────

pub fn led(ui: &mut Ui, on: bool, color: Color32) {
    let size = 6.0_f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size + 4.0), Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let c = rect.center();
        let c_draw = if on { color } else { theme::LED_OFF };
        painter.circle_filled(c, 3.0, c_draw);
        if on {
            painter.circle_stroke(c, 5.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 80)));
        }
    }
}

// ── WIRE PIP ──────────────────────────────────────────────────────────────────
// Colored dot + wire code label (e.g. "SPA")

pub fn wire_pip(ui: &mut Ui, wire: &str) {
    let color = theme::wire_color(wire);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
        let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
        if ui.is_rect_visible(dot_rect) {
            let p = ui.painter();
            p.circle_filled(dot_rect.center(), 2.5, color);
            p.circle_stroke(dot_rect.center(), 4.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 60)));
        }
        ui.label(
            egui::RichText::new(wire)
                .font(egui::FontId::monospace(7.0))
                .color(theme::FG_3),
        );
    });
}

// ── MODULE PANEL ──────────────────────────────────────────────────────────────
//
// CSS spec: panel-alloy background, 4px border-radius, corner gold L-brackets
// at top-left (top/left borders) and bottom-right (bottom/right borders),
// rack screws (8px circles) top and bottom.

pub fn module_panel(ui: &mut Ui, dept_color: Color32, add_contents: impl FnOnce(&mut Ui)) {
    let frame = egui::Frame::none()
        .fill(theme::MOD_MID)
        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(120, 180, 255, 18)))
        .inner_margin(egui::Margin::same(18.0))
        .rounding(Rounding::same(4.0));

    let resp = frame.show(ui, |ui| {
        add_contents(ui);
    });

    let rect = resp.response.rect;
    let painter = ui.painter();

    // Corner circuit-mark L-brackets (gold, 18px arms)
    let mark = 18.0_f32;
    let inset = 6.0_f32;
    // Top-left
    painter.line_segment(
        [Pos2::new(rect.min.x + inset, rect.min.y + inset), Pos2::new(rect.min.x + inset + mark, rect.min.y + inset)],
        Stroke::new(1.0, theme::GOLD_MARK),
    );
    painter.line_segment(
        [Pos2::new(rect.min.x + inset, rect.min.y + inset), Pos2::new(rect.min.x + inset, rect.min.y + inset + mark)],
        Stroke::new(1.0, theme::GOLD_MARK),
    );
    // Bottom-right
    painter.line_segment(
        [Pos2::new(rect.max.x - inset, rect.max.y - inset), Pos2::new(rect.max.x - inset - mark, rect.max.y - inset)],
        Stroke::new(1.0, theme::GOLD_MARK),
    );
    painter.line_segment(
        [Pos2::new(rect.max.x - inset, rect.max.y - inset), Pos2::new(rect.max.x - inset, rect.max.y - inset - mark)],
        Stroke::new(1.0, theme::GOLD_MARK),
    );

    // Rack-ear screws (8px circles, top and bottom)
    let screw_r = 4.0_f32;
    let screw_y_t = rect.min.y + 8.0;
    let screw_y_b = rect.max.y - 8.0;
    for &sx in &[rect.min.x + 12.0, rect.max.x - 12.0] {
        for &sy in &[screw_y_t, screw_y_b] {
            let sc = Pos2::new(sx, sy);
            painter.circle_filled(sc, screw_r, theme::SCREW_MID);
            painter.circle_filled(sc, screw_r - 1.5, theme::SCREW_EDGE);
            // Slot mark
            painter.line_segment(
                [Pos2::new(sc.x - 1.5, sc.y - 1.5), Pos2::new(sc.x + 1.5, sc.y + 1.5)],
                Stroke::new(0.5, Color32::from_rgba_unmultiplied(0, 0, 0, 140)),
            );
        }
    }

    // Left department accent bar (3px)
    painter.rect_filled(
        Rect::from_min_size(Pos2::new(rect.min.x, rect.min.y), Vec2::new(3.0, rect.height())),
        Rounding { nw: 4.0, ne: 0.0, sw: 4.0, se: 0.0 },
        dept_color,
    );
}

// ── CHANNEL STRIP ─────────────────────────────────────────────────────────────
//
// CSS spec: 62px wide, flex column, alloy dark bg, wire-type color driven.
// Contains: head label (wire color), optional sub label, stacked knobs,
// M/S/A pads, meter + fader side-by-side, wire pip + jack row.

pub struct ChannelStripState {
    pub knob_values: Vec<f32>,
    pub fader_value: f32,
    pub meter_value: f32,
    pub mute: bool,
    pub solo: bool,
    pub arm: bool,
}

impl ChannelStripState {
    pub fn new(knob_count: usize) -> Self {
        Self {
            knob_values: vec![0.5; knob_count],
            fader_value: 0.75,
            meter_value: 0.5,
            mute: false,
            solo: false,
            arm: true,
        }
    }
}

pub fn channel_strip(
    ui: &mut Ui,
    name: &str,
    wire: &str,
    state: &mut ChannelStripState,
    knob_labels: &[&str],
) {
    let wire_color = theme::wire_color(wire);

    let frame = egui::Frame::none()
        .fill(theme::ABYSS)
        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(120, 180, 255, 26)))
        .inner_margin(egui::Margin::symmetric(5.0, 8.0))
        .rounding(Rounding::same(3.0));

    frame.show(ui, |ui| {
        ui.set_width(52.0); // inner = 62 - 2*5 padding
        ui.set_min_height(200.0);

        // Head label (wire color, letterspace)
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(name)
                    .font(egui::FontId::proportional(8.0))
                    .color(wire_color),
            );
        });

        ui.add_space(2.0);

        // Knobs (2-col grid if 4+, else 1-col)
        if !knob_labels.is_empty() {
            let cols = if knob_labels.len() >= 4 { 2 } else { 1 };
            egui::Grid::new(format!("cs_knobs_{name}"))
                .num_columns(cols)
                .spacing([3.0, 3.0])
                .show(ui, |ui| {
                    for (i, &lbl) in knob_labels.iter().enumerate() {
                        if i < state.knob_values.len() {
                            knob_xs(ui, &mut state.knob_values[i], lbl, wire_color);
                        }
                        if cols == 2 && i % 2 == 1 { ui.end_row(); }
                    }
                });
        }

        ui.add_space(3.0);

        // M / S / A pads
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
            for (label, state_ref) in [("M", &mut state.mute), ("S", &mut state.solo), ("A", &mut state.arm)] {
                let pad_color = if *state_ref {
                    Color32::from_rgba_unmultiplied(wire_color.r(), wire_color.g(), wire_color.b(), 60)
                } else {
                    theme::SURFACE
                };
                let text_color = if *state_ref { wire_color } else { theme::FG_MUTED };
                let (r, resp) = ui.allocate_exact_size(Vec2::new(14.0, 12.0), Sense::click());
                if resp.clicked() { *state_ref = !*state_ref; }
                if ui.is_rect_visible(r) {
                    ui.painter().rect_filled(r, Rounding::same(1.0), pad_color);
                    ui.painter().rect_stroke(r, Rounding::same(1.0), Stroke::new(0.5, theme::BORDER));
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::monospace(5.5), text_color);
                }
            }
        });

        ui.add_space(3.0);

        // Meter + fader side-by-side
        ui.horizontal(|ui| {
            // VU meter (8×60)
            let meter_h = 60.0_f32;
            let (mr, _) = ui.allocate_exact_size(Vec2::new(8.0, meter_h), Sense::hover());
            if ui.is_rect_visible(mr) {
                let p = ui.painter();
                p.rect_filled(mr, Rounding::same(2.0), theme::VOID);
                let fill_h = state.meter_value * meter_h;
                let fill = Rect::from_min_max(
                    Pos2::new(mr.min.x, mr.max.y - fill_h),
                    mr.max,
                );
                // Gradient: bio bottom → gold mid → ember top
                p.rect_filled(fill, Rounding::same(1.0), wire_color);
                // Segment marks
                let seg = 5.0_f32;
                let count = (meter_h / seg) as u32;
                for i in 0..count {
                    let y = mr.min.y + i as f32 * seg;
                    p.line_segment(
                        [Pos2::new(mr.min.x, y), Pos2::new(mr.max.x, y)],
                        Stroke::new(0.5, Color32::from_rgba_unmultiplied(0, 0, 0, 100)),
                    );
                }
            }
            ui.add_space(2.0);
            // Mini fader
            fader_h(ui, &mut state.fader_value, wire_color, 76.0);
        });

        ui.add_space(3.0);

        // Wire pip + separator
        ui.separator();
        ui.vertical_centered(|ui| {
            wire_pip(ui, wire);
        });
    });
}

// ── STATUS BADGE ──────────────────────────────────────────────────────────────

pub fn status_badge(ui: &mut Ui, status: &mythos::quantum_module::ImplementationStatus) {
    use mythos::quantum_module::ImplementationStatus;
    let (label, color) = match status {
        ImplementationStatus::Built      => ("BUILT",    theme::BIO),
        ImplementationStatus::InProgress => ("WIP",      theme::ASTRAL_CYAN),
        ImplementationStatus::Planned    => ("PLANNED",  theme::FG_MUTED),
    };
    ui.label(egui::RichText::new(label).font(egui::FontId::monospace(7.5)).color(color));
}

// ── METER BAR (horizontal resonance) ────────────────────────────────────────

pub fn meter_bar(ui: &mut Ui, value: f32, color: Color32) {
    let desired = Vec2::new(ui.available_width(), 4.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, Rounding::same(2.0), theme::VOID);
        let fill_w = rect.width() * value.clamp(0.0, 1.0);
        p.rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height())),
            Rounding::same(2.0),
            color,
        );
    }
}
