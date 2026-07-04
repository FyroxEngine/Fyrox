// Vault interior — plugin sidebar + main content area + plugin store overlay.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Align2, FontId, Frame, Margin, Rounding, Sense, Stroke, pos2, vec2};

use crate::core_status::CoreStatus;
use crate::state::AppScreen;
use crate::theatre_state::TheatreMixerState;
use crate::vault_store::{SelectedVault, VaultStore, bdna_to_bits};
use crate::plugin_registry::{ActivePlugin, PluginRegistry, PluginStoreOpen};
use super::UiSet;
use super::theme::{a, VOID, ABYSS, CARD_BG, GOLD, FG_3, FG_MUTED};

// ── Vault content tab ─────────────────────────────────────────────────────────

/// Which panel is active inside the vault home view (no plugin selected).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VaultTab {
    #[default]
    Overview,
    Souls,
    Containers,
    Assets,
    Archive,
}

impl VaultTab {
    fn label(self) -> &'static str {
        match self {
            Self::Overview   => "OVERVIEW",
            Self::Souls      => "SOULS",
            Self::Containers => "CONTAINERS",
            Self::Assets     => "ASSETS",
            Self::Archive    => "ARCHIVE",
        }
    }
}

const VAULT_TABS: &[VaultTab] = &[
    VaultTab::Overview,
    VaultTab::Souls,
    VaultTab::Containers,
    VaultTab::Assets,
    VaultTab::Archive,
];

pub struct VaultViewPlugin;

impl Plugin for VaultViewPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<VaultTab>()
            .add_systems(
                Update,
                draw_vault_view
                    .run_if(in_state(AppScreen::VaultView))
                    .in_set(UiSet::Page),
            )
            // Restore plugin + reset tab to Overview when entering any vault.
            .add_systems(OnEnter(AppScreen::VaultView), (restore_active_plugin, reset_vault_tab))
            // Persist the active plugin back to the vault store when leaving.
            .add_systems(OnExit(AppScreen::VaultView),  save_active_plugin);
    }
}

// ── Active plugin persistence ─────────────────────────────────────────────────

fn restore_active_plugin(
    selected:    Res<SelectedVault>,
    vault_store: Res<VaultStore>,
    registry:    Res<PluginRegistry>,
    mut active:  ResMut<ActivePlugin>,
) {
    active.0 = None; // reset first
    if let Some(id) = selected.0 {
        if let Some(vault) = vault_store.by_id(id) {
            if let Some(plugin_id) = &vault.active_plugin {
                // Look up the static `&str` id from the registry so ActivePlugin stays `&'static str`.
                if let Some(def) = registry.by_id(plugin_id) {
                    active.0 = Some(def.id);
                }
            }
        }
    }
}

fn save_active_plugin(
    selected:        Res<SelectedVault>,
    mut vault_store: ResMut<VaultStore>,
    active:          Res<ActivePlugin>,
) {
    if let Some(id) = selected.0 {
        if let Some(vault) = vault_store.by_id_mut(id) {
            vault.active_plugin = active.0.map(|s| s.to_string());
        }
    }
}

/// Reset the content tab to Overview when entering a vault.
fn reset_vault_tab(mut tab: ResMut<VaultTab>) {
    *tab = VaultTab::default();
}

