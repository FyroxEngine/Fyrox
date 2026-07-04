// Vault creation wizard — counselor-style, 4 steps.
// Step 0: vault type  (auto-advance on selection)
// Step 1: name + description
// Step 2: aura / colour  (auto-advance on selection)
// Step 3: confirm + Initiate Genesis

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Align2, FontId, Frame, Margin, Rounding, Sense, Stroke, pos2, vec2};
use uuid::Uuid;

use crate::state::AppScreen;
use biospark_theatre::LayoutBlueprint;
use crate::vault_store::{
    AURA_PALETTE, SelectedVault, SetupDraft, VaultMeta, VaultStatus, VaultStore, VaultType,
    generate_bdna,
};
use super::UiSet;
use super::theme::{a, VOID, ABYSS, CARD_BG, GOLD, FG_3, FG_MUTED};

pub struct VaultSetupPlugin;

impl Plugin for VaultSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            draw_vault_setup
                .run_if(in_state(AppScreen::VaultSetup))
                .in_set(UiSet::Page),
        );
    }
}

fn draw_vault_setup(
    mut contexts: EguiContexts,
    mut draft:    ResMut<SetupDraft>,
    mut store:    ResMut<VaultStore>,
    mut selected: ResMut<SelectedVault>,
    mut next:     ResMut<NextState<AppScreen>>,
) {
    let ctx = contexts.ctx_mut();

    let mut advance    = false;
    let mut go_back    = false;
    let mut cancel     = false;
    let mut genesis    = false;

    egui::CentralPanel::default()
        .frame(Frame::none().fill(VOID).inner_margin(Margin::same(0.0)))
        .show(ctx, |ui| {
            // Centre everything in the available rect
            let rect = ui.max_rect();
            let cx   = rect.center().x;

            match draft.step {
                0 => step_type(ui, rect, cx, &mut draft, &mut advance),
                1 => step_name(ui, rect, cx, &mut draft, &mut advance, &mut go_back, &mut cancel),
                2 => step_aura(ui, rect, cx, &mut draft, &mut advance, &mut go_back),
                3 => step_layout(ui, rect, cx, &mut draft, &mut advance, &mut go_back),
                4 => step_confirm(ui, rect, cx, &draft, &mut genesis, &mut go_back, &mut cancel),
                _ => {}
            }
        });

    if advance   { draft.step += 1; }
    if go_back   { if draft.step > 0 { draft.step -= 1; } }
    if cancel    { draft.reset(); next.set(AppScreen::Landing); }
    if genesis   { initiate_genesis(&mut draft, &mut store, &mut selected, &mut next); }
}

// ── Step 0 — vault type ───────────────────────────────────────────────────────

