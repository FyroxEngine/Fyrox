// Animated splash screen — painted entirely with egui Painter.
// Mirrors the HTML splash aesthetic: dark void, gold/violet, open-book emblem.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Align2, Color32, FontId, Pos2, Rect, Rounding, Stroke, pos2};
use std::f32::consts::TAU;

use crate::state::AppScreen;
use super::theme::{self, a, VOID, GOLD, GOLD_LT, VIOLET};

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw.run_if(in_state(AppScreen::Splash)));
    }
}

// ── Resources ─────────────────────────────────────────────────────────────────

// ── Main draw system ──────────────────────────────────────────────────────────

fn draw(
    mut contexts: EguiContexts,
    time:         Res<Time>,
    mut next:     ResMut<NextState<AppScreen>>,
) {
    let t    = time.elapsed_seconds();
    let fade = (t / 0.7_f32).min(1.0);

    let ctx = contexts.ctx_mut();
    theme::apply(ctx);
    ctx.request_repaint();

    let mut advance = false;

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(VOID))
        .show(ctx, |ui| {
            let rect   = ui.max_rect();
            let center = rect.center();

            let painter = ui.painter().clone();
            paint_splash(&painter, rect, center, t, fade);

            // ENTER button — fades in after animation has settled (~2s)
            let btn_enter = spring_out(((t - 2.2) / 1.0).clamp(0.0, 1.0));
            if btn_enter > 0.01 {
                let btn_w = 180.0_f32;
                let btn_h = 38.0_f32;
                let btn_rect = Rect::from_center_size(
                    egui::pos2(center.x, rect.bottom() - rect.height() * 0.10),
                    egui::vec2(btn_w, btn_h),
                );
                let resp    = ui.interact(btn_rect, ui.id().with("enter"), egui::Sense::click());
                let hovered = resp.hovered();
                let col_a   = (btn_enter * (if hovered { 200.0 } else { 140.0 })) as u8;
                let fill_a  = (btn_enter * (if hovered { 30.0  } else { 14.0  })) as u8;
                let brd_a   = (btn_enter * (if hovered { 160.0 } else { 80.0  })) as u8;

                let p = ui.painter_at(btn_rect);
                p.rect_filled(btn_rect, Rounding::same(5.0), a(GOLD, fill_a));
                p.rect_stroke(btn_rect, Rounding::same(5.0), Stroke::new(1.0, a(GOLD, brd_a)));
                p.text(btn_rect.center(), Align2::CENTER_CENTER,
                    "ENTER THE LIBRARY",
                    FontId::proportional(13.0),
                    a(GOLD, col_a));

                if resp.clicked() { advance = true; }
            }
        });

    if advance {
        next.set(AppScreen::Landing);
    }
}

// ── Painter ───────────────────────────────────────────────────────────────────