fn draw_vault_view(
    mut contexts:   EguiContexts,
    selected:       Res<SelectedVault>,
    registry:       Res<PluginRegistry>,
    mut active:     ResMut<ActivePlugin>,
    mut store_open: ResMut<PluginStoreOpen>,
    mut vault_store: ResMut<VaultStore>,
    mut vault_tab:  ResMut<VaultTab>,
    core_status:    Res<CoreStatus>,
    tmx:            Res<TheatreMixerState>,
) {
    let ctx = contexts.ctx_mut();

    let vault_id  = selected.0;
    let vault_col = vault_id
        .and_then(|id| vault_store.by_id(id))
        .map(|v| v.color)
        .unwrap_or(GOLD);

    // ── Left sidebar — installed plugin icons ─────────────────────────────────

    egui::SidePanel::left("vault_sidebar")
        .exact_width(60.0)
        .frame(Frame::none()
            .fill(ABYSS)
            .inner_margin(Margin::same(0.0))
            .stroke(Stroke::new(1.0, a(vault_col, 22))))
        .show(ctx, |ui| {
            ui.add_space(10.0);

            // Installed plugin icons
            if let Some(id) = vault_id {
                if let Some(vault) = vault_store.by_id(id) {
                    for plugin_id in &vault.plugins {
                        if let Some(def) = registry.by_id(plugin_id) {
                            let is_active = active.0 == Some(def.id);
                            if sidebar_icon(ui, def.glyph(), def.name, def.color(), is_active) {
                                active.0 = if is_active { None } else { Some(def.id) };
                                store_open.0 = false;
                            }
                            ui.add_space(2.0);
                        }
                    }
                }
            }

            // Push utility icons to bottom
            let remaining = ui.available_height();
            if remaining > 80.0 {
                ui.add_space(remaining - 80.0);
            }

            // Plugin store button
            let was_open = store_open.0;
            if sidebar_icon(ui, "⊕", "Plugin Store", a(GOLD, 160), was_open) {
                store_open.0 = !was_open;
                if store_open.0 { active.0 = None; }
            }
            ui.add_space(4.0);

            // Settings (stub)
            sidebar_icon(ui, "⚙", "Settings", a(FG_3, 180), false);
        });

    // ── Main content area ─────────────────────────────────────────────────────

    egui::CentralPanel::default()
        .frame(Frame::none().fill(VOID).inner_margin(Margin::same(32.0)))
        .show(ctx, |ui| {
            if store_open.0 {
                draw_plugin_store(ui, vault_id, &mut vault_store, &registry, &mut active, &mut store_open);
                return;
            }

            match active.0.and_then(|id| registry.by_id(id)) {
                Some(def) => {
                    match def.id {
                        "theatre.mixer" => draw_channel_mixer(ui, def.color(), &tmx, &core_status),
                        "theatre.stage" => draw_theatre_stage(ui, def.color()),
                        "genesis.forge" => draw_plugin_forge(ui, def.color()),
                        _ => draw_plugin_stub(ui, def),
                    }
                }
                None => {
                    // Tab bar across the top of the content area, then routed content.
                    draw_tab_bar(ui, &mut *vault_tab, vault_col);
                    match *vault_tab {
                        VaultTab::Overview   => draw_vault_home(ui, vault_id, &mut vault_store, vault_col),
                        VaultTab::Souls      => draw_souls_tab(ui, vault_col),
                        VaultTab::Containers => draw_containers_tab(ui, vault_col),
                        VaultTab::Assets     => draw_assets_tab(ui, vault_col),
                        VaultTab::Archive    => draw_archive_tab(ui, vault_col),
                    }
                }
            }
        });
}

// ── Vault home (no plugin selected) ──────────────────────────────────────────

