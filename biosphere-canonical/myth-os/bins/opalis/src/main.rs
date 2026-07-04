mod graph;
mod layers;
mod mythos;
mod nodes;
mod plugin;
mod portal;
mod shaders;
mod vault;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use eframe::egui;
use egui::mutex::Mutex;
use graph::PrismViewer;
use layers::RenderMode;
use mythos::MythOS;
use nodes::PrismNode;
use plugin::{create_math_plugin, create_output_plugin, create_text_plugin};
use shaders::{ShaderCallback, ShaderRenderer};
use vault::VaultId;

fn assets_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let mut dir = exe.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
    // In dev, we're in target/debug — go up to project root
    if dir.ends_with("debug") || dir.ends_with("release") {
        dir = dir.parent().and_then(|p| p.parent()).unwrap_or(&dir).to_path_buf();
    }
    dir.join("assets")
}

fn node_image_uri(filename: &str) -> String {
    let path = assets_dir().join("nodes").join(filename);
    format!("file://{}", path.to_string_lossy().replace('\\', "/"))
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MYTHOS")
            .with_inner_size([1440.0, 900.0])
            .with_decorations(false),
        ..Default::default()
    };

    eframe::run_native(
        "mythos",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            cc.egui_ctx.set_visuals(custom_visuals());

            let mut app = PrismApp::new();

            // Initialize shader from the wgpu render state
            if let Some(rs) = cc.wgpu_render_state.as_ref() {
                let renderer = ShaderRenderer::new(&rs.device, rs.target_format);
                app.nebula_renderer = Some(Arc::new(Mutex::new(renderer)));
            }

            Ok(Box::new(app))
        }),
    )
}

fn custom_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = egui::Color32::from_rgb(10, 10, 16);
    v.window_fill = egui::Color32::from_rgb(18, 18, 26);
    v.extreme_bg_color = egui::Color32::from_rgb(8, 8, 12);
    v.faint_bg_color = egui::Color32::from_rgb(22, 22, 30);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(35, 35, 50);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 45, 70);
    v.widgets.active.bg_fill = egui::Color32::from_rgb(65, 55, 110);
    v.selection.bg_fill = egui::Color32::from_rgb(80, 60, 140);
    v.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(180, 180, 200);
    v.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(230, 230, 250);
    v.widgets.active.fg_stroke.color = egui::Color32::WHITE;
    v.window_shadow = egui::Shadow {
        offset: [0, 6].into(),
        blur: 20,
        spread: 0,
        color: egui::Color32::from_black_alpha(120),
    };
    v.window_corner_radius = egui::CornerRadius::same(10);
    v.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    v.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    v.widgets.active.corner_radius = egui::CornerRadius::same(6);
    v
}

// ── Colors ─────────────────────────────────────────────────────────────

const ACCENT: egui::Color32 = egui::Color32::from_rgb(160, 120, 255);
const ACCENT_GLOW: egui::Color32 = egui::Color32::from_rgb(120, 80, 220);
const GOLD: egui::Color32 = egui::Color32::from_rgb(220, 180, 80);
const SUCCESS: egui::Color32 = egui::Color32::from_rgb(80, 230, 160);
const DIM: egui::Color32 = egui::Color32::from_rgb(70, 70, 90);
const GHOST: egui::Color32 = egui::Color32::from_rgb(50, 50, 65);
const VOID: egui::Color32 = egui::Color32::from_rgb(10, 10, 16);
const SURFACE: egui::Color32 = egui::Color32::from_rgb(16, 16, 22);

// ── App state ──────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum AppView {
    Splash,
    Fading,
    MasterVault,
    InsideVault(VaultId),
}

struct VaultCreateModal {
    open: bool,
    name: String,
    vault_type: String,
    render_mode: RenderMode,
    selected_image: Option<String>,
    available_images: Vec<String>,
}

