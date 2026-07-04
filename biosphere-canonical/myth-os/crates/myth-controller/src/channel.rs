use egui::{Color32, Frame, Margin, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use crate::{
    state::ChannelState,
    theme,
    widgets::{Fader, Jack, Knob, Pad, level_meter},
};

const STRIP_W: f32 = 76.0;
const STRIP_INNER_W: f32 = STRIP_W - 16.0; // minus 8px margin each side

pub fn draw(ui: &mut Ui, ch: &mut ChannelState, is_active: bool) {
    let wire_col = ch.wire.color();

    // Faint wire-tinted background on the active strip
    let bg = if is_active {
        Color32::from_rgba_unmultiplied(
            wire_col.r() / 10, wire_col.g() / 10, wire_col.b() / 10, 255)
    } else {
        theme::SURFACE
    };

    let border_color = if is_active {
        Color32::from_rgba_unmultiplied(wire_col.r(), wire_col.g(), wire_col.b(), 90)
    } else {
        theme::BORDER
    };

    Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0, border_color))
        .inner_margin(Margin::same(8.0))
        .rounding(Rounding::same(3.0))
        .show(ui, |ui| {
            ui.set_width(STRIP_INNER_W);
            ui.set_min_height(260.0);
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);

            // ── Header ──────────────────────────────────────────────────
            ui.horizontal(|ui| {
                // Color dot
                let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 3.5, ch.dot);

                // Module number
                ui.label(
                    egui::RichText::new(format!("{:02}", ch.index + 1))
                        .font(theme::mono(7.0))
                        .color(theme::FG_MUTED),
                );
            });

            // Module name
            ui.label(
                egui::RichText::new(ch.name)
                    .font(theme::mono(8.0))
                    .color(theme::FG2),
            );

            // Wire tag chip
            let tag_text = egui::RichText::new(ch.wire.tag())
                .font(theme::mono(6.5))
                .color(wire_col);
            ui.label(tag_text);

            ui.add_space(4.0);

            // ── Macro Knob ──────────────────────────────────────────────
            ui.vertical_centered(|ui| {
                Knob::new(&mut ch.macro_val, wire_col)
                    .size(38.0)
                    .show(ui);
            });

            ui.add_space(3.0);

            // ── Sub-knobs 2×2 ───────────────────────────────────────────
            let sub_color = theme::darken(wire_col, 0.75);
            egui::Grid::new(format!("sub_{}", ch.index))
                .num_columns(2)
                .spacing([4.0, 3.0])
                .show(ui, |ui| {
                    for row in 0..2 {
                        for col in 0..2 {
                            let idx = row * 2 + col;
                            ui.vertical_centered(|ui| {
                                Knob::new(&mut ch.sub[idx], sub_color)
                                    .size(18.0)
                                    .label(ch.sub_labels[idx])
                                    .show(ui);
                            });
                        }
                        ui.end_row();
                    }
                });

            ui.add_space(3.0);

            // ── Pads: M / S / A ─────────────────────────────────────────
            ui.horizontal(|ui| {
                Pad::new(&mut ch.mute,  "M", theme::EMBER).show(ui);
                ui.add_space(1.0);
                Pad::new(&mut ch.solo,  "S", theme::GOLD).show(ui);
                ui.add_space(1.0);
                Pad::new(&mut ch.armed, "A", theme::BIO).show(ui);
            });

            ui.add_space(3.0);

            // ── Meter + Fader ────────────────────────────────────────────
            ui.horizontal(|ui| {
                // Meter
                let meter_h = 80.0;
                let meter_w = 8.0;
                let (mrect, _) = ui.allocate_exact_size(Vec2::new(meter_w, meter_h), Sense::hover());
                level_meter(ui.painter(), mrect, ch.meter_level(), wire_col);

                ui.add_space(2.0);

                // Fader
                Fader::new(&mut ch.fader, wire_col).height(meter_h).show(ui);
            });

            ui.add_space(4.0);

            // ── Separator ───────────────────────────────────────────────
            let (sep, _) = ui.allocate_exact_size(Vec2::new(STRIP_INNER_W, 1.0), Sense::hover());
            ui.painter().rect_filled(sep, 0.0, theme::BORDER);

            ui.add_space(3.0);

            // ── Wire pip ────────────────────────────────────────────────
            ui.vertical_centered(|ui| {
                let (pr, _) = ui.allocate_exact_size(Vec2::new(28.0, 11.0), Sense::hover());
                ui.painter().rect_filled(
                    pr, Rounding::same(2.0),
                    Color32::from_rgba_unmultiplied(
                        wire_col.r() / 7, wire_col.g() / 7, wire_col.b() / 7, 255),
                );
                ui.painter().rect_stroke(pr, Rounding::same(2.0), Stroke::new(0.5, wire_col));
                ui.painter().text(
                    pr.center(), egui::Align2::CENTER_CENTER,
                    ch.wire.tag(), theme::mono(6.5), wire_col,
                );
            });

            ui.add_space(2.0);

            // ── Jacks ────────────────────────────────────────────────────
            ui.horizontal(|ui| {
                Jack::out(wire_col).label("OUT").show(ui);
                ui.add_space(4.0);
                Jack::input(wire_col).label("IN").show(ui);
            });

            // ── Bottom: active glow bar ──────────────────────────────────
            if is_active {
                ui.add_space(2.0);
                let (bar, _) = ui.allocate_exact_size(Vec2::new(STRIP_INNER_W, 2.0), Sense::hover());
                ui.painter().rect_filled(bar, 1.0, wire_col);
            }
        });
}