fn draw_vault_home(
    ui:          &mut egui::Ui,
    vault_id:    Option<uuid::Uuid>,
    vault_store: &mut VaultStore,
    col:         egui::Color32,
) {
    // Clone the vault data upfront so vault_store is free for mutable use later.
    let vault_opt = vault_id.and_then(|id| vault_store.by_id(id)).cloned();
    if let Some(vault) = vault_opt {
        ui.label(egui::RichText::new(&vault.name).color(col).size(26.0));
        ui.add_space(4.0);
        ui.label(egui::RichText::new(vault.vault_type.tagline()).color(FG_3).size(12.0));
        ui.add_space(2.0);
        ui.label(egui::RichText::new(format!("BASE RESONANCE  {:.1} Hz", vault.resonance_hz))
            .color(a(col, 55)).size(9.0).monospace());
        ui.add_space(16.0);

        if !vault.description.is_empty() {
            ui.label(egui::RichText::new(&vault.description)
                .color(a(egui::Color32::WHITE, 160)).size(13.0));
            ui.add_space(24.0);
        }

        // Plugin summary row
        ui.label(egui::RichText::new(
            format!("{}  PLUGINS INSTALLED", vault.plugins.len()))
            .color(a(col, 120)).size(10.0).monospace());
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Select a plugin from the sidebar, or ⊕ to browse the store.")
            .color(FG_MUTED).size(11.0));

        // ── B-DNA display ─────────────────────────────────────────────────
        if !vault.bdna_signature.is_empty() {
            ui.add_space(28.0);
            ui.label(egui::RichText::new("B-DNA LINEAGE")
                .color(a(col, 80)).size(9.0).monospace());
            ui.add_space(4.0);

            let bits = bdna_to_bits(&vault.bdna_signature);
            let dot_size = 6.0_f32;
            let gap      = 3.0_f32;
            let cols     = 16_usize;
            let rows     = 4_usize;

            let total_w  = cols as f32 * dot_size + (cols - 1) as f32 * gap;
            let (rect, _) = ui.allocate_exact_size(
                vec2(total_w, rows as f32 * dot_size + (rows - 1) as f32 * gap),
                Sense::hover(),
            );
            let p = ui.painter_at(rect);

            for row in 0..rows {
                for col_i in 0..cols {
                    let bit_idx = row * cols + col_i;
                    let on = bits[bit_idx];
                    let cx = rect.left() + col_i as f32 * (dot_size + gap) + dot_size * 0.5;
                    let cy = rect.top()  + row    as f32 * (dot_size + gap) + dot_size * 0.5;
                    let alpha = if on { 200_u8 } else { 28 };
                    p.circle_filled(pos2(cx, cy), dot_size * 0.5, a(col, alpha));
                }
            }

            ui.add_space(6.0);
            let sig_short = &vault.bdna_signature[..16.min(vault.bdna_signature.len())];
            ui.label(egui::RichText::new(format!("SIG: {sig_short}…"))
                .color(a(col, 60)).size(8.0).monospace());
        }

        // ── Export ────────────────────────────────────────────────────────
        ui.add_space(24.0);
        let export_btn = egui::Button::new(
            egui::RichText::new("⇟  Export vault").color(a(col, 160)).size(11.0))
            .fill(a(col, 10))
            .stroke(egui::Stroke::new(1.0, a(col, 50)))
            .rounding(egui::Rounding::same(4.0));

        if ui.add(export_btn).clicked() {
            if let Some(id) = vault_id {
                if let Ok(json) = vault_store.export_vault(id) {
                    let filename = format!("{}.qgenesis",
                        vault.name.to_lowercase().replace(' ', "_"));
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&filename)
                        .add_filter("Genesis Capsule", &["qgenesis"])
                        .add_filter("JSON", &["json"])
                        .save_file()
                    {
                        if let Err(e) = std::fs::write(&path, &json) {
                            bevy::log::warn!("Export failed: {e}");
                        }
                    }
                }
            }
        }
    } else {
        ui.label(egui::RichText::new("No vault selected").color(FG_MUTED));
    }
}

// ── Tab bar ───────────────────────────────────────────────────────────────────

fn draw_tab_bar(ui: &mut egui::Ui, tab: &mut VaultTab, col: egui::Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for &variant in VAULT_TABS {
            let label    = variant.label();
            let is_active = *tab == variant;
            // Approximate monospace char width at 9pt ≈ 6px + 22px padding
            let tab_w    = label.len() as f32 * 6.0 + 22.0;
            let (rect, resp) = ui.allocate_exact_size(vec2(tab_w, 26.0), Sense::click());
            let p = ui.painter_at(rect);

            if is_active {
                p.rect_filled(rect, Rounding::same(3.0), a(col, 22));
                p.rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0, a(col, 60)));
                // Accent bar at bottom edge of active tab
                p.line_segment(
                    [pos2(rect.left() + 4.0, rect.bottom() - 0.5),
                     pos2(rect.right() - 4.0, rect.bottom() - 0.5)],
                    Stroke::new(2.0, a(col, 200)),
                );
            } else if resp.hovered() {
                p.rect_filled(rect, Rounding::same(3.0), a(col, 10));
            }

            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::monospace(9.0),
                if is_active { a(col, 230) } else { a(FG_3, 150) },
            );

            if resp.clicked() { *tab = variant; }
        }
    });
    ui.add_space(6.0);

    // Full-width separator below the tab row
    let (line_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [pos2(line_rect.left(), line_rect.center().y),
         pos2(line_rect.right(), line_rect.center().y)],
        Stroke::new(0.5, a(col, 25)),
    );
    ui.add_space(18.0);
}

// ── Stub tab content panels ───────────────────────────────────────────────────

fn draw_souls_tab(ui: &mut egui::Ui, col: egui::Color32) {
    tab_placeholder(
        ui, col, "◈", "SOULS",
        "Soul records and consciousness states will appear here\nonce Genesis is connected to this vault.",
    );
}