impl VaultCreateModal {
    fn new() -> Self {
        let mut images = Vec::new();
        if let Ok(entries) = std::fs::read_dir(assets_dir().join("nodes")) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".png") || name.ends_with(".jpg") {
                        images.push(name.to_string());
                    }
                }
            }
        }
        images.sort();
        Self {
            open: false,
            name: String::new(),
            vault_type: "Workspace".into(),
            render_mode: RenderMode::TwoD,
            selected_image: None,
            available_images: images,
        }
    }

    fn reset(&mut self) {
        self.name.clear();
        self.vault_type = "Workspace".into();
        self.render_mode = RenderMode::TwoD;
        self.selected_image = None;
    }
}

struct PrismApp {
    os: MythOS,
    view: AppView,
    fade_start: Option<Instant>,
    create_modal: VaultCreateModal,
    snarl_style: egui_snarl::ui::SnarlStyle,
    show_boot_log: bool,
    show_portals: bool,
    nebula_renderer: Option<Arc<Mutex<ShaderRenderer>>>,
    start_time: Instant,
}

impl PrismApp {
    fn new() -> Self {
        let mut os = MythOS::boot();

        // Seed starter vaults with card art
        let vid = os.master_vault.create_vault("Sequencer Lab", "Workspace").unwrap();
        if let Some(vault) = os.master_vault.vault_mut(vid) {
            vault.card_image = Some("mythic-nodes-009.png".into());
            vault.install_plugin(Box::new(create_text_plugin())).unwrap();
            vault.install_plugin(Box::new(create_math_plugin())).unwrap();
            vault.install_plugin(Box::new(create_output_plugin())).unwrap();

            let nodes = vault.available_nodes();
            let find = |tid: &str| nodes.iter().find(|(_, r)| r.type_id == tid).map(|(_, r)| r);
            if let (Some(t), Some(u), Some(s)) = (find("text_source"), find("uppercase"), find("sink")) {
                let a = vault.graph.insert_node(egui::pos2(150.0, 250.0), PrismNode::from_registration_with_text(t, "Hello MYTHOS"));
                let b = vault.graph.insert_node(egui::pos2(450.0, 250.0), PrismNode::from_registration(u));
                let c = vault.graph.insert_node(egui::pos2(750.0, 250.0), PrismNode::from_registration(s));
                let _ = vault.graph.connect(egui_snarl::OutPinId { node: a, output: 0 }, egui_snarl::InPinId { node: b, input: 0 });
                let _ = vault.graph.connect(egui_snarl::OutPinId { node: b, output: 0 }, egui_snarl::InPinId { node: c, input: 0 });
            }
        }

        let vid2 = os.master_vault.create_vault("The Resonance Chamber", "Narrative Engine").unwrap();
        if let Some(vault) = os.master_vault.vault_mut(vid2) {
            vault.card_image = Some("mythic-nodes-012.png".into());
        }

        let vid3 = os.master_vault.create_vault("Hope Basin", "World Map").unwrap();
        if let Some(vault) = os.master_vault.vault_mut(vid3) {
            vault.card_image = Some("mythic-nodes-006.png".into());
        }

        Self {
            os,
            view: AppView::Splash,
            fade_start: None,
            create_modal: VaultCreateModal::new(),
            snarl_style: egui_snarl::ui::SnarlStyle::default(),
            show_boot_log: false,
            show_portals: false,
            nebula_renderer: None,
            start_time: Instant::now(),
        }
    }

    fn paint_nebula(&mut self, ui: &mut egui::Ui, rect: egui::Rect, intensity: f32, tint: [f32; 3]) {
        if let Some(renderer) = &self.nebula_renderer {
            let elapsed = self.start_time.elapsed().as_secs_f32();

            // Update uniforms
            {
                let mut r = renderer.lock();
                r.uniforms.time = elapsed;
                r.uniforms.resolution_x = rect.width();
                r.uniforms.resolution_y = rect.height();
                r.uniforms.intensity = intensity;
                r.uniforms.tint_r = tint[0];
                r.uniforms.tint_g = tint[1];
                r.uniforms.tint_b = tint[2];
            }

            let callback = egui_wgpu::Callback::new_paint_callback(
                rect,
                ShaderCallback {
                    renderer: Arc::clone(renderer),
                },
            );
            ui.painter().add(callback);
        }
    }

