/// Plugin Foundry UI panels — pure data manipulation, no renderer coupling.
///
/// Each panel is a plain Rust struct with an `egui_draw()` method that accepts
/// an `&mut egui::Ui`. The FoundryApp owns all panels; the eframe `update()`
/// just calls each panel's draw method in sequence.
///
/// Panels:
///   IDENTITY   — crate name, display name, description, version, kind
///   HERALDRY   — symbol builder: kind selector, parent crest, symbol name
///   WIRE CONFIG — add/remove wire_in and wire_out entries
///   ATOMS      — declare ATOMs this plugin composes from
///   ASSETS     — import icon / sample / shader paths
///   OUTPUT     — validation summary + Forge button
use crate::spec::{AtomEntry, AssetEntry, PluginKind, PluginSpec, WireEntry};
use egui::{ComboBox, RichText, ScrollArea, TextEdit, Ui};

const WIRE_TAGS: &[&str] = &[
    "SPA", "ENR", "VIS", "BHV", "SOC", "TMP",
    "NAR", "AUD", "LGC", "DAT", "AST", "EVT",
    "IDN", "CTL", "AGT", "MET", "RES",
];

pub struct IdentityPanel;

impl IdentityPanel {
    pub fn draw(ui: &mut Ui, spec: &mut PluginSpec) {
        ui.heading(RichText::new("IDENTITY").strong());
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Kind");
            if ui.radio(spec.kind == PluginKind::Plugin, "Plugin (Glyph)").clicked() {
                spec.kind = PluginKind::Plugin;
            }
            if ui.radio(spec.kind == PluginKind::Addon,  "Addon  (Sigil)").clicked() {
                spec.kind = PluginKind::Addon;
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Crate name");
            ui.add(TextEdit::singleline(&mut spec.crate_name).hint_text("myth-erosion"));
        });
        ui.horizontal(|ui| {
            ui.label("Display name");
            ui.add(TextEdit::singleline(&mut spec.display_name).hint_text("Erosion"));
        });
        ui.label("Description");
        ui.add(
            TextEdit::multiline(&mut spec.description)
                .hint_text("What does this plugin do?")
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            ui.label("Version");
            ui.add(egui::DragValue::new(&mut spec.version.0).prefix("v").speed(1));
            ui.label(".");
            ui.add(egui::DragValue::new(&mut spec.version.1).speed(1));
            ui.label(".");
            ui.add(egui::DragValue::new(&mut spec.version.2).speed(1));
        });
        ui.horizontal(|ui| {
            ui.label("Output path");
            ui.add(TextEdit::singleline(&mut spec.output_path).hint_text("crates/"));
        });
    }
}

pub struct HeraldryPanel;

impl HeraldryPanel {
    pub fn draw(ui: &mut Ui, spec: &mut PluginSpec) {
        ui.heading(RichText::new("HERALDRY").strong());
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Symbol name");
            ui.add(TextEdit::singleline(&mut spec.symbol_name).hint_text("Erosion"));
        });
        if spec.kind == PluginKind::Plugin {
            ui.horizontal(|ui| {
                ui.label("Parent Crest");
                ui.add(TextEdit::singleline(&mut spec.parent_crest).hint_text("Atlas"));
            });
            if !spec.symbol_name.is_empty() && !spec.parent_crest.is_empty() {
                let preview = format!("Glyph:{}↑{}", spec.symbol_name, spec.parent_crest);
                ui.label(RichText::new(&preview).monospace().color(egui::Color32::GOLD));
            }
        } else {
            if !spec.symbol_name.is_empty() {
                let preview = format!("Sigil:{}", spec.symbol_name);
                ui.label(RichText::new(&preview).monospace().color(egui::Color32::LIGHT_BLUE));
            }
        }
        ui.add_space(4.0);
        if ui.button("Build Heraldry Symbol").clicked() {
            spec.build_heraldry();
        }
        if !spec.heraldry_symbol.is_empty() {
            ui.label(RichText::new(format!("→ {}", spec.heraldry_symbol)).monospace());
        }
    }
}

