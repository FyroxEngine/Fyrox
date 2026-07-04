use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use std::f32::consts::PI;

use crate::theme;

// Standard knob sweep: 135° → 405° (270° total), going clockwise on screen.
// At value=0.0 → 7–8 o'clock;  value=0.5 → 12 o'clock;  value=1.0 → 4–5 o'clock.
const KNOB_START: f32 = PI * 0.75;   // 135° — lower-left
const KNOB_SWEEP: f32 = PI * 1.5;    // 270°

// ─── Arc drawing ─────────────────────────────────────────────────────────────

fn arc(painter: &Painter, center: Pos2, radius: f32, a0: f32, a1: f32, width: f32, color: Color32) {
    if (a1 - a0).abs() < 0.001 { return; }
    let steps = ((a1 - a0).abs() * radius * 0.5).max(6.0) as usize + 1;
    let pts: Vec<Pos2> = (0..=steps).map(|i| {
        let t = i as f32 / steps as f32;
        let a = a0 + (a1 - a0) * t;
        Pos2::new(center.x + radius * a.cos(), center.y + radius * a.sin())
    }).collect();
    painter.add(egui::Shape::line(pts, Stroke::new(width, color)));
}

// ─── Knob ─────────────────────────────────────────────────────────────────────

pub struct Knob<'a> {
    pub value:    &'a mut f32,
    pub color:    Color32,
    pub diameter: f32,
    pub label:    Option<&'a str>,
}

impl<'a> Knob<'a> {
    pub fn new(value: &'a mut f32, color: Color32) -> Self {
        Self { value, color, diameter: 36.0, label: None }
    }
    pub fn size(mut self, d: f32) -> Self { self.diameter = d; self }
    pub fn label(mut self, l: &'a str) -> Self { self.label = Some(l); self }

    pub fn show(self, ui: &mut Ui) -> Response {
        let total_h = self.diameter + if self.label.is_some() { 12.0 } else { 0.0 };
        let (outer, response) = ui.allocate_exact_size(
            Vec2::new(self.diameter, total_h),
            Sense::click_and_drag(),
        );
        let rect = Rect::from_min_size(outer.min, Vec2::splat(self.diameter));

        if response.dragged() {
            *self.value = (*self.value - response.drag_delta().y * 0.006).clamp(0.0, 1.0);
        }

        if ui.is_rect_visible(outer) {
            paint_knob(ui.painter(), rect, *self.value, self.color, &response);

            if let Some(lbl) = self.label {
                let lbl_pos = Pos2::new(rect.center().x, rect.bottom() + 3.0);
                ui.painter().text(
                    lbl_pos, egui::Align2::CENTER_TOP, lbl,
                    theme::mono(6.5), theme::FG_MUTED,
                );
            }
        }

        response
    }
}

fn paint_knob(painter: &Painter, rect: Rect, value: f32, color: Color32, resp: &Response) {
    let center = rect.center();
    let r = rect.width() * 0.44;
    let hovered = resp.hovered() || resp.dragged();

    // Glow backdrop on hover
    if hovered {
        for i in 1u8..=4 {
            painter.circle_filled(
                center,
                r + i as f32 * 2.5,
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 4 * (5 - i)),
            );
        }
    }

    // Base fill
    painter.circle_filled(
        center, r,
        if hovered { theme::ELEVATED } else { theme::SURFACE },
    );

    // Bezel rim
    painter.circle_stroke(
        center, r,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 220, 255, 22)),
    );

    let track_r = r * 0.72;

    // Full-range track (dim)
    arc(painter, center, track_r,
        KNOB_START, KNOB_START + KNOB_SWEEP,
        2.5, Color32::from_rgba_unmultiplied(40, 60, 80, 140));

    // Value arc (colored)
    if value > 0.002 {
        arc(painter, center, track_r,
            KNOB_START, KNOB_START + value * KNOB_SWEEP,
            2.5, color);
    }

    // Indicator pip at value position
    let angle = KNOB_START + value * KNOB_SWEEP;
    let pip_pos = Pos2::new(
        center.x + track_r * angle.cos(),
        center.y + track_r * angle.sin(),
    );
    painter.circle_filled(pip_pos, if hovered { 2.5 } else { 2.0 }, color);
    if hovered {
        painter.circle_filled(
            pip_pos, 4.5,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40),
        );
    }

    // Centre pip
    painter.circle_filled(center, 2.5, theme::INLAY);
    painter.circle_stroke(
        center, 2.5,
        Stroke::new(0.5, Color32::from_rgba_unmultiplied(180, 210, 255, 35)),
    );
}

// ─── Fader ────────────────────────────────────────────────────────────────────

pub struct Fader<'a> {
    pub value:  &'a mut f32,
    pub color:  Color32,
    pub height: f32,
    pub width:  f32,
}

impl<'a> Fader<'a> {
    pub fn new(value: &'a mut f32, color: Color32) -> Self {
        Self { value, color, height: 80.0, width: 14.0 }
    }
    pub fn height(mut self, h: f32) -> Self { self.height = h; self }

    pub fn show(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(self.width, self.height),
            Sense::click_and_drag(),
        );

        if response.dragged() {
            *self.value = (*self.value - response.drag_delta().y / self.height).clamp(0.0, 1.0);
        }

        if ui.is_rect_visible(rect) {
            paint_fader(ui.painter(), rect, *self.value, self.color, &response);
        }

        response
    }
}

