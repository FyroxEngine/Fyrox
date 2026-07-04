// Permanent shell — outer Mythos ring + state-specific inner Vault ring.
// Runs in every non-Splash state so panel ordering is always consistent:
//   mythos_top → mythos_bottom → vault_top → vault_bottom
// Page content (SidePanel / CentralPanel) is added by each state's own system
// AFTER this one via UiSet ordering.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Align, Frame, Layout, Margin, RichText, Stroke};

use crate::core_status::CoreStatus;
use crate::state::AppScreen;
use crate::vault_store::{SelectedVault, SetupDraft, VaultStore};
use super::UiSet;
use super::theme::{self, a, GOLD, GOLD_LT, GOLD_DK, VOID, ABYSS, RAIL_BG, FG_3, FG_MUTED, TEAL, GREEN};

// The name "The Great Library" is internal lore — never shown in the shell UI.
// Externally this is MYTHOS: the Quantum OS.

pub struct ShellPlugin;

impl Plugin for ShellPlugin {
    fn build(&self, app: &mut App) {
        // Shell is silent during Splash and GenesisBootSequence — those
        // states own the entire screen themselves.
        app.add_systems(
            Update,
            draw_shell
                .run_if(
                    in_state(AppScreen::Landing)
                    .or_else(in_state(AppScreen::VaultView))
                    .or_else(in_state(AppScreen::VaultSetup))
                    .or_else(in_state(AppScreen::Error)),
                )
                .in_set(UiSet::Shell),
        );
    }
}

fn draw_shell(
    mut contexts: EguiContexts,
    state:       Res<State<AppScreen>>,
    selected:    Res<SelectedVault>,
    store:       Res<VaultStore>,
    draft:       Res<SetupDraft>,
    core_status: Res<CoreStatus>,
    mut next:    ResMut<NextState<AppScreen>>,
) {
    let ctx = contexts.ctx_mut();
    theme::apply(ctx);

    // ── Outer Mythos ring — always visible ───────────────────────────────────

    egui::TopBottomPanel::top("mythos_top")
        .exact_height(40.0)
        .frame(Frame::none()
            .fill(RAIL_BG)
            .inner_margin(Margin::symmetric(16.0, 0.0))
            .stroke(Stroke::new(1.0, a(GOLD_DK, 120))))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("⬡").color(a(GOLD, 180)).size(14.0));
                ui.add_space(6.0);
                ui.label(RichText::new("MYTHOS")
                    .color(GOLD).size(13.0).extra_letter_spacing(2.5));
                ui.add_space(4.0);
                ui.label(RichText::new("QUANTUM OS")
                    .color(a(GOLD, 55)).size(9.0).monospace().extra_letter_spacing(1.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new("v0.1.0").color(FG_3).size(9.0).monospace());
                    ui.add_space(12.0);
                    pip(ui, GREEN, "ONLINE");
                    ui.add_space(8.0);
                    let core_col = if core_status.is_online() { TEAL } else { a(TEAL, 55) };
                    pip(ui, core_col, "CORE");
                    ui.add_space(16.0);
                    let gen_btn = egui::Button::new(
                        RichText::new("⬡ Genesis").color(a(GOLD, 200)).size(10.0))
                        .fill(a(GOLD, 14))
                        .stroke(egui::Stroke::new(1.0, a(GOLD, 80)))
                        .rounding(egui::Rounding::same(3.0));
                    if ui.add(gen_btn).clicked() {
                        launch_genesis();
                    }
                });
            });
        });

    egui::TopBottomPanel::bottom("mythos_bottom")
        .exact_height(26.0)
        .frame(Frame::none()
            .fill(RAIL_BG)
            .inner_margin(Margin::symmetric(16.0, 0.0))
            .stroke(Stroke::new(1.0, a(GOLD_DK, 100))))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("MYTHOS  ·  Everything is a Vault  ·  Everything is a Plugin")
                        .color(a(GOLD, 45)).size(9.0).monospace());
                });
            });
        });

    // ── Inner Vault ring — routed by state ───────────────────────────────────

    match state.get() {
        AppScreen::Landing => {
            draw_landing_inner(ctx, &*store, &mut *next);
        }
        AppScreen::VaultView => {
            draw_vault_view_inner(ctx, &*selected, &*store, &mut *next);
        }
        AppScreen::VaultSetup => {
            draw_vault_setup_inner(ctx, &*draft);
        }
        AppScreen::Error => {
            draw_error_inner(ctx, &mut *next);
        }
        AppScreen::Splash | AppScreen::GenesisBootSequence => {}
    }
}

