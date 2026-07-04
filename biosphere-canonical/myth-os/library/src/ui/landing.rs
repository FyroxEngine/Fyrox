// Vault landing page — card grid.
// Renders inside the CentralPanel left by shell.rs.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Align2, FontId, Frame, Margin, Rounding, Sense, Stroke, pos2, vec2};
use uuid::Uuid;

use crate::state::AppScreen;
use crate::vault_store::{InitialRestoreDone, SelectedVault, VaultMeta, VaultStore, VaultType, load_prefs};
use super::UiSet;
use super::theme::{a, VOID, CARD_BG, GOLD, FG_3};

pub struct LandingPlugin;

impl Plugin for LandingPlugin {
    fn build(&self, app: &mut App) {
        app
            // Restore the last-opened vault exactly once per session.
            .add_systems(OnEnter(AppScreen::Landing), restore_last_vault)
            .add_systems(
                Update,
                draw_landing
                    .run_if(in_state(AppScreen::Landing))
                    .in_set(UiSet::Page),
            );
    }
}

/// On the first visit to Landing each session, check prefs for a saved vault ID.
/// If found and the vault still exists in the store, open it immediately so the
/// user lands back where they left off.
fn restore_last_vault(
    store:    Res<VaultStore>,
    mut sel:  ResMut<SelectedVault>,
    mut next: ResMut<NextState<AppScreen>>,
    mut done: ResMut<InitialRestoreDone>,
) {
    if done.0 { return; } // only fires once per session
    done.0 = true;

    let prefs = load_prefs();
    if let Some(id) = prefs.last_vault_id {
        if store.by_id(id).is_some() {
            sel.0 = Some(id);
            next.set(AppScreen::VaultView);
        }
    }
}

fn draw_landing(
    mut contexts: EguiContexts,
    mut store:    ResMut<VaultStore>,
    time:         Res<Time>,
    mut selected: ResMut<SelectedVault>,
    mut next:     ResMut<NextState<AppScreen>>,
) {
    let ctx = contexts.ctx_mut();
    let t   = time.elapsed_seconds();

    // Collect click events inside the panel, act on them outside.
    let mut open_vault:  Option<Uuid> = None;
    let mut do_import              = false;

    egui::CentralPanel::default()
        .frame(Frame::none().fill(VOID).inner_margin(Margin::same(28.0)))
        .show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Your Vaults")
                    .color(a(GOLD, 200)).size(14.0));
                ui.add_space(10.0);
                ui.label(egui::RichText::new(format!("({} active)", store.vaults.len()))
                    .color(FG_3).size(11.0).monospace());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let import_btn = egui::Button::new(
                        egui::RichText::new("⇡ Import").color(a(GOLD, 160)).size(10.0))
                        .fill(a(GOLD, 10))
                        .stroke(egui::Stroke::new(1.0, a(GOLD, 50)))
                        .rounding(egui::Rounding::same(3.0));
                    if ui.add(import_btn).clicked() {
                        do_import = true;
                    }
                });
            });
            ui.add_space(20.0);

            let available = ui.available_width();
            let gap       = 18.0_f32;
            let n_cols    = 3_usize;
            let card_w    = (available - gap * (n_cols as f32 - 1.0)) / n_cols as f32;
            let card_h    = 190.0_f32;

            let chunks: Vec<&[VaultMeta]> = store.vaults.chunks(n_cols).collect();

            for (row_idx, row) in chunks.iter().enumerate() {
                ui.horizontal(|ui| {
                    for (col_idx, vault) in row.iter().enumerate() {
                        if vault_card(ui, vault, card_w, card_h, t) {
                            open_vault = Some(vault.id);
                        }
                        if col_idx < row.len() - 1 {
                            ui.add_space(gap);
                        }
                    }
                });
                if row_idx < chunks.len() - 1 {
                    ui.add_space(gap);
                }
            }

            let rows = chunks.len();
            if rows > 0 { ui.add_space(gap); }

            let mut open_setup = false;
            ui.horizontal(|ui| {
                open_setup = new_vault_card(ui, card_w, card_h);
            });
            if open_setup {
                next.set(AppScreen::VaultSetup);
            }
        });

    if let Some(id) = open_vault {
        selected.0 = Some(id);
        next.set(AppScreen::VaultView);
    }

    if do_import {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Genesis Capsule", &["qgenesis"])
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(json) => {
                    match store.import_vault(&json) {
                        Ok(id) => {
                            bevy::log::info!("Vault imported successfully: {}", id);
                        }
                        Err(e) => {
                            bevy::log::warn!("Import failed: {e}");
                        }
                    }
                }
                Err(e) => {
                    bevy::log::warn!("Could not read import file: {e}");
                }
            }
        }
    }
}

