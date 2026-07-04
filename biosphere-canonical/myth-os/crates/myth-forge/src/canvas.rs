use egui::{Color32, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use crate::{
    scene::{AtomKind, AtomParams, CanvasScene, PlacedAtom},
    theme,
};

// ── Mode ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode { Atoms, Panels }

// ── Canvas interaction state ──────────────────────────────────────────────────

pub struct CanvasState {
    pub offset:      Vec2,
    pub zoom:        f32,
    pub drag_id:     Option<u64>,
    pub drag_offset: Vec2,
    pub sel_panel:   Option<u64>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self { offset: Vec2::ZERO, zoom: 1.0,
               drag_id: None, drag_offset: Vec2::ZERO, sel_panel: None }
    }
}

// ── Main entry ────────────────────────────────────────────────────────────────

pub fn draw(
    ui:       &mut Ui,
    scene:    &mut CanvasScene,
    state:    &mut CanvasState,
    selected: &mut Option<u64>,   // selected atom id
    mode:     &AppMode,
) {
    let (canvas_rect, response) =
        ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let p = ui.painter_at(canvas_rect);

    draw_grid(&p, canvas_rect, state.offset, state.zoom);

    // Pan
    if response.dragged_by(egui::PointerButton::Middle) {
        state.offset += response.drag_delta();
    }
    // Zoom
    let scroll = ui.input(|i| i.raw_scroll_delta.y);
    if scroll != 0.0 {
        state.zoom = (state.zoom * (1.0 + scroll * 0.001)).clamp(0.3, 4.0);
    }

    let origin: Pos2 = canvas_rect.min + state.offset;
    let pointer = ui.input(|i| i.pointer.interact_pos());

    // ── Panel layer ───────────────────────────────────────────────────────────
    let canvas_local = Rect::from_min_size(Pos2::ZERO,
        Vec2::new(canvas_rect.width() / state.zoom, canvas_rect.height() / state.zoom));
    let all_panels   = scene.root.layout_all(canvas_local);

    draw_panels(&p, &all_panels, origin, state.zoom,
        if *mode == AppMode::Panels { state.sel_panel } else { None });

    // Panel mode interaction
    if *mode == AppMode::Panels {
        handle_panel_input(ui, scene, state, &all_panels, origin, pointer, &response);
    }

    // ── Atom layer ────────────────────────────────────────────────────────────
    if *mode == AppMode::Atoms {
        handle_atom_input(ui, scene, state, selected, origin, pointer, &response, canvas_rect);
    }

    for atom in &scene.atoms {
        let screen = atom_screen_rect(atom, origin, state.zoom);
        let is_sel  = *mode == AppMode::Atoms && *selected == Some(atom.id);
        draw_atom(&p, atom, screen, is_sel, state.zoom);
    }

    // Drop ghost
    if let Some(drag_kind) = ui.memory(|m| m.data.get_temp::<AtomKind>(egui::Id::new("drag_atom"))) {
        if let Some(pos) = pointer {
            if canvas_rect.contains(pos) {
                let lv   = (pos - origin) / state.zoom;
                let sz   = drag_kind.default_size() * state.zoom;
                let ghost = Rect::from_min_size(origin + lv * state.zoom, sz);
                p.rect_stroke(ghost, Rounding::same(3.0),
                    Stroke::new(1.5, theme::with_alpha(theme::QUANTUM, 120)));
            }
        }
    }

    // Accept drop
    if response.hovered() {
        if let Some(kind) = ui.memory(|m| m.data.get_temp::<AtomKind>(egui::Id::new("drag_atom"))) {
            if ui.input(|i| i.pointer.any_released()) {
                if let Some(pos) = pointer {
                    let lv    = (pos - origin) / state.zoom;
                    let local = Pos2::new(lv.x, lv.y);
                    let id    = scene.add_atom(kind.clone(), local);
                    *selected = Some(id);
                    ui.memory_mut(|m| m.data.remove::<AtomKind>(egui::Id::new("drag_atom")));
                }
            }
        }
    }
}

// ── Panel drawing ─────────────────────────────────────────────────────────────