fn draw_containers_tab(ui: &mut egui::Ui, col: egui::Color32) {
    tab_placeholder(
        ui, col, "⬡", "CONTAINERS",
        "Container hierarchy and capsule management arrive in Phase 6\nwhen the Mythos Container protocol is implemented.",
    );
}

fn draw_assets_tab(ui: &mut egui::Ui, col: egui::Color32) {
    tab_placeholder(
        ui, col, "⬢", "ASSETS",
        "Asset-Forge manifests and compiled output will be indexed here\nonce the forge pipeline is connected.",
    );
}

fn draw_archive_tab(ui: &mut egui::Ui, col: egui::Color32) {
    tab_placeholder(
        ui, col, "◇", "ARCHIVE",
        "Sealed snapshots and canon event records from the Quantum Quill\nwill accumulate here during live Genesis sessions.",
    );
}

fn tab_placeholder(ui: &mut egui::Ui, col: egui::Color32, glyph: &str, label: &str, description: &str) {
    let w = ui.available_width().min(520.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, 160.0), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, Rounding::same(8.0), a(col, 5));
    dashed_rect(&p, rect, Stroke::new(1.0, a(col, 20)));

    p.text(
        pos2(rect.center().x, rect.center().y - 26.0),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(26.0),
        a(col, 55),
    );
    p.text(
        pos2(rect.center().x, rect.center().y + 4.0),
        Align2::CENTER_CENTER,
        label,
        FontId::monospace(9.0),
        a(col, 70),
    );
    // Description — two lines
    for (i, line) in description.lines().enumerate() {
        p.text(
            pos2(rect.center().x, rect.center().y + 22.0 + i as f32 * 14.0),
            Align2::CENTER_CENTER,
            line,
            FontId::proportional(10.0),
            a(egui::Color32::WHITE, 55),
        );
    }
}

// ── Plugin store overlay ──────────────────────────────────────────────────────

fn draw_plugin_store(
    ui:         &mut egui::Ui,
    vault_id:   Option<uuid::Uuid>,
    vault_store: &mut VaultStore,
    registry:   &PluginRegistry,
    _active:     &mut ActivePlugin,
    _store_open: &mut PluginStoreOpen,
) {
    ui.label(egui::RichText::new("⊕  PLUGIN STORE")
        .color(a(GOLD, 200)).size(18.0));
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Add capabilities to this vault.")
        .color(FG_3).size(11.0));
    ui.add_space(20.0);

    let installed: Vec<String> = vault_id
        .and_then(|id| vault_store.by_id(id))
        .map(|v| v.plugins.clone())
        .unwrap_or_default();

    egui::ScrollArea::vertical().show(ui, |ui| {
        let available = ui.available_width();
        let card_w    = ((available - 16.0) / 2.0).min(280.0);
        let card_h    = 88.0_f32;
        let gap       = 10.0_f32;

        let plugins = registry.plugins;
        let mut row_start = 0;

        while row_start < plugins.len() {
            ui.horizontal(|ui| {
                for i in row_start..(row_start + 2).min(plugins.len()) {
                    let def        = &plugins[i];
                    let is_installed = installed.contains(&def.id.to_string());

                    let (rect, _) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::hover());
                    let col       = def.color();

                    let p = ui.painter_at(rect);
                    p.rect_filled(rect, Rounding::same(6.0), CARD_BG);
                    p.rect_stroke(rect, Rounding::same(6.0),
                        Stroke::new(1.0, a(col, if is_installed { 80 } else { 30 })));

                    // Glyph
                    p.text(pos2(rect.left() + 22.0, rect.top() + 24.0),
                        Align2::CENTER_CENTER, def.glyph(),
                        FontId::proportional(16.0), col);

                    // Name + module
                    p.text(pos2(rect.left() + 40.0, rect.top() + 18.0),
                        Align2::LEFT_CENTER, def.name,
                        FontId::proportional(13.0), a(egui::Color32::WHITE, 200));
                    p.text(pos2(rect.left() + 40.0, rect.top() + 34.0),
                        Align2::LEFT_CENTER, def.module.label(),
                        FontId::monospace(8.0), a(col, 140));

                    // Description
                    p.text(pos2(rect.left() + 12.0, rect.top() + 54.0),
                        Align2::LEFT_CENTER, def.description,
                        FontId::proportional(10.0), a(egui::Color32::WHITE, 100));

                    // Install / Installed button
                    let btn_w   = 70.0_f32;
                    let btn_h   = 22.0_f32;
                    let btn_r   = egui::Rect::from_min_size(
                        pos2(rect.right() - btn_w - 8.0, rect.top() + 10.0),
                        vec2(btn_w, btn_h),
                    );
                    let btn_id  = egui::Id::new(("store_btn", def.id));
                    let btn_rsp = ui.interact(btn_r, btn_id, Sense::click());
                    let btn_hov = btn_rsp.hovered() && !is_installed;

                    let (btn_lbl, btn_col_a, btn_fill_a) = if is_installed {
                        ("✓  INSTALLED", 120_u8, 12_u8)
                    } else if btn_hov {
                        ("+ INSTALL", 220_u8, 35_u8)
                    } else {
                        ("+ INSTALL", 160_u8, 18_u8)
                    };

                    p.rect_filled(btn_r, Rounding::same(3.0), a(col, btn_fill_a));
                    p.rect_stroke(btn_r, Rounding::same(3.0),
                        Stroke::new(1.0, a(col, if is_installed { 40 } else { btn_col_a })));
                    p.text(btn_r.center(), Align2::CENTER_CENTER, btn_lbl,
                        FontId::monospace(8.0), a(col, btn_col_a));

                    if btn_rsp.clicked() && !is_installed {
                        if let Some(id) = vault_id {
                            if let Some(vault) = vault_store.vaults.iter_mut().find(|v| v.id == id) {
                                vault.plugins.push(def.id.to_string());
                            }
                        }
                    }

                    if i + 1 < (row_start + 2).min(plugins.len()) {
                        ui.add_space(gap);
                    }
                }
            });
            ui.add_space(gap);
            row_start += 2;
        }
    });
}