fn step_type(
    ui:      &mut egui::Ui,
    rect:    egui::Rect,
    cx:      f32,
    draft:   &mut SetupDraft,
    advance: &mut bool,
) {
    let p = ui.painter().clone();

    // Prompt
    p.text(
        pos2(cx, rect.top() + rect.height() * 0.22),
        Align2::CENTER_CENTER,
        "What kind of work will this vault hold?",
        FontId::proportional(20.0),
        a(egui::Color32::WHITE, 200),
    );
    p.text(
        pos2(cx, rect.top() + rect.height() * 0.22 + 28.0),
        Align2::CENTER_CENTER,
        "You can always add more later.",
        FontId::proportional(12.0),
        a(egui::Color32::WHITE, 80),
    );

    // Type cards
    let types: &[(VaultType, egui::Color32)] = &[
        (VaultType::Audio,  egui::Color32::from_rgb(220, 140,  30)),
        (VaultType::TwoD,   egui::Color32::from_rgb(140,  80, 255)),
        (VaultType::ThreeD, egui::Color32::from_rgb(  0, 200, 180)),
        (VaultType::Hybrid, egui::Color32::from_rgb(212, 160,  48)),
        (VaultType::Stage,  egui::Color32::from_rgb(224, 216, 255)),
    ];

    let card_w = 140.0_f32;
    let card_h = 120.0_f32;
    let gap    = 14.0_f32;
    let total  = card_w * types.len() as f32 + gap * (types.len() as f32 - 1.0);
    let start  = cx - total * 0.5;
    let top    = rect.top() + rect.height() * 0.38;

    for (i, (vtype, col)) in types.iter().enumerate() {
        let x = start + i as f32 * (card_w + gap);
        let cr = egui::Rect::from_min_size(pos2(x, top), vec2(card_w, card_h));

        let id       = egui::Id::new(("type_card", i));
        let response = ui.interact(cr, id, Sense::click());
        let hovered  = response.hovered();
        let border_a = if hovered { 180_u8 } else { 60 };
        let glow_a   = if hovered { 20_u8  } else {  6 };

        let p2 = ui.painter_at(cr);
        p2.rect_filled(cr, Rounding::same(8.0), CARD_BG);
        p2.rect_stroke(cr.expand(1.5), Rounding::same(9.5),
            Stroke::new(5.0, a(*col, glow_a)));
        p2.rect_stroke(cr, Rounding::same(8.0),
            Stroke::new(1.0, a(*col, border_a)));
        p2.text(pos2(cr.center().x, cr.top() + 38.0), Align2::CENTER_CENTER,
            vtype.label(), FontId::proportional(22.0), *col);
        p2.text(pos2(cr.center().x, cr.top() + 68.0), Align2::CENTER_CENTER,
            vtype.tagline(), FontId::proportional(9.5),
            a(egui::Color32::WHITE, 100));

        if response.clicked() {
            draft.vault_type = Some(vtype.clone());
            *advance = true;
        }
    }
}

// ── Step 1 — name + description ───────────────────────────────────────────────

fn step_name(
    ui:      &mut egui::Ui,
    rect:    egui::Rect,
    cx:      f32,
    draft:   &mut SetupDraft,
    advance: &mut bool,
    go_back: &mut bool,
    cancel:  &mut bool,
) {
    let top = rect.top() + rect.height() * 0.18;
    let p   = ui.painter().clone();

    p.text(pos2(cx, top), Align2::CENTER_CENTER,
        "Name your vault.", FontId::proportional(20.0),
        a(egui::Color32::WHITE, 200));
    p.text(pos2(cx, top + 28.0), Align2::CENTER_CENTER,
        "A name and a brief description are all that's needed.",
        FontId::proportional(12.0), a(egui::Color32::WHITE, 80));

    let field_w = 420.0_f32;
    let field_x = cx - field_w * 0.5;

    // Name field
    p.text(pos2(field_x, top + 74.0), Align2::LEFT_CENTER,
        "VAULT NAME", FontId::monospace(9.0), a(GOLD, 120));
    let name_rect = egui::Rect::from_min_size(pos2(field_x, top + 86.0), vec2(field_w, 36.0));
    ui.allocate_ui_at_rect(name_rect, |ui| {
        ui.style_mut().visuals.extreme_bg_color = ABYSS;
        let resp = ui.add_sized(
            name_rect.size(),
            egui::TextEdit::singleline(&mut draft.name)
                .hint_text("e.g. Master Vault, Project Aurora…")
                .font(FontId::proportional(14.0)),
        );
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Tab)) {
            // tab focus to description
        }
    });

    // Description field
    p.text(pos2(field_x, top + 140.0), Align2::LEFT_CENTER,
        "DESCRIPTION  (optional)", FontId::monospace(9.0), a(GOLD, 80));
    let desc_rect = egui::Rect::from_min_size(pos2(field_x, top + 152.0), vec2(field_w, 60.0));
    ui.allocate_ui_at_rect(desc_rect, |ui| {
        ui.style_mut().visuals.extreme_bg_color = ABYSS;
        ui.add_sized(
            desc_rect.size(),
            egui::TextEdit::multiline(&mut draft.description)
                .hint_text("What lives here?")
                .font(FontId::proportional(13.0)),
        );
    });

    // Buttons
    let btn_y = top + 240.0;
    let cont_rect = egui::Rect::from_min_size(pos2(cx - 10.0 - 120.0, btn_y), vec2(120.0, 36.0));
    let back_rect = egui::Rect::from_min_size(pos2(cx + 10.0,          btn_y), vec2(80.0,  36.0));
    let cncl_rect = egui::Rect::from_min_size(pos2(field_x,            btn_y), vec2(70.0,  36.0));

    if styled_btn(ui, cont_rect, "Continue →", GOLD, draft.name.trim().len() >= 2) {
        *advance = true;
    }
    if styled_btn(ui, back_rect, "← Back", FG_3, true) {
        *go_back = true;
    }
    if styled_btn(ui, cncl_rect, "Cancel", FG_MUTED, true) {
        *cancel = true;
    }
}

