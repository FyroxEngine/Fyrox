use egui::{Color32, Frame, Margin, Pos2, Sense, Stroke, Ui, Vec2};

use crate::{channel, master, state::ControllerState, theme};

pub struct ControllerApp {
    state: ControllerState,
}

impl Default for ControllerApp {
    fn default() -> Self {
        Self { state: ControllerState::default() }
    }
}

impl eframe::App for ControllerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.tick = ctx.input(|i| i.time);
        ctx.request_repaint();

        // ── Top status bar ───────────────────────────────────────────────
        egui::TopBottomPanel::top("topbar")
            .frame(Frame::none()
                .fill(theme::ABYSS)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .inner_margin(Margin { left: 14.0, right: 14.0, top: 5.0, bottom: 5.0 }))
            .show(ctx, |ui| {
                top_bar(ui, &self.state);
            });

        // ── 16-module selector tabs ──────────────────────────────────────
        egui::TopBottomPanel::top("module_tabs")
            .frame(Frame::none()
                .fill(theme::VOID)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .inner_margin(Margin { left: 8.0, right: 8.0, top: 4.0, bottom: 0.0 }))
            .show(ctx, |ui| {
                channel::draw_module_tabs(
                    ui,
                    &self.state.channels,
                    &mut self.state.active_module,
                );
            });

        // ── Nexus routing bar ────────────────────────────────────────────
        egui::TopBottomPanel::bottom("nexus_bar")
            .frame(Frame::none()
                .fill(theme::ABYSS)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .inner_margin(Margin { left: 14.0, right: 14.0, top: 5.0, bottom: 5.0 }))
            .show(ctx, |ui| {
                nexus_bar(ui, &self.state);
            });

        // ── Master section (right panel) ─────────────────────────────────
        egui::SidePanel::right("master_panel")
            .exact_width(192.0)
            .frame(Frame::none()
                .fill(theme::DEEP)
                .stroke(Stroke::new(1.0, theme::BORDER)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    master::draw(ui, &mut self.state.master, self.state.tick);
                });
            });

        // ── Main channel strip area ──────────────────────────────────────
        egui::CentralPanel::default()
            .frame(Frame::none().fill(theme::VOID))
            .show(ctx, |ui| {
                paint_stars(ui, self.state.tick);

                let active = self.state.active_module;
                channel::draw_expanded(ui, &mut self.state.channels[active], self.state.tick);
            });
    }
}

// ─── Top bar ──────────────────────────────────────────────────────────────────

fn top_bar(ui: &mut Ui, state: &ControllerState) {
    ui.horizontal(|ui| {
        // Logo pip
        let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter().circle_filled(dot.center(), 4.5, theme::GOLD);
        ui.painter().circle_stroke(dot.center(), 4.5,
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(251, 191, 36, 120)));

        ui.add_space(6.0);

        // Title
        ui.label(
            egui::RichText::new("THE  AXIOM  CONTROLLER")
                .font(theme::mono(11.0))
                .color(theme::FG1),
        );

        ui.add_space(4.0);

        // Sub-title
        ui.label(
            egui::RichText::new("GENESIS CONTAINER  ·  SSoT LIVE  ·  v0.1")
                .font(theme::mono(7.0))
                .color(theme::FG_MUTED),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Live indicator
            let live_col = theme::BIO;
            ui.label(
                egui::RichText::new(format!("◈  {:.1} HZ", state.master.frequency))
                    .font(theme::mono(9.0))
                    .color(theme::GOLD),
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(format!("INST  {}/16", state.active_module + 1))
                    .font(theme::mono(8.0))
                    .color(theme::FG2),
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(format!("ERA  {}", state.master.era))
                    .font(theme::mono(8.0))
                    .color(theme::MYTHOS),
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(format!("EPOCH  {}", state.master.epoch))
                    .font(theme::mono(8.0))
                    .color(theme::FG2),
            );
            ui.add_space(10.0);
            let (live_dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
            ui.painter().circle_filled(live_dot.center(), 3.5, live_col);
            ui.painter().circle_filled(live_dot.center(), 6.0,
                Color32::from_rgba_unmultiplied(57, 255, 20, 30));
            ui.label(
                egui::RichText::new("LIVE")
                    .font(theme::mono(8.0))
                    .color(live_col),
            );
        });
    });
}

