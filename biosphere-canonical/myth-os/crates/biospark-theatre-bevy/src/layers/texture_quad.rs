// THEATRE-QUAD: Textured quad mesh for displaying WebView layer frames in the 3D scene.
//
// Each P5 or HT channel gets one quad mesh. The quad is:
//   - Sized to fill the camera's view at its world-space z depth
//   - Lit with unlit=true so the texture displays exactly as captured
//   - Alpha blended so the BV scene shows through transparent areas
//   - Positioned at a z_depth matched to the channel's z_order (back → front)
//
// Camera assumptions (Phase 2 setup):
//   Position: (0, 1.5, 4.5)  Look-at: (0, 0, 0)  Vertical FOV: 60°
//
// Frame upload: `upload_webview_frames` runs every Update tick.
// It checks all registered FrameBuffers, takes any pending frame,
// and writes the RGBA bytes directly into the Bevy Image asset.

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use myth_wire::ChannelId;

use crate::compositor::TheatreMixer;
use super::frame_buffer::FrameBuffer;

// ── Resources ─────────────────────────────────────────────────────────────────

/// Per-channel entry managed by the upload system.
pub struct WebViewEntry {
    pub channel_id: ChannelId,
    pub frame_buffer: FrameBuffer,
    pub image_handle: Handle<Image>,
}

/// All active WebView channel entries. The upload system reads this every tick.
#[derive(Resource, Default)]
pub struct WebViewRegistry {
    pub entries: Vec<WebViewEntry>,
}

/// Configs queued before the App starts, consumed by the Startup system.
#[derive(Resource, Default)]
pub struct PendingWebViewQuads(pub Vec<QuadConfig>);

pub struct QuadConfig {
    pub channel_id: ChannelId,
    pub frame_buffer: FrameBuffer,
    /// World-space z position. Negative = further from camera.
    /// Match to channel z_order: lower z_order → more negative depth (further back).
    pub z_depth: f32,
}

// ── Components ────────────────────────────────────────────────────────────────

/// Marks a quad mesh entity that displays a WebView layer's texture.
#[derive(Component)]
pub struct WebViewQuad {
    pub channel_id: ChannelId,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TextureQuadPlugin;

impl Plugin for TextureQuadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WebViewRegistry>()
            .init_resource::<PendingWebViewQuads>()
            .add_systems(Startup, spawn_pending_quads)
            .add_systems(Update, (upload_webview_frames, sync_quad_visibility));
    }
}

// ── Startup ───────────────────────────────────────────────────────────────────

fn spawn_pending_quads(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut registry: ResMut<WebViewRegistry>,
    pending: Res<PendingWebViewQuads>,
) {
    for cfg in &pending.0 {
        spawn_quad(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            &mut registry,
            cfg,
        );
    }
}

fn spawn_quad(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    registry: &mut WebViewRegistry,
    cfg: &QuadConfig,
) {
    let fb = &cfg.frame_buffer;

    // Create a dynamic Image asset filled with transparent black.
    // MAIN_WORLD | RENDER_WORLD so the CPU can update it each frame.
    let image = Image::new_fill(
        Extent3d {
            width: fb.width,
            height: fb.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let image_handle = images.add(image);

    // Register for upload system
    registry.entries.push(WebViewEntry {
        channel_id: cfg.channel_id,
        frame_buffer: cfg.frame_buffer.clone(),
        image_handle: image_handle.clone(),
    });

    // Size the quad to exactly fill the camera view at the given z_depth.
    // Camera at z=4.5, FOV 60° vertical → tan(30°) ≈ 0.5774.
    // At depth d from camera: height = 2 * d * tan(30°), width = height * aspect.
    let dist_to_cam = 4.5 - cfg.z_depth; // positive when behind camera origin
    let quad_h = 2.0 * dist_to_cam * 0.5774;
    let quad_w = quad_h * (16.0 / 9.0);

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Rectangle::new(quad_w, quad_h)),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(image_handle),
                alpha_mode: AlphaMode::Blend,
                unlit: true,      // no lighting — display texture as-is
                double_sided: true,
                cull_mode: None,
                ..default()
            }),
            // Offset the quad upward to match the camera's look-at point (0, 0, 0)
            // which is not at screen center since camera Y = 1.5.
            // Small correction: shift quad up by 0.75 (half camera Y offset).
            transform: Transform::from_xyz(0.0, 0.75, cfg.z_depth),
            ..default()
        },
        WebViewQuad {
            channel_id: cfg.channel_id,
        },
    ));
}

// ── Update systems ────────────────────────────────────────────────────────────

/// Upload any pending frames from WebView threads to their Bevy Image assets.
/// Runs every Update tick. Reads are non-blocking (Mutex::try_lock would be ideal,
/// but lock() is fine here since the writer is a sleeping thread at 30fps).
pub fn upload_webview_frames(
    registry: Res<WebViewRegistry>,
    mut images: ResMut<Assets<Image>>,
) {
    for entry in &registry.entries {
        if let Some(rgba) = entry.frame_buffer.take() {
            if let Some(image) = images.get_mut(&entry.image_handle) {
                image.data = rgba;
            }
        }
    }
}

/// Show/hide WebView quads based on channel mute + level in the Channel Mixer.
fn sync_quad_visibility(
    mixer: Res<TheatreMixer>,
    mut query: Query<(&WebViewQuad, &mut Visibility)>,
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
