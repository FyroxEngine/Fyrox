/// FoundryApp — the eframe application host for the Plugin Foundry.
///
/// Owns the FoundryPlugin, the spec being built, and all UI panel state.
/// The plugin itself is also registered in a local PluginRegistry with
/// HeraldryAddon attached, so you can see live routing/addon behavior
/// in the "Output" panel.
use crate::{
    foundry::FoundryPlugin,
    heraldry_addon::HeraldryAddon,
    spec::PluginSpec,
    ui::{AtomsPanel, AssetsPanel, HeraldryPanel, IdentityPanel, OutputPanel, WirePanel},
};
use eframe::egui;
use egui::{CentralPanel, SidePanel, TopBottomPanel};
use myth_plugin::{PluginRegistry, PluginResult};
use myth_vault::VaultRegistry;
use std::sync::Arc;

pub struct FoundryApp {
    registry: PluginRegistry,
    spec:     PluginSpec,

    // UI panel state
    wire_panel:   WirePanel,
    atoms_panel:  AtomsPanel,
    assets_panel: AssetsPanel,
    output_panel: OutputPanel,

    // Currently selected tab
    tab: Tab,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tab {
    Identity,
    Heraldry,
    WireConfig,
    Atoms,
    Assets,
    Output,
}

impl FoundryApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> PluginResult<Self> {
        let tmp = std::env::temp_dir().join("myth-foundry-vault");
        let vault = Arc::new(VaultRegistry::open(&tmp)?);

        let mut registry = PluginRegistry::new();
        registry.register(FoundryPlugin::new(), vault.clone())?;
        registry.register_addon(Box::new(HeraldryAddon::new()))?;

        Ok(Self {
            registry,
            spec: PluginSpec::new_plugin("", ""),
            wire_panel:   WirePanel::default(),
            atoms_panel:  AtomsPanel::default(),
            assets_panel: AssetsPanel::default(),
            output_panel: OutputPanel::default(),
            tab: Tab::Identity,
        })
    }
}

impl eframe::App for FoundryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Top bar ──────────────────────────────────────────────────────────────
        TopBottomPanel::top("foundry_header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("⚒  Plugin Foundry");
                ui.separator();
                ui.label("myth-os · Glyph:Foundry↑Loom");
            });
        });

        // ── Tab selector (left sidebar) ───────────────────────────────────────
        SidePanel::left("foundry_tabs").resizable(false).default_width(120.0).show(ctx, |ui| {
            ui.add_space(8.0);
            for (label, tab) in [
                ("IDENTITY",    Tab::Identity),
                ("HERALDRY",    Tab::Heraldry),
                ("WIRE CONFIG", Tab::WireConfig),
                ("ATOMS",       Tab::Atoms),
                ("ASSETS",      Tab::Assets),
                ("OUTPUT",      Tab::Output),
            ] {
                let selected = self.tab == tab;
                if ui.selectable_label(selected, label).clicked() {
                    self.tab = tab;
                }
            }
        });

        // ── Main panel ────────────────────────────────────────────────────────
        CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.tab {
                    Tab::Identity  => IdentityPanel::draw(ui, &mut self.spec),
                    Tab::Heraldry  => HeraldryPanel::draw(ui, &mut self.spec),
                    Tab::WireConfig => self.wire_panel.draw(ui, &mut self.spec),
                    Tab::Atoms     => self.atoms_panel.draw(ui, &mut self.spec),
                    Tab::Assets    => self.assets_panel.draw(ui, &mut self.spec),
                    Tab::Output    => {
                        if let Some(json) = self.output_panel.draw(ui, &mut self.spec) {
                            // Route the forged spec through the registry so
                            // HeraldryAddon stamps it as it passes through.
                            let packet = myth_wire::WirePacket::new(
                                myth_wire::WireType::Data,
                                myth_wire::MythId::new(),
                                0,
                                json.into_bytes(),
                            );
                            match self.registry.route(&packet) {
                                Ok(_)  => tracing::info!("Spec routed through registry"),
                                Err(e) => tracing::error!("Route error: {}", e),
                            }
                        }
                    }
                }
            });
        });
    }
}