fn paint_fader(painter: &Painter, rect: Rect, value: f32, color: Color32, resp: &Response) {
    let cx   = rect.center().x;
    let top  = rect.top()    + 5.0;
    let bot  = rect.bottom() - 5.0;
    let h    = bot - top;
    let tw   = 3.0;

    // Track
    let track = Rect::from_x_y_ranges((cx - tw)..=(cx + tw), top..=bot);
    painter.rect_filled(track, egui::Rounding::same(2.0), theme::INLAY);

    // Fill
    let fill_y = top + (1.0 - value) * h;
    if fill_y < bot {
        let fill = Rect::from_x_y_ranges((cx - tw)..=(cx + tw), fill_y..=bot);
        painter.rect_filled(fill, egui::Rounding::same(2.0), theme::with_alpha(color, 200));
    }

    // Handle
    let hy = top + (1.0 - value) * h;
    let hovered = resp.hovered() || resp.dragged();
    let handle = Rect::from_center_size(
        Pos2::new(cx, hy),
        Vec2::new(rect.width() - 1.0, 7.0),
    );
    painter.rect_filled(
        handle, egui::Rounding::same(2.0),
        if hovered { theme::ELEVATED } else { theme::RAISED },
    );
    painter.rect_stroke(
        handle, egui::Rounding::same(2.0),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 220, 255, if hovered { 50 } else { 20 })),
    );
}

// ─── Pad (M / S / A toggle button) ───────────────────────────────────────────

pub struct Pad<'a> {
    pub value: &'a mut bool,
    pub label: &'a str,
    pub color: Color32,
}

impl<'a> Pad<'a> {
    pub fn new(value: &'a mut bool, label: &'a str, color: Color32) -> Self {
        Self { value, label, color }
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(18.0, 13.0),
            Sense::click(),
        );

        if response.clicked() {
            *self.value = !*self.value;
        }

        if ui.is_rect_visible(rect) {
            let lit = *self.value;
            let r = egui::Rounding::same(2.0);

            painter_pad(ui.painter(), rect, lit, self.color, r, response.hovered());

            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                self.label,
                theme::mono(6.5),
                if lit { self.color } else { theme::FG_MUTED },
            );
        }

        response
    }
}

fn painter_pad(painter: &Painter, rect: Rect, lit: bool, color: Color32, r: egui::Rounding, hovered: bool) {
    let fill = if lit {
        Color32::from_rgba_unmultiplied(color.r() / 5, color.g() / 5, color.b() / 5, 255)
    } else if hovered {
        theme::ELEVATED
    } else {
        theme::SURFACE
    };
    painter.rect_filled(rect, r, fill);
    painter.rect_stroke(rect, r, Stroke::new(1.0, if lit { color } else { theme::FG_MUTED }));

    if lit {
        painter.rect_stroke(
            rect.expand(1.5), r,
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 50)),
        );
    }
}

// ─── Jack socket ─────────────────────────────────────────────────────────────

pub struct Jack<'a> {
    pub is_input:  bool,
    pub color:     Color32,
    pub label:     Option<&'a str>,
    pub connected: bool,
}

impl<'a> Jack<'a> {
    pub fn out(color: Color32) -> Self {
        Self { is_input: false, color, label: None, connected: false }
    }
    pub fn input(color: Color32) -> Self {
        Self { is_input: true, color, label: None, connected: false }
    }
    pub fn label(mut self, l: &'a str) -> Self { self.label = Some(l); self }

    pub fn show(self, ui: &mut Ui) -> Response {
        let h = if self.label.is_some() { 26.0 } else { 18.0 };
        let (rect, response) = ui.allocate_exact_size(Vec2::new(18.0, h), Sense::click());

        if ui.is_rect_visible(rect) {
            let center = Pos2::new(rect.center().x, rect.top() + 9.0);
            let ring_r = 6.0;
            let hovered = response.hovered();
            let lit = hovered || self.connected;

            // Fill
            let fill = if self.connected {
                Color32::from_rgba_unmultiplied(
                    self.color.r() / 5, self.color.g() / 5, self.color.b() / 5, 255)
            } else {
                theme::INLAY
            };
            ui.painter().circle_filled(center, ring_r, fill);

            // Ring
            ui.painter().circle_stroke(
                center, ring_r,
                Stroke::new(if lit { 1.5 } else { 1.0 }, if lit { self.color } else { theme::FG_MUTED }),
            );

            // Direction glyph
            let glyph = if self.is_input { "▾" } else { "▴" };
            ui.painter().text(
                center, egui::Align2::CENTER_CENTER, glyph,
                theme::mono(7.0),
                if lit { self.color } else { theme::FG_MUTED },
            );

            // Label
            if let Some(lbl) = self.label {
                ui.painter().text(
                    Pos2::new(center.x, rect.bottom() - 1.0),
                    egui::Align2::CENTER_BOTTOM,
                    lbl,
                    theme::mono(5.5),
                    theme::FG_MUTED,
                );
            }
        }

        response
    }
}

// ─── Segmented level meter ────────────────────────────────────────────────────

pub fn level_meter(painter: &Painter, rect: Rect, level: f32, color: Color32) {
    painter.rect_filled(rect, egui::Rounding::same(2.0), theme::INLAY);

    let segs  = 14usize;
    let gap   = 1.0f32;
    let seg_h = (rect.height() - gap * (segs - 1) as f32) / segs as f32;
    let lit   = (level * segs as f32) as usize;

    for i in 0..segs {
        let y = rect.bottom() - (i as f32) * (seg_h + gap) - seg_h;
        let seg = Rect::from_min_size(
            Pos2::new(rect.left() + 1.0, y),
            Vec2::new(rect.width() - 2.0, seg_h),
        );
        let fill = if i < lit {
            match i {
                n if n >= segs * 5 / 6 => theme::EMBER,
                n if n >= segs * 2 / 3 => theme::GOLD,
                _                      => color,
            }
        } else {
            Color32::from_rgba_unmultiplied(20, 32, 45, 180)
        };
        painter.rect_filled(seg, 0.0, fill);
    }
}