// ── Step 2 — aura / colour ────────────────────────────────────────────────────

fn step_aura(
    ui:      &mut egui::Ui,
    rect:    egui::Rect,
    cx:      f32,
    draft:   &mut SetupDraft,
    advance: &mut bool,
    go_back: &mut bool,
) {
    let top = rect.top() + rect.height() * 0.20;
    let p   = ui.painter().clone();

    p.text(pos2(cx, top), Align2::CENTER_CENTER,
        "Choose your vault aura.", FontId::proportional(20.0),
        a(egui::Color32::WHITE, 200));
    p.text(pos2(cx, top + 28.0), Align2::CENTER_CENTER,
        "This colour becomes the vault's resonance signature.",
        FontId::proportional(12.0), a(egui::Color32::WHITE, 80));

    // Colour swatches — 4 per row
    let swatch_r = 28.0_f32;
    let gap      = 18.0_f32;
    let n_cols   = 4_usize;
    let n_rows   = (AURA_PALETTE.len() + n_cols - 1) / n_cols;
    let row_w    = swatch_r * 2.0 * n_cols as f32 + gap * (n_cols as f32 - 1.0);
    let start_x  = cx - row_w * 0.5;
    let start_y  = top + 68.0;

    for (i, &(col, name, hz)) in AURA_PALETTE.iter().enumerate() {
        let row  = i / n_cols;
        let col_ = i % n_cols;
        let sx   = start_x + col_ as f32 * (swatch_r * 2.0 + gap) + swatch_r;
        let sy   = start_y + row  as f32 * (swatch_r * 2.0 + gap + 28.0) + swatch_r;

        let selected = draft.color_idx == i;
        let sense_r  = egui::Rect::from_center_size(pos2(sx, sy), vec2(swatch_r * 2.0 + 8.0, swatch_r * 2.0 + 8.0));
        let id       = egui::Id::new(("aura", i));
        let response = ui.interact(sense_r, id, Sense::click());
        let hovered  = response.hovered();

        let pa = ui.painter_at(sense_r);
        if selected {
            pa.circle_filled(pos2(sx, sy), swatch_r + 4.0, a(col, 30));
            pa.circle_stroke(pos2(sx, sy), swatch_r + 4.0, Stroke::new(1.5, col));
        } else if hovered {
            pa.circle_filled(pos2(sx, sy), swatch_r + 2.0, a(col, 16));
        }
        pa.circle_filled(pos2(sx, sy), swatch_r, a(col, if selected { 255 } else { 180 }));
        pa.text(pos2(sx, sy + swatch_r + 12.0), Align2::CENTER_CENTER,
            name, FontId::monospace(8.0), a(col, 160));
        pa.text(pos2(sx, sy + swatch_r + 23.0), Align2::CENTER_CENTER,
            &format!("{hz:.0} Hz"), FontId::monospace(7.0), a(col, 90));

        if response.clicked() {
            draft.color_idx = i;
            *advance = true;
        }
    }

    // Back button
    let back_y   = start_y + n_rows as f32 * (swatch_r * 2.0 + gap + 20.0) + 16.0;
    let back_r   = egui::Rect::from_min_size(pos2(cx - 40.0, back_y), vec2(80.0, 36.0));
    if styled_btn(ui, back_r, "← Back", FG_3, true) {
        *go_back = true;
    }
}