fn draw_panels(
    p:          &egui::Painter,
    all:        &[(u64, Rect, usize)],
    origin:     Pos2,
    zoom:       f32,
    selected:   Option<u64>,
) {
    for (id, local_rect, depth) in all {
        let screen = Rect::from_min_size(
            origin + local_rect.min.to_vec2() * zoom,
            local_rect.size() * zoom);

        // Only draw background on leaves (depth renders innermost fill)
        let border_alpha = if *depth == 0 { 30u8 } else { 60 };
        p.rect_stroke(screen, Rounding::same(2.0),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(60, 100, 160, border_alpha)));

        if Some(*id) == selected {
            p.rect_stroke(screen.expand(1.0), Rounding::same(3.0),
                Stroke::new(1.5, theme::with_alpha(theme::QUANTUM, 180)));
            // Name tag
            p.rect_filled(
                Rect::from_min_size(screen.min, Vec2::new(60.0 * zoom, 14.0 * zoom)),
                Rounding::same(2.0), theme::with_alpha(theme::QUANTUM, 40));
            p.text(screen.min + Vec2::new(4.0, 3.0) * zoom,
                egui::Align2::LEFT_TOP, "PANEL",
                theme::mono(6.5 * zoom), theme::QUANTUM);
        }
    }
}

fn handle_panel_input(
    ui:         &mut Ui,
    scene:      &mut CanvasScene,
    state:      &mut CanvasState,
    all:        &[(u64, Rect, usize)],
    origin:     Pos2,
    pointer:    Option<Pos2>,
    response:   &egui::Response,
) {
    if response.clicked() {
        if let Some(pos) = pointer {
            let lv    = (pos - origin) / state.zoom;
            let local = Pos2::new(lv.x, lv.y);
            // Select deepest panel under cursor
            let hit = all.iter().rev()
                .find(|(_, r, _)| r.contains(local))
                .map(|(id, _, _)| *id);
            state.sel_panel = hit;
        }
    }

    // Delete key removes selected panel
    if let Some(pid) = state.sel_panel {
        if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            scene.remove_panel(pid);
            state.sel_panel = None;
        }
    }
}

// ── Atom interaction ──────────────────────────────────────────────────────────

fn handle_atom_input(
    ui:          &mut Ui,
    scene:       &mut CanvasScene,
    state:       &mut CanvasState,
    selected:    &mut Option<u64>,
    origin:      Pos2,
    pointer:     Option<Pos2>,
    response:    &egui::Response,
    _canvas:     Rect,
) {
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(pos) = pointer {
            let lv    = (pos - origin) / state.zoom;
            let local = Pos2::new(lv.x, lv.y);
            let hit   = scene.atoms.iter().rev()
                .find(|a| !a.locked && a.rect().contains(local))
                .map(|a| a.id);
            if let Some(id) = hit {
                let ap        = scene.atoms.iter().find(|a| a.id == id).unwrap().pos;
                state.drag_id = Some(id);
                state.drag_offset = Vec2::new(local.x - ap[0], local.y - ap[1]);
                *selected = Some(id);
                scene.move_to_front(id);
            } else {
                *selected = None;
                state.drag_id = None;
            }
        }
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        if let (Some(id), Some(pos)) = (state.drag_id, pointer) {
            let lv = (pos - origin) / state.zoom;
            if let Some(atom) = scene.get_atom_mut(id) {
                atom.pos[0] = (lv.x - state.drag_offset.x).max(0.0);
                atom.pos[1] = (lv.y - state.drag_offset.y).max(0.0);
            }
        }
    }

    if response.drag_stopped() { state.drag_id = None; }

    if response.clicked() {
        if let Some(pos) = pointer {
            let lv    = (pos - origin) / state.zoom;
            let local = Pos2::new(lv.x, lv.y);
            *selected = scene.atoms.iter().rev()
                .find(|a| a.rect().contains(local))
                .map(|a| a.id);
        }
    }

    if let Some(id) = *selected {
        if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            scene.remove_atom(id);
            *selected = None;
        }
    }
}

// ── Atom screen rect ──────────────────────────────────────────────────────────

fn atom_screen_rect(atom: &PlacedAtom, origin: Pos2, zoom: f32) -> Rect {
    Rect::from_min_size(
        origin + Vec2::new(atom.pos[0], atom.pos[1]) * zoom,
        Vec2::new(atom.size[0], atom.size[1]) * zoom)
}

// ── Draw a single atom ────────────────────────────────────────────────────────

