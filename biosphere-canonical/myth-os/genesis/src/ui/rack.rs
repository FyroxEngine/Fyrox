// Axiom Signal Chain Rack — full UI layout.
// Layout: brand bar (top) · genesis map (left) · rack chassis (center) · mixer (bottom)

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiSet};
use egui::{Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use mythos::quantum_module::{Department, ImplementationStatus, Lifecycle};
use crate::{
    asset_registry::AssetRegistry,
    atoms::instinct::SoulStore,
    containers::{ContainerLoadChannel, ContainerStore, request_load_dialog},
    mixer::{BusChannel, MidiCcEvent},
    scanner::ModuleRegistry,
    ui::{theme, widgets::{self, ChannelStripState, JackKind}},
};
use std::collections::HashMap;

// ── GENESIS REGISTRY (from rack-primitives.jsx) ───────────────────────────────

#[derive(Clone)]
pub struct GenesisMod {
    pub n: &'static str,
    pub name: &'static str,
    pub wire: &'static str,
    pub color: Color32,
    pub built: bool,
}

#[derive(Clone)]
pub struct GenesisDept {
    pub id: &'static str,
    pub name: &'static str,
    pub color: Color32,
    pub mods: &'static [GenesisMod],
}

static WORLD_CONSTRUCTION_MODS: &[GenesisMod] = &[
    GenesisMod { n: "01", name: "Terrain",     wire: "SPA", color: Color32::from_rgb(30, 140, 255),  built: true  },
    GenesisMod { n: "02", name: "Environment", wire: "SPA", color: Color32::from_rgb(128, 80, 224),  built: true  },
    GenesisMod { n: "03", name: "Architect",   wire: "SPA", color: Color32::from_rgb(100, 180, 255), built: true  },
    GenesisMod { n: "04", name: "Lighting",    wire: "VIS", color: Color32::from_rgb(224, 216, 255), built: true  },
];

static ENTITY_SYSTEMS_MODS: &[GenesisMod] = &[
    GenesisMod { n: "05", name: "Modeling",     wire: "VIS", color: Color32::from_rgb(244, 192, 37),  built: true  },
    GenesisMod { n: "06", name: "Choreography", wire: "BHV", color: Color32::from_rgb(220, 60, 120),  built: true  },
    GenesisMod { n: "07", name: "Behavior",     wire: "BHV", color: Color32::from_rgb(144, 48, 208),  built: false },
    GenesisMod { n: "08", name: "Society",      wire: "SOC", color: Color32::from_rgb(200, 168, 96),  built: true  },
];

static NARRATIVE_SYSTEMS_MODS: &[GenesisMod] = &[
    GenesisMod { n: "09", name: "Sequencer", wire: "TMP", color: Color32::from_rgb(176, 128, 48),  built: true  },
    GenesisMod { n: "10", name: "Story",     wire: "NAR", color: Color32::from_rgb(140, 80, 255),  built: true  },
    GenesisMod { n: "11", name: "Memory",    wire: "NAR", color: Color32::from_rgb(0, 192, 96),    built: true  },
    GenesisMod { n: "12", name: "Sound",     wire: "AUD", color: Color32::from_rgb(220, 140, 30),  built: true  },
];

static PIPELINE_SYSTEMS_MODS: &[GenesisMod] = &[
    GenesisMod { n: "13", name: "Logic",      wire: "LGC", color: Color32::from_rgb(32, 200, 208),  built: false },
    GenesisMod { n: "14", name: "Simulation", wire: "DAT", color: Color32::from_rgb(48, 224, 96),   built: false },
    GenesisMod { n: "15", name: "Forge",      wire: "AST", color: Color32::from_rgb(255, 100, 0),   built: true  },
    GenesisMod { n: "16", name: "Network",    wire: "EVT", color: Color32::from_rgb(255, 255, 255),  built: true  },
];

fn genesis_depts() -> [GenesisDept; 4] {
    [
        GenesisDept { id: "I",   name: "WORLD CONSTRUCTION", color: theme::DEPT_I,   mods: WORLD_CONSTRUCTION_MODS  },
        GenesisDept { id: "II",  name: "ENTITY SYSTEMS",     color: theme::DEPT_II,  mods: ENTITY_SYSTEMS_MODS      },
        GenesisDept { id: "III", name: "NARRATIVE SYSTEMS",  color: theme::DEPT_III, mods: NARRATIVE_SYSTEMS_MODS   },
        GenesisDept { id: "IV",  name: "PIPELINE SYSTEMS",   color: theme::DEPT_IV,  mods: PIPELINE_SYSTEMS_MODS    },
    ]
}

// ── RACK STATE ────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct RackState {
    pub bus_levels: [f32; 4],
    pub crossfader: f32,
    pub selected_module: Option<String>,
    pub module_params: HashMap<String, HashMap<String, f32>>,
    pub hier_active: HierLevel,
    pub view_mode: ViewMode,
    pub channel_strips: [ChannelStripState; 4],
    pub module_channel_strips: HashMap<String, ChannelStripState>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HierLevel { Genesis, Mythos, Standard, Capsules }

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode { #[default] Rack, World }

impl Default for RackState {
    fn default() -> Self {
        Self {
            bus_levels: [0.75; 4],
            crossfader: 0.5,
            selected_module: None,
            module_params: HashMap::new(),
            hier_active: HierLevel::Mythos,
            view_mode: ViewMode::Rack,
            channel_strips: [
                ChannelStripState::new(2),
                ChannelStripState::new(2),
                ChannelStripState::new(2),
                ChannelStripState::new(2),
            ],
            module_channel_strips: HashMap::new(),
        }
    }
}

impl RackState {
    pub fn level(&self, bus: BusChannel) -> f32 { self.bus_levels[(bus as usize) - 1] }
    pub fn set_level(&mut self, bus: BusChannel, v: f32) { self.bus_levels[(bus as usize) - 1] = v.clamp(0.0, 1.0); }

    pub fn get_param(&self, module_id: &str, param: &str) -> f32 {
        self.module_params.get(module_id).and_then(|p| p.get(param)).copied().unwrap_or(0.0)
    }
    pub fn set_param(&mut self, module_id: &str, param: &str, v: f32) {
        self.module_params.entry(module_id.to_string()).or_default().insert(param.to_string(), v);
    }
}

// ── RACK WINDOW MARKER ────────────────────────────────────────────────────────

/// Marks the secondary OS window that hosts the rack / instrument UI.
/// The primary window is the 3-D world view.
/// Queried by `apply_theme` and `draw_rack` to target the correct egui context.
#[derive(Component)]
pub struct RackWindow;

// ── PLUGIN ────────────────────────────────────────────────────────────────────

pub struct RackUiPlugin;

impl Plugin for RackUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<RackState>()
            .add_systems(Startup, spawn_rack_window)
            .add_systems(
                Update,
                (apply_theme, draw_rack, sync_midi_to_rack)
                    .chain()
                    .after(EguiSet::InitContexts),
            );
    }
}