// ── Sidebar icon button ───────────────────────────────────────────────────────

fn sidebar_icon(
    ui:     &mut egui::Ui,
    glyph:  &str,
    label:  &str,
    col:    egui::Color32,
    active: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(60.0, 46.0), Sense::click());
    if !ui.is_rect_visible(rect) { return false; }

    let hovered = response.hovered();
    let p       = ui.painter_at(rect);

    if active {
        p.rect_filled(rect, Rounding::ZERO, a(col, 28));
        p.line_segment(
            [pos2(rect.left(), rect.top()), pos2(rect.left(), rect.bottom())],
            Stroke::new(2.5, col),
        );
    } else if hovered {
        p.rect_filled(rect, Rounding::ZERO, a(col, 14));
    }

    let glyph_a = if active { 240_u8 } else if hovered { 200 } else { 100 };
    p.text(rect.center(), Align2::CENTER_CENTER,
        glyph, FontId::proportional(18.0), a(col, glyph_a));

    response.on_hover_text(label).clicked()
}

// ── Plugin content implementations ───────────────────────────────────────────

fn draw_plugin_stub(ui: &mut egui::Ui, def: &crate::plugin_registry::PluginDef) {
    let col = def.color();
    ui.label(egui::RichText::new(def.module.glyph()).color(col).size(32.0));
    ui.add_space(6.0);
    ui.label(egui::RichText::new(def.name).color(col).size(22.0));
    ui.add_space(4.0);
    ui.label(egui::RichText::new(def.description).color(FG_3).size(12.0));
    ui.add_space(32.0);
    let w = ui.available_width().min(560.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, 220.0), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, Rounding::same(8.0), a(col, 6));
    dashed_rect(&p, rect, Stroke::new(1.0, a(col, 30)));
    p.text(rect.center(), Align2::CENTER_CENTER,
        "Plugin content loading…", FontId::proportional(13.0), a(col, 80));
}

// ── Channel Mixer ─────────────────────────────────────────────────────────────

