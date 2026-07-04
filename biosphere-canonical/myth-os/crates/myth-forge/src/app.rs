use egui::{Color32, Frame, Margin, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use crate::{
    canvas::{self, AppMode, CanvasState},
    export,
    inspector,
    library::ComponentLibrary,
    scene::{AtomKind, CanvasScene, Component, SplitDir},
    theme,
};

// ── Left panel tabs ───────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum LeftTab { Atoms, Library }

const ATOM_KINDS: &[AtomKind] = &[
    AtomKind::Knob, AtomKind::Fader, AtomKind::Pad, AtomKind::Jack,
    AtomKind::Meter, AtomKind::Scope, AtomKind::Label, AtomKind::Divider,
];

// ── App ───────────────────────────────────────────────────────────────────────

pub struct ForgeApp {
    scene:        CanvasScene,
    canvas_state: CanvasState,
    sel_atom:     Option<u64>,
    mode:         AppMode,
    left_tab:     LeftTab,
    lib:          ComponentLibrary,
    comp_name:    String,
    comp_desc:    String,
    export_text:  Option<String>,
    save_modal:   bool,
}

impl Default for ForgeApp {
    fn default() -> Self {
        let lib_dir = std::env::current_exe()
            .unwrap_or_default().parent()
            .unwrap_or(std::path::Path::new("."))
            .join("components");
        Self {
            scene:        CanvasScene::new("New Instrument", "INST"),
            canvas_state: CanvasState::default(),
            sel_atom:     None,
            mode:         AppMode::Atoms,
            left_tab:     LeftTab::Atoms,
            lib:          ComponentLibrary::load(&lib_dir),
            comp_name:    "My Component".into(),
            comp_desc:    String::new(),
            export_text:  None,
            save_modal:   false,
        }
    }
}

impl eframe::App for ForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        // ── Top bar ────────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("forge_top")
            .frame(Frame::none().fill(theme::ABYSS)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .inner_margin(Margin { left: 14.0, right: 14.0, top: 5.0, bottom: 5.0 }))
            .show(ctx, |ui| {
                top_bar(ui, &mut self.scene, &mut self.mode,
                    &mut self.export_text, &mut self.save_modal);
            });

        // Panel toolbar (only in panel mode, shown below top bar)
        if self.mode == AppMode::Panels {
            egui::TopBottomPanel::top("panel_toolbar")
                .frame(Frame::none().fill(theme::DEEP)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .inner_margin(Margin { left: 10.0, right: 10.0, top: 4.0, bottom: 4.0 }))
                .show(ctx, |ui| {
                    panel_toolbar(ui, &mut self.scene, &mut self.canvas_state);
                });
        }

        // ── Left panel ─────────────────────────────────────────────────────────
        egui::SidePanel::left("left_panel")
            .exact_width(108.0)
            .frame(Frame::none().fill(theme::DEEP)
                .stroke(Stroke::new(1.0, theme::BORDER)))
            .show(ctx, |ui| {
                left_tabs(ui, &mut self.left_tab);
                match self.left_tab {
                    LeftTab::Atoms   => atom_library(ui),
                    LeftTab::Library => component_library(ui, &mut self.lib,
                        &mut self.scene, &mut self.sel_atom),
                }
            });

        // ── Inspector (right) ──────────────────────────────────────────────────
        egui::SidePanel::right("inspector")
            .exact_width(184.0)
            .frame(Frame::none().fill(theme::DEEP)
                .stroke(Stroke::new(1.0, theme::BORDER)))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                match self.mode {
                    AppMode::Atoms => {
                        if let Some(id) = self.sel_atom {
                            if let Some(atom) = self.scene.get_atom_mut(id) {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    inspector::draw(ui, atom);
                                });
                            }
                        } else {
                            empty_inspector(ui, "select an atom");
                        }

                        // Save selection as component
                        if self.sel_atom.is_some() {
                            ui.add_space(8.0);
                            let sep_r = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 1.0), Sense::hover()).0;
                            ui.painter().rect_filled(sep_r, 0.0, theme::BORDER);
                            ui.add_space(6.0);
                            if tool_btn(ui, "SAVE AS COMPONENT", theme::BIO) {
                                self.save_modal = true;
                            }
                        }
                    }
                    AppMode::Panels => {
                        if let Some(pid) = self.canvas_state.sel_panel {
                            panel_inspector(ui, &mut self.scene, pid);
                        } else {
                            empty_inspector(ui, "select a panel");
                        }
                    }
                }
            });

        // ── Save-as-component modal ────────────────────────────────────────────
        if self.save_modal {
            let mut open = true;
            egui::Window::new("SAVE COMPONENT")
                .open(&mut open)
                .resizable(false)
                .default_size([320.0, 160.0])
                .frame(Frame::none().fill(theme::DEEP)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .inner_margin(Margin::same(12.0))
                    .rounding(Rounding::same(4.0)))
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 6.0);
                    labeled_field(ui, "Name", &mut self.comp_name);
                    labeled_field(ui, "Description", &mut self.comp_desc);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if tool_btn(ui, "SAVE", theme::BIO) {
                            let selected_atoms: Vec<&crate::scene::PlacedAtom> =
                                self.scene.atoms.iter().collect();
                            if !selected_atoms.is_empty() {
                                let comp = Component::from_atoms(
                                    &self.comp_name, &self.comp_desc, &selected_atoms);
                                self.lib.save(comp);
                            }
                            self.save_modal = false;
                        }
                        ui.add_space(4.0);
                        if tool_btn(ui, "CANCEL", theme::FG_MUTED) {
                            self.save_modal = false;
                        }
                    });
                });
            if !open { self.save_modal = false; }
        }

        // ── Export modal ───────────────────────────────────────────────────────
        if let Some(ref code) = self.export_text.clone() {
            let mut open = true;
            egui::Window::new("EXPORTED RUST")
                .open(&mut open)
                .default_size([640.0, 480.0])
                .frame(Frame::none().fill(theme::DEEP)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .inner_margin(Margin::same(12.0))
                    .rounding(Rounding::same(4.0)))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Paste into your instrument module.")
                            .font(theme::mono(8.0)).color(theme::FG2));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("COPY").font(theme::mono(8.0))).clicked() {
                                ui.output_mut(|o| o.copied_text = code.clone());
                            }
                        });
                    });
                    ui.add_space(6.0);
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut code.clone())
                            .font(theme::mono(9.0))
                            .desired_width(f32::INFINITY)
                            .code_editor());
                    });
                });
            if !open { self.export_text = None; }
        }

        // ── Canvas ─────────────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(Frame::none().fill(theme::VOID))
            .show(ctx, |ui| {
                canvas::draw(ui, &mut self.scene, &mut self.canvas_state,
                    &mut self.sel_atom, &self.mode);
            });
    }
}