fn spawn_rack_window(mut commands: Commands) {
    commands.spawn((
        Window {
            title: "Genesis — Instruments".into(),
            ..Default::default()
        },
        RackWindow,
    ));
}

// ── SYSTEMS ───────────────────────────────────────────────────────────────────

fn apply_theme(
    mut contexts: EguiContexts,
    rack_win_q:   Query<Entity, With<RackWindow>>,
) {
    let Ok(rack_entity) = rack_win_q.get_single() else { return };
    if let Some(ctx) = contexts.try_ctx_for_window_mut(rack_entity) {
        theme::apply(ctx);
    }
}

fn sync_midi_to_rack(
    mut events: EventReader<MidiCcEvent>,
    mut rack: ResMut<RackState>,
    registry: Res<ModuleRegistry>,
) {
    for ev in events.read() {
        if let Some(bus) = BusChannel::from_traktor_channel(ev.channel + 1) {
            for module in registry.by_department(bus_to_dept(bus)) {
                for binding in &module.traktor_map {
                    if binding.midi_cc == ev.cc {
                        let norm = ev.normalized();
                        let scaled = binding.scale_min + norm * (binding.scale_max - binding.scale_min);
                        rack.set_param(&module.id, &binding.parameter, scaled);
                    }
                }
            }
        }
    }
}