// ── Vault card — returns true when clicked ────────────────────────────────────

/// Unicode glyph representing each vault type — used in the card emblem.
fn vault_type_glyph(vtype: &VaultType) -> &'static str {
    match vtype {
        VaultType::Audio  => "♫",  // waveform / music
        VaultType::TwoD   => "◈",  // diamond with center — flat plane
        VaultType::ThreeD => "⬡",  // hexagon — volumetric depth
        VaultType::Hybrid => "⬢",  // filled hexagon — composite form
        VaultType::Stage  => "◇",  // diamond — theatre/stage
    }
}

fn vault_card(ui: &mut egui::Ui, vault: &VaultMeta, w: f32, h: f32, t: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    if !ui.is_rect_visible(rect) { return false; }

    let hovered  = response.hovered();
    let col      = vault.color;
    let glow_a   = if hovered { 24_u8 } else { 10 };
    let border_a = if hovered { 160_u8 } else { 70 };

    let painter = ui.painter_at(rect);

    // Card body + glow border + crisp border
    painter.rect_filled(rect, Rounding::same(8.0), CARD_BG);
    painter.rect_stroke(rect.expand(1.5), Rounding::same(9.5),
        Stroke::new(6.0, a(col, glow_a)));
    painter.rect_stroke(rect, Rounding::same(8.0),
        Stroke::new(1.0, a(col, border_a)));

    corner_marks(&painter, rect, 8.0, 12.0, Stroke::new(1.0, a(col, 90)));

    // ── Vault type emblem (top-left) ────────────────────────────────────────
    let em_cx = rect.left() + 27.0;
    let em_cy = rect.top() + 28.0;
    painter.circle_filled(pos2(em_cx, em_cy), 15.0, a(col, 16));
    painter.circle_stroke(pos2(em_cx, em_cy), 15.0, Stroke::new(1.0, a(col, 65)));
    painter.text(
        pos2(em_cx, em_cy),
        Align2::CENTER_CENTER,
        vault_type_glyph(&vault.vault_type),
        FontId::proportional(13.0),
        a(col, 210),
    );

    // ── Vault name (right of emblem) ────────────────────────────────────────
    painter.text(
        pos2(rect.left() + 50.0, rect.top() + 20.0),
        Align2::LEFT_CENTER,
        &vault.name,
        FontId::proportional(15.0),
        col,
    );

    // ── Resonance Hz (below name) ───────────────────────────────────────────
    painter.text(
        pos2(rect.left() + 50.0, rect.top() + 36.0),
        Align2::LEFT_CENTER,
        &format!("{:.1} Hz", vault.resonance_hz),
        FontId::monospace(8.0),
        a(col, 65),
    );

    // ── Type badge (top-right) ──────────────────────────────────────────────
    painter.text(
        pos2(rect.right() - 14.0, rect.top() + 14.0),
        Align2::RIGHT_CENTER,
        vault.vault_type.label(),
        FontId::monospace(8.0),
        a(col, 100),
    );

    // ── Separator ───────────────────────────────────────────────────────────
    painter.line_segment(
        [pos2(rect.left() + 18.0, rect.top() + 54.0),
         pos2(rect.right() - 18.0, rect.top() + 54.0)],
        Stroke::new(0.8, a(col, 35)),
    );

    // ── Description ─────────────────────────────────────────────────────────
    painter.text(
        pos2(rect.left() + 18.0, rect.top() + 68.0),
        Align2::LEFT_TOP,
        &vault.description,
        FontId::proportional(11.0),
        FG_3,
    );

    // ── Status pip (bottom-right) ───────────────────────────────────────────
    let pip_x    = rect.right() - 16.0;
    let pip_y    = rect.bottom() - 18.0;
    let st_col   = vault.status.color();
    let st_label = vault.status.label();
    let blink    = 0.70 + 0.30 * (t * 1.4).sin();
    painter.circle_filled(pos2(pip_x, pip_y), 5.0, a(st_col, (blink * 60.0) as u8));
    painter.circle_filled(pos2(pip_x, pip_y), 3.0, a(st_col, (blink * 220.0) as u8));
    painter.text(
        pos2(pip_x - 9.0, pip_y),
        Align2::RIGHT_CENTER,
        st_label,
        FontId::monospace(8.0),
        a(st_col, 160),
    );

    // ── Hover state: "Enter →" hint + scan-line shimmer ────────────────────
    if hovered {
        painter.text(
            pos2(rect.left() + 18.0, rect.bottom() - 18.0),
            Align2::LEFT_CENTER,
            "Enter →",
            FontId::monospace(9.0),
            a(col, 140),
        );
        let shimmer_y = rect.top() + 65.0
            + ((t * 0.8 + rect.left() * 0.01).sin() * 0.5 + 0.5) * (h - 82.0);
        painter.line_segment(
            [pos2(rect.left() + 18.0, shimmer_y), pos2(rect.right() - 18.0, shimmer_y)],
            Stroke::new(0.5, a(col, 14)),
        );
    }

    response.clicked()
}

