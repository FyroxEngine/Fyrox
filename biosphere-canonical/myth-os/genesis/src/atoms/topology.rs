// GEN-ATOM-02: Procedural Topology Generator
//
// Builds a 100×100 heightmap mesh from inline multi-octave hash noise.
// `TerrainEntity` marker lets the MIDI-sync system scale it via Transform.scale.y.

use bevy::{
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
    render::render_asset::RenderAssetUsages,
};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct TopologyConfig {
    pub seed:         u64,
    pub width:        u32,   // quads along X
    pub depth:        u32,   // quads along Z
    pub height_scale: f32,   // peak height in world units
}

impl Default for TopologyConfig {
    fn default() -> Self {
        Self {
            seed:         42,
            width:        100,
            depth:        100,
            height_scale: 8.0,
        }
    }
}

/// Marks the terrain mesh entity so the MIDI system can locate it.
#[derive(Component)]
pub struct TerrainEntity;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TopologyPlugin;

impl Plugin for TopologyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TopologyConfig>()
            .add_systems(Startup, generate_terrain);
    }
}

// ── Startup system ────────────────────────────────────────────────────────────

fn generate_terrain(
    mut commands:  Commands,
    cfg:           Res<TopologyConfig>,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = build_heightmap(&cfg);

    commands.spawn((
        PbrBundle {
            mesh:     meshes.add(mesh),
            material: materials.add(StandardMaterial {
                base_color:          Color::srgb(0.22, 0.38, 0.16),
                perceptual_roughness: 0.92,
                metallic:            0.0,
                ..Default::default()
            }),
            ..Default::default()
        },
        TerrainEntity,
        Name::new("Terrain"),
    ));

    info!(
        seed  = cfg.seed,
        width = cfg.width,
        depth = cfg.depth,
        h     = cfg.height_scale,
        "Terrain heightmap generated ({}×{} quads)", cfg.width, cfg.depth
    );
}

// ── Mesh builder ──────────────────────────────────────────────────────────────

fn build_heightmap(cfg: &TopologyConfig) -> Mesh {
    let w  = cfg.width  as usize;
    let d  = cfg.depth  as usize;
    let wv = w + 1; // vertex columns
    let dv = d + 1; // vertex rows
    let hw = cfg.width  as f32 * 0.5;
    let hd = cfg.depth  as f32 * 0.5;

    // Height sampler — clamped grid coords → world Y
    let h = |xi: i32, zi: i32| -> f32 {
        let xi = xi.clamp(0, cfg.width  as i32);
        let zi = zi.clamp(0, cfg.depth  as i32);
        let x  = xi as f32 / cfg.width  as f32 * 6.0; // noise UV scale
        let z  = zi as f32 / cfg.depth  as f32 * 6.0;
        fbm(x, z, cfg.seed) * cfg.height_scale
    };

    let mut positions = Vec::<[f32; 3]>::with_capacity(wv * dv);
    let mut normals   = Vec::<[f32; 3]>::with_capacity(wv * dv);
    let mut uvs       = Vec::<[f32; 2]>::with_capacity(wv * dv);

    for zi in 0..dv {
        for xi in 0..wv {
            let xf = xi as f32 - hw;
            let zf = zi as f32 - hd;
            let y  = h(xi as i32, zi as i32);
            positions.push([xf, y, zf]);
            uvs.push([xi as f32 / w as f32, zi as f32 / d as f32]);

            // Surface normal via finite differences (one cell = 1 world unit)
            let hl = h(xi as i32 - 1, zi as i32);
            let hr = h(xi as i32 + 1, zi as i32);
            let hb = h(xi as i32, zi as i32 - 1);
            let hf = h(xi as i32, zi as i32 + 1);
            let n  = Vec3::new(hl - hr, 2.0, hb - hf).normalize();
            normals.push([n.x, n.y, n.z]);
        }
    }

    // Two CCW-winding triangles per quad
    let mut indices = Vec::<u32>::with_capacity(w * d * 6);
    for z in 0..d {
        for x in 0..w {
            let i0 = (z * wv + x) as u32;
            let i1 = i0 + 1;
            let i2 = ((z + 1) * wv + x) as u32;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

// ── Inline noise ──────────────────────────────────────────────────────────────
// No external crate — pure integer hash → smooth value noise → fBm.

fn hash2(x: i32, y: i32, seed: u64) -> f32 {
    // Murmur-style finalizer on two 32-bit integers + seed
    let mut v: u64 = (x as u64 ^ 0xdeadbeef_u64)
        .wrapping_mul(2_246_822_519)
        .wrapping_add((y as u64 ^ 0xcafe1234_u64).wrapping_mul(3_266_489_917))
        .wrapping_add(seed.wrapping_mul(6_364_136_223_846_793_005));
    v ^= v >> 33;
    v  = v.wrapping_mul(0xff51afd7ed558ccd);
    v ^= v >> 33;
    v  = v.wrapping_mul(0xc4ceb9fe1a85ec53);
    v ^= v >> 33;
    // Map top 53 bits to [0, 1)
    (v >> 11) as f32 / (1u64 << 53) as f32
}

#[inline]
fn smooth(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

/// Bilinear value noise at (x, z).
fn vnoise(x: f32, z: f32, seed: u64) -> f32 {
    let xi = x.floor() as i32;
    let zi = z.floor() as i32;
    let xf = smooth(x - x.floor());
    let zf = smooth(z - z.floor());
    lerp(
        lerp(hash2(xi,     zi,     seed), hash2(xi + 1, zi,     seed), xf),
        lerp(hash2(xi,     zi + 1, seed), hash2(xi + 1, zi + 1, seed), xf),
        zf,
    )
}

/// 5-octave fractional Brownian motion.
fn fbm(x: f32, z: f32, seed: u64) -> f32 {
    let (mut val, mut amp, mut frq, mut max) = (0.0_f32, 0.6_f32, 1.0_f32, 0.0_f32);
    for i in 0u64..5 {
        val += vnoise(x * frq, z * frq, seed.wrapping_add(i.wrapping_mul(123_456_789))) * amp;
        max += amp;
        amp *= 0.5;
        frq *= 2.0;
    }
    val / max
}
