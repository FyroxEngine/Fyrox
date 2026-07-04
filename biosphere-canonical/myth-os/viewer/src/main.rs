mod field;
mod generate;
mod obj;
mod orbit;

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        mesh::Indices,
        render_asset::RenderAssetUsages,
        render_resource::PrimitiveTopology,
    },
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use ndarray::Array3;
use std::path::PathBuf;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "myth-os viewer".into(),
                    resolution: (1440.0, 900.0).into(),
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins(EguiPlugin)
        .insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.07)))
        .init_resource::<ViewerState>()
        .init_resource::<orbit::Orbit>()
        .init_resource::<generate::GenState>()
        .init_resource::<SceneState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                draw_ui,
                orbit::run.after(draw_ui),
                field::refresh_texture,
                apply_load_requests,
                generate::check_status,
                update_sun.after(draw_ui),
                update_material.after(apply_load_requests),
            ),
        )
        .run();
}

// ── Scene state (user-controlled) ────────────────────────────────────────────

/// Everything the user can tweak about the scene's lighting and material.
/// Lives as a Resource so systems and the UI can both access it.
#[derive(Resource)]
pub struct SceneState {
    // Sun
    pub sun_azimuth:    f32,  // 0..360 degrees, clockwise from north
    pub sun_elevation:  f32,  // 0..90 degrees above horizon
    pub sun_illuminance: f32, // lux
    pub sun_color:      [f32; 3],
    pub shadows:        bool,

    // Material
    pub base_color:  [f32; 3],
    pub roughness:   f32,
    pub metallic:    f32,
    pub tex_path:    Option<PathBuf>,
    pub mat_dirty:   bool,
}

impl Default for SceneState {
    fn default() -> Self {
        Self {
            sun_azimuth:     45.0,
            sun_elevation:   40.0,
            sun_illuminance: 12_000.0,
            sun_color:       [1.0, 0.96, 0.88],
            shadows:         true,
            base_color:      [0.55, 0.72, 0.88],
            roughness:       0.65,
            metallic:        0.05,
            tex_path:        None,
            mat_dirty:       false,
        }
    }
}

// ── Viewer state ──────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct ViewerState {
    pub mode: Mode,

    // mesh
    pub obj_path:    Option<PathBuf>,
    pub mesh_ent:    Option<Entity>,
    pub mesh_stats:  Option<(usize, usize)>,
    pub obj_pending: bool,
    pub mat_handle:  Option<Handle<StandardMaterial>>,

    // field
    pub raw_path:    Option<PathBuf>,
    pub field:       Option<Array3<f32>>,
    pub field_res:   u32,
    pub axis:        Axis,
    pub slice:       u32,
    pub tex:         Option<Handle<Image>>,
    pub tex_dirty:   bool,
    pub raw_pending: bool,

    pub status: String,
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum Mode {
    #[default]
    Mesh,
    Field,
    Generate,
    Scene,
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum Axis {
    X,
    Y,
    #[default]
    Z,
}

#[derive(Component)]
struct MeshPreview;

#[derive(Component)]
struct SunLight;

// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup(mut commands: Commands, scene: Res<SceneState>) {
    commands.spawn(Camera3dBundle {
        transform:   Transform::from_xyz(0.0, 1.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        tonemapping: Tonemapping::AcesFitted,
        ..default()
    });

    // Sun — spawned from SceneState defaults so the UI is immediately in sync
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color:       Color::srgb(scene.sun_color[0], scene.sun_color[1], scene.sun_color[2]),
                illuminance: scene.sun_illuminance,
                shadows_enabled: scene.shadows,
                ..default()
            },
            transform: sun_transform(scene.sun_azimuth, scene.sun_elevation),
            ..default()
        },
        SunLight,
    ));

    // Soft fill light (not user-controllable for now — ambient substitute)
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 800.0,
            range:     200.0,
            color:     Color::srgb(0.5, 0.6, 0.9),
            ..default()
        },
        transform: Transform::from_xyz(-8.0, 5.0, -8.0),
        ..default()
    });
}

// ── Sun math ──────────────────────────────────────────────────────────────────