// ── "Create New Vault" card ───────────────────────────────────────────────────

fn new_vault_card(ui: &mut egui::Ui, w: f32, h: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    if !ui.is_rect_visible(rect) { return false; }

    let hovered  = response.hovered();
    let border_a = if hovered { 100_u8 } else { 45 };
    let text_a   = if hovered { 180_u8 } else { 90 };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(8.0), a(GOLD, 4));
    dashed_rect(&painter, rect, Stroke::new(1.0, a(GOLD, border_a)));

    let cx = rect.center().x;
    let cy = rect.center().y;
    let arm = 16.0_f32;
    let s = Stroke::new(1.5, a(GOLD, text_a));
    painter.line_segment([pos2(cx - arm, cy), pos2(cx + arm, cy)], s);
    painter.line_segment([pos2(cx, cy - arm), pos2(cx, cy + arm)], s);
    painter.text(pos2(cx, cy + arm + 16.0), Align2::CENTER_CENTER,
        "New Vault", FontId::proportional(13.0), a(GOLD, text_a));

    response.clicked()
}

// ── Draw helpers ──────────────────────────────────────────────────────────────

fn corner_marks(painter: &egui::Painter, rect: egui::Rect, margin: f32, size: f32, stroke: Stroke) {
    let tl = pos2(rect.left()  + margin, rect.top()    + margin);
    let tr = pos2(rect.right() - margin, rect.top()    + margin);
    let bl = pos2(rect.left()  + margin, rect.bottom() - margin);
    let br = pos2(rect.right() - margin, rect.bottom() - margin);
    for (p, dx, dy) in [
        (tl,  size,  0.0), (tl, 0.0,  size),
        (tr, -size,  0.0), (tr, 0.0,  size),
        (bl,  size,  0.0), (bl, 0.0, -size),
        (br, -size,  0.0), (br, 0.0, -size),
    ] {
        painter.line_segment([p, pos2(p.x + dx, p.y + dy)], stroke);
    }
}

fn dashed_rect(painter: &egui::Painter, rect: egui::Rect, stroke: Stroke) {
    let dash = 8.0_f32;
    let step = dash + 5.0;
    let mut x = rect.left();
    while x < rect.right() {
        let end = (x + dash).min(rect.right());
        painter.line_segment([pos2(x, rect.top()),    pos2(end, rect.top())],    stroke);
        painter.line_segment([pos2(x, rect.bottom()), pos2(end, rect.bottom())], stroke);
        x += step;
    }
    let mut y = rect.top();
    while y < rect.bottom() {
        let end = (y + dash).min(rect.bottom());
        painter.line_segment([pos2(rect.left(),  y), pos2(rect.left(),  end)], stroke);
        painter.line_segment([pos2(rect.right(), y), pos2(rect.right(), end)], stroke);
        y += step;
    }
}