pub struct WirePanel {
    pub new_in_tag:   String,
    pub new_in_note:  String,
    pub new_out_tag:  String,
    pub new_out_note: String,
}

impl Default for WirePanel {
    fn default() -> Self {
        Self {
            new_in_tag:   WIRE_TAGS[9].into(),  // DAT
            new_in_note:  String::new(),
            new_out_tag:  WIRE_TAGS[9].into(),
            new_out_note: String::new(),
        }
    }
}

impl WirePanel {
    pub fn draw(&mut self, ui: &mut Ui, spec: &mut PluginSpec) {
        ui.heading(RichText::new("WIRE CONFIG").strong());
        ui.separator();

        // wire_in
        ui.label(RichText::new("Inputs (wire_in)").strong());
        let mut remove_in: Option<usize> = None;
        for (i, entry) in spec.wire_in.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.monospace(&entry.tag);
                ui.label(&entry.note);
                if ui.small_button("✕").clicked() { remove_in = Some(i); }
            });
        }
        if let Some(i) = remove_in { spec.wire_in.remove(i); }

        ui.horizontal(|ui| {
            ComboBox::from_id_source("wire_in_tag")
                .selected_text(&self.new_in_tag)
                .show_ui(ui, |ui| {
                    for tag in WIRE_TAGS {
                        ui.selectable_value(&mut self.new_in_tag, (*tag).into(), *tag);
                    }
                });
            ui.add(TextEdit::singleline(&mut self.new_in_note).hint_text("note"));
            if ui.button("+ IN").clicked() && !self.new_in_tag.is_empty() {
                spec.wire_in.push(WireEntry::new(&self.new_in_tag, &self.new_in_note));
                self.new_in_note.clear();
            }
        });

        ui.add_space(6.0);

        // wire_out
        ui.label(RichText::new("Outputs (wire_out)").strong());
        let mut remove_out: Option<usize> = None;
        for (i, entry) in spec.wire_out.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.monospace(&entry.tag);
                ui.label(&entry.note);
                if ui.small_button("✕").clicked() { remove_out = Some(i); }
            });
        }
        if let Some(i) = remove_out { spec.wire_out.remove(i); }

        ui.horizontal(|ui| {
            ComboBox::from_id_source("wire_out_tag")
                .selected_text(&self.new_out_tag)
                .show_ui(ui, |ui| {
                    for tag in WIRE_TAGS {
                        ui.selectable_value(&mut self.new_out_tag, (*tag).into(), *tag);
                    }
                });
            ui.add(TextEdit::singleline(&mut self.new_out_note).hint_text("note"));
            if ui.button("+ OUT").clicked() && !self.new_out_tag.is_empty() {
                spec.wire_out.push(WireEntry::new(&self.new_out_tag, &self.new_out_note));
                self.new_out_note.clear();
            }
        });
    }
}

pub struct AtomsPanel {
    pub new_id:   String,
    pub new_label: String,
    pub new_desc:  String,
}

impl Default for AtomsPanel {
    fn default() -> Self {
        Self { new_id: String::new(), new_label: String::new(), new_desc: String::new() }
    }
}