// ─── Full-window expanded instrument view ─────────────────────────────────────

pub fn draw_expanded(ui: &mut Ui, ch: &mut ChannelState, tick: f64) {
    let wire_col  = ch.wire.color();
    let avail     = ui.available_rect_before_wrap();

    // Tinted background wash
    ui.painter().rect_filled(
        avail,
        0.0,
        Color32::from_rgba_unmultiplied(
            wire_col.r() / 18, wire_col.g() / 18, wire_col.b() / 18, 255),
    );

    // Outer border
    ui.painter().rect_stroke(
        avail.shrink(4.0),
        Rounding::same(4.0),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(
            wire_col.r(), wire_col.g(), wire_col.b(), 55)),
    );

    ui.add_space(16.0);

    // ── Module title bar ────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.add_space(24.0);

        // Large color pip
        let (dot_r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
        ui.painter().circle_filled(dot_r.center(), 6.0, ch.dot);
        ui.painter().circle_stroke(dot_r.center(), 8.0,
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(
                ch.dot.r(), ch.dot.g(), ch.dot.b(), 50)));

        ui.add_space(8.0);

        // Module index + name
        ui.label(
            egui::RichText::new(format!("{:02}", ch.index + 1))
                .font(theme::mono(11.0))
                .color(theme::FG_MUTED),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(ch.name)
                .font(theme::mono(22.0))
                .color(theme::FG1),
        );

        ui.add_space(16.0);

        // Wire tag chip
        let tag_label = format!(" {} ", ch.wire.tag());
        let (tag_r, _) = ui.allocate_exact_size(Vec2::new(38.0, 20.0), Sense::hover());
        ui.painter().rect_filled(tag_r, Rounding::same(3.0),
            Color32::from_rgba_unmultiplied(
                wire_col.r() / 6, wire_col.g() / 6, wire_col.b() / 6, 255));
        ui.painter().rect_stroke(tag_r, Rounding::same(3.0),
            Stroke::new(1.0, wire_col));
        ui.painter().text(tag_r.center(), egui::Align2::CENTER_CENTER,
            ch.wire.tag(), theme::mono(9.0), wire_col);
        let _ = tag_label;

        // Motto (right-aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new(format!("MYTH-{:02}  ·  LAYER 0  ·  DJ CONTROLLER", ch.index + 1))
                    .font(theme::mono(8.0))
                    .color(theme::FG_MUTED),
            );
        });
    });

    ui.add_space(12.0);

    // Horizontal glow rule
    let rule_rect = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 1.0), Sense::hover()).0;
    ui.painter().rect_filled(rule_rect, 0.0,
        Color32::from_rgba_unmultiplied(wire_col.r(), wire_col.g(), wire_col.b(), 60));

    ui.add_space(16.0);

    // ── Three-column layout ─────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.spacing_mut().item_spacing.x = 16.0;

        // LEFT: macro controls (4 large channel strips)
        let col_w = (ui.available_width() - 192.0 - 32.0) * 0.28;
        ui.vertical(|ui| {
            ui.set_width(col_w);
            section_label(ui, "MACRO CONTROLS");
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for i in 0..4 {
                    let lbl = ch.sub_labels[i];
                    let val = &mut ch.sub[i];
                    let subcol = theme::darken(wire_col, 0.85);
                    ui.vertical_centered(|ui| {
                        Knob::new(val, subcol).size(52.0).show(ui);
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(lbl)
                            .font(theme::mono(8.0)).color(theme::FG2));
                    });
                }
            });

            ui.add_space(16.0);
            section_label(ui, "MAIN");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                // Big macro knob
                ui.vertical_centered(|ui| {
                    Knob::new(&mut ch.macro_val, wire_col).size(72.0).show(ui);
                    ui.label(egui::RichText::new("MACRO")
                        .font(theme::mono(8.0)).color(theme::FG2));
                });

                // Level meter + fader side by side
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("LEVEL")
                        .font(theme::mono(7.0)).color(theme::FG_MUTED));
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        let (mr, _) = ui.allocate_exact_size(Vec2::new(14.0, 130.0), Sense::hover());
                        level_meter(ui.painter(), mr, ch.meter_level(), wire_col);
                        ui.add_space(2.0);
                        Fader::new(&mut ch.fader, wire_col).height(130.0).show(ui);
                    });
                });

                // M / S / A pads (vertical)
                ui.vertical(|ui| {
                    ui.add_space(20.0);
                    ui.spacing_mut().item_spacing.y = 6.0;
                    Pad::new(&mut ch.mute,  "M", theme::EMBER).show(ui);
                    Pad::new(&mut ch.solo,  "S", theme::GOLD).show(ui);
                    Pad::new(&mut ch.armed, "A", theme::BIO).show(ui);
                });
            });
        });

        // CENTER: scope display
        let scope_w = (ui.available_width() - 192.0 - 16.0) * 0.58;
        ui.vertical(|ui| {
            ui.set_width(scope_w);
            section_label(ui, "SIGNAL MONITOR");
            ui.add_space(6.0);

            let scope_h = ui.available_height().min(300.0);
            let (scope_r, _) = ui.allocate_exact_size(
                Vec2::new(scope_w, scope_h), Sense::hover());
            draw_scope(ui.painter(), scope_r, ch, tick);
        });

        // RIGHT: patch jacks
        ui.vertical(|ui| {
            ui.set_width(180.0);
            section_label(ui, "PATCH");
            ui.add_space(6.0);
            ui.spacing_mut().item_spacing.y = 4.0;

            let out_label = format!("{}.OUT", ch.tag);
            let in_label  = format!("{}.IN",  ch.tag);
            let bus_label = format!("{}.BUS", ch.tag);

            for (lbl, is_in) in [
                (out_label.as_str(), false),
                (in_label.as_str(),  true),
                (bus_label.as_str(), false),
                ("SYNC.IN",   true),
                ("SYNC.OUT",  false),
                ("CV.MOD",    true),
                ("CV.TRIG",   true),
                ("AUX.SEND",  false),
                ("AUX.RET",   true),
            ] {
                ui.horizontal(|ui| {
                    if is_in {
                        Jack::input(wire_col).label(lbl).show(ui);
                    } else {
                        Jack::out(wire_col).label(lbl).show(ui);
                    }
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(lbl)
                        .font(theme::mono(7.5))
                        .color(if is_in { theme::FG2 } else { wire_col }));
                });
            }
        });
    });
}

fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .font(theme::mono(7.0))
            .color(theme::FG_MUTED),
    );
}

fn draw_scope(painter: &egui::Painter, rect: Rect, ch: &ChannelState, tick: f64) {
    let wire_col = ch.wire.color();

    // Background
    painter.rect_filled(rect, Rounding::same(4.0), theme::VOID);
    painter.rect_stroke(rect, Rounding::same(4.0),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(
            wire_col.r() / 3, wire_col.g() / 3, wire_col.b() / 3, 120)));

    let cx = rect.center().x;
    let cy = rect.center().y;
    let w  = rect.width();
    let h  = rect.height();

    // Cross-hairs
    let dim = Color32::from_rgba_unmultiplied(40, 60, 80, 80);
    painter.line_segment([Pos2::new(cx, rect.top() + 8.0), Pos2::new(cx, rect.bottom() - 8.0)],
        Stroke::new(0.5, dim));
    painter.line_segment([Pos2::new(rect.left() + 8.0, cy), Pos2::new(rect.right() - 8.0, cy)],
        Stroke::new(0.5, dim));

    // Grid lines
    for i in 1..4 {
        let x = rect.left() + w * i as f32 / 4.0;
        let y = rect.top()  + h * i as f32 / 4.0;
        painter.line_segment([Pos2::new(x, rect.top() + 4.0), Pos2::new(x, rect.bottom() - 4.0)],
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(30, 50, 70, 60)));
        painter.line_segment([Pos2::new(rect.left() + 4.0, y), Pos2::new(rect.right() - 4.0, y)],
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(30, 50, 70, 60)));
    }

    // Live Lissajous curve (driven by macro_val + tick)
    let rx = w * 0.38;
    let ry = h * 0.38;
    let a = 1.0 + ch.macro_val * 3.0;
    let b = 2.0 + ch.sub[0];
    let delta = tick as f32 * 0.5 * (0.3 + ch.sub[1] * 0.7);
    let steps = 320usize;

    let pts: Vec<Pos2> = (0..=steps).map(|i| {
        let t = i as f32 / steps as f32 * std::f32::consts::TAU;
        Pos2::new(
            cx + rx * (a * t + delta).sin(),
            cy + ry * (b * t).sin(),
        )
    }).collect();

    painter.add(egui::Shape::line(pts,
        Stroke::new(1.5, Color32::from_rgba_unmultiplied(
            wire_col.r(), wire_col.g(), wire_col.b(), 200))));

    // Glow copy at lower alpha
    let glow_pts: Vec<Pos2> = (0..=steps).map(|i| {
        let t = i as f32 / steps as f32 * std::f32::consts::TAU;
        Pos2::new(
            cx + rx * (a * t + delta).sin(),
            cy + ry * (b * t).sin(),
        )
    }).collect();
    painter.add(egui::Shape::line(glow_pts,
        Stroke::new(4.0, Color32::from_rgba_unmultiplied(
            wire_col.r(), wire_col.g(), wire_col.b(), 25))));

    // Corner labels
    let t = tick as f32;
    painter.text(Pos2::new(rect.left() + 8.0, rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        format!("a:{:.2}  b:{:.2}  δ:{:.0}°", a, b, (delta % std::f32::consts::TAU).to_degrees()),
        theme::mono(6.5),
        Color32::from_rgba_unmultiplied(wire_col.r(), wire_col.g(), wire_col.b(), 120));

    painter.text(Pos2::new(rect.right() - 8.0, rect.top() + 8.0),
        egui::Align2::RIGHT_TOP,
        "LIVE",
        theme::mono(7.0), theme::BIO);

    painter.text(Pos2::new(rect.left() + 8.0, rect.bottom() - 8.0),
        egui::Align2::LEFT_BOTTOM,
        format!("MYTH-{:02} · {}", ch.index + 1, ch.name),
        theme::mono(7.0),
        Color32::from_rgba_unmultiplied(wire_col.r(), wire_col.g(), wire_col.b(), 150));

    let _ = t;
}