fn draw_atom(p: &egui::Painter, atom: &PlacedAtom, rect: Rect, selected: bool, zoom: f32) {
    match &atom.kind {
        AtomKind::Knob    => draw_knob(p, rect, &atom.params, zoom),
        AtomKind::Fader   => draw_fader(p, rect, &atom.params, zoom),
        AtomKind::Pad     => draw_pad(p, rect, &atom.params, zoom),
        AtomKind::Jack    => draw_jack(p, rect, &atom.params, zoom),
        AtomKind::Meter   => draw_meter(p, rect, &atom.params, zoom),
        AtomKind::Scope   => draw_scope(p, rect, zoom),
        AtomKind::Label   => draw_label(p, rect, &atom.params, zoom),
        AtomKind::Divider => draw_divider(p, rect),
    }
    if selected {
        p.rect_stroke(rect.expand(2.0), Rounding::same(3.0),
            Stroke::new(1.5, theme::QUANTUM));
        let handle = Rect::from_center_size(rect.right_bottom(), Vec2::splat(8.0 * zoom.min(1.0)));
        p.rect_filled(handle, Rounding::same(1.0), theme::QUANTUM);
    }
}

fn draw_knob(p: &egui::Painter, rect: Rect, params: &AtomParams, zoom: f32) {
    use std::f32::consts::PI;
    let c   = rect.center();
    let r   = rect.width().min(rect.height()) * 0.38;
    let col = params.color32();

    p.circle_filled(c, r, theme::RAISED);
    p.circle_stroke(c, r, Stroke::new(1.5 * zoom, col));
    p.circle_filled(c, r + 3.0 * zoom, theme::with_alpha(col, 18));

    const START: f32 = PI * 0.75;
    const SWEEP: f32 = PI * 1.5;
    let end = START + params.value * SWEEP;
    let steps = 48usize;
    let arc: Vec<Pos2> = (0..=steps).map(|i| {
        let t = START + (i as f32 / steps as f32) * (end - START);
        Pos2::new(c.x + (r - 2.0 * zoom) * t.cos(), c.y + (r - 2.0 * zoom) * t.sin())
    }).collect();
    if arc.len() >= 2 {
        p.add(egui::Shape::line(arc, Stroke::new(2.0 * zoom, col)));
    }
    let pa = START + params.value * SWEEP;
    let pip = Pos2::new(c.x + (r - 5.0 * zoom) * pa.cos(), c.y + (r - 5.0 * zoom) * pa.sin());
    p.circle_filled(pip, 2.5 * zoom, col);

    if !params.label.is_empty() {
        p.text(Pos2::new(rect.center().x, rect.bottom() - 8.0 * zoom),
            egui::Align2::CENTER_CENTER, &params.label,
            theme::mono(7.0 * zoom), theme::FG3);
    }
}

fn draw_fader(p: &egui::Painter, rect: Rect, params: &AtomParams, zoom: f32) {
    let col          = params.color32();
    let cx           = rect.center().x;
    let track_top    = rect.top() + 8.0 * zoom;
    let track_bottom = rect.bottom() - 8.0 * zoom;
    let track_h      = track_bottom - track_top;

    p.line_segment([Pos2::new(cx, track_top), Pos2::new(cx, track_bottom)],
        Stroke::new(2.0 * zoom, theme::INLAY));

    let fill_y = track_bottom - params.value * track_h;
    if fill_y < track_bottom {
        p.line_segment([Pos2::new(cx, fill_y), Pos2::new(cx, track_bottom)],
            Stroke::new(2.0 * zoom, theme::with_alpha(col, 180)));
    }
    let hy     = track_bottom - params.value * track_h;
    let handle = Rect::from_center_size(Pos2::new(cx, hy),
        Vec2::new(rect.width() * 0.7, 10.0 * zoom));
    p.rect_filled(handle, Rounding::same(2.0), theme::ELEVATED);
    p.rect_stroke(handle, Rounding::same(2.0), Stroke::new(zoom, col));
}

fn draw_pad(p: &egui::Painter, rect: Rect, params: &AtomParams, zoom: f32) {
    let col  = params.color32();
    let fill = if params.lit { theme::with_alpha(col, 60) } else { theme::SURFACE };
    p.rect_filled(rect, Rounding::same(4.0 * zoom), fill);
    p.rect_stroke(rect, Rounding::same(4.0 * zoom),
        Stroke::new(if params.lit { 1.5 } else { 1.0 } * zoom, col));
    if !params.label.is_empty() {
        p.text(rect.center(), egui::Align2::CENTER_CENTER, &params.label,
            theme::mono(7.0 * zoom), if params.lit { col } else { theme::FG3 });
    }
}

