// THEATRE-APP: Bevy App setup, camera, and all egui panels.
//
// Layout (top → bottom):
//   TopPanel     — header  (46 px): wordmark · beat blink · library toggle
//   BottomPanel  — footer  (38 px): channel pips · BPM readout · mixer toggle
//   BottomPanel  — board  (220 px): transport · per-channel strips (conditional)
//   SidePanel    — library (220 px): glyph list · assign · new form (conditional)
//   CentralPanel — canvas:          transparent — 3-D scene shows through
//
// BORROW PATTERN:
//   All Bevy Resource data is snapshotted into owned / Copy locals BEFORE
//   any egui closure.  Closures capture only their own flag variables by
//   &mut — no two closures touch the same variable.  After all panels return
//   the accumulated flags are applied back to the resources in one sequential
//   block.
//
// Phase 8 stubs: `midi_sync_active` and `link_sync_active` in TheatreUiState
// will be set by background threads (midir / rusty_link).

use bevy::{log::LogPlugin, prelude::*};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use biospark_theatre::{ChannelMixer, ChannelRouter, GlyphPreset, LayerType};
use myth_wire::ChannelId;

use crate::{
    bv_layer::BevyLayerPlugin,
    compositor::{TheatreMixer, TheatreRouter, TheatreState, advance_beat},
    glyph_vault::{GlyphLibrary, GlyphVaultPlugin, MixerScene, TheatreUiState},
    layers::{
        ActiveAuLayers, AuLayer, AuLayerPlugin,
        GlLayerPlugin, GlQuadConfig, PendingGlQuads,
        PendingWebViewQuads, QuadConfig, TextureQuadPlugin, WebViewLayer,
    },
    rack_ui,
};

// ── Colour palette (local — same values as rack_ui but avoids path noise) ────

const VOID:     egui::Color32 = egui::Color32::from_rgb(3,   5,  10);
const SURFACE:  egui::Color32 = egui::Color32::from_rgb(17,  24,  39);
const QUANTUM:  egui::Color32 = egui::Color32::from_rgb(0,  229, 255);
const MYTHOS:   egui::Color32 = egui::Color32::from_rgb(192, 132, 252);
const FG_2:     egui::Color32 = egui::Color32::from_rgb(148, 163, 184);
const FG_MUTED: egui::Color32 = egui::Color32::from_rgb(71,   85, 105);
const SUCCESS:  egui::Color32 = egui::Color32::from_rgb(0,  200,  90);
const WARN:     egui::Color32 = egui::Color32::from_rgb(255, 180,  40);
const DANGER:   egui::Color32 = egui::Color32::from_rgb(255,  60,  80);