fn paint_splash(painter: &egui::Painter, rect: Rect, center: Pos2, t: f32, fade: f32) {
    // 0. Full background fill
    painter.rect_filled(rect, Rounding::ZERO, VOID);

    // 1. Subtle grid ────────────────────────────────────────────────────────
    let grid       = 44.0_f32;
    let grid_alpha = (fade * 12.0) as u8;
    let grid_col   = a(GOLD, grid_alpha);
    let offset_x   = rect.left() % grid;
    let offset_y   = rect.top()  % grid;
    let mut x = rect.left() - offset_x;
    while x <= rect.right() {
        painter.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(0.5, grid_col));
        x += grid;
    }
    let mut y = rect.top() - offset_y;
    while y <= rect.bottom() {
        painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(0.5, grid_col));
        y += grid;
    }

    // 2. Ambient glow blobs ─────────────────────────────────────────────────
    let p1 = (t / 13.0 * TAU).sin();
    let p2 = (t / 17.0 * TAU).sin();
    let g1_center = pos2(center.x, rect.top() + rect.height() * 0.30);
    let g2_center = pos2(rect.right() - rect.width() * 0.18, rect.bottom() - rect.height() * 0.20);

    for (r, base_a) in [(200.0_f32, 5_u8), (140.0, 9), (90.0, 15), (55.0, 22)] {
        let aa = ((base_a as f32) * fade * (1.0 + 0.08 * p1)) as u8;
        painter.circle_filled(g1_center, r, a(GOLD,   aa));
    }
    for (r, base_a) in [(160.0_f32, 4_u8), (100.0, 8), (60.0, 13)] {
        let aa = ((base_a as f32) * fade * (1.0 + 0.10 * p2)) as u8;
        painter.circle_filled(g2_center, r, a(VIOLET, aa));
    }

    // 3. Library corridor perspective lines ─────────────────────────────────
    let vp = center;
    let corr_alpha = (fade * 20.0) as u8;
    let corr = a(GOLD, corr_alpha);
    let shelf_fracs: [f32; 5] = [0.15, 0.28, 0.42, 0.58, 0.72];
    for &fy in &shelf_fracs {
        let ly = rect.top() + rect.height() * fy;
        painter.line_segment([pos2(rect.left(),  ly), vp], Stroke::new(0.6, corr));
        painter.line_segment([pos2(rect.right(), ly), vp], Stroke::new(0.6, corr));
    }
    painter.line_segment([pos2(rect.left(),  rect.top()),    vp], Stroke::new(0.6, corr));
    painter.line_segment([pos2(rect.right(), rect.top()),    vp], Stroke::new(0.6, corr));
    painter.line_segment([pos2(rect.left(),  rect.bottom()), vp], Stroke::new(0.6, corr));
    painter.line_segment([pos2(rect.right(), rect.bottom()), vp], Stroke::new(0.6, corr));

    // 4. Vignette — dark rings radiating from edges inward ──────────────────
    // Approximate radial vignette by painting the outer portions dark again.
    let vig_w  = rect.width()  * 0.22;
    let vig_h  = rect.height() * 0.22;
    for step in 0..8_u8 {
        let frac  = step as f32 / 8.0;
        let alpha = ((1.0 - frac) * (1.0 - frac) * 160.0 * fade) as u8;
        let shrink = frac * vig_w.min(vig_h);
        let vig_rect = Rect::from_min_max(
            pos2(rect.left() + shrink, rect.top() + shrink),
            pos2(rect.right() - shrink, rect.bottom() - shrink),
        );
        // paint only the border band by drawing the full rect and the inner rect
        // We fake it: each ring draws nothing at center. Instead paint outer shadow slabs.
        let _ = (vig_rect, alpha); // used below via edge slabs
    }
    // Simpler: just four dark gradient slabs at the edges
    for step in 0..12_u8 {
        let f  = step as f32 / 12.0;
        let aa = ((1.0 - f) * (1.0 - f) * 140.0 * fade) as u8;
        let d  = f * rect.width() * 0.20;
        painter.rect_filled(
            Rect::from_min_max(pos2(rect.left(), rect.top()), pos2(rect.left() + d, rect.bottom())),
            Rounding::ZERO, a(VOID, aa),
        );
        painter.rect_filled(
            Rect::from_min_max(pos2(rect.right() - d, rect.top()), pos2(rect.right(), rect.bottom())),
            Rounding::ZERO, a(VOID, aa),
        );
        let dv = f * rect.height() * 0.18;
        painter.rect_filled(
            Rect::from_min_max(pos2(rect.left(), rect.top()), pos2(rect.right(), rect.top() + dv)),
            Rounding::ZERO, a(VOID, aa),
        );
        painter.rect_filled(
            Rect::from_min_max(pos2(rect.left(), rect.bottom() - dv), pos2(rect.right(), rect.bottom())),
            Rounding::ZERO, a(VOID, aa),
        );
    }

    // 5. Emblem (book + rings) ───────────────────────────────────────────────
    // Spring-settle entrance: 0.4s delay, 1.8s duration
    let enter   = spring_out(((t - 0.4) / 1.8).clamp(0.0, 1.0));
    let e_alpha = (enter * fade * 255.0) as u8;
    // Float up from y+12 → y-30
    let e_y     = center.y - 55.0 - 30.0 * enter + (1.0 - enter) * 12.0;
    let emblem  = pos2(center.x, e_y);

    // Outer ring — slow CW spin + glow pulse
    let glow_p   = 0.5 + 0.5 * (t / 3.2 * TAU).sin();
    let outer_r  = 57.0_f32;
    let outer_a  = (e_alpha as f32 * 0.65) as u8;
    let glow_a   = (outer_a  as f32 * glow_p * 0.55) as u8;
    painter.circle_stroke(emblem, outer_r + 5.0, Stroke::new(5.0, a(GOLD, glow_a)));
    painter.circle_stroke(emblem, outer_r,        Stroke::new(2.0, a(GOLD, outer_a)));

    // Inner ring — slow CCW
    let inner_a = (e_alpha as f32 * 0.55) as u8;
    painter.circle_stroke(emblem, 38.0, Stroke::new(1.5, a(VIOLET, inner_a)));

    // Book symbol with page flutter
    let flutter = (t / 5.0 * TAU).sin() * 0.09;
    draw_book(painter, emblem, 27.0, flutter, e_alpha);

    // Core dot — heartbeat
    let pulse  = 1.0 + 0.20 * (t / 2.2 * TAU).sin();
    let dot_r  = 8.0 * pulse;
    let dot    = pos2(emblem.x + 19.0, emblem.y + 19.0);
    painter.circle_filled(dot, dot_r + 5.0, a(GOLD,   (e_alpha as f32 * 0.30) as u8));
    painter.circle_filled(dot, dot_r,        a(GOLD_LT, e_alpha));

    // 6. Wordmark ────────────────────────────────────────────────────────────
    let w_enter  = spring_out(((t - 0.8) / 1.2).clamp(0.0, 1.0));
    let w_alpha  = (w_enter * fade * 255.0) as u8;
    let word_y   = emblem.y + outer_r + 18.0;

    painter.text(
        pos2(center.x, word_y),
        Align2::CENTER_CENTER,
        "QUANTUM",
        FontId::proportional(11.0),
        a(GOLD, (w_alpha as f32 * 0.65) as u8),
    );
    painter.text(
        pos2(center.x, word_y + 24.0),
        Align2::CENTER_CENTER,
        "THE GREAT LIBRARY",
        FontId::proportional(40.0),
        a(GOLD, w_alpha),
    );

    // 7. Bottom info ─────────────────────────────────────────────────────────
    let b_enter = spring_out(((t - 1.1) / 1.0).clamp(0.0, 1.0));
    let b_alpha = (b_enter * fade * 255.0) as u8;
    let bot_y   = rect.bottom() - rect.height() * 0.20;

    painter.text(
        pos2(center.x, bot_y),
        Align2::CENTER_CENTER,
        "— THE REPOSITORY OF ALL THINGS —",
        FontId::proportional(11.0),
        a(GOLD, (b_alpha as f32 * 0.50) as u8),
    );

    let pips: &[(&str, Color32)] = &[
        ("MASTER VAULT", Color32::from_rgb(212, 160,  48)),
        ("ARCHIVE",      Color32::from_rgb(140,  80, 255)),
        ("QUANTUM CORE", Color32::from_rgb(  0, 200, 180)),
        ("ONLINE",       Color32::from_rgb(  0, 192,  96)),
    ];
    let spacing    = 132.0_f32;
    let total      = spacing * (pips.len() as f32 - 1.0);
    let pip_start  = center.x - total * 0.5;
    let pip_y      = bot_y + 24.0;

    for (i, &(label, col)) in pips.iter().enumerate() {
        let blink = 0.5 + 0.5 * ((t + i as f32 * 0.5) / 2.1 * TAU).sin();
        let pip_a = (b_alpha as f32 * blink) as u8;
        let lbl_a = (b_alpha as f32 * 0.58)  as u8;
        let px    = pip_start + i as f32 * spacing;

        painter.circle_filled(pos2(px, pip_y), 5.0, a(col, pip_a / 2));
        painter.circle_filled(pos2(px, pip_y), 3.0, a(col, pip_a));
        painter.text(
            pos2(px + 9.0, pip_y),
            Align2::LEFT_CENTER,
            label,
            FontId::monospace(9.0),
            Color32::from_rgba_unmultiplied(255, 255, 255, lbl_a),
        );
    }

    // 8. Version tag ─────────────────────────────────────────────────────────
    painter.text(
        pos2(center.x, rect.bottom() - 14.0),
        Align2::CENTER_CENTER,
        "BIOSPARK STUDIOS  ·  QUANTUM ECOSYSTEM  ·  v0.1.0",
        FontId::monospace(9.0),
        a(GOLD, (fade * 60.0) as u8),
    );

    // 9. Corner marks ────────────────────────────────────────────────────────
    let cs   = 16.0_f32;
    let marg = 20.0_f32;
    let ca   = (fade * 88.0) as u8;
    let cs_  = Stroke::new(1.5, a(GOLD, ca));
    corner_marks(painter, rect, marg, cs, cs_);

    // 10. Floating ambient fragments ─────────────────────────────────────────
    let frags: &[(&str, f32, f32, f32)] = &[
        ("IDX:4096", 0.06, 0.18, 0.0),
        ("ECHO:ON",  0.90, 0.34, 2.1),
        ("VOL:\u{221e}", 0.05, 0.57, 4.3),
        ("SIG:7E8A", 0.88, 0.70, 1.2),
        ("CAP:16",   0.89, 0.26, 3.0),
        ("SEAL:\u{2205}", 0.05, 0.52, 6.5),
    ];
    for &(text, fx, fy, delay) in frags {
        let float_y = ((t + delay) * 0.62).sin() * 7.0;
        let fa      = (fade * 120.0) as u8;
        painter.text(
            pos2(rect.left() + rect.width() * fx, rect.top() + rect.height() * fy + float_y),
            Align2::LEFT_CENTER,
            text,
            FontId::monospace(9.0),
            a(GOLD, fa),
        );
    }
}

