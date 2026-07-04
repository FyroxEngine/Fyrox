// Error fallback page — shown when AppScreen::Error is active.
// The shell draws the top/bottom rails; this fills the CentralPanel.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Align2, FontId, Frame, Margin, pos2};

use crate::state::AppScreen;
use super::UiSet;
use super::theme::{a, VOID, GOLD, FG_3, FG_MUTED};

pub struct ErrorScreenPlugin;

impl Plugin for ErrorScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            draw_error_screen
                .run_if(in_state(AppScreen::Error))
                .in_set(UiSet::Page),
        );
    }
}

fn draw_error_screen(mut contexts: EguiContexts) {
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default()
        .frame(Frame::none().fill(VOID).inner_margin(Margin::same(48.0)))
        .show(ctx, |ui| {
            let available = ui.available_size();
            let cx = available.x * 0.5;
            let cy = available.y * 0.5;

            // Draw the warning glyph
            let (rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());
            let p = ui.painter_at(rect);

            let warn_col = egui::Color32::from_rgb(220, 80, 80);
            let center = pos2(rect.left() + cx, rect.top() + cy - 40.0);

            // Outer ring
            p.circle_stroke(
                center,
                48.0,
                egui::Stroke::new(1.0, a(warn_col, 40)),
            );
            p.circle_stroke(
                center,
                50.0,
                egui::Stroke::new(4.0, a(warn_col, 16)),
            );

            // Warning icon (triangle outline made from line segments)
            let half = 32.0_f32;
            let h = half * (3.0_f32).sqrt();
            let top   = pos2(center.x, center.y - h * 0.66);
            let left  = pos2(center.x - half, center.y + h * 0.34);
            let right = pos2(center.x + half, center.y + h * 0.34);

            let icon_stroke = egui::Stroke::new(1.5, a(warn_col, 180));
            p.line_segment([top,   left],  icon_stroke);
            p.line_segment([left,  right], icon_stroke);
            p.line_segment([right, top],   icon_stroke);

            // Exclamation mark
            p.line_segment(
                [pos2(center.x, center.y - 14.0), pos2(center.x, center.y + 4.0)],
                egui::Stroke::new(2.0, a(warn_col, 200)),
            );
            p.circle_filled(pos2(center.x, center.y + 10.0), 2.0, a(warn_col, 200));

            // Title
            p.text(
                pos2(rect.left() + cx, rect.top() + cy + 30.0),
                Align2::CENTER_CENTER,
                "COHERENCE FAULT",
                FontId::proportional(20.0),
                a(warn_col, 200),
            );

            // Subtitle
            p.text(
                pos2(rect.left() + cx, rect.top() + cy + 58.0),
                Align2::CENTER_CENTER,
                "An unexpected state transition occurred.",
                FontId::proportional(12.0),
                FG_3,
            );

            p.text(
                pos2(rect.left() + cx, rect.top() + cy + 76.0),
                Align2::CENTER_CENTER,
                "Use  ↩ Return to Library  to recover.",
                FontId::proportional(11.0),
                FG_MUTED,
            );

            // Decorative corner marks
            let margin = 24.0_f32;
            let arm    = 14.0_f32;
            let cs     = egui::Stroke::new(1.0, a(warn_col, 30));
            for (ox, oy, dx1, dy1, dx2, dy2) in [
                (margin,          margin,          arm,  0.0, 0.0,  arm),
                (available.x - margin, margin,    -arm, 0.0, 0.0,  arm),
                (margin,          available.y - margin, arm,  0.0, 0.0, -arm),
                (available.x - margin, available.y - margin, -arm, 0.0, 0.0, -arm),
            ] {
                let p0 = pos2(rect.left() + ox, rect.top() + oy);
                p.line_segment([p0, pos2(p0.x + dx1, p0.y + dy1)], cs);
                p.line_segment([p0, pos2(p0.x + dx2, p0.y + dy2)], cs);
            }

            // Status line
            p.text(
                pos2(rect.left() + cx, rect.top() + available.y - 24.0),
                Align2::CENTER_CENTER,
                "SYSTEM STANDBY  ·  AWAITING USER INPUT",
                FontId::monospace(8.0),
                a(GOLD, 50),
            );

        });
}