fn a(c: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

// ── ActiveWebViewLayers ───────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct ActiveWebViewLayers(Vec<WebViewLayer>);

// ── TheatreApp ────────────────────────────────────────────────────────────────

pub struct TheatreApp {
    inner:           App,
    pending_webview: Vec<(WebViewLayer, f32)>,
    pending_gl:      Vec<(ChannelId, f32)>,
    pending_au:      Vec<AuLayer>,
    seed_glyphs:     Vec<GlyphPreset>,
}

impl TheatreApp {
    pub fn new(mixer: ChannelMixer, router: ChannelRouter) -> Self {
        let mut app = App::new();

        app.add_plugins(
            DefaultPlugins
                .build()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "BioSpark Theatre".into(),
                        resolution: (1280.0, 720.0).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    filter: "biospark_theatre_bevy=debug,bevy_render=warn,wgpu=error,egui=warn"
                        .into(),
                    level: bevy::log::Level::DEBUG,
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin)
        .insert_resource(ClearColor(Color::srgb(0.012, 0.020, 0.039)))
        // Neutral blue-grey ambient — previous violet tint made cubes look pink
        .insert_resource(AmbientLight {
            color:      Color::srgb(0.06, 0.07, 0.10),
            brightness: 50.0,
        })
        .insert_resource(TheatreMixer(mixer))
        .insert_resource(TheatreRouter(router))
        .insert_resource(TheatreState::default())
        .add_plugins(BevyLayerPlugin)
        .add_plugins(TextureQuadPlugin)
        .add_plugins(GlLayerPlugin)
        .add_plugins(AuLayerPlugin)
        .add_plugins(TheatrePlugin);

        Self {
            inner:           app,
            pending_webview: vec![],
            pending_gl:      vec![],
            pending_au:      vec![],
            seed_glyphs:     vec![],
        }
    }

    pub fn with_webview_layer(mut self, layer: WebViewLayer, z_depth: f32) -> Self {
        self.pending_webview.push((layer, z_depth));
        self
    }

    pub fn with_gl_layer(mut self, channel_id: ChannelId, z_depth: f32) -> Self {
        self.pending_gl.push((channel_id, z_depth));
        self
    }

    pub fn with_au_layer(mut self, layer: Option<AuLayer>) -> Self {
        if let Some(l) = layer { self.pending_au.push(l); }
        self
    }

    pub fn with_seed_glyphs(mut self, glyphs: Vec<GlyphPreset>) -> Self {
        self.seed_glyphs.extend(glyphs);
        self
    }

    pub fn run(mut self) {
        // ── WebView layers ────────────────────────────────────────────────────
        let mut active_wv  = ActiveWebViewLayers::default();
        let mut pending_wv = PendingWebViewQuads::default();
        for (layer, z_depth) in self.pending_webview.drain(..) {
            pending_wv.0.push(QuadConfig {
                channel_id:   layer.channel_id,
                frame_buffer: layer.frame_buffer.clone(),
                z_depth,
            });
            active_wv.0.push(layer);
        }

        // ── GL layers ─────────────────────────────────────────────────────────
        let mut pending_gl = PendingGlQuads::default();
        for (channel_id, z_depth) in self.pending_gl.drain(..) {
            pending_gl.0.push(GlQuadConfig { channel_id, z_depth });
        }

        // ── AU layers ─────────────────────────────────────────────────────────
        let mut active_au = ActiveAuLayers::default();
        for layer in self.pending_au.drain(..) { active_au.0.push(layer); }

        self.inner
            .insert_resource(active_wv)
            .insert_resource(pending_wv)
            .insert_resource(pending_gl)
            .insert_non_send_resource(active_au);

        // ── Seed glyph library (first run only) ───────────────────────────────
        if !self.seed_glyphs.is_empty() {
            let seeds: Vec<GlyphPreset> = self.seed_glyphs.drain(..).collect();
            let mut lib = self.inner.world_mut().resource_mut::<GlyphLibrary>();
            if lib.glyphs.is_empty() {
                tracing::info!("GlyphLibrary: seeding {} starter glyph(s)", seeds.len());
                for g in seeds { lib.add_and_save(g); }
            }
        }

        self.inner.run();
    }
}

// ── TheatrePlugin ─────────────────────────────────────────────────────────────

struct TheatrePlugin;

impl Plugin for TheatrePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GlyphVaultPlugin)
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (advance_beat, draw_ui));
    }
}

// ── Camera ────────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct TheatreCamera;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 1.5, 4.5)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        TheatreCamera,
    ));
}

// ── Per-channel mixer snapshot ────────────────────────────────────────────────

struct ChMixData {
    id:         ChannelId,
    tag:        &'static str,
    layer_type: LayerType,
    name:       String,
    user_muted: bool,
    is_soloed:  bool,
    glyph_name: Option<String>,
}