// ── Step 3 — layout blueprint ─────────────────────────────────────────────────

fn step_layout(
    ui:      &mut egui::Ui,
    rect:    egui::Rect,
    cx:      f32,
    draft:   &mut SetupDraft,
    advance: &mut bool,
    go_back: &mut bool,
) {
    let top = rect.top() + rect.height() * 0.18;
    let p   = ui.painter().clone();

    p.text(pos2(cx, top), Align2::CENTER_CENTER,
        "Choose a layout blueprint.", FontId::proportional(20.0),
        a(egui::Color32::WHITE, 200));
    p.text(pos2(cx, top + 28.0), Align2::CENTER_CENTER,
        "Controls how the Theatre canvas is partitioned into zones.",
        FontId::proportional(12.0), a(egui::Color32::WHITE, 80));

    let col    = draft.chosen_color();
    let card_w = 148.0_f32;
    let card_h = 100.0_f32;
    let gap    = 10.0_f32;
    let cols   = 3_usize;

    let blueprints = LayoutBlueprint::ALL;
    let start_y    = top + 64.0;
    let total_w    = card_w * cols as f32 + gap * (cols as f32 - 1.0);
    let start_x    = cx - total_w * 0.5;

    for (i, &bp) in blueprints.iter().enumerate() {
        let row = i / cols;
        let col_i = i % cols;
        let x = start_x + col_i as f32 * (card_w + gap);
        let y = start_y + row as f32 * (card_h + gap);
        let cr = egui::Rect::from_min_size(pos2(x, y), vec2(card_w, card_h));

        let id       = egui::Id::new(("layout_card", bp.id()));
        let response = ui.interact(cr, id, Sense::click());
        let selected = draft.layout_blueprint == bp;
        let hovered  = response.hovered();

        let p2 = ui.painter_at(cr);
        p2.rect_filled(cr, Rounding::same(6.0), CARD_BG);
        p2.rect_stroke(cr, Rounding::same(6.0),
            Stroke::new(if selected { 1.5 } else { 1.0 },
                a(col, if selected { 200 } else if hovered { 100 } else { 30 })));
        if selected {
            p2.rect_stroke(cr.expand(2.0), Rounding::same(8.0),
                Stroke::new(3.0, a(col, 18)));
        }

        // Mini layout preview — abstract shapes per blueprint
        let prev_r = egui::Rect::from_center_size(
            pos2(cr.center().x, cr.top() + 36.0),
            vec2(cr.width() - 20.0, 44.0),
        );
        draw_layout_preview(&ui.painter_at(prev_r), prev_r, bp, col, selected);

        p2.text(pos2(cr.center().x, cr.top() + 66.0), Align2::CENTER_CENTER,
            bp.title(), FontId::monospace(8.0),
            a(col, if selected { 220 } else { 120 }));
        p2.text(pos2(cr.center().x, cr.top() + 80.0), Align2::CENTER_CENTER,
            bp.description(), FontId::proportional(8.0),
            a(egui::Color32::WHITE, if selected { 100 } else { 55 }));

        if response.clicked() {
            draft.layout_blueprint = bp;
            *advance = true;
        }
    }

    // Back button
    let n_rows  = (blueprints.len() + cols - 1) / cols;
    let back_y  = start_y + n_rows as f32 * (card_h + gap) + 10.0;
    let back_r  = egui::Rect::from_min_size(pos2(cx - 40.0, back_y), vec2(80.0, 36.0));
    if styled_btn(ui, back_r, "← Back", FG_3, true) {
        *go_back = true;
    }
}