// ── Landing inner shell ───────────────────────────────────────────────────────

fn draw_landing_inner(ctx: &egui::Context, store: &VaultStore, next: &mut NextState<AppScreen>) {
    egui::TopBottomPanel::top("vault_top")
        .exact_height(46.0)
        .frame(Frame::none()
            .fill(ABYSS)
            .inner_margin(Margin::symmetric(20.0, 0.0))
            .stroke(Stroke::new(1.0, a(GOLD, 40))))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("◈").color(a(GOLD, 160)).size(14.0));
                ui.add_space(8.0);
                ui.label(RichText::new("VAULTS")
                    .color(GOLD_LT).size(15.0).extra_letter_spacing(1.5));
                ui.add_space(12.0);
                ui.label(RichText::new(
                    format!("{} vault{}", store.vaults.len(),
                        if store.vaults.len() == 1 { "" } else { "s" }))
                    .color(FG_MUTED).size(9.0).monospace());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    let btn = egui::Button::new(
                        RichText::new("+ New Vault").color(GOLD).size(11.0))
                        .fill(a(GOLD, 18))
                        .stroke(Stroke::new(1.0, a(GOLD, 100)))
                        .rounding(egui::Rounding::same(4.0));
                    if ui.add(btn).clicked() {
                        next.set(AppScreen::VaultSetup);
                    }
                });
            });
        });

    egui::TopBottomPanel::bottom("vault_bottom")
        .exact_height(30.0)
        .frame(Frame::none()
            .fill(VOID)
            .inner_margin(Margin::symmetric(20.0, 0.0))
            .stroke(Stroke::new(1.0, a(GOLD, 28))))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                // Dynamic pips from the actual vault store — first 5 active vaults
                for vault in store.vaults.iter().filter(|v| {
                    v.status == crate::vault_store::VaultStatus::Active
                }).take(5) {
                    pip(ui, a(vault.color, 200), &vault.name);
                    ui.add_space(12.0);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Coherence: —")
                        .color(FG_MUTED).size(9.0).monospace());
                });
            });
        });
}

// ── Vault view inner shell ────────────────────────────────────────────────────

fn draw_vault_view_inner(
    ctx:      &egui::Context,
    selected: &SelectedVault,
    store:    &VaultStore,
    next:     &mut NextState<AppScreen>,
) {
    let vault = selected.0.and_then(|id| store.by_id(id));
    let name  = vault.map(|v| v.name.as_str()).unwrap_or("Unknown Vault");
    let col   = vault.map(|v| v.color).unwrap_or(GOLD);
    let vtype = vault.map(|v| v.vault_type.label()).unwrap_or("—");

    egui::TopBottomPanel::top("vault_top")
        .exact_height(46.0)
        .frame(Frame::none()
            .fill(ABYSS)
            .inner_margin(Margin::symmetric(20.0, 0.0))
            .stroke(Stroke::new(1.0, a(col, 60))))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("◈").color(col).size(14.0));
                ui.add_space(8.0);
                ui.label(RichText::new(name).color(a(col, 220)).size(15.0));
                ui.add_space(12.0);
                ui.label(RichText::new(vtype).color(FG_3).size(9.0).monospace());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    let back = egui::Button::new(
                        RichText::new("↩  MYTHOS").color(FG_3).size(11.0))
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0, a(GOLD, 50)))
                        .rounding(egui::Rounding::same(4.0));
                    if ui.add(back).clicked() {
                        next.set(AppScreen::Landing);
                    }
                });
            });
        });

    egui::TopBottomPanel::bottom("vault_bottom")
        .exact_height(30.0)
        .frame(Frame::none()
            .fill(VOID)
            .inner_margin(Margin::symmetric(20.0, 0.0))
            .stroke(Stroke::new(1.0, a(col, 22))))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let n = vault.map(|v| v.plugins.len()).unwrap_or(0);
                let plugin_label = if n == 0 {
                    "No plugins loaded".to_string()
                } else {
                    format!("{} plugin{} active", n, if n == 1 { "" } else { "s" })
                };
                pip(ui, a(col, 200), "ACTIVE");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(plugin_label).color(FG_MUTED).size(9.0).monospace());
                });
            });
        });
}