fn draw_jack(p: &egui::Painter, rect: Rect, params: &AtomParams, zoom: f32) {
    let col = params.color32();
    let c   = rect.center();
    let r   = rect.width().min(rect.height()) * 0.35;
    p.circle_filled(c, r, theme::ABYSS);
    p.circle_stroke(c, r, Stroke::new(1.5 * zoom, col));
    p.circle_stroke(c, r * 0.5, Stroke::new(zoom, theme::with_alpha(col, 100)));
    p.text(c, egui::Align2::CENTER_CENTER,
        if params.is_output { "▴" } else { "▾" },
        theme::mono(8.0 * zoom), col);
}

fn draw_meter(p: &egui::Painter, rect: Rect, params: &AtomParams, zoom: f32) {
    let segments = 14usize;
    let seg_h    = (rect.height() - (segments as f32 - 1.0) * 2.0 * zoom) / segments as f32;
    let lit      = (params.value * segments as f32) as usize;
    for i in 0..segments {
        let seg_i    = segments - 1 - i;
        let y        = rect.top() + i as f32 * (seg_h + 2.0 * zoom);
        let seg_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), seg_h));
        let col = if seg_i >= lit { theme::INLAY }
                  else if seg_i >= segments - 2 { theme::EMBER }
                  else if seg_i >= segments - 4 { theme::GOLD  }
                  else { params.color32() };
        p.rect_filled(seg_rect, Rounding::same(1.0), col);
    }
}

fn draw_scope(p: &egui::Painter, rect: Rect, zoom: f32) {
    p.rect_filled(rect, Rounding::same(3.0 * zoom), theme::VOID);
    p.rect_stroke(rect, Rounding::same(3.0 * zoom),
        Stroke::new(zoom, Color32::from_rgba_unmultiplied(0, 80, 120, 120)));
    for i in 1..4 {
        let x = rect.left() + rect.width()  * i as f32 / 4.0;
        let y = rect.top()  + rect.height() * i as f32 / 4.0;
        let dim = Color32::from_rgba_unmultiplied(0, 80, 120, 40);
        p.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(0.5 * zoom, dim));
        p.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5 * zoom, dim));
    }
    let cx = rect.center().x;
    let cy = rect.center().y;
    let rx = rect.width() * 0.35;
    let ry = rect.height() * 0.35;
    let pts: Vec<Pos2> = (0..=128).map(|i| {
        let t = i as f32 / 128.0 * std::f32::consts::TAU;
        Pos2::new(cx + rx * (3.0 * t).sin(), cy + ry * (2.0 * t).sin())
    }).collect();
    p.add(egui::Shape::line(pts, Stroke::new(zoom, theme::with_alpha(theme::GOLD, 160))));
    p.text(rect.min + Vec2::splat(4.0) * zoom, egui::Align2::LEFT_TOP,
        "SCOPE", theme::mono(6.0 * zoom),
        Color32::from_rgba_unmultiplied(251, 191, 36, 80));
}

fn draw_label(p: &egui::Painter, rect: Rect, params: &AtomParams, zoom: f32) {
    p.text(rect.left_center(), egui::Align2::LEFT_CENTER,
        &params.text, theme::mono(params.size_px * 0.2 * zoom), theme::FG2);
}

fn draw_divider(p: &egui::Painter, rect: Rect) {
    let cx = rect.center().x;
    p.line_segment([Pos2::new(cx, rect.top()), Pos2::new(cx, rect.bottom())],
        Stroke::new(1.0, theme::BORDER));
}

// ── Grid ──────────────────────────────────────────────────────────────────────

fn draw_grid(p: &egui::Painter, rect: Rect, offset: Vec2, zoom: f32) {
    p.rect_filled(rect, Rounding::ZERO, theme::VOID);
    let grid    = 24.0 * zoom;
    let origin  = rect.min + offset;
    let start_x = (origin.x).rem_euclid(grid);
    let start_y = (origin.y).rem_euclid(grid);
    let mut x   = rect.left() + start_x;
    while x < rect.right() {
        let mut y = rect.top() + start_y;
        while y < rect.bottom() {
            p.circle_filled(Pos2::new(x, y), 0.8,
                Color32::from_rgba_unmultiplied(60, 80, 120, 50));
            y += grid;
        }
        x += grid;
    }
}