fn draw_rack(
    mut contexts:  EguiContexts,
    rack_win_q:    Query<Entity, With<RackWindow>>,
    mut rack:      ResMut<RackState>,
    registry:      Res<ModuleRegistry>,
    assets:        Res<AssetRegistry>,
    souls:         Res<SoulStore>,
    store:         Res<ContainerStore>,
    channel:       Res<ContainerLoadChannel>,
) {
    let Ok(rack_entity) = rack_win_q.get_single() else { return };
    let Some(ctx) = contexts.try_ctx_for_window_mut(rack_entity) else { return };

    // ── Brand bar (top) ──────────────────────────────────────────────────────
    egui::TopBottomPanel::top("brand_bar")
        .frame(egui::Frame::none()
            .fill(theme::VOID)
            .inner_margin(egui::Margin::symmetric(20.0, 10.0)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Sigil glyph
                let (sr, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                if ui.is_rect_visible(sr) {
                    let p = ui.painter();
                    p.circle_stroke(sr.center(), 11.0, Stroke::new(1.2, theme::GOLD));
                    p.line_segment([Pos2::new(sr.center().x, sr.min.y + 4.0), Pos2::new(sr.center().x, sr.min.y + 10.0)], Stroke::new(1.0, theme::GOLD));
                    p.line_segment([Pos2::new(sr.center().x, sr.max.y - 4.0), Pos2::new(sr.center().x, sr.max.y - 10.0)], Stroke::new(1.0, theme::GOLD));
                    p.line_segment([Pos2::new(sr.min.x + 4.0, sr.center().y), Pos2::new(sr.min.x + 10.0, sr.center().y)], Stroke::new(1.0, theme::GOLD));
                    p.line_segment([Pos2::new(sr.max.x - 4.0, sr.center().y), Pos2::new(sr.max.x - 10.0, sr.center().y)], Stroke::new(1.0, theme::GOLD));
                    p.circle_filled(sr.center(), 3.0, theme::GOLD);
                }
                ui.add_space(10.0);

                // Wordmark
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("BIOSPARK STUDIOS")
                        .font(egui::FontId::proportional(13.0))
                        .color(theme::FG_1));
                    ui.label(egui::RichText::new("QUANTUM GENESIS ENGINE")
                        .font(egui::FontId::monospace(8.0))
                        .color(theme::GOLD));
                });

                ui.add_space(20.0);

                // HierCrumb breadcrumb
                hier_crumb(ui, rack.hier_active);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // LIVE chip
                    chip_live(ui);
                    ui.add_space(6.0);
                    // Module count chip
                    let total = registry.0.len();
                    let built = registry.0.iter().filter(|m| m.implementation_status == ImplementationStatus::Built).count();
                    chip(ui, &format!("{built}/{total} ONLINE"), theme::ASTRAL_CYAN);
                    ui.add_space(6.0);
                    chip(ui, "16 MODULES", theme::GOLD);
                    ui.add_space(12.0);
                    // ── View toggle ──────────────────────────────────────
                    view_toggle(ui, &mut rack.view_mode);
                });
            });
        });

    // ── World mode: minimal HUD overlay, skip all other panels ──────────────
    if rack.view_mode == ViewMode::World {
        // Small floating RACK button in the bottom-right corner
        egui::Area::new("world_hud".into())
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(72.0, 22.0), Sense::click());
                if resp.clicked() { rack.view_mode = ViewMode::Rack; }
                if ui.is_rect_visible(rect) {
                    let p = ui.painter();
                    let bg = if resp.hovered() {
                        Color32::from_rgba_unmultiplied(0, 229, 255, 30)
                    } else {
                        Color32::from_rgba_unmultiplied(3, 5, 10, 200)
                    };
                    p.rect_filled(rect, Rounding::same(3.0), bg);
                    p.rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0, theme::QUANTUM));
                    p.text(rect.center(), egui::Align2::CENTER_CENTER,
                        "▤  RACK", egui::FontId::monospace(9.0), theme::QUANTUM);
                }
            });
        return;
    }

    // ── Mixer strip (bottom) ─────────────────────────────────────────────────
    egui::TopBottomPanel::bottom("mixer_strip")
        .frame(egui::Frame::none()
            .fill(theme::ABYSS)
            .inner_margin(egui::Margin::symmetric(16.0, 8.0)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Label
                ui.vertical(|ui| {
                    ui.set_width(60.0);
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("TRAKTOR\nS4 MIXER")
                        .font(egui::FontId::monospace(7.0))
                        .color(theme::FG_MUTED));
                });

                ui.separator();

                let buses = [
                    (BusChannel::Structure,  "STRUCTURE",  theme::DEPT_I,   0),
                    (BusChannel::Entities,   "ENTITIES",   theme::DEPT_II,  1),
                    (BusChannel::Atmosphere, "ATMOSPHERE", theme::DEPT_III, 2),
                    (BusChannel::Dynamics,   "DYNAMICS",   theme::DEPT_IV,  3),
                ];

                for (bus, label, color, idx) in buses {
                    ui.vertical(|ui| {
                        ui.set_width(80.0);
                        ui.label(egui::RichText::new(label)
                            .font(egui::FontId::monospace(7.0))
                            .color(color));
                        ui.add_space(2.0);
                        let mut level = rack.level(bus);
                        widgets::fader_h(ui, &mut level, color, 80.0);
                        rack.set_level(bus, level);
                        // Channel strip pads
                        let cs = &mut rack.channel_strips[idx];
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
                            for (lbl, st) in [("M", &mut cs.mute), ("S", &mut cs.solo)] {
                                let bg = if *st { Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 50) } else { theme::SURFACE };
                                let tc = if *st { color } else { theme::FG_MUTED };
                                let (r, resp) = ui.allocate_exact_size(Vec2::new(14.0, 10.0), Sense::click());
                                if resp.clicked() { *st = !*st; }
                                if ui.is_rect_visible(r) {
                                    ui.painter().rect_filled(r, Rounding::same(1.0), bg);
                                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, lbl, egui::FontId::monospace(5.5), tc);
                                }
                            }
                        });
                    });
                    ui.add_space(4.0);
                }

                ui.separator();

                // Crossfader
                ui.vertical(|ui| {
                    ui.set_width(100.0);
                    ui.label(egui::RichText::new("CROSSFADER")
                        .font(egui::FontId::monospace(7.0))
                        .color(theme::FG_MUTED));
                    let mut cf = rack.crossfader;
                    widgets::fader_h(ui, &mut cf, theme::MYTHOS, 80.0);
                    rack.crossfader = cf;
                });
            });
        });

    // ── Genesis Map (left panel) ─────────────────────────────────────────────
    egui::SidePanel::left("genesis_map")
        .resizable(false)
        .exact_width(220.0)
        .frame(egui::Frame::none()
            .fill(theme::ABYSS)
            .inner_margin(egui::Margin::same(8.0)))
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("GENESIS · 16 MODULES")
                .font(egui::FontId::monospace(8.0))
                .color(theme::GOLD));
            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                for dept in genesis_depts() {
                    // Dept header
                    ui.horizontal(|ui| {
                        let (dr, _) = ui.allocate_exact_size(Vec2::new(3.0, 12.0), Sense::hover());
                        ui.painter().rect_filled(dr, Rounding::same(1.0), dept.color);
                        ui.label(egui::RichText::new(format!("DEPT {} · {}", dept.id, dept.name))
                            .font(egui::FontId::monospace(7.0))
                            .color(dept.color));
                    });
                    ui.add_space(3.0);

                    for m in dept.mods {
                        let is_selected = rack.selected_module.as_deref() == Some(m.n);
                        let bg = if is_selected {
                            Color32::from_rgba_unmultiplied(m.color.r(), m.color.g(), m.color.b(), 30)
                        } else {
                            theme::SURFACE
                        };
                        let border_color = if is_selected { m.color } else { theme::BORDER };

                        let (row_rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::click());
                        if resp.clicked() {
                            rack.selected_module = Some(m.n.to_string());
                        }
                        if ui.is_rect_visible(row_rect) {
                            let p = ui.painter();
                            p.rect_filled(row_rect, Rounding::same(2.0), bg);
                            p.rect_stroke(row_rect, Rounding::same(2.0), Stroke::new(0.5, border_color));

                            // Status mark
                            let mark = if m.built { "✓" } else { "◐" };
                            let mark_color = if m.built { theme::BIO } else { theme::GOLD };
                            let x = row_rect.min.x + 6.0;
                            p.text(Pos2::new(x + 4.0, row_rect.center().y), egui::Align2::CENTER_CENTER, mark, egui::FontId::monospace(8.0), mark_color);

                            // Number
                            p.text(Pos2::new(x + 20.0, row_rect.center().y), egui::Align2::LEFT_CENTER,
                                m.n, egui::FontId::monospace(7.0), theme::FG_MUTED);

                            // Name
                            p.text(Pos2::new(x + 38.0, row_rect.center().y), egui::Align2::LEFT_CENTER,
                                m.name.to_uppercase(), egui::FontId::proportional(8.5), m.color);

                            // Wire pip
                            let wc = theme::wire_color(m.wire);
                            p.circle_filled(Pos2::new(row_rect.max.x - 18.0, row_rect.center().y), 2.5, wc);
                            p.text(Pos2::new(row_rect.max.x - 12.0, row_rect.center().y), egui::Align2::LEFT_CENTER,
                                m.wire, egui::FontId::monospace(6.0), theme::FG_MUTED);
                        }
                    }
                    ui.add_space(8.0);
                }

                // Built/total summary
                let total = 16u32;
                let built = genesis_depts().iter().flat_map(|d| d.mods).filter(|m| m.built).count() as u32;
                ui.separator();
                ui.label(egui::RichText::new(format!("{built}/{total} MODULES SEALED"))
                    .font(egui::FontId::monospace(7.5))
                    .color(theme::FG_MUTED));

                // ── CONTAINERS ───────────────────────────────────────────
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("CONTAINERS")
                        .font(egui::FontId::monospace(8.0))
                        .color(theme::GOLD));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (btn, resp) = ui.allocate_exact_size(Vec2::new(52.0, 14.0), Sense::click());
                        if resp.clicked() {
                            request_load_dialog(channel.tx.clone());
                        }
                        if ui.is_rect_visible(btn) {
                            let p = ui.painter();
                            let hover = resp.hovered();
                            let bg = if hover { Color32::from_rgba_unmultiplied(0, 191, 255, 25) } else { theme::SURFACE };
                            p.rect_filled(btn, Rounding::same(2.0), bg);
                            p.rect_stroke(btn, Rounding::same(2.0), Stroke::new(0.5, theme::ASTRAL_CYAN));
                            p.text(btn.center(), egui::Align2::CENTER_CENTER,
                                "⊕  LOAD", egui::FontId::monospace(6.5), theme::ASTRAL_CYAN);
                        }
                    });
                });
                ui.add_space(4.0);

                if store.containers.is_empty() {
                    let hint = egui::RichText::new("No containers loaded.\nUse ⊕ LOAD or drop\n.qgenesis files here.")
                        .font(egui::FontId::monospace(6.5))
                        .color(theme::FG_MUTED);
                    ui.label(hint);
                } else {
                    for c in store.containers.iter() {
                        let color = c.kind_color();
                        let (row, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 26.0), Sense::hover(),
                        );
                        if ui.is_rect_visible(row) {
                            let p = ui.painter();
                            p.rect_filled(row, Rounding::same(2.0), theme::SURFACE);
                            p.rect_stroke(row, Rounding::same(2.0), Stroke::new(0.5, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 60)));
                            // Kind accent bar
                            p.rect_filled(
                                Rect::from_min_size(row.min, Vec2::new(3.0, row.height())),
                                Rounding { nw: 2.0, ne: 0.0, sw: 2.0, se: 0.0 },
                                color,
                            );
                            // Icon + name
                            p.text(Pos2::new(row.min.x + 10.0, row.center().y - 4.0),
                                egui::Align2::LEFT_CENTER, c.kind_icon(),
                                egui::FontId::proportional(10.0), color);
                            p.text(Pos2::new(row.min.x + 24.0, row.center().y - 4.0),
                                egui::Align2::LEFT_CENTER, &c.name,
                                egui::FontId::proportional(8.5), theme::FG_1);
                            // Kind + size sub-row
                            let size_str = if c.byte_len > 0 {
                                format!("{} · {}B", c.kind, human_bytes(c.byte_len))
                            } else {
                                c.kind.clone()
                            };
                            p.text(Pos2::new(row.min.x + 24.0, row.center().y + 5.0),
                                egui::Align2::LEFT_CENTER, &size_str,
                                egui::FontId::monospace(6.0), theme::FG_MUTED);
                            // Tags
                            if !c.tags.is_empty() {
                                let tag_str = c.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ");
                                p.text(Pos2::new(row.max.x - 4.0, row.center().y),
                                    egui::Align2::RIGHT_CENTER, &tag_str,
                                    egui::FontId::monospace(5.5), color);
                            }
                        }
                        ui.add_space(2.0);
                    }
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(format!("{} container(s) loaded", store.containers.len()))
                        .font(egui::FontId::monospace(6.5))
                        .color(theme::FG_MUTED));
                }
            });
        });

    // ── Asset Browser (right panel) ──────────────────────────────────────────
    egui::SidePanel::right("asset_browser")
        .resizable(false)
        .exact_width(190.0)
        .frame(egui::Frame::none()
            .fill(theme::ABYSS)
            .inner_margin(egui::Margin::same(8.0)))
        .show(ctx, |ui| {
            let total = assets.total();
            let gltf_count = assets.gltf_count();

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("ASSET BROWSER")
                    .font(egui::FontId::monospace(8.0))
                    .color(theme::GOLD));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let count_str = format!("{total}");
                    let (cr, _) = ui.allocate_exact_size(
                        Vec2::new(count_str.len() as f32 * 5.5 + 12.0, 14.0),
                        Sense::hover(),
                    );
                    if ui.is_rect_visible(cr) {
                        ui.painter().rect_filled(cr, Rounding::same(2.0),
                            Color32::from_rgba_unmultiplied(theme::GOLD.r(), theme::GOLD.g(), theme::GOLD.b(), 18));
                        ui.painter().text(cr.center(), egui::Align2::CENTER_CENTER,
                            &count_str, egui::FontId::monospace(7.0), theme::GOLD);
                    }
                });
            });

            // GLTF status line
            ui.label(egui::RichText::new(format!("{gltf_count}/{total} with GLTF"))
                .font(egui::FontId::monospace(6.5))
                .color(theme::FG_MUTED));
            ui.add_space(6.0);

            egui::ScrollArea::vertical().id_source("asset_browser_scroll").show(ui, |ui| {
                if total == 0 {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("No assets found.\nAdd qforge JSON files\nunder assets/")
                        .font(egui::FontId::monospace(6.5))
                        .color(theme::FG_MUTED));
                } else {
                    let domains = ["arch", "bio", "mech", "char", "prop"];
                    for domain in &domains {
                        let entries: Vec<_> = assets.by_domain(domain).collect();
                        if entries.is_empty() { continue; }

                        let dc = asset_domain_color(domain);

                        // Domain header
                        ui.horizontal(|ui| {
                            let (bar, _) = ui.allocate_exact_size(Vec2::new(3.0, 10.0), Sense::hover());
                            ui.painter().rect_filled(bar, Rounding::same(1.0), dc);
                            ui.label(egui::RichText::new(format!("{} · {}", domain.to_uppercase(), entries.len()))
                                .font(egui::FontId::monospace(7.0))
                                .color(dc));
                        });
                        ui.add_space(2.0);

                        for entry in &entries {
                            let has_gltf = entry.gltf_abs_path().is_some();
                            let (row, _) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 20.0), Sense::hover(),
                            );
                            if ui.is_rect_visible(row) {
                                let p = ui.painter();
                                p.rect_filled(row, Rounding::same(2.0), theme::SURFACE);

                                // Domain pip
                                p.circle_filled(
                                    Pos2::new(row.min.x + 6.0, row.center().y),
                                    2.5, dc,
                                );

                                // Stem name
                                let stem = if entry.manifest.stem.len() > 16 {
                                    &entry.manifest.stem[..16]
                                } else {
                                    &entry.manifest.stem
                                };
                                p.text(
                                    Pos2::new(row.min.x + 14.0, row.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    stem, egui::FontId::proportional(8.0), theme::FG_2,
                                );

                                // GLTF indicator
                                let (mark, mark_col) = if has_gltf {
                                    ("✓", theme::BIO)
                                } else {
                                    ("◐", theme::FG_MUTED)
                                };
                                p.text(
                                    Pos2::new(row.max.x - 6.0, row.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    mark, egui::FontId::monospace(7.0), mark_col,
                                );

                                // Zone tag (if present)
                                if let Some(zone) = entry.zone() {
                                    let zlen = zone.len().min(8);
                                    p.text(
                                        Pos2::new(row.max.x - 18.0, row.center().y),
                                        egui::Align2::RIGHT_CENTER,
                                        &zone[..zlen],
                                        egui::FontId::monospace(5.5),
                                        Color32::from_rgba_unmultiplied(dc.r(), dc.g(), dc.b(), 140),
                                    );
                                }
                            }
                            ui.add_space(1.0);
                        }
                        ui.add_space(6.0);
                    }

                    // Unknown domains
                    let known: std::collections::HashSet<&str> =
                        ["arch", "bio", "mech", "char", "prop"].iter().copied().collect();
                    let others: Vec<_> = assets.entries.iter()
                        .filter(|e| !known.contains(e.domain()))
                        .collect();
                    if !others.is_empty() {
                        let dc = theme::FG_MUTED;
                        ui.horizontal(|ui| {
                            let (bar, _) = ui.allocate_exact_size(Vec2::new(3.0, 10.0), Sense::hover());
                            ui.painter().rect_filled(bar, Rounding::same(1.0), dc);
                            ui.label(egui::RichText::new(format!("OTHER · {}", others.len()))
                                .font(egui::FontId::monospace(7.0))
                                .color(dc));
                        });
                        ui.add_space(2.0);
                        for entry in &others {
                            ui.label(egui::RichText::new(&entry.manifest.stem)
                                .font(egui::FontId::proportional(7.5))
                                .color(theme::FG_MUTED));
                        }
                        ui.add_space(6.0);
                    }
                }

                // ── Soul count separator ─────────────────────────────────
                ui.separator();
                ui.add_space(4.0);
                ui.label(egui::RichText::new("SOULS")
                    .font(egui::FontId::monospace(8.0))
                    .color(theme::MYTHOS));
                ui.add_space(2.0);
                let soul_count = souls.souls.len();
                if soul_count == 0 {
                    ui.label(egui::RichText::new("No souls active")
                        .font(egui::FontId::monospace(6.5))
                        .color(theme::FG_MUTED));
                } else {
                    ui.label(egui::RichText::new(format!("{soul_count} soul(s) loaded"))
                        .font(egui::FontId::monospace(6.5))
                        .color(theme::MYTHOS));
                    for (id, soul) in souls.souls.iter().take(8) {
                        let short_id = if id.len() > 20 { &id[..20] } else { id.as_str() };
                        let cf = soul.conscious_fraction();
                        let (row, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 18.0), Sense::hover(),
                        );
                        if ui.is_rect_visible(row) {
                            let p = ui.painter();
                            p.rect_filled(row, Rounding::same(2.0), theme::SURFACE);
                            // Consciousness bar
                            let bar_w = (row.width() - 6.0) * cf;
                            p.rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(row.min.x + 3.0, row.max.y - 3.0),
                                    Vec2::new(bar_w, 2.0),
                                ),
                                Rounding::same(1.0),
                                theme::MYTHOS,
                            );
                            p.text(
                                Pos2::new(row.min.x + 6.0, row.center().y - 1.0),
                                egui::Align2::LEFT_CENTER,
                                short_id,
                                egui::FontId::monospace(5.5),
                                theme::FG_3,
                            );
                            // CF value
                            p.text(
                                Pos2::new(row.max.x - 4.0, row.center().y - 1.0),
                                egui::Align2::RIGHT_CENTER,
                                &format!("{:.0}%", cf * 100.0),
                                egui::FontId::monospace(5.5),
                                theme::MYTHOS,
                            );
                        }
                        ui.add_space(1.0);
                    }
                    if soul_count > 8 {
                        ui.label(egui::RichText::new(format!("… +{} more", soul_count - 8))
                            .font(egui::FontId::monospace(6.0))
                            .color(theme::FG_MUTED));
                    }
                }
            });
        });

    // ── Rack chassis (central panel) ─────────────────────────────────────────
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(theme::DEEP))
        .show(ctx, |ui| {
            // Paint rack chassis background (iridescent rail)
            let chassis_rect = ui.available_rect_before_wrap();
            let painter = ui.painter();
            painter.rect_filled(chassis_rect, Rounding::same(8.0), theme::RAIL_TOP);

            // Iridescent shimmer overlay
            let shimmer = Color32::from_rgba_unmultiplied(192, 132, 252, 8);
            painter.rect_filled(
                Rect::from_min_size(chassis_rect.min, Vec2::new(chassis_rect.width() * 0.35, chassis_rect.height())),
                Rounding::same(8.0), shimmer,
            );
            let shimmer2 = Color32::from_rgba_unmultiplied(0, 191, 255, 6);
            let mid_x = chassis_rect.min.x + chassis_rect.width() * 0.35;
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(mid_x, chassis_rect.min.y), Pos2::new(mid_x + chassis_rect.width() * 0.20, chassis_rect.max.y)),
                Rounding::same(0.0), shimmer2,
            );

            // Side rails (44px each)
            let rail_w = 44.0_f32;
            let left_rail = Rect::from_min_size(chassis_rect.min, Vec2::new(rail_w, chassis_rect.height()));
            let right_rail = Rect::from_min_max(
                Pos2::new(chassis_rect.max.x - rail_w, chassis_rect.min.y),
                chassis_rect.max,
            );
            paint_rail(painter, left_rail);
            paint_rail(painter, right_rail);

            // Rack body (between rails) — indent via Frame inner_margin to clear rails
            egui::Frame::none()
                .inner_margin(egui::Margin {
                    left: rail_w + 8.0,
                    right: rail_w + 8.0,
                    top: 14.0,
                    bottom: 14.0,
                })
                .show(ui, |ui| {

            egui::ScrollArea::vertical().id_source("rack_body").show(ui, |ui| {
                let available_w = ui.available_width();
                let col_w = (available_w - 8.0) * 0.5 - 4.0;

                if registry.0.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("NO MODULES LOADED\nPlace .json files in assets/modules/")
                            .font(egui::FontId::monospace(11.0))
                            .color(theme::FG_MUTED));
                    });
                    return;
                }

                // Render modules grouped by department, 2 per row
                let all_modules: Vec<_> = registry.0.iter().collect();
                for chunk in all_modules.chunks(2) {
                    ui.horizontal(|ui| {
                        for module in chunk {
                            ui.vertical(|ui| {
                                ui.set_width(col_w);
                                let dc = theme::dept_color(&module.department);
                                let is_sel = rack.selected_module.as_deref() == Some(&module.id);

                                widgets::module_panel(ui, dc, |ui| {
                                    // Module header
                                    ui.horizontal(|ui| {
                                        widgets::led(ui, module.lifecycle == Lifecycle::Active, dc);
                                        ui.add_space(3.0);
                                        ui.label(egui::RichText::new(module.name.to_uppercase())
                                            .font(egui::FontId::proportional(10.0))
                                            .color(dc));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            widgets::status_badge(ui, &module.implementation_status);
                                        });
                                    });

                                    // ID + wire chip
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&module.id)
                                            .font(egui::FontId::monospace(7.0))
                                            .color(theme::FG_MUTED));
                                        ui.add_space(6.0);
                                        widgets::wire_pip(ui, &module.primary_wire_out);
                                    });

                                    ui.add_space(4.0);

                                    // Description
                                    let desc = if module.description.len() > 80 {
                                        &module.description[..80]
                                    } else {
                                        &module.description
                                    };
                                    ui.label(egui::RichText::new(desc)
                                        .font(egui::FontId::proportional(8.5))
                                        .color(theme::FG_3));

                                    ui.add_space(6.0);

                                    // Traktor knobs + jacks
                                    if !module.traktor_map.is_empty() {
                                        ui.horizontal_wrapped(|ui| {
                                            for binding in &module.traktor_map {
                                                let mut v = rack.get_param(&module.id, &binding.parameter);
                                                widgets::knob_sm(ui, &mut v, &binding.parameter, dc);
                                                rack.set_param(&module.id, &binding.parameter, v);
                                            }
                                        });

                                        ui.add_space(4.0);

                                        // Jack row (out per binding, max 3)
                                        ui.horizontal(|ui| {
                                            for binding in module.traktor_map.iter().take(3) {
                                                widgets::jack(ui, JackKind::Out, &binding.parameter[..binding.parameter.len().min(3)], dc);
                                                ui.add_space(3.0);
                                            }
                                        });
                                    }

                                    // Selection highlight border
                                    if is_sel {
                                        let r = ui.clip_rect();
                                        ui.painter().rect_stroke(r.shrink(2.0), Rounding::same(3.0), Stroke::new(1.0, dc));
                                    }
                                });

                                ui.add_space(8.0);
                            });

                            ui.add_space(8.0);
                        }
                    });
                }
            }); // ScrollArea
            }); // inner Frame (rail margins)
        });
}