// ── Vault setup inner shell ───────────────────────────────────────────────────

fn draw_vault_setup_inner(ctx: &egui::Context, draft: &SetupDraft) {
    let step_label = format!("STEP {} OF 5", draft.step + 1);

    egui::TopBottomPanel::top("vault_top")
        .exact_height(46.0)
        .frame(Frame::none()
            .fill(ABYSS)
            .inner_margin(Margin::symmetric(20.0, 0.0))
            .stroke(Stroke::new(1.0, a(GOLD, 40))))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("◈").color(a(GOLD, 160)).size(14.0));
                ui.add_space(8.0);
                ui.label(RichText::new("NEW VAULT")
                    .color(GOLD_LT).size(15.0).extra_letter_spacing(1.2));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(&step_label).color(FG_3).size(9.0).monospace());
                });
            });
        });

    egui::TopBottomPanel::bottom("vault_bottom")
        .exact_height(30.0)
        .frame(Frame::none()
            .fill(VOID)
            .inner_margin(Margin::symmetric(20.0, 0.0))
            .stroke(Stroke::new(1.0, a(GOLD, 22))))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("Genesis Protocol  ·  Awaiting Seed")
                    .color(FG_MUTED).size(9.0).monospace());
            });
        });
}

// ── Error inner shell ─────────────────────────────────────────────────────────

fn draw_error_inner(ctx: &egui::Context, next: &mut NextState<AppScreen>) {
    egui::TopBottomPanel::top("vault_top")
        .exact_height(46.0)
        .frame(Frame::none()
            .fill(ABYSS)
            .inner_margin(Margin::symmetric(20.0, 0.0))
            .stroke(Stroke::new(1.0, a(egui::Color32::from_rgb(220, 60, 60), 80))))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("⚠  COHERENCE FAULT")
                    .color(egui::Color32::from_rgb(220, 80, 80)).size(15.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    let btn = egui::Button::new(
                        RichText::new("↩  MYTHOS").color(GOLD).size(11.0))
                        .fill(a(GOLD, 18))
                        .stroke(Stroke::new(1.0, a(GOLD, 100)))
                        .rounding(egui::Rounding::same(4.0));
                    if ui.add(btn).clicked() {
                        next.set(AppScreen::Landing);
                    }
                });
            });
        });

    egui::TopBottomPanel::bottom("vault_bottom")
        .exact_height(30.0)
        .frame(Frame::none()
            .fill(VOID)
            .inner_margin(Margin::symmetric(20.0, 0.0))
            .stroke(Stroke::new(1.0, a(egui::Color32::from_rgb(220, 60, 60), 28))))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("COHERENCE: 0%  ·  SYSTEM FAULT  ·  AWAITING RECOVERY")
                    .color(FG_MUTED).size(9.0).monospace());
            });
        });
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Spawn genesis as a detached child process using the workspace's `cargo run`.
/// Runs in a background OS thread so the library UI is never blocked.
fn launch_genesis() {
    let workspace = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    std::thread::spawn(move || {
        match std::process::Command::new("cargo")
            .args(["run", "-p", "genesis"])
            .current_dir(&workspace)
            .spawn()
        {
            Ok(mut child) => {
                bevy::log::info!("Genesis launched (pid {})", child.id());
                let _ = child.wait(); // reap when done; thread exits quietly
            }
            Err(e) => {
                bevy::log::warn!("Failed to launch Genesis: {e}");
            }
        }
    });
}

fn pip(ui: &mut egui::Ui, color: egui::Color32, label: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        let (r, _) = ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
        ui.painter().circle_filled(r.center(), 3.0, color);
        ui.label(RichText::new(label).color(FG_3).size(9.0).monospace());
    });
}