// ── draw_ui ───────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn draw_ui(
    mut contexts:  EguiContexts,
    mut state:     ResMut<TheatreState>,
    mut mixer:     ResMut<TheatreMixer>,
    mut library:   ResMut<GlyphLibrary>,
    mut ui_state:  ResMut<TheatreUiState>,
) {
    let ctx = contexts.ctx_mut();

    // Apply the BioSpark Rack Design System theme
    rack_ui::apply_rack_theme(ctx);

    // ══════════════════════════════════════════════════════════════════════════
    // SNAPSHOT — extract all data from Resources before any closures
    // ══════════════════════════════════════════════════════════════════════════

    let beat       = state.beat;
    let tick       = state.tick;
    let tempo      = state.tempo_bpm;
    let bar_count  = state.bar_count;
    let is_playing = state.is_playing;

    let lib_open   = ui_state.library_open;
    let mixer_open = ui_state.mixer_open;

    let sel_ch    = ui_state.selected_channel;
    let sel_gl    = ui_state.selected_glyph;
    let new_form_open = ui_state.new_form_open;
    let glyph_snap: Vec<GlyphPreset> = library.glyphs.clone();

    let sel_ch_type:  Option<LayerType> = sel_ch.and_then(|id| mixer.0.channel(id)).map(|ch| ch.layer_type);
    let sel_ch_name:  Option<String>    = sel_ch.and_then(|id| mixer.0.channel(id)).map(|ch| ch.name.clone());
    let sel_ch_glyph: Option<GlyphPreset> = sel_ch.and_then(|id| mixer.0.channel(id)).and_then(|ch| ch.glyph.clone());
    let can_save_ch  = sel_ch_glyph.is_some();
    let can_assign: bool = match (sel_ch_type, sel_gl) {
        (Some(t), Some(i)) => glyph_snap.get(i)
            .map(|g| g.layer_type == t && g.code.is_some()).unwrap_or(false),
        _ => false,
    };
    let assign_label: String = sel_ch
        .map(|id| format!("ASSIGN → {id}"))
        .unwrap_or_else(|| "ASSIGN".into());

    // Mixer channel snapshots
    let ch_mix_data: Vec<ChMixData> = mixer.0.all_channels().iter().map(|ch| ChMixData {
        id:         ch.id,
        tag:        ch.layer_type.tag(),
        layer_type: ch.layer_type,
        name:       ch.name.clone(),
        user_muted: *ui_state.user_mutes.get(&ch.id).unwrap_or(&false),
        is_soloed:  Some(ch.id) == ui_state.solo_channel,
        glyph_name: ch.glyph.as_ref().filter(|g| g.code.is_some()).map(|g| g.name.clone()),
    }).collect();

    // Per-channel fader values (user-intended).
    // GL channels default to 0.30 — subtle ambient; everything else to 0.85.
    let mut ch_levels: Vec<f32> = ch_mix_data.iter().map(|c| {
        *ui_state.channel_levels.get(&c.id).unwrap_or(
            if c.layer_type == LayerType::Gl { &0.30 } else { &0.85 }
        )
    }).collect();

    let ch_snap: Vec<(ChannelId, &'static str, bool, bool)> = ch_mix_data.iter()
        .map(|c| (c.id, c.tag, c.user_muted, Some(c.id) == sel_ch))
        .collect();

    let mut master_level = ui_state.master_level;
    let master_muted     = ui_state.master_muted;

    let scenes_filled: [bool; 4] = std::array::from_fn(|i| ui_state.scenes[i].is_some());

    let midi_sync = ui_state.midi_sync_active;
    let link_sync = ui_state.link_sync_active;

    let mut new_name  = ui_state.new_name.clone();
    let mut new_code  = ui_state.new_code.clone();
    let mut new_layer = ui_state.new_layer;

    let mut bpm_value = tempo;

    // ══════════════════════════════════════════════════════════════════════════
    // ACTION FLAGS
    // ══════════════════════════════════════════════════════════════════════════

    let mut toggle_library  = false;
    let mut toggle_mixer    = false;
    let mut close_library   = false;
    let mut new_sel_ch:      Option<Option<ChannelId>> = None;
    let mut new_sel_gl:      Option<Option<usize>>     = None;
    let mut toggle_new_form = false;
    let mut assign_clicked  = false;
    let mut save_ch_clicked = false;
    let mut save_new_clicked= false;

    let mut rewind_clicked  = false;
    let mut toggle_play     = false;
    let mut stop_clicked    = false;
    let mut ffwd_clicked    = false;
    let mut tap_clicked     = false;
    let mut bpm_up          = false;
    let mut bpm_down        = false;

    let mut mute_toggles:    Vec<usize>    = Vec::new();
    let mut solo_click:      Option<usize> = None;
    let mut toggle_master_muted            = false;
    let mut panic_clicked                  = false;
    let mut recall_scene:    Option<usize> = None;
    let mut save_scene:      Option<usize> = None;

    // ══════════════════════════════════════════════════════════════════════════
    // PANELS
    // ══════════════════════════════════════════════════════════════════════════

    // ── HEADER ─────────────────────────────────────────────────────────────────
    egui::TopBottomPanel::top("theatre_header")
        .exact_height(46.0)
        .frame(egui::Frame::none()
            .fill(egui::Color32::from_rgba_unmultiplied(3, 5, 10, 230))
            .inner_margin(egui::Margin::symmetric(14.0, 0.0)))
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(egui::RichText::new("BIOSPARK").monospace().size(11.0).color(a(MYTHOS, 150)));
                ui.label(egui::RichText::new("THEATRE").monospace().size(11.0).color(a(QUANTUM, 210)));
                ui.add_space(16.0);
                ui.label(egui::RichText::new("BV").monospace().size(9.0).color(a(MYTHOS, 160)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("BEAT {:.2}", beat))
                        .monospace().size(10.0).color(a(QUANTUM, 120)));
                    ui.add_space(8.0);

                    let blink = if tick % 60 < 30 { 230u8 } else { 70u8 };
                    ui.colored_label(
                        egui::Color32::from_rgba_unmultiplied(0, 220, 100, blink), "●");
                    ui.add_space(10.0);

                    let lib_icon  = if lib_open { "⊟" } else { "⊞" };
                    let lib_color = if lib_open { QUANTUM } else { a(QUANTUM, 120) };
                    if ui.add(egui::Button::new(
                        egui::RichText::new(lib_icon).monospace().size(14.0).color(lib_color)
                    ).frame(false)).clicked() { toggle_library = true; }
                    ui.add_space(4.0);

                    if link_sync {
                        ui.label(egui::RichText::new("LINK").monospace().size(8.0).color(a(SUCCESS, 180)));
                        ui.add_space(4.0);
                    }
                    if midi_sync {
                        ui.label(egui::RichText::new("MIDI").monospace().size(8.0).color(a(WARN, 180)));
                        ui.add_space(4.0);
                    }
                });
            });
        });

    // ── FOOTER ─────────────────────────────────────────────────────────────────
    egui::TopBottomPanel::bottom("theatre_footer")
        .exact_height(38.0)
        .frame(egui::Frame::none()
            .fill(egui::Color32::from_rgba_unmultiplied(3, 5, 10, 230))
            .inner_margin(egui::Margin::symmetric(14.0, 0.0)))
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                for &(ch_id, ch_tag, ch_muted, ch_selected) in &ch_snap {
                    let dot_color = if ch_selected { SUCCESS }
                                   else if ch_muted { a(DANGER, 140) }
                                   else { a(QUANTUM, 120) };
                    let resp = ui.add(
                        egui::Label::new(egui::RichText::new("●").color(dot_color))
                            .sense(egui::Sense::click()));
                    if resp.clicked() {
                        new_sel_ch = Some(if ch_selected { None } else { Some(ch_id) });
                    }
                    ui.label(egui::RichText::new(ch_tag).monospace().size(9.0)
                        .color(if ch_selected { a(SUCCESS, 200) } else { a(FG_MUTED, 180) }));
                    ui.add_space(6.0);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("BIOSPARK STUDIOS · QUANTUM THEATRE · v0.5.0")
                        .monospace().size(8.0).color(a(FG_MUTED, 80)));
                    ui.add_space(12.0);

                    let mix_icon  = if mixer_open { "⊟" } else { "⊞" };
                    let mix_color = if mixer_open { WARN } else { a(WARN, 100) };
                    if ui.add(egui::Button::new(
                        egui::RichText::new(mix_icon).monospace().size(13.0).color(mix_color)
                    ).frame(false)).clicked() { toggle_mixer = true; }
                    ui.add_space(8.0);

                    ui.label(egui::RichText::new(format!("{:.1} BPM", tempo))
                        .monospace().size(10.0).color(a(FG_2, 140)));
                    ui.add_space(8.0);

                    if !is_playing {
                        ui.label(egui::RichText::new("⏸ PAUSED").monospace().size(8.0).color(a(WARN, 180)));
                        ui.add_space(6.0);
                    }
                    if let Some(ref name) = sel_ch_name {
                        ui.label(egui::RichText::new(format!("[ {name} ]"))
                            .monospace().size(9.0).color(a(SUCCESS, 160)));
                        ui.add_space(6.0);
                    }
                });
            });
        });

    // ── MIXING BOARD ────────────────────────────────────────────────────────────
    if mixer_open {
        egui::TopBottomPanel::bottom("mixer_board")
            .exact_height(220.0)
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgba_unmultiplied(7, 9, 15, 252))
                .inner_margin(egui::Margin::same(10.0)))
            .show(ctx, |ui| {

                // Gold corner marks + WARN left accent bar
                let panel_rect = ui.max_rect();
                rack_ui::board_decorations(ui, panel_rect, WARN);

                // ── Row 1: board header + master knob + scenes ─────────────────
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("CHANNEL BOARD")
                        .monospace().size(9.0).color(a(WARN, 200)));
                    ui.add_space(8.0);

                    // Master knob (xs, no label — label is the "MASTER" text beside it)
                    ui.label(egui::RichText::new("MASTER").monospace().size(7.0).color(a(FG_MUTED, 200)));
                    rack_ui::knob_xs(ui, &mut master_level, "", QUANTUM);
                    ui.label(egui::RichText::new(format!("{:.0}%", master_level * 100.0))
                        .monospace().size(7.0).color(a(QUANTUM, 160)));
                    ui.add_space(6.0);

                    // MUTE AU
                    let mute_lbl = if master_muted { "▪ MUTED" } else { "MUTE AU" };
                    if rack_ui::rack_pad(ui, mute_lbl, master_muted, DANGER, 48.0, 18.0) {
                        toggle_master_muted = true;
                    }
                    ui.add_space(4.0);

                    // Panic
                    if rack_ui::rack_pad(ui, "⚡ PANIC", false, DANGER, 50.0, 18.0) {
                        panic_clicked = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Scene save pads (right-to-left so order reads A→D left-to-right)
                        for i in (0..4usize).rev() {
                            let lbl = ["⬇A","⬇B","⬇C","⬇D"][i];
                            if rack_ui::scene_pad(ui, lbl, false, a(QUANTUM, 120), 26.0, 16.0) {
                                save_scene = Some(i);
                            }
                        }
                        ui.label(egui::RichText::new("SAVE").monospace().size(7.0).color(a(FG_MUTED, 120)));
                        ui.add_space(8.0);

                        // Scene recall pads
                        for i in (0..4usize).rev() {
                            let lbl = ["A","B","C","D"][i];
                            if rack_ui::scene_pad(ui, lbl, scenes_filled[i], QUANTUM, 22.0, 16.0) && scenes_filled[i] {
                                recall_scene = Some(i);
                            }
                        }
                        ui.label(egui::RichText::new("SCENES").monospace().size(7.0).color(a(FG_MUTED, 120)));
                        ui.add_space(8.0);
                    });
                });

                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);

                // ── Row 2: transport (left) + channel strips (scrollable) ──────
                ui.horizontal(|ui| {

                    // ── TRANSPORT ──────────────────────────────────────────────
                    ui.vertical(|ui| {
                        ui.set_width(196.0);

                        // BPM row
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("BPM").monospace().size(8.0).color(a(FG_MUTED, 180)));
                            ui.add(egui::DragValue::new(&mut bpm_value)
                                .speed(0.5)
                                .range(20.0_f32..=300.0)
                                .fixed_decimals(1));
                            if ui.small_button("−").clicked() { bpm_down = true; }
                            if ui.small_button("+").clicked() { bpm_up   = true; }
                        });

                        ui.add_space(2.0);

                        // TAP TEMPO pad
                        if rack_ui::rack_pad(ui, "TAP TEMPO", false, QUANTUM, 96.0, 18.0) {
                            tap_clicked = true;
                        }

                        ui.add_space(3.0);

                        // Transport pads
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
                            if rack_ui::transport_pad(ui, "◀◀", false, QUANTUM) { rewind_clicked = true; }

                            let (play_icon, play_col) = if is_playing {
                                ("⏸", SUCCESS)
                            } else {
                                ("▶", a(SUCCESS, 170).into())
                            };
                            if rack_ui::transport_pad(ui, play_icon, is_playing, play_col) { toggle_play = true; }
                            if rack_ui::transport_pad(ui, "■",  false,     DANGER)  { stop_clicked   = true; }
                            if rack_ui::transport_pad(ui, "▶▶", false,     QUANTUM) { ffwd_clicked   = true; }
                        });

                        ui.add_space(4.0);

                        // Beat progress bar (rack-style segmented VU)
                        rack_ui::vu_horiz(ui, beat, a(QUANTUM, 160), 182.0);

                        ui.add_space(3.0);

                        // Bar + tick counters
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("BAR {:5}", bar_count))
                                .monospace().size(8.0).color(a(FG_2, 160)));
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new(format!("TK {:6}", tick))
                                .monospace().size(8.0).color(a(FG_MUTED, 120)));
                        });
                    });

                    ui.separator();

                    // ── CHANNEL STRIPS ─────────────────────────────────────────
                    egui::ScrollArea::horizontal()
                        .id_source("ch_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(6.0, 3.0);

                                for (i, ch) in ch_mix_data.iter().enumerate() {
                                    let col       = rack_ui::layer_accent(ch.tag);
                                    let is_soloed = ch.is_soloed;
                                    let solo_active = ui_state.solo_channel.is_some();
                                    let muted_eff   = ch.user_muted || (solo_active && !is_soloed);

                                    // Channel strip frame
                                    egui::Frame::none()
                                        .fill(egui::Color32::from_rgba_unmultiplied(13, 17, 23, 220))
                                        .stroke(egui::Stroke::new(1.0,
                                            if muted_eff { a(DANGER, 40) }
                                            else if is_soloed { a(WARN, 80) }
                                            else { a(col, 30) }
                                        ))
                                        .inner_margin(egui::Margin::symmetric(5.0, 4.0))
                                        .rounding(egui::Rounding::same(3.0))
                                        .show(ui, |ui| {
                                            ui.set_width(134.0);

                                            // Type badge + channel name
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
                                                rack_ui::led(ui, !muted_eff, col);
                                                ui.label(egui::RichText::new(ch.tag)
                                                    .monospace().size(8.0).color(col));
                                                ui.label(egui::RichText::new(&ch.name)
                                                    .monospace().size(8.0).color(a(FG_2, 160)));
                                            });

                                            ui.add_space(2.0);

                                            // Level fader (rack-style horizontal)
                                            rack_ui::fader_horiz(ui, &mut ch_levels[i], col, 134.0);

                                            ui.add_space(2.0);

                                            // VU meter
                                            let vu_level = if muted_eff { 0.0 } else { ch_levels[i] };
                                            let vu_col   = if muted_eff { a(DANGER, 100) } else { a(col, 200) };
                                            rack_ui::vu_horiz(ui, vu_level, vu_col, 134.0);

                                            ui.add_space(2.0);

                                            // MUTE + SOLO pads
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
                                                if rack_ui::rack_pad(ui,
                                                    if ch.user_muted { "MUTED" } else { "M" },
                                                    ch.user_muted, DANGER, 36.0, 14.0,
                                                ) { mute_toggles.push(i); }

                                                if rack_ui::rack_pad(ui,
                                                    if is_soloed { "SOLO!" } else { "S" },
                                                    is_soloed, WARN, 36.0, 14.0,
                                                ) { solo_click = Some(i); }
                                            });

                                            ui.add_space(2.0);

                                            // Loaded glyph name
                                            let gname = ch.glyph_name.as_deref().unwrap_or("— none —");
                                            ui.label(egui::RichText::new(gname)
                                                .monospace().size(7.0).color(a(FG_MUTED, 110)));
                                        });
                                }
                            });
                        });
                });
            });
    }

    // ── GLYPH LIBRARY PANEL ────────────────────────────────────────────────────
    if lib_open {
        egui::SidePanel::right("glyph_library")
            .min_width(200.0).default_width(220.0)
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgba_unmultiplied(10, 14, 24, 245))
                .inner_margin(egui::Margin::same(10.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("GLYPH LIBRARY")
                        .monospace().size(9.0).color(a(QUANTUM, 200)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("×").clicked() { close_library = true; }
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_source("glyph_scroll").max_height(190.0)
                    .show(ui, |ui| {
                        if glyph_snap.is_empty() {
                            ui.label(egui::RichText::new("No glyphs yet")
                                .size(8.0).color(a(FG_MUTED, 120)));
                        }
                        for (idx, glyph) in glyph_snap.iter().enumerate() {
                            let g_sel      = sel_gl == Some(idx);
                            let type_match = sel_ch_type == Some(glyph.layer_type);
                            let col = if g_sel { QUANTUM }
                                      else if type_match { a(QUANTUM, 160) }
                                      else { a(FG_MUTED, 160) };
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(glyph.layer_type.tag())
                                    .monospace().size(8.0).color(a(MYTHOS, 160)));
                                if ui.add(egui::Label::new(
                                    egui::RichText::new(&glyph.name).monospace().size(9.0).color(col)
                                ).sense(egui::Sense::click())).clicked() {
                                    new_sel_gl = Some(if g_sel { None } else { Some(idx) });
                                }
                                if glyph.code.is_none() {
                                    ui.label(egui::RichText::new("⟳").size(8.0).color(a(FG_MUTED, 120)));
                                }
                            });
                        }
                    });

                ui.separator();

                ui.add_enabled_ui(can_assign, |ui| {
                    if ui.button(egui::RichText::new(&assign_label).monospace().size(9.0)).clicked() {
                        assign_clicked = true;
                    }
                });
                if !can_assign {
                    let hint = if sel_ch.is_none() { "click a pip to select channel" }
                               else if sel_gl.is_none() { "click a glyph" }
                               else { "layer type mismatch" };
                    ui.label(egui::RichText::new(hint).size(8.0).color(a(FG_MUTED, 100)));
                }

                ui.separator();

                ui.add_enabled_ui(can_save_ch, |ui| {
                    if ui.button(egui::RichText::new("SAVE CH GLYPH").monospace().size(9.0)).clicked() {
                        save_ch_clicked = true;
                    }
                });

                ui.separator();

                let nf_lbl = if new_form_open { "▲ NEW GLYPH" } else { "▼ NEW GLYPH" };
                if ui.button(egui::RichText::new(nf_lbl).monospace().size(9.0)
                    .color(a(QUANTUM, 180))).clicked() { toggle_new_form = true; }

                if new_form_open {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Name").size(8.0).color(a(FG_MUTED, 180)));
                    ui.add(egui::TextEdit::singleline(&mut new_name)
                        .hint_text("glyph name").desired_width(f32::INFINITY));
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Type").size(8.0).color(a(FG_MUTED, 180)));
                    ui.horizontal(|ui| {
                        for lt in [LayerType::P5, LayerType::Gl, LayerType::Ht, LayerType::Au] {
                            if ui.selectable_label(new_layer == lt,
                                egui::RichText::new(lt.tag()).monospace().size(9.0)).clicked() {
                                new_layer = lt;
                            }
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Code").size(8.0).color(a(FG_MUTED, 180)));
                    ui.add(egui::TextEdit::multiline(&mut new_code)
                        .hint_text("paste code here").desired_rows(6)
                        .code_editor().desired_width(f32::INFINITY));
                    ui.add_space(4.0);
                    let can_create = !new_name.trim().is_empty() && !new_code.trim().is_empty();
                    ui.add_enabled_ui(can_create, |ui| {
                        if ui.button(egui::RichText::new("SAVE GLYPH")
                            .monospace().size(9.0).color(QUANTUM)).clicked() {
                            save_new_clicked = true;
                        }
                    });
                }
            });
    }

    // ── CANVAS ────────────────────────────────────────────────────────────────
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |_ui| {});

    // ══════════════════════════════════════════════════════════════════════════
    // APPLY ACTIONS — all closures returned, Resources are free to mutate
    // ══════════════════════════════════════════════════════════════════════════

    if toggle_library { ui_state.library_open = !ui_state.library_open; }
    if toggle_mixer   { ui_state.mixer_open   = !ui_state.mixer_open;   }
    if close_library  { ui_state.library_open = false; }
    if toggle_new_form { ui_state.new_form_open = !ui_state.new_form_open; }

    if let Some(s) = new_sel_ch { ui_state.selected_channel = s; }
    if let Some(s) = new_sel_gl { ui_state.selected_glyph   = s; }

    ui_state.new_name  = new_name;
    ui_state.new_code  = new_code;
    ui_state.new_layer = new_layer;

    // Transport
    if toggle_play    { state.is_playing = !state.is_playing; }
    if stop_clicked   { state.is_playing = false; state.beat = 0.0; state.bar_count = 0; }
    if rewind_clicked { state.beat = 0.0; }
    if ffwd_clicked   { state.bar_count = state.bar_count.wrapping_add(1); state.beat = 0.0; }

    // BPM (DragValue wins over +/− buttons; tap wins over both)
    state.tempo_bpm = bpm_value.clamp(20.0, 300.0);
    if bpm_down { state.tempo_bpm = (state.tempo_bpm - 1.0).max(20.0); }
    if bpm_up   { state.tempo_bpm = (state.tempo_bpm + 1.0).min(300.0); }

    // Tap tempo
    if tap_clicked {
        let now = std::time::Instant::now();
        ui_state.tap_times.retain(|&t| now.duration_since(t).as_secs_f32() < 3.0);
        ui_state.tap_times.push(now);
        let n = ui_state.tap_times.len();
        if n >= 2 {
            let span = ui_state.tap_times[n - 1]
                .duration_since(ui_state.tap_times[0]).as_secs_f32();
            state.tempo_bpm = (60.0 / (span / (n - 1) as f32)).clamp(20.0, 300.0);
        }
    }

    // Master controls
    ui_state.master_level = master_level;
    if toggle_master_muted { ui_state.master_muted = !ui_state.master_muted; }

    // Panic
    if panic_clicked {
        for ch_info in &ch_mix_data {
            ui_state.channel_levels.insert(ch_info.id, 1.0);
            ui_state.user_mutes.insert(ch_info.id, false);
        }
        ui_state.solo_channel = None;
        ui_state.master_level = 1.0;
        ui_state.master_muted = false;
    }

    // Per-channel mute toggles
    for i in mute_toggles {
        let id      = ch_mix_data[i].id;
        let current = *ui_state.user_mutes.get(&id).unwrap_or(&false);
        ui_state.user_mutes.insert(id, !current);
    }

    // Solo toggle
    if let Some(i) = solo_click {
        let id = ch_mix_data[i].id;
        ui_state.solo_channel = if ui_state.solo_channel == Some(id) { None } else { Some(id) };
    }

    // Scene save
    if let Some(slot) = save_scene {
        let channels: Vec<(ChannelId, f32, bool)> = ch_mix_data.iter().enumerate()
            .map(|(i, c)| (c.id, ch_levels[i], *ui_state.user_mutes.get(&c.id).unwrap_or(&false)))
            .collect();
        ui_state.scenes[slot] = Some(MixerScene { bpm: state.tempo_bpm, channels });
        tracing::info!("Scene {} saved", ["A","B","C","D"][slot]);
    }

    // Scene recall
    if let Some(slot) = recall_scene {
        if let Some(scene) = ui_state.scenes[slot].clone() {
            state.tempo_bpm = scene.bpm;
            for (ch_id, level, muted) in scene.channels {
                ui_state.channel_levels.insert(ch_id, level);
                ui_state.user_mutes.insert(ch_id, muted);
            }
            tracing::info!("Scene {} recalled", ["A","B","C","D"][slot]);
        }
    }

    // ── Write effective levels + mutes to the mixer every frame ───────────────
    let solo_active = ui_state.solo_channel;
    let ml          = ui_state.master_level;
    let au_muted    = ui_state.master_muted;

    for (i, ch_info) in ch_mix_data.iter().enumerate() {
        let user_level = ch_levels[i];
        ui_state.channel_levels.insert(ch_info.id, user_level);

        if let Some(ch) = mixer.0.channel_mut(ch_info.id) {
            ch.level = (user_level * ml).clamp(0.0, 1.0);

            let user_muted    = *ui_state.user_mutes.get(&ch_info.id).unwrap_or(&false);
            let solo_muted    = solo_active.is_some() && solo_active != Some(ch_info.id);
            let master_au_cut = au_muted && ch_info.layer_type == LayerType::Au;
            ch.muted = user_muted || solo_muted || master_au_cut;
        }
    }

    // ── Glyph library mutations ────────────────────────────────────────────────
    if assign_clicked {
        if let (Some(ch_id), Some(idx)) = (ui_state.selected_channel, ui_state.selected_glyph) {
            if let Some(glyph) = library.glyphs.get(idx).cloned() {
                if let Some(ch) = mixer.0.channel_mut(ch_id) {
                    match ch.drop_glyph(glyph) {
                        Ok(())  => tracing::info!("Glyph assigned to {ch_id}"),
                        Err(e)  => tracing::warn!("drop_glyph: {e}"),
                    }
                }
            }
        }
    }

    if save_ch_clicked {
        if let Some(g) = sel_ch_glyph { library.add_and_save(g); }
    }

    if save_new_clicked {
        let glyph = GlyphPreset::new_inline(
            ui_state.new_name.clone(), ui_state.new_layer, ui_state.new_code.clone());
        library.add_and_save(glyph);
        ui_state.new_name.clear();
        ui_state.new_code.clear();
        ui_state.new_form_open = false;
    }
}