    fn draw_splash(&mut self, ctx: &egui::Context) {


        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(VOID))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                let center = rect.center();

                // Live nebula background
                self.paint_nebula(ui, rect, 0.35, [0.5, 0.3, 1.0]);

                // Background seal image over nebula
                let seal_size = 320.0;
                let seal_rect = egui::Rect::from_center_size(center + egui::vec2(0.0, -10.0), egui::vec2(seal_size, seal_size));
                let img = egui::Image::new(node_image_uri("mythic-nodes-009.png"))
                    .tint(egui::Color32::from_rgba_premultiplied(255, 255, 255, 50));
                img.paint_at(ui, seal_rect);

                // Title
                ui.painter().text(
                    center + egui::vec2(0.0, -60.0),
                    egui::Align2::CENTER_CENTER,
                    "M Y T H O S",
                    egui::FontId::proportional(64.0),
                    ACCENT,
                );

                // Subtitle
                ui.painter().text(
                    center + egui::vec2(0.0, 0.0),
                    egui::Align2::CENTER_CENTER,
                    "Opalis Software Sequencer",
                    egui::FontId::proportional(18.0),
                    DIM,
                );

                // Pulsing prompt
                let alpha = ((self.os.clock_tick as f32 * 0.05).sin() * 0.5 + 0.5) * 180.0;
                ui.painter().text(
                    center + egui::vec2(0.0, 80.0),
                    egui::Align2::CENTER_CENTER,
                    "click to enter",
                    egui::FontId::proportional(14.0),
                    egui::Color32::from_rgba_premultiplied(180, 180, 200, alpha as u8),
                );

                // Version
                ui.painter().text(
                    egui::pos2(rect.right() - 16.0, rect.bottom() - 16.0),
                    egui::Align2::RIGHT_BOTTOM,
                    "v0.1.0",
                    egui::FontId::proportional(11.0),
                    GHOST,
                );