/// Paint a tiny schematic of the layout in the card preview area.
fn draw_layout_preview(
    p: &egui::Painter,
    r: egui::Rect,
    bp: LayoutBlueprint,
    col: egui::Color32,
    active: bool,
) {
    let fill = a(col, if active { 30 } else { 12 });
    let stroke = Stroke::new(0.8, a(col, if active { 80 } else { 35 }));
    let gap = 2.0_f32;

    match bp {
        LayoutBlueprint::Fullscreen => {
            p.rect_filled(r, Rounding::same(2.0), fill);
            p.rect_stroke(r, Rounding::same(2.0), stroke);
        }
        LayoutBlueprint::PancakeStack => {
            let hdr = egui::Rect::from_min_size(r.min, vec2(r.width(), r.height() * 0.18));
            let mid = egui::Rect::from_min_size(
                pos2(r.left(), hdr.bottom() + gap),
                vec2(r.width(), r.height() * 0.64));
            let ftr = egui::Rect::from_min_size(
                pos2(r.left(), mid.bottom() + gap),
                vec2(r.width(), r.height() * 0.18));
            for rect in [hdr, mid, ftr] {
                p.rect_filled(rect, Rounding::same(1.0), fill);
                p.rect_stroke(rect, Rounding::same(1.0), stroke);
            }
        }
        LayoutBlueprint::SidebarLeft => {
            let side = egui::Rect::from_min_size(r.min, vec2(r.width() * 0.28, r.height()));
            let main = egui::Rect::from_min_size(
                pos2(side.right() + gap, r.top()),
                vec2(r.width() - side.width() - gap, r.height()));
            p.rect_filled(side, Rounding::same(1.0), fill);
            p.rect_stroke(side, Rounding::same(1.0), stroke);
            p.rect_filled(main, Rounding::same(1.0), a(col, if active { 15 } else { 6 }));
            p.rect_stroke(main, Rounding::same(1.0), stroke);
        }
        LayoutBlueprint::HolyGrail => {
            let hdr_h = r.height() * 0.18;
            let ftr_h = r.height() * 0.18;
            let mid_h = r.height() - hdr_h - ftr_h - gap * 2.0;
            let hdr = egui::Rect::from_min_size(r.min, vec2(r.width(), hdr_h));
            let ftr = egui::Rect::from_min_size(pos2(r.left(), r.bottom() - ftr_h), vec2(r.width(), ftr_h));
            let mid_y = hdr.bottom() + gap;
            let lw = r.width() * 0.22;
            let left  = egui::Rect::from_min_size(pos2(r.left(), mid_y), vec2(lw, mid_h));
            let right = egui::Rect::from_min_size(pos2(r.right() - lw, mid_y), vec2(lw, mid_h));
            let main  = egui::Rect::from_min_size(pos2(left.right() + gap, mid_y), vec2(r.width() - lw * 2.0 - gap * 2.0, mid_h));
            for rect in [hdr, ftr, left, right, main] {
                p.rect_filled(rect, Rounding::same(1.0), fill);
                p.rect_stroke(rect, Rounding::same(1.0), stroke);
            }
        }
        LayoutBlueprint::SplitScreen => {
            let left = egui::Rect::from_min_size(r.min, vec2(r.width() * 0.5 - gap * 0.5, r.height()));
            let right = egui::Rect::from_min_size(pos2(left.right() + gap, r.top()), vec2(left.width(), r.height()));
            for rect in [left, right] {
                p.rect_filled(rect, Rounding::same(1.0), fill);
                p.rect_stroke(rect, Rounding::same(1.0), stroke);
            }
        }
        LayoutBlueprint::Masonry => {
            let cw = (r.width() - gap * 2.0) / 3.0;
            let heights = [r.height() * 0.7, r.height(), r.height() * 0.8];
            for (i, &h) in heights.iter().enumerate() {
                let cr = egui::Rect::from_min_size(
                    pos2(r.left() + i as f32 * (cw + gap), r.top()),
                    vec2(cw, h));
                p.rect_filled(cr, Rounding::same(1.0), fill);
                p.rect_stroke(cr, Rounding::same(1.0), stroke);
            }
        }
        LayoutBlueprint::PerfectCenter => {
            p.rect_stroke(r, Rounding::same(2.0), stroke);
            let inner = r.shrink(r.width() * 0.22);
            p.rect_filled(inner, Rounding::same(1.0), fill);
            p.rect_stroke(inner, Rounding::same(1.0), stroke);
        }
        LayoutBlueprint::StickyFooter => {
            let body = egui::Rect::from_min_size(r.min, vec2(r.width(), r.height() * 0.80));
            let ftr  = egui::Rect::from_min_size(pos2(r.left(), body.bottom() + gap), vec2(r.width(), r.height() * 0.20 - gap));
            p.rect_filled(body, Rounding::same(1.0), a(col, if active { 12 } else { 5 }));
            p.rect_stroke(body, Rounding::same(1.0), stroke);
            p.rect_filled(ftr, Rounding::same(1.0), fill);
            p.rect_stroke(ftr, Rounding::same(1.0), stroke);
        }
        LayoutBlueprint::TwelveColumn => {
            let cw = (r.width() - 11.0 * 1.5) / 12.0;
            for i in 0..12_u32 {
                let cr = egui::Rect::from_min_size(
                    pos2(r.left() + i as f32 * (cw + 1.5), r.top()),
                    vec2(cw, r.height()));
                p.rect_filled(cr, Rounding::ZERO, a(col, if active { 20 } else { 8 }));
            }
        }
    }
}