// ── Top bar ───────────────────────────────────────────────────────────────────

fn top_bar(
    ui:          &mut Ui,
    scene:       &mut CanvasScene,
    mode:        &mut AppMode,
    export_text: &mut Option<String>,
    save_modal:  &mut bool,
) {
    ui.horizontal(|ui| {
        // Logo
        let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter().circle_filled(dot.center(), 4.5, theme::EMBER);
        ui.add_space(6.0);
        ui.label(egui::RichText::new("MYTH-FORGE").font(theme::mono(11.0)).color(theme::FG1));
        ui.add_space(4.0);
        ui.label(egui::RichText::new("INSTRUMENT BUILDER")
            .font(theme::mono(7.0)).color(theme::FG_MUTED));

        ui.add_space(16.0);

        // Mode toggle
        mode_toggle(ui, mode);

        ui.add_space(12.0);
        ui.label(egui::RichText::new("NAME").font(theme::mono(7.0)).color(theme::FG3));
        ui.text_edit_singleline(&mut scene.name);
        ui.add_space(4.0);
        ui.label(egui::RichText::new("TAG").font(theme::mono(7.0)).color(theme::FG3));
        ui.add(egui::TextEdit::singleline(&mut scene.module_tag).desired_width(40.0));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if action_btn(ui, "EXPORT ▸", theme::BIO) {
                *export_text = Some(export::to_rust(scene));
            }
            ui.add_space(6.0);
            if action_btn(ui, "SAVE", theme::QUANTUM) {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(&format!("{}.json",
                        scene.name.to_lowercase().replace(' ', "_")))
                    .add_filter("Forge Scene", &["json"]).save_file()
                {
                    if let Ok(json) = serde_json::to_string_pretty(scene) {
                        let _ = std::fs::write(path, json);
                    }
                }
            }
            ui.add_space(3.0);
            if action_btn(ui, "LOAD", theme::FG2) {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Forge Scene", &["json"]).pick_file()
                {
                    if let Ok(s) = std::fs::read_to_string(&path) {
                        if let Ok(loaded) = serde_json::from_str(&s) {
                            *scene = loaded;
                        }
                    }
                }
            }
            ui.add_space(3.0);
            if action_btn(ui, "CLEAR", theme::EMBER) {
                scene.atoms.clear();
            }
            ui.add_space(10.0);
            ui.label(egui::RichText::new(format!("{} ATOMS", scene.atoms.len()))
                .font(theme::mono(7.5)).color(theme::FG3));
        });
    });
    let _ = save_modal;
}