fn draw_channel_mixer(
    ui:   &mut egui::Ui,
    col:  egui::Color32,
    tmx:  &TheatreMixerState,
    core: &CoreStatus,
) {
    // Header row with live beat/tempo from myth-core
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("♪  CHANNEL MIXER")
            .color(col).size(18.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            // Core clock status
            let (core_col, core_lbl) = if core.is_online() {
                (egui::Color32::from_rgb(0, 192, 96), "CORE ●")
            } else {
                (a(col, 55), "CORE ○")
            };
            ui.label(egui::RichText::new(core_lbl).color(core_col).size(9.0).monospace());
            ui.add_space(12.0);
            ui.label(egui::RichText::new(
                format!("{:.1} BPM  BEAT {:.2}", tmx.frame.tempo_bpm, tmx.frame.beat))
                .color(a(col, 120)).size(9.0).monospace());
            ui.add_space(12.0);
            ui.label(egui::RichText::new(format!("TICK {}", tmx.frame.tick))
                .color(a(col, 60)).size(9.0).monospace());
        });
    });
    ui.add_space(2.0);

    // Beat progress bar
    let bar_w = ui.available_width();
    let (bar_r, _) = ui.allocate_exact_size(vec2(bar_w, 3.0), Sense::hover());
    let fill_w = bar_r.width() * tmx.frame.beat;
    let fill_r = egui::Rect::from_min_size(bar_r.min, vec2(fill_w, bar_r.height()));
    ui.painter_at(bar_r).rect_filled(bar_r, Rounding::ZERO, a(col, 14));
    ui.painter_at(fill_r).rect_filled(fill_r, Rounding::ZERO, a(col, 180));

    ui.add_space(12.0);

    let channels = tmx.mixer.all_channels();
    let n        = channels.len();
    if n == 0 {
        ui.label(egui::RichText::new("No channels loaded.").color(FG_MUTED).size(11.0));
        return;
    }

    let strip_w = (ui.available_width() / n as f32).min(92.0).floor();
    let strip_h = 280.0_f32;
    let gap     = 4.0_f32;
    let total_w = strip_w * n as f32 + gap * (n as f32 - 1.0);

    let (master_rect, _) = ui.allocate_exact_size(vec2(total_w, strip_h), Sense::hover());

    for (i, ch) in channels.iter().enumerate() {
        let x  = master_rect.left() + i as f32 * (strip_w + gap);
        let sr = egui::Rect::from_min_size(pos2(x, master_rect.top()), vec2(strip_w, strip_h));
        draw_mixer_strip(ui, sr, i, ch.name.as_str(), ch.layer_type.tag(),
            ch.level, ch.muted, ch.glyph.is_some(), col);
    }

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
        ui.painter_at(r).circle_filled(r.center(), 5.0, egui::Color32::from_rgb(0, 192, 96));
        ui.label(egui::RichText::new(
            format!("{}/{} CHANNELS", n, tmx.mixer.capacity()))
            .color(a(col, 100)).size(9.0).monospace());
        ui.separator();
        ui.label(egui::RichText::new("EXPAND →32")
            .color(a(col, 55)).size(9.0).monospace());
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_mixer_strip(
    ui:         &mut egui::Ui,
    rect:       egui::Rect,
    ch_idx:     usize,
    name:       &str,
    layer_tag:  &str,
    level:      f32,
    muted:      bool,
    has_glyph:  bool,
    accent:     egui::Color32,
) {
    let p = ui.painter_at(rect);
    let strip_bg = egui::Color32::from_rgb(11, 14, 22);
    p.rect_filled(rect, Rounding::same(4.0), strip_bg);
    p.rect_stroke(rect, Rounding::same(4.0), Stroke::new(1.0, a(accent, 18)));

    // Header — channel number
    let hdr_h = 22.0_f32;
    let hdr   = egui::Rect::from_min_size(rect.min, vec2(rect.width(), hdr_h));
    let hdr_fill = if muted { a(egui::Color32::from_rgb(220, 60, 60), 20) } else { a(accent, 14) };
    p.rect_filled(hdr, Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 }, hdr_fill);
    p.text(hdr.center(), Align2::CENTER_CENTER,
        &format!("{:02}", ch_idx + 1),
        FontId::monospace(9.0), a(accent, if muted { 60 } else { 180 }));

    // Channel name (truncated)
    let name_display: String = name.chars().take(6).collect();
    p.text(pos2(rect.center().x, rect.top() + hdr_h + 10.0), Align2::CENTER_CENTER,
        &name_display, FontId::proportional(8.0), a(accent, 100));

    // Layer type tag
    let tag_r = egui::Rect::from_center_size(
        pos2(rect.center().x, rect.top() + hdr_h + 26.0),
        vec2(28.0, 14.0),
    );
    p.rect_filled(tag_r, Rounding::same(2.0), a(accent, 10));
    p.rect_stroke(tag_r, Rounding::same(2.0), Stroke::new(0.8, a(accent, 40)));
    p.text(tag_r.center(), Align2::CENTER_CENTER,
        layer_tag, FontId::monospace(7.0), a(accent, 140));

    // Glyph drop zone — lit if glyph is loaded
    let glyph_r = egui::Rect::from_center_size(
        pos2(rect.center().x, rect.top() + hdr_h + 56.0),
        vec2(rect.width() - 10.0, 40.0),
    );
    let glyph_col = if has_glyph { a(accent, 120) } else { a(accent, 22) };
    dashed_rect(&p, glyph_r, Stroke::new(0.8, glyph_col));
    p.text(glyph_r.center(), Align2::CENTER_CENTER,
        if has_glyph { "◈" } else { "◇" },
        FontId::proportional(14.0), a(accent, if has_glyph { 180 } else { 35 }));

    // Fader track
    let fader_top = glyph_r.bottom() + 8.0;
    let fader_bot = rect.bottom() - 34.0;
    let fader_h   = (fader_bot - fader_top).max(20.0);
    let fader_cx  = rect.center().x;
    let track_r   = egui::Rect::from_center_size(
        pos2(fader_cx, fader_top + fader_h * 0.5),
        vec2(4.0, fader_h),
    );
    p.rect_filled(track_r, Rounding::same(2.0), a(accent, 18));

    // Fader fill (level indicator)
    let fill_h = fader_h * level;
    let fill_r = egui::Rect::from_min_size(
        pos2(track_r.left(), fader_bot - fill_h),
        vec2(4.0, fill_h),
    );
    p.rect_filled(fill_r, Rounding::same(2.0), a(accent, 100));

    // Fader knob at current level position
    let knob_y = fader_bot - fader_h * level;
    let knob_r = egui::Rect::from_center_size(pos2(fader_cx, knob_y), vec2(20.0, 8.0));
    p.rect_filled(knob_r, Rounding::same(3.0), a(accent, if muted { 40 } else { 160 }));
    p.rect_stroke(knob_r, Rounding::same(3.0), Stroke::new(0.5, a(accent, if muted { 30 } else { 255 })));

    // Level readout
    p.text(pos2(fader_cx, fader_bot + 8.0), Align2::CENTER_CENTER,
        &format!("{:.1}", level),
        FontId::monospace(7.0), a(accent, 80));

    // Active / mute dot
    let dot_y = rect.bottom() - 12.0;
    let active = !muted && level > 0.0;
    p.circle_filled(pos2(fader_cx, dot_y), 4.0,
        if muted       { egui::Color32::from_rgb(220, 60, 60) }
        else if active { egui::Color32::from_rgb(0, 192, 96) }
        else           { a(accent, 30) });
}