// ─── Module selector tabs (16 channels at the top) ───────────────────────────

pub fn draw_module_tabs(ui: &mut Ui, channels: &[ChannelState], active: &mut usize) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        for ch in channels {
            let is_active = ch.index == *active;
            let wire_col  = ch.wire.color();

            let (bg, text_col, border_col) = if is_active {
                (
                    Color32::from_rgba_unmultiplied(
                        wire_col.r() / 7, wire_col.g() / 7, wire_col.b() / 7, 255),
                    wire_col,
                    wire_col,
                )
            } else {
                (theme::SURFACE, theme::FG3, theme::BORDER)
            };

            let (tab_rect, response) = ui.allocate_exact_size(Vec2::new(68.0, 26.0), Sense::click());

            if response.clicked() {
                *active = ch.index;
            }

            if ui.is_rect_visible(tab_rect) {
                let r = Rounding {
                    nw: 3.0, ne: 3.0, sw: 0.0, se: 0.0,
                };

                let fill = if response.hovered() && !is_active {
                    theme::RAISED
                } else {
                    bg
                };

                ui.painter().rect_filled(tab_rect, r, fill);
                ui.painter().rect_stroke(tab_rect, r, Stroke::new(1.0, border_col));

                // Dot
                let dot_pos = Pos2::new(tab_rect.left() + 8.0, tab_rect.center().y);
                ui.painter().circle_filled(dot_pos, 3.0, ch.dot);

                // Tag text
                ui.painter().text(
                    Pos2::new(tab_rect.left() + 17.0, tab_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    ch.tag,
                    theme::mono(8.5),
                    text_col,
                );

                // Active bottom glow line
                if is_active {
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            Pos2::new(tab_rect.left() + 2.0, tab_rect.bottom() - 2.0),
                            Vec2::new(tab_rect.width() - 4.0, 2.0),
                        ),
                        1.0,
                        wire_col,
                    );
                }
            }
        }
    });
}