fn mode_toggle(ui: &mut Ui, mode: &mut AppMode) {
    ui.horizontal(|ui| {
        let atom_col  = if *mode == AppMode::Atoms  { theme::QUANTUM } else { theme::FG_MUTED };
        let panel_col = if *mode == AppMode::Panels { theme::GOLD    } else { theme::FG_MUTED };

        if tab_btn(ui, "ATOMS",  atom_col,  *mode == AppMode::Atoms)  { *mode = AppMode::Atoms; }
        if tab_btn(ui, "PANELS", panel_col, *mode == AppMode::Panels) { *mode = AppMode::Panels; }
    });
}

fn tab_btn(ui: &mut Ui, label: &str, color: Color32, active: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(label.len() as f32 * 6.5 + 16.0, 22.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if active { theme::with_alpha(color, 30) }
                   else if response.hovered() { theme::ELEVATED }
                   else { theme::SURFACE };
        ui.painter().rect_filled(rect, Rounding::same(2.0), fill);
        if active {
            ui.painter().rect_stroke(rect, Rounding::same(2.0),
                Stroke::new(1.0, theme::with_alpha(color, 120)));
        }
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
            label, theme::mono(7.5), color);
    }
    response.clicked()
}

fn action_btn(ui: &mut Ui, label: &str, color: Color32) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(label.len() as f32 * 6.5 + 12.0, 22.0), Sense::click());
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

fn tool_btn(ui: &mut Ui, label: &str, color: Color32) -> bool {
    action_btn(ui, label, color)
}

// ── Panel toolbar (shown below top bar in panel mode) ─────────────────────────

fn panel_toolbar(ui: &mut Ui, scene: &mut CanvasScene, state: &mut CanvasState) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("PANELS").font(theme::mono(7.0)).color(theme::GOLD));
        ui.add_space(8.0);

        let pid = state.sel_panel;

        ui.label(egui::RichText::new("SPLIT").font(theme::mono(6.5)).color(theme::FG_MUTED));
        ui.add_space(4.0);

        for (label, dir, count, color) in [
            ("÷ H×2",  SplitDir::Horizontal, 2, theme::QUANTUM),
            ("÷ H×3",  SplitDir::Horizontal, 3, theme::QUANTUM),
            ("÷ V×2",  SplitDir::Vertical,   2, theme::BIO),
            ("÷ V×3",  SplitDir::Vertical,   3, theme::BIO),
        ] {
            if action_btn(ui, label, color) {
                if let Some(id) = pid {
                    scene.split_panel(id, dir, count);
                }
            }
            ui.add_space(2.0);
        }

        ui.add_space(10.0);
        ui.label(egui::RichText::new("│").font(theme::mono(7.0)).color(theme::FG_MUTED));
        ui.add_space(10.0);

        if action_btn(ui, "COLLAPSE", theme::EMBER) {
            if let Some(id) = pid { scene.collapse_panel(id); }
        }
        ui.add_space(2.0);
        if action_btn(ui, "DELETE", theme::ROSE) {
            if let Some(id) = pid { scene.remove_panel(id); state.sel_panel = None; }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if pid.is_some() {
                ui.label(egui::RichText::new(format!("panel {:04X}", pid.unwrap()))
                    .font(theme::mono(7.5)).color(theme::FG3));
            } else {
                ui.label(egui::RichText::new("click canvas to select a panel")
                    .font(theme::mono(7.5)).color(theme::FG_MUTED));
            }
        });
    });
}

