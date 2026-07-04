// THEATRE-GL: WGSL shader layer — GPU-native visual effects via Bevy Materials.
//
// Phase 4 ships a built-in "nebula swirl" WGSL shader as the mock effect,
// proving the pipeline:
//   GlMaterial (custom bind group) → GlUniforms updated each frame
//   → Bevy fragment shader pipeline → alpha-blended quad in the 3D scene
//
// The GL layer sits between the BV cubes (front) and P5 wave (behind),
// adding a deep crimson / violet nebula that responds to beat position.
//
// TODO(phase-4-glsl): accept user WGSL at runtime via:
//     let mut shaders = world.resource_mut::<Assets<Shader>>();
//     shaders.insert(&GL_LAYER_SHADER_HANDLE,
//         Shader::from_wgsl(user_code, "user://gl_sketch.wgsl"));
//   This replaces the shader for all active GlMaterial instances immediately.

use bevy::{
    pbr::{MaterialMeshBundle, MaterialPlugin},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
};
use myth_wire::ChannelId;

use crate::compositor::{TheatreMixer, TheatreState};

// ── Embedded WGSL shader ──────────────────────────────────────────────────────

/// Unique asset handle for the GL layer shader.
/// `weak_from_u128` creates a handle that never prevents the asset from unloading,
/// but since we insert it directly into `Assets<Shader>` it stays alive for the
/// lifetime of the app.
pub const GL_LAYER_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xB105_0A4B_3C2D_1E0F_6789_ABCD_EF01_2345_u128);

/// "Nebula swirl" — domain-warped FBM in crimson/violet/void palette.
///
/// Uniforms (group 2, binding 0):
///   time : f32  — monotonic seconds elapsed
///   beat : f32  — beat position [0, 1)  driven by TheatreState
const GL_LAYER_SHADER_SOURCE: &str = r#"
#import bevy_pbr::forward_io::VertexOutput

struct GlUniforms {
    time:  f32,
    beat:  f32,
    level: f32,   // channel fader level [0,1] — multiplied into final alpha
    _pad1: f32,
};

@group(2) @binding(0) var<uniform> uniforms: GlUniforms;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// 2-layer sine FBM.  Returns [0, 1].
fn fbm2(p: vec2<f32>, t: f32) -> f32 {
    var uv  = p;
    var f   = 0.0;
    var amp = 0.5;
    for (var i = 0; i < 4; i++) {
        f   += amp * (sin(uv.x * 3.1 + t * 0.28) * cos(uv.y * 2.7 - t * 0.19));
        uv   = uv * 1.72 + vec2<f32>(0.31, 0.17);
        amp *= 0.54;
    }
    return f * 0.5 + 0.5;
}

// ── Fragment ──────────────────────────────────────────────────────────────────

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // UV centred on [−0.5, 0.5]
    let uv = in.uv - vec2<f32>(0.5, 0.5);
    let t  = uniforms.time;
    let bt = uniforms.beat;

    // Two-pass domain warp — displaces sample coordinates so the FBM
    // develops organic tendrils rather than banded stripes.
    let w1 = vec2<f32>(
        sin(uv.y * 2.3 + t * 0.41) * 0.28,
        cos(uv.x * 1.9 - t * 0.33) * 0.28,
    );
    let w2 = vec2<f32>(
        cos(uv.y * 3.1 - t * 0.22) * 0.14,
        sin(uv.x * 2.5 + t * 0.17) * 0.14,
    );
    let wp = uv + w1 + w2;

    // Two FBM samples offset in both space and time for variety.
    let n1 = fbm2(wp * 2.4, t);
    let n2 = fbm2(wp * 1.7 + vec2<f32>(4.3, -2.1), -t * 0.6);
    let n  = n1 * 0.65 + n2 * 0.35;

    // ── Palette: near-void → deep violet → crimson flare ─────────────────────
    let void_col    = vec3<f32>(0.03, 0.01, 0.06);   // background void
    let violet_col  = vec3<f32>(0.28, 0.04, 0.42);   // nebula body
    let crimson_col = vec3<f32>(0.72, 0.06, 0.18);   // hot core filaments

    var color = mix(void_col, violet_col, n);
    color     = mix(color, crimson_col, n * n * 1.1);

    // Central hot glow — white-orange bloom at canvas centre.
    // Beat modulates its intensity so it pulses on each hit.
    let dist = length(uv) * 2.0;
    let glow = exp(-dist * dist * 3.5) * (0.20 + bt * 0.18);
    color   += vec3<f32>(0.95, 0.42, 0.12) * glow;

    // Rim darkening — fade edges into the void so the quad boundary is invisible.
    let rim  = 1.0 - smoothstep(0.32, 0.72, length(uv));
    color   *= rim * 0.85 + 0.15;

    // Alpha: opaque in bright nebula regions, transparent in the void.
    // Multiplied by the channel fader level so the mixer board controls intensity.
    let alpha = clamp(n * 1.35 + glow * 2.6, 0.0, 1.0) * 0.78 * uniforms.level;

    return vec4<f32>(color, alpha);
}
"#;

// ── Uniforms ──────────────────────────────────────────────────────────────────