// ── Theatre Stage ─────────────────────────────────────────────────────────────

fn draw_theatre_stage(ui: &mut egui::Ui, col: egui::Color32) {
    ui.label(egui::RichText::new("◇  MASTER STAGE").color(col).size(18.0).strong());
    ui.add_space(2.0);
    ui.label(egui::RichText::new("BioSpark Theatre — composite renderer")
        .color(FG_3).size(10.0).monospace());
    ui.add_space(16.0);

    // Canvas preview area
    let canvas_w = ui.available_width().min(640.0);
    let canvas_h = canvas_w * 9.0 / 16.0;
    let (rect, _) = ui.allocate_exact_size(vec2(canvas_w, canvas_h), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, Rounding::same(6.0), egui::Color32::from_rgb(4, 6, 12));
    p.rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0, a(col, 40)));

    // Grid overlay (12 cols)
    let col_w = canvas_w / 12.0;
    for i in 1..12 {
        let x = rect.left() + col_w * i as f32;
        p.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(0.5, a(col, 8)),
        );
    }

    // Center placeholder text
    p.text(rect.center() - vec2(0.0, 12.0), Align2::CENTER_CENTER,
        "◇", FontId::proportional(36.0), a(col, 30));
    p.text(rect.center() + vec2(0.0, 16.0), Align2::CENTER_CENTER,
        "STAGE CANVAS  —  Phase 2", FontId::monospace(9.0), a(col, 40));

    ui.add_space(10.0);

    // Layout blueprint picker (static labels for now)
    ui.label(egui::RichText::new("LAYOUT BLUEPRINT").color(a(col, 80)).size(9.0).monospace());
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        let blueprints = [
            "FULLSCREEN", "PANCAKE", "SIDEBAR-L",
            "HOLY-GRAIL", "12-COL", "SPLIT",
            "MASONRY", "CENTER", "STICKY-FT",
        ];
        for bp in blueprints {
            let is_active = bp == "FULLSCREEN";
            let btn_col = if is_active { col } else { a(col, 50) };
            let btn_fill = if is_active { a(col, 18) } else { a(col, 5) };
            let btn = egui::Button::new(
                egui::RichText::new(bp).color(btn_col).size(8.0).monospace())
                .fill(btn_fill)
                .stroke(Stroke::new(0.8, a(col, if is_active { 80 } else { 25 })))
                .rounding(Rounding::same(3.0));
            ui.add(btn);
        }
    });
}

