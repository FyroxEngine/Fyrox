use egui::{Color32, Frame, Margin, Pos2, Rounding, Sense, Stroke, Ui, Vec2};

use crate::{
    state::MasterState,
    theme,
    widgets::Knob,
};

pub fn draw(ui: &mut Ui, master: &mut MasterState, tick: f64) {
    Frame::none()
        .fill(theme::DEEP)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .inner_margin(Margin::same(10.0))
        .rounding(Rounding::same(3.0))
        .show(ui, |ui| {
            ui.set_width(170.0);
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 5.0);

            // ── Section label ────────────────────────────────────────────
            eyebrow(ui, "MASTER");

            ui.add_space(4.0);

            // ── Lissajous placeholder ─────────────────────────────────────
            lissajous_display(ui, master, tick);

            ui.add_space(6.0);

            // ── Master volume ────────────────────────────────────────────
            eyebrow(ui, "MASTER VOL");
            ui.vertical_centered(|ui| {
                Knob::new(&mut master.master_vol, theme::GOLD)
                    .size(40.0)
                    .show(ui);
            });

            ui.add_space(4.0);

            // ── Crossfader ──────────────────────────────────────────────
            eyebrow(ui, "CROSSFADER");
            crossfader(ui, &mut master.crossfader);

            ui.add_space(6.0);

            separator(ui);

            // ── Tempo ────────────────────────────────────────────────────
            ui.add_space(4.0);
            eyebrow(ui, "TEMPO");
            ui.vertical_centered(|ui| {
                Knob::new(&mut master.bpm, theme::QUANTUM)
                    .size(34.0)
                    .show(ui);
                ui.label(
                    egui::RichText::new(format!("{:.0} BPM", 60.0 + master.bpm * 200.0))
                        .font(theme::mono(7.0))
                        .color(theme::QUANTUM),
                );
            });

            ui.add_space(6.0);

            separator(ui);

            // ── Transport ────────────────────────────────────────────────
            ui.add_space(4.0);
            transport(ui, master);

            ui.add_space(6.0);

            separator(ui);

            // ── Frequency readout ────────────────────────────────────────
            ui.add_space(4.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(format!("◈  {:.1} HZ", master.frequency))
                        .font(theme::mono(9.5))
                        .color(theme::GOLD),
                );
                ui.label(
                    egui::RichText::new(format!("EPOCH  {}", master.epoch))
                        .font(theme::mono(7.5))
                        .color(theme::FG3),
                );
                ui.label(
                    egui::RichText::new(master.era)
                        .font(theme::mono(8.0))
                        .color(theme::MYTHOS),
                );
            });
        });
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn eyebrow(ui: &mut Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .font(theme::mono(6.5))
            .color(theme::FG3),
    );
}

fn separator(ui: &mut Ui) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, theme::BORDER);
}

fn crossfader(ui: &mut Ui, value: &mut f32) {
    let w = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(w, 24.0), Sense::click_and_drag());

    if response.dragged() {
        *value = (*value + response.drag_delta().x / rect.width()).clamp(0.0, 1.0);
    }

    if ui.is_rect_visible(rect) {
        let p = ui.painter();

        // Track
        let track_y = rect.center().y;
        p.line_segment(
            [Pos2::new(rect.left() + 4.0, track_y), Pos2::new(rect.right() - 4.0, track_y)],
            Stroke::new(2.0, theme::INLAY),
        );

        // Left fill (cyan → center)
        let cx = rect.left() + 4.0 + *value * (rect.width() - 8.0);
        if cx > rect.left() + 4.0 {
            p.line_segment(
                [Pos2::new(rect.left() + 4.0, track_y), Pos2::new(cx, track_y)],
                Stroke::new(2.0, theme::with_alpha(theme::QUANTUM, 180)),
            );
        }

        // Handle
        let handle = egui::Rect::from_center_size(Pos2::new(cx, track_y), Vec2::new(10.0, 20.0));
        p.rect_filled(handle, Rounding::same(2.0),
            if response.hovered() || response.dragged() { theme::ELEVATED } else { theme::RAISED });
        p.rect_stroke(handle, Rounding::same(2.0),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 220, 255, 35)));

        // Center mark
        let mid = rect.left() + 4.0 + 0.5 * (rect.width() - 8.0);
        p.line_segment(
            [Pos2::new(mid, rect.top() + 2.0), Pos2::new(mid, rect.bottom() - 2.0)],
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(100, 150, 200, 60)),
        );
    }
}

fn transport(ui: &mut Ui, master: &mut MasterState) {
    ui.horizontal(|ui| {
        let (play_col, play_lbl) = if master.playing {
            (theme::BIO, "■ HALT")
        } else {
            (theme::FG2, "▶ WEAVE")
        };
        if small_btn(ui, play_lbl, play_col) {
            master.playing = !master.playing;
        }
        ui.add_space(2.0);
        let loop_col = if master.looping { theme::QUANTUM } else { theme::FG_MUTED };
        if small_btn(ui, "⟲ LOOP", loop_col) {
            master.looping = !master.looping;
        }
    });
}

fn small_btn(ui: &mut Ui, label: &str, color: Color32) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(72.0, 20.0), Sense::click());

    if ui.is_rect_visible(rect) {
        let fill = if response.is_pointer_button_down_on() { theme::INLAY }
                   else if response.hovered() { theme::ELEVATED }
                   else { theme::SURFACE };

        ui.painter().rect_filled(rect, Rounding::same(2.0), fill);
        ui.painter().rect_stroke(rect, Rounding::same(2.0), Stroke::new(1.0, color));
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
            label, theme::mono(7.5), color);
    }

    response.clicked()
}

fn lissajous_display(ui: &mut Ui, master: &MasterState, tick: f64) {
    let size = Vec2::new(ui.available_width(), 80.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let p = ui.painter();

        // Background
        p.rect_filled(rect, Rounding::same(3.0), theme::VOID);
        p.rect_stroke(rect, Rounding::same(3.0),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 60, 80, 120)));

        // Draw a simple Lissajous curve
        let cx = rect.center().x;
        let cy = rect.center().y;
        let rx = rect.width()  * 0.38;
        let ry = rect.height() * 0.38;
        let delta = tick as f32 * 0.4;
        let a = 1.0 + master.master_vol * 2.0;
        let b = 2.0 + master.crossfader;

        let steps = 256usize;
        let pts: Vec<Pos2> = (0..=steps).map(|i| {
            let t = i as f32 / steps as f32 * std::f32::consts::TAU;
            Pos2::new(
                cx + rx * (a * t + delta).sin(),
                cy + ry * (b * t).sin(),
            )
        }).collect();

        p.add(egui::Shape::line(pts, Stroke::new(1.0, theme::with_alpha(theme::GOLD, 180))));

        // Center cross-hair
        let ch_alpha = Color32::from_rgba_unmultiplied(0, 150, 200, 40);
        p.line_segment([Pos2::new(cx, rect.top() + 4.0), Pos2::new(cx, rect.bottom() - 4.0)],
            Stroke::new(0.5, ch_alpha));
        p.line_segment([Pos2::new(rect.left() + 4.0, cy), Pos2::new(rect.right() - 4.0, cy)],
            Stroke::new(0.5, ch_alpha));

        // Label
        p.text(Pos2::new(rect.left() + 5.0, rect.top() + 4.0),
            egui::Align2::LEFT_TOP, "ASTRAL GATEWAY",
            theme::mono(6.0), Color32::from_rgba_unmultiplied(251, 191, 36, 100));
    }
}