// ── Open book ─────────────────────────────────────────────────────────────────

fn draw_book(painter: &egui::Painter, center: Pos2, hw: f32, flutter: f32, alpha: u8) {
    let top_y = center.y - hw;
    let bot_y = center.y + hw * 0.65;
    let sx    = center.x;

    let gold_a   = (alpha as f32 * 0.72) as u8;
    let violet_a = (alpha as f32 * 0.62) as u8;

    // Left page
    let lx = sx - hw * (1.0 + flutter);
    let left_tip = pos2(lx, top_y + 4.0);
    let s_l = Stroke::new(1.1, a(GOLD, gold_a));
    painter.line_segment([pos2(sx, top_y), left_tip], s_l);
    painter.line_segment([pos2(sx, bot_y), pos2(lx, bot_y + 2.0)], s_l);
    painter.line_segment([left_tip, pos2(lx, bot_y + 2.0)], s_l);

    // Right page
    let rx = sx + hw * (1.0 - flutter);
    let right_tip = pos2(rx, top_y + 4.0);
    let s_r = Stroke::new(1.1, a(VIOLET, violet_a));
    painter.line_segment([pos2(sx, top_y), right_tip], s_r);
    painter.line_segment([pos2(sx, bot_y), pos2(rx, bot_y + 2.0)], s_r);
    painter.line_segment([right_tip, pos2(rx, bot_y + 2.0)], s_r);

    // Spine
    painter.line_segment([pos2(sx, top_y), pos2(sx, bot_y)], Stroke::new(1.8, a(GOLD_LT, alpha)));

    // Text lines — left page (4 lines, fading)
    for i in 0..4_u8 {
        let ly = top_y + 11.0 + i as f32 * 8.0;
        let la = (gold_a as f32 * (1.0 - i as f32 * 0.20)) as u8;
        painter.line_segment([pos2(sx - hw * 0.82, ly), pos2(sx - 3.0, ly - 1.5)], Stroke::new(0.9, a(GOLD, la)));
    }
    // Text lines — right page
    for i in 0..4_u8 {
        let ly = top_y + 11.0 + i as f32 * 8.0;
        let la = (violet_a as f32 * (1.0 - i as f32 * 0.20)) as u8;
        painter.line_segment([pos2(sx + 3.0, ly - 1.5), pos2(sx + hw * 0.82, ly)], Stroke::new(0.9, a(VIOLET, la)));
    }

    // Crown ornament (small filled circle at spine top)
    painter.circle_filled(pos2(sx, top_y - 3.0), 2.8, a(GOLD_LT, alpha));
}

