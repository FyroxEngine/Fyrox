use egui::{Color32, Ui};

use crate::{scene::PlacedAtom, theme};

pub fn draw(ui: &mut Ui, atom: &mut PlacedAtom) {
    ui.spacing_mut().item_spacing = egui::vec2(4.0, 6.0);

    section(ui, "ATOM");

    row(ui, "Kind", |ui| {
        ui.label(egui::RichText::new(atom.kind.label())
            .font(theme::mono(9.0)).color(theme::QUANTUM));
    });

    row(ui, "ID", |ui| {
        ui.label(egui::RichText::new(format!("{:04X}", atom.id))
            .font(theme::mono(8.0)).color(theme::FG3));
    });

    ui.add_space(4.0);
    separator(ui);

    // ── Position & size ────────────────────────────────────────────────────────
    section(ui, "TRANSFORM");

    row(ui, "X", |ui| {
        let mut v = atom.pos[0];
        if ui.add(egui::DragValue::new(&mut v).speed(0.5).suffix(" px")).changed() {
            atom.pos[0] = v.max(0.0);
        }
    });
    row(ui, "Y", |ui| {
        let mut v = atom.pos[1];
        if ui.add(egui::DragValue::new(&mut v).speed(0.5).suffix(" px")).changed() {
            atom.pos[1] = v.max(0.0);
        }
    });
    row(ui, "W", |ui| {
        let mut v = atom.size[0];
        if ui.add(egui::DragValue::new(&mut v).speed(0.5).suffix(" px")).changed() {
            atom.size[0] = v.max(8.0);
        }
    });
    row(ui, "H", |ui| {
        let mut v = atom.size[1];
        if ui.add(egui::DragValue::new(&mut v).speed(0.5).suffix(" px")).changed() {
            atom.size[1] = v.max(8.0);
        }
    });

    ui.add_space(4.0);
    separator(ui);

    // ── Params ─────────────────────────────────────────────────────────────────
    section(ui, "PARAMS");

    row(ui, "Label", |ui| {
        ui.text_edit_singleline(&mut atom.params.label);
    });

    row(ui, "Value", |ui| {
        ui.add(egui::Slider::new(&mut atom.params.value, 0.0..=1.0)
            .show_value(true));
    });

    row(ui, "Size px", |ui| {
        ui.add(egui::DragValue::new(&mut atom.params.size_px).speed(1.0).range(8.0..=200.0));
    });

    // Color picker
    row(ui, "Color", |ui| {
        let [r, g, b, a] = atom.params.color;
        let mut col32 = Color32::from_rgba_unmultiplied(r, g, b, a);
        if ui.color_edit_button_srgba(&mut col32).changed() {
            atom.params.color = [col32.r(), col32.g(), col32.b(), col32.a()];
        }
    });

    // Kind-specific params
    use crate::scene::AtomKind;
    match atom.kind {
        AtomKind::Jack => {
            row(ui, "Output", |ui| {
                ui.checkbox(&mut atom.params.is_output, "");
            });
        }
        AtomKind::Pad => {
            row(ui, "Lit", |ui| {
                ui.checkbox(&mut atom.params.lit, "");
            });
        }
        AtomKind::Label => {
            row(ui, "Text", |ui| {
                ui.text_edit_singleline(&mut atom.params.text);
            });
        }
        _ => {}
    }

    ui.add_space(4.0);
    separator(ui);

    // ── Flags ──────────────────────────────────────────────────────────────────
    section(ui, "FLAGS");

    row(ui, "Layer", |ui| {
        ui.add(egui::DragValue::new(&mut atom.layer).range(0u8..=3u8));
    });

    row(ui, "Locked", |ui| {
        ui.checkbox(&mut atom.locked, "");
    });
}

// ── Layout helpers ─────────────────────────────────────────────────────────────

fn section(ui: &mut Ui, title: &str) {
    ui.label(egui::RichText::new(title).font(theme::mono(6.5)).color(theme::FG3));
    ui.add_space(2.0);
}

fn separator(ui: &mut Ui) {
    let (r, _) = ui.allocate_exact_size(
        egui::Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, theme::BORDER);
    ui.add_space(4.0);
}

fn row(ui: &mut Ui, label: &str, content: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_width(ui.available_width());
        ui.label(egui::RichText::new(label).font(theme::mono(7.5)).color(theme::FG3));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), content);
    });
}