fn sun_transform(azimuth_deg: f32, elevation_deg: f32) -> Transform {
    let az  = azimuth_deg.to_radians();
    let el  = elevation_deg.to_radians();
    // Direction the light travels (pointing inward toward scene centre)
    let dir = Vec3::new(
        -az.sin() * el.cos(),
        -el.sin(),
        -az.cos() * el.cos(),
    ).normalize();
    Transform::from_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, dir))
}

// ── System: update sun from SceneState ───────────────────────────────────────

fn update_sun(
    scene: Res<SceneState>,
    mut q:  Query<(&mut DirectionalLight, &mut Transform), With<SunLight>>,
) {
    if !scene.is_changed() { return; }
    let Ok((mut light, mut xf)) = q.get_single_mut() else { return };
    light.color            = Color::srgb(scene.sun_color[0], scene.sun_color[1], scene.sun_color[2]);
    light.illuminance      = scene.sun_illuminance;
    light.shadows_enabled  = scene.shadows;
    *xf = sun_transform(scene.sun_azimuth, scene.sun_elevation);
}

// ── System: update material from SceneState ───────────────────────────────────

fn update_material(
    mut scene:     ResMut<SceneState>,
    viewer:        Res<ViewerState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !scene.mat_dirty { return; }
    scene.mat_dirty = false;

    let Some(handle) = &viewer.mat_handle else { return };
    let Some(mat)    = materials.get_mut(handle) else { return };

    mat.base_color         = Color::srgb(scene.base_color[0], scene.base_color[1], scene.base_color[2]);
    mat.perceptual_roughness = scene.roughness;
    mat.metallic           = scene.metallic;
}

// ── UI ────────────────────────────────────────────────────────────────────────

fn draw_ui(
    mut contexts: EguiContexts,
    mut state:    ResMut<ViewerState>,
    mut orbit:    ResMut<orbit::Orbit>,
    mut gen:      ResMut<generate::GenState>,
    mut scene:    ResMut<SceneState>,
) {
    let tex_id = state.tex.as_ref().map(|h| contexts.add_image(h.clone_weak()));
    let ctx    = contexts.ctx_mut();

    // ── Left sidebar ─────────────────────────────────────────────────────────
    egui::SidePanel::left("controls")
        .min_width(260.0)
        .max_width(320.0)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.heading("myth-os viewer");
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Tab row
            ui.horizontal_wrapped(|ui| {
                for (label, m) in [
                    ("Mesh",     Mode::Mesh),
                    ("Field",    Mode::Field),
                    ("Generate", Mode::Generate),
                    ("Scene",    Mode::Scene),
                ] {
                    if ui.selectable_label(state.mode == m, label).clicked() {
                        state.mode = m;
                    }
                }
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            match state.mode {
                Mode::Mesh     => mesh_panel(ui, &mut state, &mut orbit),
                Mode::Field    => field_panel(ui, &mut state),
                Mode::Generate => {
                    if generate::panel(ui, &mut gen) {
                        generate::launch(&mut gen);
                    }
                }
                Mode::Scene    => scene_panel(ui, &mut scene),
            }

            // Status bar at bottom of sidebar
            let msg = match state.mode {
                Mode::Generate if !gen.msg.is_empty() => gen.msg.clone(),
                _ if !state.status.is_empty()         => state.status.clone(),
                _                                     => String::new(),
            };
            if !msg.is_empty() {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.separator();
                    ui.label(egui::RichText::new(&msg).weak().small());
                });
            }
        });

    // ── Central panel — field slice viewer ───────────────────────────────────
    if state.mode == Mode::Field {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(10, 10, 16))
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                if let Some(tid) = tex_id {
                    let avail = ui.available_size();
                    let dim   = avail.x.min(avail.y);
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(
                        tid,
                        egui::Vec2::splat(dim),
                    )));
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("Open a .raw file to view a slice.").weak());
                    });
                }
            });
    }
}

// ── Mesh panel ────────────────────────────────────────────────────────────────