// ── Corner bracket marks ──────────────────────────────────────────────────────

fn corner_marks(painter: &egui::Painter, rect: Rect, margin: f32, size: f32, stroke: Stroke) {
    let tl = pos2(rect.left()  + margin, rect.top()    + margin);
    let tr = pos2(rect.right() - margin, rect.top()    + margin);
    let bl = pos2(rect.left()  + margin, rect.bottom() - margin);
    let br = pos2(rect.right() - margin, rect.bottom() - margin);

    // top-left
    painter.line_segment([tl, pos2(tl.x + size, tl.y)], stroke);
    painter.line_segment([tl, pos2(tl.x, tl.y + size)], stroke);
    // top-right
    painter.line_segment([tr, pos2(tr.x - size, tr.y)], stroke);
    painter.line_segment([tr, pos2(tr.x, tr.y + size)], stroke);
    // bottom-left
    painter.line_segment([bl, pos2(bl.x + size, bl.y)], stroke);
    painter.line_segment([bl, pos2(bl.x, bl.y - size)], stroke);
    // bottom-right
    painter.line_segment([br, pos2(br.x - size, br.y)], stroke);
    painter.line_segment([br, pos2(br.x, br.y - size)], stroke);
}

// ── Easing ────────────────────────────────────────────────────────────────────

/// Approximation of cubic-bezier(0.16, 1, 0.3, 1) — spring settle.
fn spring_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}