impl AtomsPanel {
    pub fn draw(&mut self, ui: &mut Ui, spec: &mut PluginSpec) {
        ui.heading(RichText::new("ATOMS").strong());
        ui.separator();

        let mut remove: Option<usize> = None;
        for (i, atom) in spec.atoms.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.monospace(&atom.atom_id);
                ui.label(&atom.label);
                ui.label(RichText::new(&atom.description).weak());
                if ui.small_button("✕").clicked() { remove = Some(i); }
            });
        }
        if let Some(i) = remove { spec.atoms.remove(i); }

        ui.separator();
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.new_id).hint_text("ATOM_ID").desired_width(90.0));
            ui.add(TextEdit::singleline(&mut self.new_label).hint_text("Label").desired_width(90.0));
            ui.add(TextEdit::singleline(&mut self.new_desc).hint_text("What it does").desired_width(180.0));
            if ui.button("+ ATOM").clicked() && !self.new_id.is_empty() {
                spec.atoms.push(AtomEntry::new(&self.new_id, &self.new_label, &self.new_desc));
                self.new_id.clear();
                self.new_label.clear();
                self.new_desc.clear();
            }
        });
    }
}

pub struct AssetsPanel {
    pub new_name:  String,
    pub new_path:  String,
    pub new_type:  String,
}

impl Default for AssetsPanel {
    fn default() -> Self {
        Self {
            new_name: String::new(),
            new_path: String::new(),
            new_type: "Image".into(),
        }
    }
}

const MEDIA_TYPES: &[&str] = &["Image", "Audio", "Video", "Model", "Texture", "Skybox", "Font"];

impl AssetsPanel {
    pub fn draw(&mut self, ui: &mut Ui, spec: &mut PluginSpec) {
        ui.heading(RichText::new("ASSETS").strong());
        ui.separator();

        let mut remove: Option<usize> = None;
        for (i, asset) in spec.assets.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&asset.media_type).monospace());
                ui.label(&asset.name);
                ui.label(RichText::new(&asset.path).weak());
                if ui.small_button("✕").clicked() { remove = Some(i); }
            });
        }
        if let Some(i) = remove { spec.assets.remove(i); }

        ui.separator();
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.new_name).hint_text("name").desired_width(80.0));
            ui.add(TextEdit::singleline(&mut self.new_path).hint_text("path/to/asset").desired_width(160.0));
            ComboBox::from_id_source("asset_type")
                .selected_text(&self.new_type)
                .show_ui(ui, |ui| {
                    for mt in MEDIA_TYPES {
                        ui.selectable_value(&mut self.new_type, (*mt).into(), *mt);
                    }
                });
            if ui.button("+ ASSET").clicked() && !self.new_name.is_empty() {
                spec.assets.push(AssetEntry::new(&self.new_name, &self.new_path, &self.new_type));
                self.new_name.clear();
                self.new_path.clear();
            }
        });
    }
}

/// The output panel — shows validation errors and the Forge button.
/// Returns `Some(json_spec)` when the user clicks Forge and the spec is valid.
pub struct OutputPanel {
    pub last_json: String,
    pub last_errors: Vec<String>,
}

impl Default for OutputPanel {
    fn default() -> Self {
        Self { last_json: String::new(), last_errors: Vec::new() }
    }
}

impl OutputPanel {
    /// Returns `Some(json)` if the user clicked Forge and the spec validated.
    pub fn draw(&mut self, ui: &mut Ui, spec: &mut PluginSpec) -> Option<String> {
        ui.heading(RichText::new("OUTPUT").strong());
        ui.separator();

        let errors = spec.validate();

        if errors.is_empty() {
            ui.label(RichText::new("✓ Spec is valid").color(egui::Color32::GREEN));
        } else {
            for e in &errors {
                ui.label(RichText::new(format!("✗ {}", e)).color(egui::Color32::RED));
            }
        }
        self.last_errors = errors.clone();

        ui.add_space(8.0);

        let forge_enabled = errors.is_empty();
        let btn = ui.add_enabled(forge_enabled, egui::Button::new(
            RichText::new("⚒  FORGE").strong(),
        ));

        if btn.clicked() {
            if let Ok(json) = spec.to_json() {
                self.last_json = json.clone();
                return Some(json);
            }
        }

        if !self.last_json.is_empty() {
            ui.add_space(6.0);
            ui.label("Last forged spec:");
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                ui.monospace(&self.last_json);
            });
        }

        None
    }
}