fn mesh_panel(ui: &mut egui::Ui, state: &mut ViewerState, orbit: &mut orbit::Orbit) {
    if ui.button("Open .obj …").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Wavefront OBJ", &["obj"])
            .pick_file()
        {
            state.obj_path    = Some(path);
            state.obj_pending = true;
        }
    }
    if let Some(p) = &state.obj_path {
        ui.label(
            egui::RichText::new(p.file_name().unwrap_or_default().to_string_lossy().as_ref())
                .weak().small(),
        );
    }
    if let Some((v, t)) = state.mesh_stats {
        ui.label(format!("{v} vertices  /  {t} triangles"));
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Camera").strong());
    ui.label("Left-drag  → orbit");
    ui.label("Scroll     → zoom");
    ui.add_space(4.0);
    if ui.button("Reset camera").clicked() {
        *orbit = orbit::Orbit::default();
    }
}

// ── Field panel ───────────────────────────────────────────────────────────────

fn field_panel(ui: &mut egui::Ui, state: &mut ViewerState) {
    ui.label("Resolution (voxels per axis):");
    let mut res_str = if state.field_res > 0 { state.field_res.to_string() } else { String::new() };
    if ui.text_edit_singleline(&mut res_str).changed() {
        if let Ok(n) = res_str.parse::<u32>() { state.field_res = n; }
    }
    if ui.button("Open .raw …").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Raw float32 field", &["raw"])
            .pick_file()
        {
            state.raw_path    = Some(path);
            state.raw_pending = true;
        }
    }
    if let Some(p) = &state.raw_path {
        ui.label(
            egui::RichText::new(p.file_name().unwrap_or_default().to_string_lossy().as_ref())
                .weak().small(),
        );
    }
    if state.field.is_some() {
        ui.add_space(4.0);
        ui.separator();
        ui.label("Axis:");
        ui.horizontal(|ui| {
            for (label, ax) in [("X", Axis::X), ("Y", Axis::Y), ("Z", Axis::Z)] {
                if ui.selectable_label(state.axis == ax, label).clicked() {
                    state.axis      = ax;
                    state.slice     = state.field_res / 2;
                    state.tex_dirty = true;
                }
            }
        });
        let max_slice = state.field_res.saturating_sub(1);
        let prev = state.slice;
        ui.add(egui::Slider::new(&mut state.slice, 0..=max_slice).text("Slice"));
        if state.slice != prev { state.tex_dirty = true; }
    }
}

// ── Scene panel (light + material) ───────────────────────────────────────────

fn scene_panel(ui: &mut egui::Ui, scene: &mut SceneState) {
    // ── Sun ──────────────────────────────────────────────────────────────────
    ui.label(egui::RichText::new("Sun").strong());
    ui.add_space(2.0);

    ui.add(
        egui::Slider::new(&mut scene.sun_azimuth, 0.0..=360.0)
            .text("Azimuth °")
            .step_by(1.0),
    );
    ui.add(
        egui::Slider::new(&mut scene.sun_elevation, 0.0..=90.0)
            .text("Elevation °")
            .step_by(0.5),
    );
    ui.add(
        egui::Slider::new(&mut scene.sun_illuminance, 500.0..=120_000.0)
            .text("Illuminance (lux)")
            .logarithmic(true),
    );

    ui.horizontal(|ui| {
        ui.label("Color");
        egui::color_picker::color_edit_button_rgb(ui, &mut scene.sun_color);
    });

    ui.checkbox(&mut scene.shadows, "Cast shadows");

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);

    // ── Material ─────────────────────────────────────────────────────────────
    ui.label(egui::RichText::new("Material").strong());
    ui.add_space(2.0);

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("Base color");
        changed |= egui::color_picker::color_edit_button_rgb(ui, &mut scene.base_color).changed();
    });

    changed |= ui.add(
        egui::Slider::new(&mut scene.roughness, 0.0..=1.0)
            .text("Roughness")
            .step_by(0.01),
    ).changed();

    changed |= ui.add(
        egui::Slider::new(&mut scene.metallic, 0.0..=1.0)
            .text("Metallic")
            .step_by(0.01),
    ).changed();

    if changed {
        scene.mat_dirty = true;
    }

    ui.add_space(6.0);
    if let Some(p) = &scene.tex_path {
        ui.label(
            egui::RichText::new(p.file_name().unwrap_or_default().to_string_lossy().as_ref())
                .weak().small(),
        );
    } else {
        ui.label(egui::RichText::new("No texture loaded").weak().small());
    }
    if ui.button("Load texture PNG …").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg", "webp"])
            .pick_file()
        {
            scene.tex_path  = Some(path);
            scene.mat_dirty = true;
        }
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "Tip: run Generate → Run pipeline to\nbuild a mandelbulb and load it here."
        )
        .weak().small(),
    );
}