/// GPU-side uniform block for the GL layer.
///
/// Must be 16-byte aligned (ShaderType enforces this via encase).
/// Padding fields fill the 16-byte block.
///
/// Fields are written from Rust and consumed by the WGSL shader on the GPU.
/// The dead_code lint can't see GPU reads, so we suppress it.
#[allow(dead_code)]
#[derive(ShaderType, Clone, Default, Debug)]
pub struct GlUniforms {
    pub time:  f32,
    pub beat:  f32,
    pub level: f32,   // fader level — passed from TheatreMixer each frame
    pub _pad1: f32,
}

// ── GlMaterial ────────────────────────────────────────────────────────────────

/// Custom Bevy Material that forwards `GlUniforms` to the fragment shader.
///
/// `AsBindGroup` maps `uniforms` to bind group 2, binding 0.
/// `Material` binds it to Bevy's PBR forward pass.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct GlMaterial {
    #[uniform(0)]
    pub uniforms: GlUniforms,
}

impl Material for GlMaterial {
    fn fragment_shader() -> ShaderRef {
        GL_LAYER_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

// ── Resources ─────────────────────────────────────────────────────────────────

/// Config for one GL quad — mirrors the pattern in `texture_quad.rs`.
pub struct GlQuadConfig {
    pub channel_id: ChannelId,
    /// World-space z position. Negative = further from camera.
    pub z_depth: f32,
}

/// Configs queued before `App::run()`, consumed by the Startup system.
#[derive(Resource, Default)]
pub struct PendingGlQuads(pub Vec<GlQuadConfig>);

/// Marker on each spawned GL quad mesh entity.
#[derive(Component)]
pub struct GlQuad {
    pub channel_id: ChannelId,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct GlLayerPlugin;

impl Plugin for GlLayerPlugin {
    fn build(&self, app: &mut App) {
        // Register the embedded WGSL shader so GlMaterial can reference it
        // by handle without any file-system access.
        {
            let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
            shaders.insert(
                &GL_LAYER_SHADER_HANDLE,
                Shader::from_wgsl(GL_LAYER_SHADER_SOURCE, "embedded://gl_layer.wgsl"),
            );
        }

        app.add_plugins(MaterialPlugin::<GlMaterial>::default())
            .init_resource::<PendingGlQuads>()
            .add_systems(Startup, spawn_pending_gl_quads)
            .add_systems(Update, (update_gl_uniforms, sync_gl_visibility));
    }
}

// ── Startup ───────────────────────────────────────────────────────────────────

/// Consume `PendingGlQuads` and spawn one `MaterialMeshBundle::<GlMaterial>` per config.
///
/// Quad sizing uses the same FOV math as `texture_quad.rs`:
///   Camera at (0, 1.5, 4.5), look-at (0, 0, 0), vertical FOV 60°.
///   tan(30°) ≈ 0.5774.  At depth d from camera: h = 2d·tan(30°), w = h·(16/9).
fn spawn_pending_gl_quads(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
    pending: Res<PendingGlQuads>,
) {
    for cfg in &pending.0 {
        let dist   = 4.5 - cfg.z_depth;   // camera_z - quad_z
        let quad_h = 2.0 * dist * 0.5774;
        let quad_w = quad_h * (16.0 / 9.0);

        let mat = materials.add(GlMaterial {
            uniforms: GlUniforms::default(),
        });

        commands.spawn((
            MaterialMeshBundle::<GlMaterial> {
                mesh:      meshes.add(Rectangle::new(quad_w, quad_h)),
                material:  mat,
                // Shift the quad up to match the camera look-at (same correction as the
                // WebView quads — camera Y=1.5 so the look-at sits above world origin).
                transform: Transform::from_xyz(0.0, 0.75, cfg.z_depth),
                ..default()
            },
            GlQuad { channel_id: cfg.channel_id },
        ));
    }
}

// ── Update systems ────────────────────────────────────────────────────────────

/// Push time, beat, and channel level to every active GlMaterial.
///
/// `level` is read from `TheatreMixer` so the egui fader slider directly
/// fades the nebula alpha — previously the fader could only toggle visibility.
fn update_gl_uniforms(
    state:     Res<TheatreState>,
    time:      Res<Time>,
    mixer:     Res<TheatreMixer>,
    query:     Query<(&GlQuad, &Handle<GlMaterial>)>,
    mut mats:  ResMut<Assets<GlMaterial>>,
) {
    let elapsed = time.elapsed_seconds();
    let beat    = state.beat;

    for (quad, mat_handle) in query.iter() {
        if let Some(mat) = mats.get_mut(mat_handle) {
            mat.uniforms.time  = elapsed;
            mat.uniforms.beat  = beat;
            mat.uniforms.level = mixer.0.channel(quad.channel_id)
                .map(|ch| if ch.muted { 0.0 } else { ch.level })
                .unwrap_or(1.0);
        }
    }
}

/// Show/hide GL quads based on their channel's mute + level state.
///
/// Only runs when `TheatreMixer` is marked changed — identical guard
/// to `sync_quad_visibility` in `texture_quad.rs`.
fn sync_gl_visibility(
    mixer: Res<TheatreMixer>,
    mut query: Query<(&GlQuad, &mut Visibility)>,
) {
    if !mixer.is_changed() {
        return;
    }
    for (quad, mut vis) in query.iter_mut() {
        *vis = match mixer.0.channel(quad.channel_id) {
            Some(ch) if !ch.muted && ch.level > 0.0 => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }
}