// ── Step 4 — confirm ──────────────────────────────────────────────────────────

fn step_confirm(
    ui:      &mut egui::Ui,
    rect:    egui::Rect,
    cx:      f32,
    draft:   &SetupDraft,
    genesis: &mut bool,
    go_back: &mut bool,
    cancel:  &mut bool,
) {
    let top    = rect.top() + rect.height() * 0.15;
    let col    = draft.chosen_color();
    let p      = ui.painter().clone();

    p.text(pos2(cx, top), Align2::CENTER_CENTER,
        "Your vault awaits.", FontId::proportional(22.0),
        a(egui::Color32::WHITE, 200));
    p.text(pos2(cx, top + 30.0), Align2::CENTER_CENTER,
        "Review and initiate the Genesis process.",
        FontId::proportional(12.0), a(egui::Color32::WHITE, 80));

    // Summary card
    let card_w = 440.0_f32;
    let card_h = 160.0_f32;
    let card   = egui::Rect::from_min_size(
        pos2(cx - card_w * 0.5, top + 60.0),
        vec2(card_w, card_h),
    );
    let p2 = ui.painter_at(card);
    p2.rect_filled(card, Rounding::same(10.0), CARD_BG);
    p2.rect_stroke(card, Rounding::same(10.0), Stroke::new(1.0, a(col, 80)));
    p2.rect_stroke(card.expand(2.0), Rounding::same(12.0), Stroke::new(4.0, a(col, 16)));

    // Colour swatch
    p2.circle_filled(pos2(card.left() + 28.0, card.top() + 28.0), 10.0, col);

    // Name + type + desc
    let name_display = if draft.name.trim().is_empty() { "Unnamed Vault" } else { draft.name.trim() };
    let vtype_label  = draft.vault_type.as_ref().map(|t| t.label()).unwrap_or("—");

    p2.text(pos2(card.left() + 48.0, card.top() + 28.0), Align2::LEFT_CENTER,
        name_display, FontId::proportional(18.0), col);
    p2.text(pos2(card.right() - 14.0, card.top() + 14.0), Align2::RIGHT_CENTER,
        vtype_label, FontId::monospace(9.0), a(col, 120));
    p2.text(pos2(card.right() - 14.0, card.top() + 28.0), Align2::RIGHT_CENTER,
        draft.layout_blueprint.title(), FontId::monospace(7.0), a(col, 70));
    p2.line_segment(
        [pos2(card.left() + 16.0, card.top() + 48.0),
         pos2(card.right() - 16.0, card.top() + 48.0)],
        Stroke::new(0.8, a(col, 30)),
    );
    let desc_text = if draft.description.trim().is_empty() {
        "No description provided."
    } else {
        draft.description.trim()
    };
    p2.text(pos2(card.left() + 16.0, card.top() + 68.0), Align2::LEFT_TOP,
        desc_text, FontId::proportional(12.0), a(egui::Color32::WHITE, 140));

    // Genesis button
    let gen_w  = 200.0_f32;
    let gen_y  = top + 60.0 + card_h + 28.0;
    let gen_r  = egui::Rect::from_min_size(pos2(cx - gen_w * 0.5, gen_y), vec2(gen_w, 44.0));
    let id     = egui::Id::new("genesis_btn");
    let resp   = ui.interact(gen_r, id, Sense::click());
    let hov    = resp.hovered();

    let gp     = ui.painter_at(gen_r);
    gp.rect_filled(gen_r, Rounding::same(6.0), a(col, if hov { 40 } else { 22 }));
    gp.rect_stroke(gen_r, Rounding::same(6.0), Stroke::new(1.0, a(col, if hov { 200 } else { 120 })));
    gp.text(gen_r.center(), Align2::CENTER_CENTER,
        "⬡  INITIATE GENESIS", FontId::proportional(13.0),
        a(egui::Color32::WHITE, if hov { 240 } else { 180 }));
    if resp.clicked() { *genesis = true; }

    // Back / Cancel
    let btn_y  = gen_y + 52.0;
    let back_r = egui::Rect::from_min_size(pos2(cx - 100.0, btn_y), vec2(80.0, 32.0));
    let cncl_r = egui::Rect::from_min_size(pos2(cx + 20.0,  btn_y), vec2(80.0, 32.0));
    if styled_btn(ui, back_r, "← Back",  FG_3,    true) { *go_back = true; }
    if styled_btn(ui, cncl_r, "Cancel",  FG_MUTED, true) { *cancel  = true; }
}