// ── Load requests ─────────────────────────────────────────────────────────────

fn apply_load_requests(
    mut commands:  Commands,
    mut state:     ResMut<ViewerState>,
    mut orbit:     ResMut<orbit::Orbit>,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images:    ResMut<Assets<Image>>,
    scene:         Res<SceneState>,
    asset_server:  Res<AssetServer>,
) {
    if state.obj_pending {
        state.obj_pending = false;
        if let Some(path) = state.obj_path.clone() {
            if let Some(ent) = state.mesh_ent.take() { commands.entity(ent).despawn(); }
            match obj::load(&path) {
                Ok((positions, normals, indices)) => {
                    let (centered, radius) = center_and_radius(&positions);
                    let vtx = centered.len();
                    let tri = indices.len() / 3;

                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, centered);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
                    mesh.insert_indices(Indices::U32(indices));

                    // Build material from current SceneState — not hardcoded
                    let base_tex = scene.tex_path.as_ref().map(|p| {
                        asset_server.load(p.to_string_lossy().to_string())
                    });
                    let mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(
                            scene.base_color[0],
                            scene.base_color[1],
                            scene.base_color[2],
                        ),
                        base_color_texture:  base_tex,
                        metallic:            scene.metallic,
                        perceptual_roughness: scene.roughness,
                        double_sided:        true,
                        cull_mode:           None,
                        ..default()
                    });

                    state.mat_handle = Some(mat.clone());

                    let ent = commands.spawn((
                        PbrBundle {
                            mesh:     meshes.add(mesh),
                            material: mat,
                            ..default()
                        },
                        MeshPreview,
                    )).id();

                    state.mesh_ent   = Some(ent);
                    state.mesh_stats = Some((vtx, tri));
                    state.status     = format!("Loaded {vtx} verts, {tri} tris");
                    orbit.target     = Vec3::ZERO;
                    orbit.radius     = radius.max(1.0);
                }
                Err(e) => { state.status = format!("OBJ error: {e}"); }
            }
        }
    }

    if state.raw_pending {
        state.raw_pending = false;
        if let Some(path) = state.raw_path.clone() {
            if state.field_res == 0 {
                state.status = "Set resolution before loading.".into();
            } else {
                match field::load(&path, state.field_res) {
                    Ok(arr) => {
                        let handle  = images.add(Image::default());
                        state.field     = Some(arr);
                        state.tex       = Some(handle);
                        state.slice     = state.field_res / 2;
                        state.tex_dirty = true;
                        state.status    = format!("Loaded {}³ field", state.field_res);
                    }
                    Err(e) => { state.status = format!("Field error: {e}"); }
                }
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn center_and_radius(positions: &[[f32; 3]]) -> (Vec<[f32; 3]>, f32) {
    if positions.is_empty() { return (vec![], 1.0); }
    let mut mn = [f32::INFINITY; 3];
    let mut mx = [f32::NEG_INFINITY; 3];
    for p in positions {
        for i in 0..3 { mn[i] = mn[i].min(p[i]); mx[i] = mx[i].max(p[i]); }
    }
    let c      = [(mn[0]+mx[0])*0.5, (mn[1]+mx[1])*0.5, (mn[2]+mx[2])*0.5];
    let radius = (0..3).map(|i| mx[i]-mn[i]).fold(0.0f32, f32::max) * 0.75;
    let centered = positions.iter().map(|p| [p[0]-c[0], p[1]-c[1], p[2]-c[2]]).collect();
    (centered, radius)
}