// ─── Nexus routing bar ────────────────────────────────────────────────────────

fn nexus_bar(ui: &mut Ui, state: &ControllerState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("NEXUS  ROUTING")
                .font(theme::mono(7.0))
                .color(theme::FG_MUTED),
        );

        ui.add_space(10.0);

        // Show first 4 active channels in the chain
        let chain: Vec<_> = state.channels.iter()
            .filter(|ch| ch.armed || ch.index == state.active_module)
            .take(4)
            .collect();

        for (i, ch) in chain.iter().enumerate() {
            // Node circle
            let (node_r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
            ui.painter().circle_filled(node_r.center(), 4.0, ch.dot);
            ui.painter().circle_stroke(node_r.center(), 4.0,
                Stroke::new(0.5, ch.wire.color()));

            // Tag + blend mode
            ui.label(
                egui::RichText::new(format!("{} SCREEN", ch.tag))
                    .font(theme::mono(7.5))
                    .color(ch.wire.color()),
            );

            if i < chain.len() - 1 {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("──→")
                        .font(theme::mono(7.5))
                        .color(theme::FG_MUTED),
                );
                ui.add_space(4.0);
            }
        }

        // Terminal: BioSpark Theater
        ui.add_space(6.0);
        ui.label(egui::RichText::new("──→").font(theme::mono(7.5)).color(theme::FG_MUTED));
        ui.add_space(4.0);
        let (hex_r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        paint_hexagon(ui.painter(), hex_r.center(), 5.0, theme::QUANTUM);
        ui.label(
            egui::RichText::new("BIOSPARK THEATER")
                .font(theme::mono(8.0))
                .color(theme::QUANTUM),
        );

        // Right side: wire type legend
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new("WS // AXIOM")
                    .font(theme::mono(7.0))
                    .color(theme::FG_MUTED),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("SSoT · RUST")
                    .font(theme::mono(7.0))
                    .color(theme::FG_MUTED),
            );
        });
    });
}

// ─── Starfield ────────────────────────────────────────────────────────────────

fn paint_stars(ui: &mut Ui, tick: f64) {
    // A handful of static-ish stars scattered across the panel.
    // Using simple LCG positions seeded from index.
    let rect = ui.max_rect();
    let p = ui.painter();

    for i in 0u32..48 {
        let h = (i.wrapping_mul(1664525).wrapping_add(1013904223)) as f32 / u32::MAX as f32;
        let v = (i.wrapping_mul(22695477).wrapping_add(1))          as f32 / u32::MAX as f32;
        let x = rect.left() + h * rect.width();
        let y = rect.top()  + v * rect.height();

        // Slow twinkle
        let alpha = (0.25 + 0.25 * ((tick * 0.3 + i as f64 * 0.7).sin() as f32)).clamp(0.0, 1.0);
        let sz = if i % 7 == 0 { 1.5 } else { 0.8 };

        p.circle_filled(Pos2::new(x, y), sz,
            Color32::from_rgba_unmultiplied(180, 210, 255, (alpha * 180.0) as u8));
    }
}

// ─── Hexagon glyph ────────────────────────────────────────────────────────────

fn paint_hexagon(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    use std::f32::consts::TAU;
    let pts: Vec<Pos2> = (0..6).map(|i| {
        let a = TAU * i as f32 / 6.0 - TAU / 12.0;
        Pos2::new(center.x + r * a.cos(), center.y + r * a.sin())
    }).collect();
    painter.add(egui::Shape::closed_line(pts, Stroke::new(1.0, color)));
}