// ── Panel inspector ───────────────────────────────────────────────────────────

fn panel_inspector(ui: &mut Ui, scene: &mut CanvasScene, pid: u64) {
    if let Some(node) = scene.root.find_mut(pid) {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 6.0);

        ui.label(egui::RichText::new("PANEL").font(theme::mono(6.5)).color(theme::FG3));
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Name").font(theme::mono(7.5)).color(theme::FG3));
            ui.text_edit_singleline(&mut node.name);
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Flex").font(theme::mono(7.5)).color(theme::FG3));
            ui.add(egui::DragValue::new(&mut node.flex).speed(0.05).range(0.1..=10.0));
        });

        let sep = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover()).0;
        ui.painter().rect_filled(sep, 0.0, theme::BORDER);
        ui.add_space(4.0);

        ui.label(egui::RichText::new("BACKGROUND").font(theme::mono(6.5)).color(theme::FG3));
        ui.horizontal(|ui| {
            ui.checkbox(&mut node.show_bg, "");
            ui.label(egui::RichText::new("show").font(theme::mono(7.5)).color(theme::FG3));
        });
        ui.horizontal(|ui| {
            let [r, g, b, a] = node.bg;
            let mut col = Color32::from_rgba_unmultiplied(r, g, b, a);
            if ui.color_edit_button_srgba(&mut col).changed() {
                node.bg = [col.r(), col.g(), col.b(), col.a()];
            }
        });

        let sep = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover()).0;
        ui.painter().rect_filled(sep, 0.0, theme::BORDER);
        ui.add_space(4.0);

        ui.label(egui::RichText::new(
            format!("Split: {:?}  Children: {}", node.split, node.children.len()))
            .font(theme::mono(7.0)).color(theme::FG3));
    }
}

// ── Left panel tabs ───────────────────────────────────────────────────────────

fn left_tabs(ui: &mut Ui, tab: &mut LeftTab) {
    ui.horizontal(|ui| {
        ui.set_min_width(ui.available_width());
        let aw = ui.available_width();
        for (label, t) in [("ATOMS", LeftTab::Atoms), ("LIBRARY", LeftTab::Library)] {
            let active = *tab == t;
            let (rect, response) = ui.allocate_exact_size(
                Vec2::new(aw / 2.0, 24.0), Sense::click());
            if ui.is_rect_visible(rect) {
                let fill = if active { theme::SURFACE } else { theme::ABYSS };
                let col  = if active { theme::FG1 } else { theme::FG_MUTED };
                ui.painter().rect_filled(rect, Rounding::ZERO, fill);
                if active {
                    let bot = egui::Rect::from_min_size(
                        Pos2::new(rect.left(), rect.bottom() - 2.0),
                        Vec2::new(rect.width(), 2.0));
                    ui.painter().rect_filled(bot, Rounding::ZERO, theme::QUANTUM);
                }
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
                    label, theme::mono(7.5), col);
            }
            if response.clicked() { *tab = t; }
        }
    });
    let sep = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover()).0;
    ui.painter().rect_filled(sep, 0.0, theme::BORDER);
    ui.add_space(4.0);
}

// ── Atom library ──────────────────────────────────────────────────────────────