// ── Genesis commit ────────────────────────────────────────────────────────────

fn initiate_genesis(
    draft:    &mut SetupDraft,
    store:    &mut VaultStore,
    selected: &mut SelectedVault,
    next:     &mut NextState<AppScreen>,
) {
    let id   = Uuid::new_v4();
    let name = if draft.name.trim().is_empty() {
        "Unnamed Vault".to_string()
    } else {
        draft.name.trim().to_string()
    };

    // Generate B-DNA lineage fingerprint at the moment of genesis.
    // with_time = true mixes in nanosecond timestamp for uniqueness.
    let bdna = generate_bdna(id, &name, true);

    let meta = VaultMeta {
        id,
        name,
        description:      draft.description.trim().to_string(),
        color:            draft.chosen_color(),
        resonance_hz:     draft.chosen_resonance_hz(),
        status:           VaultStatus::Active,
        vault_type:       draft.vault_type.clone().unwrap_or_default(),
        is_protected:     false,
        plugins:          draft.vault_type.as_ref()
                              .map(|t| t.default_plugins())
                              .unwrap_or_default(),
        bdna_signature:   bdna,
        active_plugin:    None,
        layout_blueprint: draft.layout_blueprint,
    };
    store.add(meta);
    selected.0 = Some(id);
    draft.reset();
    // Play the cinematic Genesis boot sequence before opening the vault.
    next.set(AppScreen::GenesisBootSequence);
}

// ── Button helper ─────────────────────────────────────────────────────────────

fn styled_btn(
    ui:      &mut egui::Ui,
    rect:    egui::Rect,
    label:   &str,
    col:     egui::Color32,
    enabled: bool,
) -> bool {
    let id       = egui::Id::new(label);
    let response = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let hovered  = response.hovered() && enabled;

    let p = ui.painter_at(rect);
    p.rect_filled(rect, Rounding::same(4.0), a(col, if hovered { 18 } else { 8 }));
    p.rect_stroke(rect, Rounding::same(4.0), Stroke::new(1.0, a(col, if hovered { 100 } else { 40 })));
    p.text(rect.center(), Align2::CENTER_CENTER, label,
        FontId::proportional(12.0), a(col, if enabled { if hovered { 220 } else { 140 } } else { 60 }));

    response.clicked() && enabled
}