// ── Plugin Forge ──────────────────────────────────────────────────────────────

fn draw_plugin_forge(ui: &mut egui::Ui, col: egui::Color32) {
    ui.label(egui::RichText::new("⬡  PLUGIN FORGE").color(col).size(18.0).strong());
    ui.add_space(2.0);
    ui.label(egui::RichText::new("Build, upload, and remix .qgcp / .qgenesis files")
        .color(FG_3).size(10.0).monospace());
    ui.add_space(20.0);

    // File type cards
    let card_data = [
        ("⬡", ".qgenesis", "Genesis Capsule — vault identity, plugins, B-DNA lineage", col),
        ("◈", ".qgcp",     "User World File — scenes, settings, loom state",
            egui::Color32::from_rgb(220, 60, 120)),
    ];

    for (glyph, ext, desc, card_col) in card_data {
        let available = ui.available_width().min(560.0);
        let (rect, _) = ui.allocate_exact_size(vec2(available, 64.0), Sense::hover());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, Rounding::same(6.0), a(card_col, 8));
        p.rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0, a(card_col, 35)));
        p.text(pos2(rect.left() + 24.0, rect.center().y - 4.0), Align2::CENTER_CENTER,
            glyph, FontId::proportional(18.0), card_col);
        p.text(pos2(rect.left() + 52.0, rect.center().y - 8.0), Align2::LEFT_CENTER,
            ext, FontId::monospace(12.0), a(card_col, 200));
        p.text(pos2(rect.left() + 52.0, rect.center().y + 10.0), Align2::LEFT_CENTER,
            desc, FontId::proportional(10.0), a(egui::Color32::WHITE, 100));

        // Upload button
        let btn_w = 72.0_f32;
        let btn_r = egui::Rect::from_center_size(
            pos2(rect.right() - btn_w * 0.5 - 10.0, rect.center().y),
            vec2(btn_w, 26.0),
        );
        let btn_id = egui::Id::new(("forge_upload", ext));
        let btn_rsp = ui.interact(btn_r, btn_id, Sense::click());
        let hov = btn_rsp.hovered();
        p.rect_filled(btn_r, Rounding::same(3.0), a(card_col, if hov { 30 } else { 14 }));
        p.rect_stroke(btn_r, Rounding::same(3.0), Stroke::new(0.8, a(card_col, if hov { 180 } else { 60 })));
        p.text(btn_r.center(), Align2::CENTER_CENTER,
            "⇡  UPLOAD", FontId::monospace(8.0), a(card_col, if hov { 220 } else { 140 }));

        ui.add_space(6.0);
    }

    ui.add_space(16.0);
    ui.label(egui::RichText::new("CREATE NEW").color(a(col, 80)).size(9.0).monospace());
    ui.add_space(6.0);

    let new_forge_w = ui.available_width().min(560.0);
    let (r, _) = ui.allocate_exact_size(vec2(new_forge_w, 44.0), Sense::hover());
    let fp = ui.painter_at(r);
    fp.rect_filled(r, Rounding::same(6.0), a(col, 6));
    dashed_rect(&fp, r, Stroke::new(0.8, a(col, 22)));
    fp.text(r.center(), Align2::CENTER_CENTER,
        "+ Create Plugin Blueprint",
        FontId::proportional(12.0), a(col, 60));
}

// ── Dashed rect helper ────────────────────────────────────────────────────────

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