fn atom_library(ui: &mut Ui) {
    let aw = ui.available_width() - 12.0;
    for kind in ATOM_KINDS {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(aw, 42.0), Sense::click_and_drag());
        if ui.is_rect_visible(rect) {
            let fill = if response.hovered() || response.is_pointer_button_down_on() {
                theme::ELEVATED } else { theme::SURFACE };
            ui.painter().rect_filled(rect, Rounding::same(3.0), fill);
            ui.painter().rect_stroke(rect, Rounding::same(3.0),
                Stroke::new(1.0, theme::BORDER));
            ui.painter().text(Pos2::new(rect.left() + 16.0, rect.center().y),
                egui::Align2::CENTER_CENTER, kind.icon(),
                theme::mono(13.0), Color32::from_rgba_unmultiplied(0, 200, 255, 200));
            ui.painter().text(Pos2::new(rect.left() + 32.0, rect.center().y),
                egui::Align2::LEFT_CENTER, kind.label(), theme::mono(8.0), theme::FG2);
        }
        if response.drag_started() || response.clicked() {
            ui.memory_mut(|m| m.data.insert_temp(egui::Id::new("drag_atom"), kind.clone()));
        }
        ui.add_space(3.0);
    }
    ui.add_space(10.0);
    let sep = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover()).0;
    ui.painter().rect_filled(sep, 0.0, theme::BORDER);
    ui.add_space(6.0);
    ui.vertical_centered(|ui| {
        for hint in ["drag → canvas", "DEL to remove", "scroll = zoom", "mid-drag = pan"] {
            ui.label(egui::RichText::new(hint).font(theme::mono(6.5)).color(theme::FG_MUTED));
        }
    });
}

// ── Component library ─────────────────────────────────────────────────────────

fn component_library(
    ui:       &mut Ui,
    lib:      &mut ComponentLibrary,
    scene:    &mut CanvasScene,
    sel_atom: &mut Option<u64>,
) {
    ui.horizontal(|ui| {
        if action_btn(ui, "RELOAD", theme::FG2) { lib.reload(); }
        ui.add_space(4.0);
        if action_btn(ui, "OPEN DIR", theme::QUANTUM) {
            let _ = opener::open(&lib.dir).or_else(|_|
                std::process::Command::new("explorer").arg(&lib.dir).spawn().map(|_| ()));
        }
    });
    ui.add_space(6.0);

    if lib.components.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("no components yet").font(theme::mono(7.5)).color(theme::FG_MUTED));
            ui.label(egui::RichText::new("select atoms →").font(theme::mono(7.0)).color(theme::FG_MUTED));
            ui.label(egui::RichText::new("Save As Component").font(theme::mono(7.0)).color(theme::FG_MUTED));
        });
        return;
    }

    let aw   = ui.available_width() - 12.0;
    let mut to_delete = None;
    for (path, comp) in &lib.components {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(aw, 50.0), Sense::click_and_drag());
        if ui.is_rect_visible(rect) {
            let fill = if response.hovered() { theme::ELEVATED } else { theme::SURFACE };
            ui.painter().rect_filled(rect, Rounding::same(3.0), fill);
            ui.painter().rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0, theme::BORDER));

            // Name
            ui.painter().text(Pos2::new(rect.left() + 6.0, rect.top() + 10.0),
                egui::Align2::LEFT_CENTER, &comp.name,
                theme::mono(9.0), theme::FG1);
            // Atom count
            ui.painter().text(Pos2::new(rect.left() + 6.0, rect.bottom() - 10.0),
                egui::Align2::LEFT_CENTER,
                format!("{} atoms", comp.atoms.len()),
                theme::mono(7.0), theme::FG3);
            // Delete ×
            let del_r = Rect::from_center_size(
                Pos2::new(rect.right() - 10.0, rect.top() + 10.0), Vec2::splat(14.0));
            let del_resp = ui.interact(del_r, egui::Id::new(path), Sense::click());
            ui.painter().text(del_r.center(), egui::Align2::CENTER_CENTER,
                "×", theme::mono(10.0),
                if del_resp.hovered() { theme::EMBER } else { theme::FG_MUTED });
            if del_resp.clicked() { to_delete = Some(path.clone()); }
        }

        // Drag or click to place
        if response.drag_started() || response.clicked() {
            let ids = scene.add_atoms_from_component(comp, Pos2::new(40.0, 40.0));
            *sel_atom = ids.last().copied();
        }
        ui.add_space(3.0);
    }
    if let Some(p) = to_delete { lib.delete(&p); }
}

// ── Inspector helpers ─────────────────────────────────────────────────────────

fn empty_inspector(ui: &mut Ui, msg: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(egui::RichText::new(msg).font(theme::mono(8.0)).color(theme::FG_MUTED));
    });
}

fn labeled_field(ui: &mut Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).font(theme::mono(7.5)).color(theme::FG3));
        ui.text_edit_singleline(value);
    });
}