                // Click to proceed
                if ui.input(|i| i.pointer.any_click()) {
                    self.view = AppView::Fading;
                    self.fade_start = Some(Instant::now());
                }
            });
    }

    fn draw_fade(&mut self, ctx: &egui::Context) {
        let elapsed = self.fade_start.map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0);
        let alpha = ((1.0 - elapsed) * 255.0).clamp(0.0, 255.0) as u8;

        // Draw the master vault underneath
        self.draw_master_vault(ctx);

        // Overlay fade
        if alpha > 0 {
            egui::Area::new(egui::Id::new("fade_overlay"))
                .fixed_pos(egui::pos2(0.0, 0.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let screen = ctx.content_rect();
                    ui.painter().rect_filled(
                        screen,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(10, 10, 16, alpha),
                    );
                });
            ctx.request_repaint();
        } else {
            self.view = AppView::MasterVault;
        }
    }

    fn draw_header(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::new().fill(SURFACE).inner_margin(6.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Draggable title area
                    let title_resp = ui.label(
                        egui::RichText::new("MYTHOS")
                            .size(16.0)
                            .strong()
                            .color(ACCENT),
                    );

                    // Window drag
                    if title_resp.interact(egui::Sense::drag()).dragged() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }

                    ui.label(egui::RichText::new("|").color(GHOST));

                    // Breadcrumb
                    match &self.view {
                        AppView::MasterVault => {
                            ui.label(egui::RichText::new("Master Vault").small().color(GOLD));
                        }
                        AppView::InsideVault(vid) => {
                            // "Master Vault" is clickable — handled in main update
                            ui.label(egui::RichText::new("Master Vault").small().color(DIM));
                            ui.label(egui::RichText::new(">").small().color(GHOST));
                            if let Some(v) = self.os.master_vault.vault(*vid) {
                                ui.label(egui::RichText::new(&v.name).small().color(GOLD));
                                ui.label(egui::RichText::new(&v.vault_type).small().color(ACCENT));
                            }
                        }
                        _ => {}
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Window controls
                        if ui.button(egui::RichText::new("X").small().color(egui::Color32::from_rgb(255, 80, 90))).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button(egui::RichText::new("_").small()).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }

                        ui.add_space(12.0);
                        ui.label(egui::RichText::new(format!("tick {}", self.os.clock_tick)).small().color(GHOST));
                    });
                });
            });
    }

    fn draw_footer(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer")
            .frame(egui::Frame::new().fill(SURFACE).inner_margin(4.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let os_color = if self.os.is_booted() { SUCCESS } else { GOLD };
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 3.0, os_color);
                    ui.label(egui::RichText::new("MYTH-OS").small().color(os_color));

                    ui.label(egui::RichText::new("|").color(GHOST));
                    ui.label(egui::RichText::new(format!("{}/16 vaults", self.os.master_vault.vaults.len())).small().color(DIM));

                    ui.label(egui::RichText::new("|").color(GHOST));
                    ui.label(egui::RichText::new(format!("{} portals", self.os.portals.all_portals().len())).small().color(DIM));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("MYTHOS — Opalis Software Sequencer v0.1.0").small().color(GHOST));
                    });
                });
            });
    }

    fn draw_master_vault(&mut self, ctx: &egui::Context) {

        self.draw_header(ctx);
        self.draw_footer(ctx);
        self.draw_create_modal(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(VOID))
            .show(ctx, |ui| {
                let avail = ui.available_rect_before_wrap();

                // Subtle nebula on the master vault background
                self.paint_nebula(ui, avail, 0.15, [0.4, 0.2, 0.8]);

                // Vault cards grid
                let vault_ids: Vec<VaultId> = self.os.master_vault.vaults.keys().copied().collect();
                let card_w = 200.0_f32;
                let card_h = 260.0_f32;
                let spacing = 24.0_f32;
                let total_cards = vault_ids.len() + 1; // +1 for the "create" card
                let cols = ((avail.width() - 60.0) / (card_w + spacing)).floor().max(1.0) as usize;
                let rows = (total_cards + cols - 1) / cols;
                let grid_w = cols as f32 * (card_w + spacing) - spacing;
                let grid_h = rows as f32 * (card_h + spacing) - spacing;
                let start_x = avail.center().x - grid_w / 2.0;
                let start_y = avail.center().y - grid_h / 2.0;

                // Collect vault info to avoid borrow conflicts
                let vault_infos: Vec<(VaultId, String, String, Option<String>, usize, usize, usize, vault::VaultStatus)> = vault_ids.iter().filter_map(|vid| {
                    self.os.master_vault.vault(*vid).map(|v| {
                        let portal_count = self.os.portals.portals_for_vault(*vid).len();
                        (*vid, v.name.clone(), v.vault_type.clone(), v.card_image.clone(), v.plugin_count(), v.graph.node_ids().count(), portal_count, v.status)
                    })
                }).collect();

                let mut clicked_vault = None;

                for (i, (vid, name, vtype, card_img, plugins, atoms, portals, status)) in vault_infos.iter().enumerate() {
                    let col = i % cols;
                    let row = i / cols;
                    let x = start_x + col as f32 * (card_w + spacing);
                    let y = start_y + row as f32 * (card_h + spacing);
                    let card_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_w, card_h));

                    if Self::draw_vault_card_static(ui, card_rect, name, vtype, card_img.as_deref(), *plugins, *atoms, *portals, *status) {
                        clicked_vault = Some(*vid);
                    }
                }

                if let Some(vid) = clicked_vault {
                    self.view = AppView::InsideVault(vid);
                }

                // "Create new vault" card
                {
                    let i = vault_ids.len();
                    let col = i % cols;
                    let row = i / cols;
                    let x = start_x + col as f32 * (card_w + spacing);
                    let y = start_y + row as f32 * (card_h + spacing);
                    let card_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(card_w, card_h));

                    if self.os.master_vault.vaults.len() < mythos::MAX_VAULTS {
                        // Simple "+" card that opens the creation modal
                        let hover = ui.rect_contains_pointer(card_rect);
                        let painter = ui.painter();
                        let bg = if hover { egui::Color32::from_rgb(22, 20, 35) } else { egui::Color32::TRANSPARENT };
                        painter.rect_filled(card_rect, 14.0, bg);
                        painter.rect_stroke(card_rect, 14.0, egui::Stroke::new(1.0, if hover { ACCENT_GLOW } else { GHOST }), egui::StrokeKind::Outside);
                        painter.text(card_rect.center() + egui::vec2(0.0, -10.0), egui::Align2::CENTER_CENTER, "+", egui::FontId::proportional(48.0), if hover { ACCENT } else { GHOST });
                        painter.text(card_rect.center() + egui::vec2(0.0, 30.0), egui::Align2::CENTER_CENTER, "New Vault", egui::FontId::proportional(12.0), if hover { GOLD } else { GHOST });

                        let resp = ui.allocate_rect(card_rect, egui::Sense::click());
                        if resp.clicked() {
                            self.create_modal.reset();
                            self.create_modal.open = true;
                        }
                    }
                }
            });
    }

    /// Returns true if clicked
    fn draw_vault_card_static(
        ui: &mut egui::Ui,
        rect: egui::Rect,
        name: &str,
        vtype: &str,
        card_image: Option<&str>,
        plugins: usize,
        atoms: usize,
        portals: usize,
        status: vault::VaultStatus,
    ) -> bool {
        let hover = ui.rect_contains_pointer(rect);
        let painter = ui.painter();

        let bg = if hover {
            egui::Color32::from_rgb(22, 20, 38)
        } else {
            egui::Color32::from_rgb(14, 14, 22)
        };

        // Outer glow on hover
        if hover {
            let glow_rect = rect.expand(6.0);
            painter.rect_filled(glow_rect, 16.0, egui::Color32::from_rgba_premultiplied(100, 70, 200, 25));
        }

        // Card body
        painter.rect_filled(rect, 14.0, bg);

        // Card image — centered in the upper portion
        if let Some(img_file) = card_image {
            let img_size = 140.0;
            let img_rect = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.top() + 10.0 + img_size / 2.0),
                egui::vec2(img_size, img_size),
            );
            let tint_alpha = if hover { 200 } else { 120 };
            let img = egui::Image::new(node_image_uri(img_file))
                .tint(egui::Color32::from_rgba_premultiplied(255, 255, 255, tint_alpha))
                .corner_radius(8.0);
            img.paint_at(ui, img_rect);
        } else {
            // Fallback: draw the concentric seal circles
            let seal_center = egui::pos2(rect.center().x, rect.top() + 75.0);
            painter.circle_stroke(seal_center, 40.0, egui::Stroke::new(1.5, if hover { ACCENT } else { GHOST }));
            painter.circle_stroke(seal_center, 28.0, egui::Stroke::new(1.0, if hover { GOLD } else { egui::Color32::from_rgb(35, 35, 50) }));
            painter.circle_stroke(seal_center, 16.0, egui::Stroke::new(0.5, GHOST));

            let first_char = vtype.chars().next().unwrap_or('V');
            painter.text(seal_center, egui::Align2::CENTER_CENTER, first_char.to_uppercase().to_string(), egui::FontId::proportional(24.0), if hover { GOLD } else { DIM });
        }

        // Name
        let name_y = rect.top() + 165.0;
        painter.text(
            egui::pos2(rect.center().x, name_y),
            egui::Align2::CENTER_CENTER,
            name,
            egui::FontId::proportional(14.0),
            if hover { egui::Color32::WHITE } else { egui::Color32::from_rgb(200, 200, 220) },
        );

        // Type tag
        painter.text(
            egui::pos2(rect.center().x, name_y + 18.0),
            egui::Align2::CENTER_CENTER,
            vtype,
            egui::FontId::proportional(10.0),
            ACCENT,
        );

        // Thin divider
        let div_y = name_y + 32.0;
        painter.line_segment(
            [egui::pos2(rect.left() + 24.0, div_y), egui::pos2(rect.right() - 24.0, div_y)],
            egui::Stroke::new(0.5, GHOST),
        );

        // Stats row
        let stats_y = div_y + 16.0;
        let stats_text = if portals > 0 {
            format!("{plugins}p  {atoms}a  {portals}x")
        } else {
            format!("{plugins} plugins  /  {atoms} atoms")
        };
        painter.text(
            egui::pos2(rect.center().x, stats_y),
            egui::Align2::CENTER_CENTER,
            stats_text,
            egui::FontId::proportional(10.0),
            DIM,
        );

        // Status dot
        let status_color = match status {
            vault::VaultStatus::Active => SUCCESS,
            vault::VaultStatus::Booting => GOLD,
            _ => DIM,
        };
        painter.circle_filled(egui::pos2(rect.right() - 12.0, rect.top() + 12.0), 4.0, status_color);

        // Border
        painter.rect_stroke(rect, 14.0, egui::Stroke::new(1.0, if hover { ACCENT_GLOW } else { egui::Color32::from_rgb(30, 30, 45) }), egui::StrokeKind::Outside);

        // Bottom: ENTER on hover, sealed otherwise
        if hover {
            painter.text(egui::pos2(rect.center().x, rect.bottom() - 16.0), egui::Align2::CENTER_CENTER, "E N T E R", egui::FontId::proportional(11.0), GOLD);
        } else {
            painter.text(egui::pos2(rect.center().x, rect.bottom() - 16.0), egui::Align2::CENTER_CENTER, "sealed", egui::FontId::proportional(9.0), GHOST);
        }

        let resp = ui.allocate_rect(rect, egui::Sense::click());
        resp.clicked()
    }

    fn draw_create_modal(&mut self, ctx: &egui::Context) {
        if !self.create_modal.open {
            return;
        }

        // Dim background
        let screen = ctx.input(|i| i.screen_rect());
        egui::Area::new(egui::Id::new("modal_dim"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.painter().rect_filled(screen, 0.0, egui::Color32::from_black_alpha(180));
                // Click outside to close
                let resp = ui.allocate_rect(screen, egui::Sense::click());
                if resp.clicked() {
                    self.create_modal.open = false;
                }
            });

        // Modal panel
        egui::Window::new("")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([480.0, 520.0])
            .frame(egui::Frame::new()
                .fill(egui::Color32::from_rgb(16, 16, 24))
                .stroke(egui::Stroke::new(1.0, ACCENT_GLOW))
                .corner_radius(14)
                .inner_margin(24.0)
                .shadow(egui::Shadow {
                    offset: [0, 8].into(),
                    blur: 30,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(160),
                }))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // Title
                ui.label(egui::RichText::new("Create New Vault").size(20.0).strong().color(GOLD));
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Configure your isolated workspace").small().color(DIM));
                ui.add_space(16.0);

                // Name
                ui.label(egui::RichText::new("NAME").small().strong().color(ACCENT));
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.create_modal.name)
                        .desired_width(f32::INFINITY)
                        .hint_text("The Forgotten Archives...")
                        .font(egui::FontId::proportional(14.0)),
                );
                ui.add_space(12.0);

                // Type
                ui.label(egui::RichText::new("TYPE").small().strong().color(ACCENT));
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.create_modal.vault_type)
                        .desired_width(f32::INFINITY)
                        .hint_text("Workspace, Library, Engine, anything...")
                        .font(egui::FontId::proportional(14.0)),
                );
                ui.add_space(12.0);

                // Render mode
                ui.label(egui::RichText::new("RENDER MODE").small().strong().color(ACCENT));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for mode in RenderMode::ALL {
                        let selected = self.create_modal.render_mode == mode;
                        let text = egui::RichText::new(mode.label())
                            .color(if selected { GOLD } else { DIM });
                        if ui.selectable_label(selected, text).clicked() {
                            self.create_modal.render_mode = mode;
                        }
                    }
                });
                ui.add_space(12.0);

                // Card art picker
                ui.label(egui::RichText::new("VAULT SEAL").small().strong().color(ACCENT));
                ui.add_space(4.0);

                let thumb_size = 48.0;
                let images = self.create_modal.available_images.clone();
                egui::ScrollArea::horizontal().max_height(thumb_size + 12.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // "None" option
                        let is_none = self.create_modal.selected_image.is_none();
                        let resp = egui::Frame::new()
                            .fill(if is_none { egui::Color32::from_rgb(40, 35, 65) } else { egui::Color32::from_rgb(25, 25, 35) })
                            .corner_radius(6)
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(thumb_size, thumb_size));
                                ui.centered_and_justified(|ui| {
                                    ui.label(egui::RichText::new("--").color(DIM));
                                });
                            });
                        if resp.response.interact(egui::Sense::click()).clicked() {
                            self.create_modal.selected_image = None;
                        }

                        for img_name in &images {
                            let is_sel = self.create_modal.selected_image.as_deref() == Some(img_name);
                            let border = if is_sel { GOLD } else { GHOST };

                            let resp = egui::Frame::new()
                                .stroke(egui::Stroke::new(if is_sel { 2.0 } else { 0.5 }, border))
                                .corner_radius(6)
                                .show(ui, |ui| {
                                    let img = egui::Image::new(node_image_uri(img_name))
                                        .fit_to_exact_size(egui::vec2(thumb_size, thumb_size))
                                        .corner_radius(4.0);
                                    ui.add(img);
                                });
                            if resp.response.interact(egui::Sense::click()).clicked() {
                                self.create_modal.selected_image = Some(img_name.clone());
                            }
                        }
                    });
                });

                ui.add_space(20.0);

                // Bottom buttons
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("Cancel").color(DIM)).clicked() {
                        self.create_modal.open = false;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let create_text = egui::RichText::new("Create Vault")
                            .strong()
                            .color(GOLD);
                        if ui.button(create_text).clicked() {
                            let name = if self.create_modal.name.is_empty() {
                                format!("Vault {}", self.os.master_vault.vaults.len() + 1)
                            } else {
                                self.create_modal.name.clone()
                            };
                            let vtype = if self.create_modal.vault_type.is_empty() {
                                "Workspace".to_string()
                            } else {
                                self.create_modal.vault_type.clone()
                            };

                            if let Ok(id) = self.os.master_vault.create_vault(&name, &vtype) {
                                if let Some(vault) = self.os.master_vault.vault_mut(id) {
                                    vault.card_image = self.create_modal.selected_image.clone();
                                    vault.render_mode = self.create_modal.render_mode;
                                }
                            }
                            self.create_modal.open = false;
                        }
                    });
                });
            });
    }

    fn draw_inside_vault(&mut self, ctx: &egui::Context, vid: VaultId) {
        self.draw_header(ctx);
        self.draw_footer(ctx);

        // Toolbar for vault operations
        egui::TopBottomPanel::top("vault_toolbar")
            .frame(egui::Frame::new().fill(SURFACE).inner_margin(4.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("< Back").small().color(DIM)).clicked() {
                        self.view = AppView::MasterVault;
                    }

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if ui.button(egui::RichText::new("Execute").color(SUCCESS)).clicked() {
                        if let Some(vault) = self.os.master_vault.vault_mut(vid) {
                            graph::execute_graph(&mut vault.graph);
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.toggle_value(&mut self.show_boot_log, egui::RichText::new("Boot").small());
                    ui.toggle_value(&mut self.show_portals, egui::RichText::new("Portals").small());

                    ui.add_space(8.0);

                    if let Some(vault) = self.os.master_vault.vault(vid) {
                        let plugins = vault.plugin_count();
                        let atoms = vault.available_nodes().len();
                        ui.label(egui::RichText::new(format!("{plugins} plugins / {atoms} atom types")).small().color(DIM));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Install core plugins if empty
                        if let Some(vault) = self.os.master_vault.vault(vid) {
                            if vault.plugin_count() == 0 {
                                if ui.button(egui::RichText::new("Install Core Plugins").small().color(GOLD)).clicked() {
                                    if let Some(v) = self.os.master_vault.vault_mut(vid) {
                                        let _ = v.install_plugin(Box::new(create_text_plugin()));
                                        let _ = v.install_plugin(Box::new(create_math_plugin()));
                                        let _ = v.install_plugin(Box::new(create_output_plugin()));
                                    }
                                }
                            }
                        }

                        if let Some(vault) = self.os.master_vault.vault(vid) {
                            let node_count = vault.graph.node_ids().count();
                            ui.label(egui::RichText::new(format!("{node_count} atoms on graph")).small().color(DIM));
                        }
                    });
                });
            });

        // Graph canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(VOID))
            .show(ctx, |ui| {
                let available = self.os.master_vault.vault(vid)
                    .map(|v| v.available_nodes_by_category())
                    .unwrap_or_default();

                let mut viewer = PrismViewer::new(available);

                if let Some(vault) = self.os.master_vault.vault_mut(vid) {
                    vault.graph.show(
                        &mut viewer,
                        &self.snarl_style,
                        egui::Id::new("prism_snarl"),
                        ui,
                    );
                }
            });

        // Windows
        if self.show_boot_log {
            egui::Window::new("Boot Sequence").default_width(400.0).show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for entry in &self.os.boot_log {
                        let (c, icon) = match entry.status {
                            mythos::BootStatus::Success => (SUCCESS, "OK"),
                            mythos::BootStatus::Pending => (GOLD, ".."),
                            mythos::BootStatus::Failed => (egui::Color32::from_rgb(255, 70, 90), "!!"),
                        };
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("[{icon}]")).small().monospace().color(c));
                            ui.label(egui::RichText::new(format!("{:02}. {}", entry.atom_index, entry.name)).small());
                        });
                    }
                });
            });
        }

        if self.show_portals {
            egui::Window::new("Portals").default_width(360.0).show(ctx, |ui| {
                let portals = self.os.portals.all_portals();
                if portals.is_empty() {
                    ui.colored_label(DIM, "No portals configured");
                } else {
                    for p in portals {
                        let src = self.os.master_vault.vault(p.source).map(|v| v.name.as_str()).unwrap_or("?");
                        let tgt = self.os.master_vault.vault(p.target).map(|v| v.name.as_str()).unwrap_or("?");
                        let dir = if p.direction == portal::PortalDirection::Bidirectional { "<->" } else { "-->" };
                        ui.label(format!("{src} {dir} {tgt}"));
                    }
                }
            });
        }
    }
}

impl eframe::App for PrismApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.os.tick();

        // Continuous repaint for animations and live shaders
        if matches!(self.view, AppView::Splash | AppView::Fading | AppView::MasterVault) {
            ctx.request_repaint();
        }

        match self.view {
            AppView::Splash => self.draw_splash(ctx),
            AppView::Fading => self.draw_fade(ctx),
            AppView::MasterVault => self.draw_master_vault(ctx),
            AppView::InsideVault(vid) => {
                // Check vault still exists
                if self.os.master_vault.vault(vid).is_some() {
                    self.draw_inside_vault(ctx, vid);
                } else {
                    self.view = AppView::MasterVault;
                    self.draw_master_vault(ctx);
                }
            }
        }
    }
}