// ── HELPERS ───────────────────────────────────────────────────────────────────

fn paint_rail(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, Rounding::same(4.0), theme::RAIL_DARK);
    // Vertical stitch line
    painter.line_segment(
        [Pos2::new(rect.center().x, rect.min.y + 20.0), Pos2::new(rect.center().x, rect.max.y - 20.0)],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 8)),
    );
    // Screws at top and bottom
    let screw_xs = [rect.min.x + 14.0, rect.max.x - 14.0];
    let screw_ys = [rect.min.y + 14.0, rect.min.y + 36.0, rect.max.y - 36.0, rect.max.y - 14.0];
    for &sx in &screw_xs {
        for &sy in &screw_ys {
            let c = Pos2::new(sx, sy);
            painter.circle_filled(c, 6.0, Color32::from_rgb(160, 168, 186));
            painter.circle_filled(c, 4.5, Color32::from_rgb(42, 46, 56));
            painter.line_segment(
                [Pos2::new(c.x - 2.0, c.y - 2.0), Pos2::new(c.x + 2.0, c.y + 2.0)],
                Stroke::new(0.5, Color32::from_rgba_unmultiplied(0, 0, 0, 160)),
            );
        }
    }
}

fn hier_crumb(ui: &mut egui::Ui, active: HierLevel) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
        let tiers = [
            (HierLevel::Genesis,  "GENESIS",  "1 sealed"),
            (HierLevel::Mythos,   "MYTHOS",   "16 modules"),
            (HierLevel::Standard, "STANDARD", "256 components"),
            (HierLevel::Capsules, "CAPSULES", "4096 sealed"),
        ];
        for (i, (level, label, meta)) in tiers.iter().enumerate() {
            let is_active = active == *level;
            let color = if is_active { theme::ASTRAL_CYAN } else { theme::FG_MUTED };
            ui.label(egui::RichText::new(*label).font(egui::FontId::monospace(8.0)).color(color));
            ui.label(egui::RichText::new(format!("·{meta}")).font(egui::FontId::monospace(6.5)).color(theme::FG_MUTED));
            if i < tiers.len() - 1 {
                ui.label(egui::RichText::new("▸").font(egui::FontId::monospace(7.0)).color(theme::FG_MUTED));
            }
        }
    });
}

fn chip(ui: &mut egui::Ui, label: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(label.len() as f32 * 5.5 + 16.0, 18.0),
        Sense::hover(),
    );
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, Rounding::same(2.0), Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 15));
        p.rect_stroke(rect, Rounding::same(2.0), Stroke::new(0.5, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 71)));
        p.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::monospace(8.0), color);
    }
}

fn chip_live(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(52.0, 18.0), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, Rounding::same(2.0), Color32::from_rgba_unmultiplied(57, 255, 20, 15));
        p.rect_stroke(rect, Rounding::same(2.0), Stroke::new(0.5, Color32::from_rgba_unmultiplied(57, 255, 20, 90)));
        p.circle_filled(Pos2::new(rect.min.x + 10.0, rect.center().y), 3.0, theme::BIO);
        p.text(Pos2::new(rect.min.x + 18.0, rect.center().y), egui::Align2::LEFT_CENTER,
            "LIVE", egui::FontId::monospace(8.0), theme::BIO);
    }
}

fn view_toggle(ui: &mut egui::Ui, mode: &mut ViewMode) {
    let modes = [(ViewMode::Rack, "▤  RACK"), (ViewMode::World, "◉  WORLD")];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        for (i, (m, label)) in modes.iter().enumerate() {
            let active = *mode == *m;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(58.0, 20.0), Sense::click());
            if resp.clicked() { *mode = *m; }
            if ui.is_rect_visible(rect) {
                let p = ui.painter();
                let rounding = match i {
                    0 => Rounding { nw: 3.0, ne: 0.0, sw: 3.0, se: 0.0 },
                    _ => Rounding { nw: 0.0, ne: 3.0, sw: 0.0, se: 3.0 },
                };
                let (bg, tc, border) = if active {
                    (Color32::from_rgba_unmultiplied(0, 229, 255, 25), theme::QUANTUM, theme::QUANTUM)
                } else {
                    let h = resp.hovered();
                    let bg = if h { theme::RAISED } else { theme::SURFACE };
                    (bg, theme::FG_MUTED, theme::FG_MUTED)
                };
                p.rect_filled(rect, rounding, bg);
                p.rect_stroke(rect, rounding, Stroke::new(0.5, border));
                p.text(rect.center(), egui::Align2::CENTER_CENTER,
                    *label, egui::FontId::monospace(7.5), tc);
            }
        }
    });
}

fn human_bytes(n: usize) -> String {
    if n >= 1_048_576 { format!("{:.1}M", n as f64 / 1_048_576.0) }
    else if n >= 1_024  { format!("{:.1}K", n as f64 / 1_024.0) }
    else                { format!("{n}") }
}

fn bus_to_dept(bus: BusChannel) -> Department {
    match bus {
        BusChannel::Structure  => Department::Structure,
        BusChannel::Entities   => Department::Entities,
        BusChannel::Atmosphere => Department::Atmosphere,
        BusChannel::Dynamics   => Department::Dynamics,
    }
}

/// Domain → egui Color32 matching the `domain_color()` floats in asset_registry.rs.
fn asset_domain_color(domain: &str) -> Color32 {
    match domain {
        "arch" => Color32::from_rgb( 31,  71, 140),  // [0.12, 0.28, 0.55]
        "bio"  => Color32::from_rgb( 38, 140,  56),  // [0.15, 0.55, 0.22]
        "mech" => Color32::from_rgb(140,  89,  31),  // [0.55, 0.35, 0.12]
        "char" => Color32::from_rgb(140,  38, 115),  // [0.55, 0.15, 0.45]
        _      => theme::FG_MUTED,
    }
}
